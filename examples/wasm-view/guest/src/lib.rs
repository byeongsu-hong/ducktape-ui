//! An Ice application that runs inside wasm. `src/ui/app.ice` is an ordinary
//! app; `driver` runs it headlessly; the four exports below are the whole ABI
//! the host speaks: write events into `input_ptr`, call `tick`, read the frame
//! from `output_ptr`.

use std::cell::RefCell;

pub mod driver;
pub mod items;

ui_lang::include_app!("src/ui/app.ice");

thread_local! {
    static DRIVER: RefCell<Option<driver::Driver>> = const { RefCell::new(None) };
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn init() {
    DRIVER.with(|driver| *driver.borrow_mut() = Some(driver::Driver::new()));
}

/// Reserves `len` bytes for the next event batch and returns where to write them.
#[unsafe(no_mangle)]
pub extern "C" fn input_ptr(len: u32) -> u32 {
    INPUT.with(|input| {
        let mut input = input.borrow_mut();
        input.clear();
        input.resize(len as usize, 0);
        input.as_mut_ptr() as u32
    })
}

/// Applies the `len` bytes of events in the input buffer, draws a frame into
/// the output buffer, and returns the frame's length.
#[unsafe(no_mangle)]
pub extern "C" fn tick(len: u32) -> u32 {
    let events: Vec<wasm_view_frame::Event> = INPUT
        .with(|input| wasm_view_frame::decode(&input.borrow()[..len as usize]))
        .unwrap_or_default();
    let frame = DRIVER.with(|driver| {
        driver
            .borrow_mut()
            .as_mut()
            .expect("init first")
            .tick(events)
    });
    let bytes = wasm_view_frame::encode(&frame);
    let len = bytes.len() as u32;
    OUTPUT.with(|output| *output.borrow_mut() = bytes);
    len
}

#[unsafe(no_mangle)]
pub extern "C" fn output_ptr() -> u32 {
    OUTPUT.with(|output| output.borrow().as_ptr() as u32)
}
