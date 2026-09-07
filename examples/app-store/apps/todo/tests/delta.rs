//! What a change to a long list costs on the wire: toggling one item crosses
//! as patches to the tree the host holds, not as the tree again. The bytes
//! printed here are the ones quoted in the store's README.

use app_store_todo::items::{Item, encode as encode_items};
use app_store_todo::{boot_native, tick_native};
use ui_lang_guest::testing::{answer, toggle};
use ui_lang_guest::wire::{self, Frame};

/// What the component export sends: a tree the host can rebuild is dropped.
fn crossing(frame: &Frame) -> usize {
    let mut frame = frame.clone();
    if frame.unchanged || !frame.patches.is_empty() {
        frame.root = None;
    }
    wire::encode(&frame).len()
}

#[test]
fn toggling_one_of_two_hundred_items_crosses_as_a_patch_not_the_list() {
    boot_native();
    let frame = tick_native(Vec::new());
    let load = frame
        .requests
        .iter()
        .find(|request| request.kind == "storage.get" && request.payload == b"items")
        .expect("the load");
    let items: Vec<Item> = (0..200)
        .map(|id| Item {
            id,
            text: format!("Item number {id} on a long list"),
            done: false,
            priority: 0,
        })
        .collect();
    let full = tick_native(vec![answer(load.id, &encode_items(&items))]);
    assert!(full.root.is_some() && full.patches.is_empty());
    let full_bytes = crossing(&full);

    let patched = tick_native(toggle(&full, "Item number 100 on a long list", true));
    assert!(!patched.unchanged);
    assert!(!patched.patches.is_empty(), "the toggle crossed whole");
    let patch_bytes = crossing(&patched);
    // The toggle also writes the list back, and that request's payload is
    // the list itself: the tree's share of the frame is the patches.
    let tree_bytes = wire::encoded_size(&full.root) as usize;
    let patches_bytes = wire::encoded_size(&patched.patches) as usize;
    let save_bytes: usize = patched
        .requests
        .iter()
        .map(|request| request.payload.len())
        .sum();
    eprintln!(
        "full frame: {full_bytes} bytes (tree {tree_bytes}); toggle: {patch_bytes} bytes \
         ({} patches {patches_bytes}, storage.set payload {save_bytes})",
        patched.patches.len()
    );
    assert!(
        patches_bytes * 50 < tree_bytes,
        "tree {tree_bytes} bytes, patches {patches_bytes} bytes"
    );
    assert!(patch_bytes < full_bytes / 10);
}
