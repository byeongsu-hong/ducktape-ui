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
        if id.reset != state.reset
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
