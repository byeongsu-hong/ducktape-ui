//! Guest-local editor decisions and post-acceptance authored event routes.
use crate::{Editor, slots, wire};
use std::rc::Rc;

pub use wire::{EditorDecision, EditorKeyRequest, EditorTransactionEvent};

pub struct EditorBinding<P> {
    claims: Vec<wire::EditorKeyClaim>,
    decide: Rc<dyn Fn(EditorKeyRequest) -> EditorDecision>,
    on_event: Rc<dyn Fn(EditorTransactionEvent) -> Option<P>>,
}
impl<P: 'static> EditorBinding<P> {
    pub fn new(
        claims: Vec<wire::EditorKeyClaim>,
        decide: impl Fn(EditorKeyRequest) -> EditorDecision + 'static,
        on_event: impl Fn(EditorTransactionEvent) -> Option<P> + 'static,
    ) -> Self {
        assert!(
            claims.len() <= wire::editor_transaction::MAX_EDITOR_CLAIMS,
            "editor claim limit"
        );
        Self {
            claims,
            decide: Rc::new(decide),
            on_event: Rc::new(on_event),
        }
    }
    pub fn register<M: 'static>(
        self,
        route: impl Fn(P) -> M + 'static,
        wrap: impl Fn(EditorTransaction<M>) -> M + 'static,
    ) -> wire::EditorBinding {
        let decide = self.decide;
        let on_request = slots::handler::<EditorKeyRequest, M>(Box::new(move |request| {
            let decision = decide(request.clone());
            slots::editor_response(wire::EditorResponse {
                id: request.id,
                decision,
            });
            None
        }));
        let map = slots::handler::<EditorTransactionEvent, M>(Box::new(move |event| {
            (self.on_event)(event).map(&route)
        }));
        let on_event = slots::handler::<EditorTransactionEvent, M>(Box::new(move |event| {
            Some(wrap(EditorTransaction {
                event,
                map,
                message: std::marker::PhantomData,
            }))
        }));
        wire::EditorBinding {
            claims: self.claims,
            on_request,
            on_event,
        }
    }
}

pub struct EditorTransaction<M> {
    event: EditorTransactionEvent,
    map: u32,
    message: std::marker::PhantomData<fn() -> M>,
}
impl<M> Clone for EditorTransaction<M> {
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone(),
            map: self.map,
            message: std::marker::PhantomData,
        }
    }
}
impl<M> std::fmt::Debug for EditorTransaction<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EditorTransaction")
            .field(&self.event)
            .finish()
    }
}
impl<M: 'static> EditorTransaction<M> {
    pub fn apply(self, editor: &mut Editor) -> Option<M> {
        let (id, state, commit) = match &self.event {
            EditorTransactionEvent::Commit { id, after, .. } => (id, after, true),
            EditorTransactionEvent::Fault { id, state, .. }
            | EditorTransactionEvent::Cancelled { id, state } => (id, state, false),
        };
        if !slots::editor_matches_pending(id)
            || state.text.len() > wire::MAX_STRING_BYTES
            || id.reset != state.reset
            || state.reset != editor.reset_revision()
            || state.revision < editor.observation_revision()
            || (commit && state.revision == editor.observation_revision())
        {
            return None;
        }
        if !slots::has_handler::<EditorTransactionEvent, M>(self.map) {
            return None;
        }
        editor.accept(state.clone());
        let mapped = slots::run_handler::<EditorTransactionEvent, M>(self.map, self.event.clone());
        slots::editor_acknowledge(&self.event);
        mapped
    }
}

#[cfg(test)]
mod retry_tests {
    use super::*;

    #[test]
    fn an_old_retry_cannot_commit_over_the_current_pending_attempt() {
        let context = slots::Context::default();
        let _entered = context.enter();
        let mut editor = Editor::new("before");
        let current = wire::EditorTransactionId {
            instance: 1,
            document: "app:draft".into(),
            reset: 0,
            sequence: 7,
            attempt: 2,
            text_revision: 0,
            revision: 0,
        };
        slots::editor_response(wire::EditorResponse {
            id: current.clone(),
            decision: wire::EditorDecision::Noop,
        });
        let calls = Rc::new(std::cell::Cell::new(0));
        let counted = calls.clone();
        let map = slots::handler::<EditorTransactionEvent, ()>(Box::new(move |_| {
            counted.set(counted.get() + 1);
            None
        }));
        let transaction = |id, text: &str| EditorTransaction::<()> {
            event: EditorTransactionEvent::Commit {
                id,
                before: wire::EditorState {
                    text: "before".into(),
                    ..Default::default()
                },
                after: wire::EditorState {
                    text: text.into(),
                    revision: 1,
                    ..Default::default()
                },
                kind: wire::EditorEditKind::GuestPatch,
                history: wire::EditorHistoryEffect::NewGroup,
                input_time_ms: 1,
            },
            map,
            message: std::marker::PhantomData,
        };
        let mut old = current.clone();
        old.attempt = 1;
        transaction(old, "stale").apply(&mut editor);
        assert_eq!(
            editor.text(),
            "before",
            "an old attempt must not replace document state"
        );
        assert_eq!(
            calls.get(),
            0,
            "stale retry must not run the history reducer"
        );
        assert!(
            slots::editor_pending(),
            "current attempt remains outstanding"
        );
        let valid = transaction(current, "accepted");
        valid.clone().apply(&mut editor);
        assert_eq!(editor.text(), "accepted");
        assert_eq!(calls.get(), 1);
        assert!(!slots::editor_pending());
        valid.apply(&mut editor);
        assert_eq!(
            calls.get(),
            1,
            "duplicate accepted commit does not repeat history"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_message_envelope_is_send_and_stale_commit_cannot_acknowledge() {
        fn is_send<T: Send>() {}
        is_send::<EditorTransaction<()>>();
        let context = slots::Context::default();
        let _entered = context.enter();
        let mut editor = Editor::new("before");
        let id = wire::EditorTransactionId {
            instance: 1,
            document: "app:draft".into(),
            reset: 0,
            sequence: 1,
            attempt: 1,
            text_revision: 0,
            revision: 0,
        };
        slots::editor_response(wire::EditorResponse {
            id: id.clone(),
            decision: wire::EditorDecision::Noop,
        });
        let mapped = Rc::new(std::cell::Cell::new(0));
        let counted = mapped.clone();
        let map = slots::handler::<wire::EditorTransactionEvent, ()>(Box::new(move |_| {
            counted.set(counted.get() + 1);
            None
        }));
        let before = wire::EditorState {
            text: "before".into(),
            ..Default::default()
        };
        let mut after = before.clone();
        after.text = "after".into();
        after.revision = 1;
        after.reset = 99;
        let event = |after| wire::EditorTransactionEvent::Commit {
            id: id.clone(),
            before: before.clone(),
            after,
            kind: wire::EditorEditKind::GuestPatch,
            history: wire::EditorHistoryEffect::NewGroup,
            input_time_ms: 1,
        };
        let tx = |event| EditorTransaction::<()> {
            event,
            map,
            message: std::marker::PhantomData,
        };
        tx(event(after.clone())).apply(&mut editor);
        assert!(slots::editor_pending());
        assert_eq!(mapped.get(), 0);
        after.reset = 0;
        let valid = tx(event(after));
        valid.clone().apply(&mut editor);
        assert_eq!(editor.text(), "after");
        assert_eq!(mapped.get(), 1);
        assert!(!slots::editor_pending());
        valid.apply(&mut editor);
        assert_eq!(mapped.get(), 1, "duplicate commit does not re-run history");
    }
}
