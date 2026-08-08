//! The reload contract, exercised end to end without a window.
//!
//! A running process holds a compiled slot table and reads its template from
//! disk. These tests cover what that buys and what it does not: an edit that
//! only moves structure and literals is picked up on the next frame, while one
//! that needs a value the binary does not compute must be refused.

use std::io::Write;

use ui_lang_runtime::template::{Slots, Template, accepts};

const STARTER: &str = r#"{
  "root": {
    "kind": "linear",
    "a11y": { "segment": "content", "named": true },
    "axis": "column",
    "spacing": 16.0,
    "children": [
      {
        "kind": "text",
        "a11y": { "segment": "title", "named": true },
        "value": { "literal": "Ice starter" },
        "size": 30.0
      },
      {
        "kind": "text",
        "a11y": { "segment": "count", "named": true },
        "value": { "slot": 0 }
      }
    ]
  },
  "slots": { "texts": 1 }
}"#;

fn write_template(path: &std::path::Path, source: &str) {
    let mut file = std::fs::File::create(path).expect("template file is writable");
    file.write_all(source.as_bytes()).expect("template writes");
    file.sync_all().expect("template reaches disk");
}

#[test]
fn a_structural_edit_reloads_against_the_same_slot_table() {
    let compiled = Template::from_json(STARTER).expect("starter template parses");

    // Reordering, restyling, and adding literal nodes changes only the data.
    let edited = STARTER
        .replace("\"Ice starter\"", "\"Ice starter, edited\"")
        .replace("\"spacing\": 16.0", "\"spacing\": 4.0")
        .replace("\"size\": 30.0", "\"size\": 48.0");
    let edited = Template::from_json(&edited).expect("edited template parses");

    assert_ne!(compiled, edited, "the edit changed the template");
    assert!(
        accepts(&compiled, &edited),
        "a structural edit needs no rebuild"
    );
}

#[test]
fn an_edit_needing_a_new_value_is_refused() {
    let compiled = Template::from_json(STARTER).expect("starter template parses");

    // Referring to a second piece of state means the compiled `__view` would
    // have to evaluate an expression it does not contain.
    let edited = STARTER
        .replace("\"slot\": 0", "\"slot\": 1")
        .replace("\"texts\": 1", "\"texts\": 2");
    let edited = Template::from_json(&edited).expect("edited template parses");

    assert!(
        !accepts(&compiled, &edited),
        "a new value has to come from the compiler"
    );
}

#[test]
fn a_running_process_picks_up_a_rewritten_file() {
    let directory = std::env::temp_dir().join(format!(
        "ice-template-reload-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&directory).expect("scratch directory is creatable");
    let path = directory.join("template.json");
    write_template(&path, STARTER);

    let source = ui_lang_runtime::template::TemplateSource::from_path(STARTER, Some(path.clone()));

    let first = source.current();
    assert_eq!(first.slots.texts, 1);

    // Rewrite the file the way `cargo ice dev` does after a view-only edit.
    let edited = STARTER.replace("\"Ice starter\"", "\"Reloaded\"");
    // Modification times have coarse resolution on some filesystems; make the
    // stamp unambiguously newer rather than racing it.
    std::thread::sleep(std::time::Duration::from_millis(10));
    write_template(&path, &edited);

    let second = source.current();
    assert_ne!(
        first, second,
        "the next frame renders the rewritten template"
    );
    assert!(
        template_contains_literal(&second, "Reloaded"),
        "the reloaded template carries the edited literal"
    );

    // A truncated or half-written save must not blank the window.
    std::thread::sleep(std::time::Duration::from_millis(10));
    write_template(&path, "{ \"root\": ");
    let third = source.current();
    assert_eq!(
        second, third,
        "an unparseable template leaves the last good one rendering"
    );

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn slots_outlive_a_template_that_no_longer_needs_them() {
    // The renderer clones template strings rather than borrowing them, which
    // is what lets a reload drop the tree an earlier frame was built from.
    let template = Template::from_json(STARTER).expect("starter template parses");
    let slots = Slots::<()> {
        texts: vec!["7".into()],
        ..Slots::default()
    };
    let palette = [iced::Color::BLACK];
    let element = ui_lang_runtime::template::render(&template, &slots, &palette, "App", &[]);
    drop(template);
    drop(element);
}

fn template_contains_literal(template: &Template, needle: &str) -> bool {
    template
        .to_json()
        .expect("template serializes")
        .contains(needle)
}
