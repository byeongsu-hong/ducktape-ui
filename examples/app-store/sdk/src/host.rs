//! The guest's side of the request/response channel.
//!
//! [`request`] asks once and resolves on the first answer; [`subscribe`]
//! asks once and yields every answer until the host closes the stream. Both
//! hand the host a [`Request`] through the next frame. The guest never
//! blocks: the driver polls the app's tasks on every tick, so a task waiting
//! on the host simply stays pending until a response event arrives.
//!
//! A request's `kind` is `<capability>.<operation>`. The host refuses a
//! capability the app's manifest did not declare, and the refusal arrives
//! as the `Err` of the answer — an ordinary error the app's handler routes.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use iced::futures::Stream;

use crate::frame::Request;

/// What one answer carries; the host's refusal is the `Err`.
pub type Answer = Result<Vec<u8>, String>;

#[derive(Default)]
struct Slot {
    answers: VecDeque<Answer>,
    closed: bool,
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

fn open(kind: &str, payload: &[u8]) -> Arc<Mutex<Slot>> {
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
    slot
}

/// Asks the host for one thing.
pub fn request(kind: &str, payload: &[u8]) -> Response {
    Response {
        slot: open(kind, payload),
    }
}

/// Asks the host for a stream of things — timer ticks, bus messages.
pub fn subscribe(kind: &str, payload: &[u8]) -> Subscription {
    Subscription {
        slot: open(kind, payload),
    }
}

/// The host's eventual answer to a [`request`].
pub struct Response {
    slot: Arc<Mutex<Slot>>,
}

impl Future for Response {
    type Output = Answer;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Answer> {
        let mut slot = self.slot.lock().expect("response slot");
        match slot.answers.pop_front() {
            Some(answer) => Poll::Ready(answer),
            None if slot.closed => Poll::Ready(Err("the host closed the request".into())),
            None => {
                slot.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// Every answer the host sends to a [`subscribe`], until it closes.
pub struct Subscription {
    slot: Arc<Mutex<Slot>>,
}

impl Stream for Subscription {
    type Item = Answer;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Answer>> {
        let mut slot = self.slot.lock().expect("subscription slot");
        match slot.answers.pop_front() {
            Some(answer) => Poll::Ready(Some(answer)),
            None if slot.closed => Poll::Ready(None),
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

/// Delivers one answer; an id nobody waits for is dropped.
pub(crate) fn fulfill(id: u64, answer: Answer, done: bool) {
    let slot = REGISTRY.with_borrow_mut(|registry| {
        if done {
            registry.pending.remove(&id)
        } else {
            registry.pending.get(&id).cloned()
        }
    });
    if let Some(slot) = slot {
        let mut slot = slot.lock().expect("answer slot");
        slot.answers.push_back(answer);
        slot.closed |= done;
        if let Some(waker) = slot.waker.take() {
            waker.wake();
        }
    }
}
