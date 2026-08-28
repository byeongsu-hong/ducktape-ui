use std::cell::RefCell;
use viewcore::Session;

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn init(rows: u32) {
    SESSION.with(|s| *s.borrow_mut() = Some(Session::new(rows as usize)));
}

/// Records one frame into the output buffer; returns its length.
#[unsafe(no_mangle)]
pub extern "C" fn frame(changed: u32) -> u32 {
    let bytes = SESSION.with(|s| s.borrow_mut().as_mut().expect("init").frame_bytes(changed != 0));
    let len = bytes.len() as u32;
    OUT.with(|o| *o.borrow_mut() = bytes);
    len
}

#[unsafe(no_mangle)]
pub extern "C" fn raster() -> u32 {
    let bytes = SESSION.with(|s| s.borrow_mut().as_mut().expect("init").raster());
    let len = bytes.len() as u32;
    OUT.with(|o| *o.borrow_mut() = bytes);
    len
}

#[unsafe(no_mangle)]
pub extern "C" fn out_ptr() -> u32 {
    OUT.with(|o| o.borrow().as_ptr() as u32)
}
