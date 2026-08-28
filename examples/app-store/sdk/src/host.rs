//! The guest's side of the request/response channel.
//!
//! [`request`] hands the host a [`Request`] through the next frame and returns
//! a future that resolves when the matching [`Event::Response`] arrives. The
//! guest never blocks: the driver polls the app's tasks on every tick, so a
//! task awaiting an answer simply stays pending until the host delivers it.
//!
//! [`Event::Response`]: crate::frame::Event::Response

use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use crate::frame::Request;

#[derive(Default)]
struct Slot {
    answer: Option<Vec<u8>>,
    waker: Option<Waker>,
}

#[derive(Default)]
struct Registry {
    next_id: u64,
    outbox: Vec<Request>,
    pending: HashMap<u64, Arc<Mutex<Slot>>>,
}

thread_local! {
    // One registry per thread = one per driver: a wasm module has one
    // thread, and every native test drives its own app on its own thread.
    static REGISTRY: RefCell<Registry> = RefCell::default();
}

/// Asks the host for something. `kind` names the request; the host decides
/// what it means and when to answer.
pub fn request(kind: &str, payload: &[u8]) -> Response {
    let slot = Arc::new(Mutex::new(Slot::default()));
    REGISTRY.with_borrow_mut(|registry| {
        let id = registry.next_id;
        registry.next_id += 1;
        registry.pending.insert(id, slot.clone());
        registry.outbox.push(Request {
            id,
            kind: kind.to_string(),
            payload: payload.to_vec(),
        });
    });
    Response { slot }
}

/// The host's eventual answer.
pub struct Response {
    slot: Arc<Mutex<Slot>>,
}

impl Future for Response {
    type Output = Vec<u8>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Vec<u8>> {
        let mut slot = self.slot.lock().expect("response slot");
        match slot.answer.take() {
            Some(answer) => Poll::Ready(answer),
            None => {
                slot.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// Everything asked since the last frame, in order.
pub(crate) fn drain_outbox() -> Vec<Request> {
    REGISTRY.with_borrow_mut(|registry| std::mem::take(&mut registry.outbox))
}

/// Delivers the host's answer; an id nobody waits for is dropped.
pub(crate) fn fulfill(id: u64, payload: Vec<u8>) {
    let slot = REGISTRY.with_borrow_mut(|registry| registry.pending.remove(&id));
    if let Some(slot) = slot {
        let mut slot = slot.lock().expect("response slot");
        slot.answer = Some(payload);
        if let Some(waker) = slot.waker.take() {
            waker.wake();
        }
    }
}
