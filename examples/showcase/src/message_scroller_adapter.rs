use std::sync::{Arc, Mutex};

use iced::Task;
use ui_lang_components::ui::message_scroller::{MessageScrollerEvent, MessageScrollerState};

/// Assign `state` immediately, then route the effects back through the current
/// state. Returning deferred state snapshots loses simultaneous widget events.
#[derive(Clone)]
pub struct MessageScrollerTransition {
    pub state: MessageScrollerState,
    effects: Arc<Mutex<Option<Task<MessageScrollerEvent>>>>,
}

pub fn message_scroller_apply(
    mut state: MessageScrollerState,
    event: MessageScrollerEvent,
) -> MessageScrollerTransition {
    let effects = state.update(event);
    MessageScrollerTransition {
        state,
        effects: Arc::new(Mutex::new(Some(effects))),
    }
}

pub fn message_scroller_effects(
    transition: MessageScrollerTransition,
) -> Task<MessageScrollerEvent> {
    // Ice values are cloneable; all copies share this one task consumption.
    transition
        .effects
        .lock()
        .expect("scroller effects lock")
        .take()
        .unwrap_or_else(Task::none)
}
