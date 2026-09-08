//! Event routes and sent pictures belong to one running driver.
use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Default)]
struct Tables {
    editor_responses: Vec<crate::wire::EditorResponse>,
    editor_documents: Vec<crate::wire::editor_document::EditorDocumentMessage>,
    editor_sender: Option<crate::wire::editor_document::EditorTransferSender>,
    editor_pending: Vec<crate::wire::EditorTransactionId>,
    macos: bool,
    mouse_interest: bool,
    deferred: Vec<Box<dyn Any>>,
    messages: Vec<Rc<dyn Any>>,
    handlers: Vec<Rc<dyn Any>>,
    pictures: HashSet<(bool, u64)>,
    cached_messages: HashMap<u32, Rc<dyn Any>>,
    cached_handlers: HashMap<u32, Rc<dyn Any>>,
    next_cached: u32,
    captures: Vec<SavedRoutes>,
    memo: Rc<RefCell<crate::memo::Cache>>,
}

const CACHED: u32 = 1 << 31;

/// The callables owned by one cached subtree, including nested cache hits.
#[derive(Clone, Default)]
pub(crate) struct SavedRoutes {
    messages: Vec<(u32, Rc<dyn Any>)>,
    handlers: Vec<(u32, Rc<dyn Any>)>,
}

impl SavedRoutes {
    pub(crate) fn restore(&self) {
        let tables = tables();
        let mut tables = tables.borrow_mut();
        tables.cached_messages.extend(self.messages.iter().cloned());
        tables.cached_handlers.extend(self.handlers.iter().cloned());
        for capture in &mut tables.captures {
            capture.messages.extend(self.messages.iter().cloned());
            capture.handlers.extend(self.handlers.iter().cloned());
        }
    }
}

struct Capture {
    tables: Rc<RefCell<Tables>>,
    depth: usize,
    finished: bool,
}

impl Drop for Capture {
    fn drop(&mut self) {
        if !self.finished {
            let abandoned = self.tables.borrow_mut().captures.split_off(self.depth);
            drop(abandoned);
        }
    }
}

pub(crate) fn capture<R>(build: impl FnOnce() -> R) -> (R, SavedRoutes) {
    let tables = tables();
    let depth = tables.borrow().captures.len();
    tables.borrow_mut().captures.push(SavedRoutes::default());
    let mut guard = Capture {
        tables,
        depth,
        finished: false,
    };
    let result = build();
    let routes = guard
        .tables
        .borrow_mut()
        .captures
        .pop()
        .expect("active route capture");
    guard.finished = true;
    (result, routes)
}

impl Tables {
    fn cached_id(&mut self) -> u32 {
        assert!(
            self.next_cached < CACHED,
            "cached route identifiers exhausted"
        );
        let id = CACHED | self.next_cached;
        self.next_cached += 1;
        id
    }
}

#[derive(Clone, Default)]
pub(crate) struct Context(Rc<RefCell<Tables>>);

thread_local! {
    static CURRENT: RefCell<Context> = RefCell::new(Context::default());
}

impl Context {
    pub(crate) fn with_macos(macos: bool) -> Self {
        Self(Rc::new(RefCell::new(Tables {
            macos,
            ..Tables::default()
        })))
    }

    pub(crate) fn enter(&self) -> Guard {
        Guard(CURRENT.with(|current| current.replace(self.clone())))
    }
}

pub(crate) struct Guard(Context);
impl Drop for Guard {
    fn drop(&mut self) {
        CURRENT.with(|current| {
            current.replace(self.0.clone());
        });
    }
}

fn tables() -> Rc<RefCell<Tables>> {
    CURRENT.with_borrow(|current| current.0.clone())
}

/// First-render component messages run on the next driver tick, before input.
pub fn defer<M: 'static>(messages: Vec<M>) {
    tables().borrow_mut().deferred.extend(
        messages
            .into_iter()
            .map(|message| Box::new(message) as Box<dyn Any>),
    );
}

pub(crate) fn take_deferred<M: 'static>() -> Vec<M> {
    let messages = std::mem::take(&mut tables().borrow_mut().deferred);
    messages
        .into_iter()
        .map(|message| {
            *message
                .downcast::<M>()
                .expect("deferred message belongs to the active driver")
        })
        .collect()
}

pub(crate) fn has_deferred() -> bool {
    !tables().borrow().deferred.is_empty()
}

pub(crate) fn memo_cache() -> Rc<RefCell<crate::memo::Cache>> {
    tables().borrow().memo.clone()
}

/// Returns a picture hash and its bytes the first time this driver sends it.
pub fn picture(bytes: impl AsRef<[u8]>) -> (u64, Option<Vec<u8>>) {
    use std::hash::{Hash, Hasher};
    let bytes = bytes.as_ref();
    let mut hasher = std::hash::DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    let first = tables().borrow_mut().pictures.insert((false, hash));
    (hash, first.then(|| bytes.to_vec()))
}

/// Copies an in-memory raster once per driver. A path never reaches the host.
pub fn image(
    handle: &iced::advanced::image::Handle,
) -> Option<(u64, Option<ui_lang_wire::ImageData>)> {
    use iced::advanced::image::Handle;
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    match handle {
        Handle::Path(..) => {
            crate::host::log(
                "Tree image refused: runtime path handles are unsupported; use embedded assets, encoded(bytes), or rgba(width, height, bytes)",
            );
            return None;
        }
        Handle::Bytes(_, bytes) => {
            0u8.hash(&mut hasher);
            bytes.as_ref().hash(&mut hasher);
        }
        Handle::Rgba {
            width,
            height,
            pixels,
            ..
        } => {
            1u8.hash(&mut hasher);
            width.hash(&mut hasher);
            height.hash(&mut hasher);
            pixels.as_ref().hash(&mut hasher);
        }
    }
    let hash = hasher.finish();
    let first = tables().borrow_mut().pictures.insert((true, hash));
    let data = first.then(|| match handle {
        Handle::Bytes(_, bytes) => ui_lang_wire::ImageData::Encoded(bytes.as_ref().to_vec()),
        Handle::Rgba {
            width,
            height,
            pixels,
            ..
        } => ui_lang_wire::ImageData::Rgba {
            width: *width,
            height: *height,
            pixels: pixels.as_ref().to_vec(),
        },
        Handle::Path(..) => unreachable!("paths refused before recording picture history"),
    });
    Some((hash, data))
}

/// Registers a message in the frame currently being built.
pub fn message<M: 'static>(message: M) -> u32 {
    let tables = tables();
    let mut tables = tables.borrow_mut();
    let message: Rc<dyn Any> = Rc::new(message);
    if !tables.captures.is_empty() {
        let id = tables.cached_id();
        tables.cached_messages.insert(id, message.clone());
        for capture in &mut tables.captures {
            capture.messages.push((id, message.clone()));
        }
        return id;
    }
    let index = u32::try_from(tables.messages.len()).expect("too many message routes");
    assert!(index < CACHED, "too many ordinary message routes");
    tables.messages.push(message);
    index
}

/// A typed handler returns None for a value it cannot route.
pub fn handler<A: 'static, M: 'static>(handler: Box<dyn Fn(A) -> Option<M>>) -> u32 {
    let tables = tables();
    let mut tables = tables.borrow_mut();
    let handler: Rc<dyn Any> = Rc::new(handler);
    if !tables.captures.is_empty() {
        let id = tables.cached_id();
        tables.cached_handlers.insert(id, handler.clone());
        for capture in &mut tables.captures {
            capture.handlers.push((id, handler.clone()));
        }
        return id;
    }
    let index = u32::try_from(tables.handlers.len()).expect("too many handler routes");
    assert!(index < CACHED, "too many ordinary handler routes");
    tables.handlers.push(handler);
    index
}

pub(crate) fn reset() {
    let tables = tables();
    let old = {
        let mut tables = tables.borrow_mut();
        (
            std::mem::take(&mut tables.messages),
            std::mem::take(&mut tables.handlers),
            std::mem::take(&mut tables.cached_messages),
            std::mem::take(&mut tables.cached_handlers),
        )
    };
    drop(old);
}

pub(crate) fn take_message<M: Clone + 'static>(index: u32) -> Option<M> {
    let tables = tables();
    let message = {
        let tables = tables.borrow();
        if index & CACHED != 0 {
            tables.cached_messages.get(&index)
        } else {
            tables.messages.get(index as usize)
        }
        .cloned()?
    };
    message.downcast_ref::<M>().cloned()
}

pub(crate) fn run_handler<A: 'static, M: 'static>(index: u32, value: A) -> Option<M> {
    let tables = tables();
    let handler = {
        let tables = tables.borrow();
        if index & CACHED != 0 {
            tables.cached_handlers.get(&index)
        } else {
            tables.handlers.get(index as usize)
        }
        .cloned()?
    };
    handler.downcast_ref::<Box<dyn Fn(A) -> Option<M>>>()?(value)
}

pub(crate) fn macos() -> bool {
    tables().borrow().macos
}

pub(crate) fn set_mouse_interest(interested: bool) {
    tables().borrow_mut().mouse_interest = interested;
}

pub(crate) fn mouse_interest() -> bool {
    tables().borrow().mouse_interest
}

pub(crate) fn editor_response(response: crate::wire::EditorResponse) {
    let tables = tables();
    let mut tables = tables.borrow_mut();
    tables.editor_pending.retain(|id| {
        !(id.instance == response.id.instance
            && id.document == response.id.document
            && id.sequence == response.id.sequence)
    });
    tables.editor_pending.push(response.id.clone());
    tables.editor_responses.push(response);
}
/// A native commit may have no decision, but an outstanding decision must
/// match its complete attempt/version before any state or route is accepted.
pub(crate) fn editor_matches_pending(id: &crate::wire::EditorTransactionId) -> bool {
    tables().borrow().editor_pending.iter().all(|pending| {
        pending.instance != id.instance
            || pending.document != id.document
            || pending.sequence != id.sequence
            || pending == id
    })
}
pub(crate) fn editor_acknowledge(event: &crate::wire::EditorTransactionEvent) {
    use crate::wire::EditorTransactionEvent;
    let id = match event {
        EditorTransactionEvent::Commit { id, .. }
        | EditorTransactionEvent::Fault { id, .. }
        | EditorTransactionEvent::Cancelled { id, .. } => id,
    };
    tables()
        .borrow_mut()
        .editor_pending
        .retain(|pending| pending != id);
}
pub(crate) fn start_editor_transfer(
    id: crate::wire::editor_document::EditorTransferId,
    target: crate::wire::editor_document::EditorDocumentRef,
) -> Result<(), crate::wire::editor_document::EditorTransferError> {
    use crate::wire::editor_document::{EditorTransferError, EditorTransferSender};
    let tables = tables();
    let mut tables = tables.borrow_mut();
    if let Some(sender) = &tables.editor_sender {
        return if sender.id() == &id {
            Ok(())
        } else {
            Err(EditorTransferError::Limit)
        };
    }
    tables.editor_sender = Some(EditorTransferSender::new(id, target)?);
    Ok(())
}

pub(crate) fn editor_document_frame(
    reference: &crate::wire::editor_document::EditorDocumentRef,
    text: &str,
) {
    use crate::wire::editor_document::EditorDocumentMessage;
    let tables = tables();
    let mut tables = tables.borrow_mut();
    if !tables.editor_documents.is_empty() {
        return;
    }
    let Some(sender) = &mut tables.editor_sender else {
        return;
    };
    if sender.id().document != reference.document {
        return;
    }
    let message = match sender.next_frame(reference, text) {
        Ok(Some(transfer)) => EditorDocumentMessage::Transfer(transfer),
        Ok(None) => return,
        Err(reason) => EditorDocumentMessage::Failed {
            id: sender.id().clone(),
            reason,
        },
    };
    tables.editor_documents.push(message);
}

pub(crate) fn editor_document_failure(
    id: crate::wire::editor_document::EditorTransferId,
    reason: crate::wire::editor_document::EditorTransferError,
) {
    let tables = tables();
    let mut tables = tables.borrow_mut();
    if tables.editor_documents.is_empty() {
        tables
            .editor_documents
            .push(crate::wire::editor_document::EditorDocumentMessage::Failed { id, reason });
    }
}

pub(crate) fn finish_editor_transfer(id: &crate::wire::editor_document::EditorTransferId) {
    let tables = tables();
    let mut tables = tables.borrow_mut();
    if tables
        .editor_sender
        .as_ref()
        .is_some_and(|sender| sender.id() == id)
    {
        tables.editor_sender = None;
    }
}

pub(crate) fn editor_transferring() -> bool {
    tables().borrow().editor_sender.is_some()
}

pub(crate) fn take_editor_documents() -> Vec<crate::wire::editor_document::EditorDocumentMessage> {
    std::mem::take(&mut tables().borrow_mut().editor_documents)
}
pub(crate) fn take_editor_responses() -> Vec<crate::wire::EditorResponse> {
    std::mem::take(&mut tables().borrow_mut().editor_responses)
}
pub(crate) fn editor_pending() -> bool {
    !tables().borrow().editor_pending.is_empty()
}

pub(crate) fn has_handler<A: 'static, M: 'static>(index: u32) -> bool {
    let tables = tables();
    let tables = tables.borrow();
    let handler = if index & CACHED != 0 {
        tables.cached_handlers.get(&index)
    } else {
        tables.handlers.get(index as usize)
    };
    handler.is_some_and(|handler| handler.is::<Box<dyn Fn(A) -> Option<M>>>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_history_includes_shape_and_is_scoped_to_the_driver() {
        let context = Context::default();
        let _entered = context.enter();
        let wide = iced::advanced::image::Handle::from_rgba(2, 1, vec![255; 8]);
        let tall = iced::advanced::image::Handle::from_rgba(1, 2, vec![255; 8]);
        let (wide_hash, data) = image(&wide).unwrap();
        assert!(data.is_some());
        let (tall_hash, data) = image(&tall).unwrap();
        assert_ne!(
            wide_hash, tall_hash,
            "dimensions are part of the raster identity"
        );
        assert!(data.is_some());
        assert!(image(&wide).unwrap().1.is_none());
        {
            let other = Context::default();
            let _other = other.enter();
            assert!(
                image(&wide).unwrap().1.is_some(),
                "each driver must send its own raster payload"
            );
        }
        assert!(image(&wide).unwrap().1.is_none());
        crate::host::drain_outbox();
        assert!(
            image(&iced::advanced::image::Handle::from_path(
                "must-not-be-read.png"
            ))
            .is_none()
        );
        let errors = crate::host::drain_outbox();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "host.log");
        assert!(
            String::from_utf8_lossy(&errors[0].payload)
                .contains("runtime path handles are unsupported")
        );
    }

    #[test]
    fn nested_contexts_restore_typed_routes_and_picture_history() {
        let first = Context::default();
        let _first = first.enter();
        let first_route = handler::<String, String>(Box::new(|text| Some(format!("first:{text}"))));
        assert!(picture(b"svg").1.is_some());
        {
            let second = Context::default();
            let _second = second.enter();
            let route = handler::<String, String>(Box::new(|text| Some(format!("second:{text}"))));
            assert_eq!(route, 0, "each driver starts its own typed route table");
            assert_eq!(
                run_handler::<String, String>(route, "x".into()).as_deref(),
                Some("second:x")
            );
            assert!(
                picture(b"svg").1.is_some(),
                "a new host needs its own picture bytes"
            );
        }
        assert_eq!(
            run_handler::<String, String>(first_route, "x".into()).as_deref(),
            Some("first:x")
        );
        assert!(
            picture(b"svg").1.is_none(),
            "returning to the first driver preserves its picture history"
        );
    }
    #[test]
    fn cached_routes_replay_independently_of_ordinary_slots_and_expire() {
        let context = Context::default();
        let _context = context.enter();
        let ordinary = message("before".to_owned());
        let ((press, edit), saved) = capture(|| {
            (
                message("cached".to_owned()),
                handler::<String, String>(Box::new(|text| Some(format!("cached:{text}")))),
            )
        });
        assert_eq!(ordinary, 0);
        reset();
        for text in ["moved", "new", "surrounding"] {
            message(text.to_owned());
        }
        saved.restore();
        assert_eq!(take_message::<String>(press).as_deref(), Some("cached"));
        assert_eq!(
            run_handler::<String, String>(edit, "typed".into()).as_deref(),
            Some("cached:typed")
        );
        assert_eq!(take_message::<String>(0).as_deref(), Some("moved"));
        assert!(run_handler::<bool, String>(edit, true).is_none());
        reset();
        assert!(take_message::<String>(press).is_none());
        assert!(run_handler::<String, String>(edit, "stale".into()).is_none());
        let (fresh, _) = capture(|| message("replacement".to_owned()));
        assert_ne!(fresh, press, "rebuilt caches cannot alias stale IDs");
    }

    #[test]
    fn outer_capture_keeps_nested_hits_and_nested_misses() {
        let context = Context::default();
        let _context = context.enter();
        let (hit, inner) = capture(|| message(7u32));
        reset();
        let (miss, outer) = capture(|| {
            inner.restore();
            capture(|| message(9u32)).0
        });
        reset();
        outer.restore();
        assert_eq!(take_message::<u32>(hit), Some(7));
        assert_eq!(take_message::<u32>(miss), Some(9));
    }
}
