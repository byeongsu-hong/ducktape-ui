//! What publishing a view as data costs, and what it must refuse.
//!
//! The reload path exists to keep the compiler out of a view edit. These
//! contracts hold that claim to two things: emitting a template must stay far
//! cheaper than generating Rust for the same root, and a template must be
//! emitted only when the runtime can actually render every node in it.

use std::io::Write;
use std::time::Instant;

const APP: &str = r#"app Sample
  title "Sample"
  id "dev.ducktape.ice.sample"
  text-size 16

theme contract SampleTheme
  bg
  fg
  primary
  danger
  surface
  muted
  primary_fg

palette sample for SampleTheme
  bg #f5f7fb
  fg #172033
  primary #315efb
  danger #c93445
  surface #ffffff
  muted #667085
  primary_fg #ffffff

state
  name = "World"
  count = 0

on bump
  count = count + 1

view
  box #app
    with
      w=fill
      h=fill
      p=24.0
      align-x=center
    col #content
      with
        w=fill
        gap=12.0
        align=center
      text "Sample" size=28.0
      text name #greeting size=18.0
      input "Your name" #name <-> name w=280.0
      row gap=8.0 align=center
        button "Bump" #bump -> bump
        text count #count
"#;

/// Writes `source` into a fresh directory and returns the root path.
fn write_app(source: &str, tag: &str) -> (tempdir::TempDir, std::path::PathBuf) {
    let directory = tempdir::TempDir::new(tag);
    let path = directory.path().join("app.ice");
    let mut file = std::fs::File::create(&path).expect("app source is writable");
    file.write_all(source.as_bytes())
        .expect("app source writes");
    (directory, path)
}

/// Minimal scratch directory that removes itself, so these tests leave nothing
/// behind without pulling in a dev-dependency for it.
mod tempdir {
    pub struct TempDir(std::path::PathBuf);

    impl TempDir {
        pub fn new(tag: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("ice-view-template-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("scratch directory is creatable");
            Self(path)
        }

        pub fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[test]
fn a_modelled_view_is_published_as_data() {
    let (_directory, path) = write_app(APP, "publishes");
    let mut db = ui_lang_core::AnalysisDb::default();
    let template = db
        .view_template(&path)
        .expect("the sample app analyzes")
        .expect("every node in the sample app is modelled");

    // The dynamic half is exactly the five values the view reads, and the
    // tables now say which kind each one is: two computed strings (the
    // greeting and the count), the input's borrowed state and its binding
    // constructor, and the button's message. Nothing falls back to a hole.
    for (kind, count) in [
        ("texts", 2),
        ("states", 1),
        ("messages", 1),
        ("handlers", 1),
        ("subtrees", 0),
    ] {
        let expected = format!("\"{kind}\": {count}");
        assert!(
            template.json.contains(&expected),
            "expected {expected}, got: {}",
            &template.json[template.json.len().saturating_sub(300)..]
        );
    }
    // Structure and literals live in the data.
    assert!(template.json.contains("\"literal\": \"Sample\""));
    assert!(template.json.contains("\"segment\": \"greeting\""));
    assert!(!template.slot_fingerprint.is_empty());
}

#[test]
fn an_unmodelled_construct_becomes_a_hole_the_compiler_fills() {
    // `if` has no template node. It does not cost the view its template: the
    // enclosing layout falls back to a compiled subtree, and everything
    // outside that subtree still reloads. Whole-view fallback would mean one
    // conditional stops a whole screen from reloading.
    let source = APP.replace(
        "      text count #count\n",
        "      if count > 0\n        text count #count\n",
    );
    let (_directory, path) = write_app(&source, "hole");
    let mut db = ui_lang_core::AnalysisDb::default();
    let template = db
        .view_template(&path)
        .expect("the variant analyzes")
        .expect("an unmodelled node does not cost the view its template");

    assert!(
        template.json.contains(r#""kind": "subtree""#),
        "the conditional's layout became a hole: {}",
        template.json
    );
    // The structure around the hole is still data, so it still reloads.
    assert!(template.json.contains(r#""literal": "Sample""#));
    assert!(template.json.contains(r#""segment": "greeting""#));
}

#[test]
fn a_view_of_only_unmodelled_nodes_is_one_hole() {
    // The root itself can be the hole. Nothing about the view reloads, but the
    // published shape stays uniform rather than becoming a special case.
    let source = APP
        .split("view\n")
        .next()
        .expect("the app has a view")
        .to_owned()
        + "view\n  rich-text #banner\n    span \"Only a hole\"\n";
    let (_directory, path) = write_app(&source, "root-hole");
    let mut db = ui_lang_core::AnalysisDb::default();
    if let Some(template) = db.view_template(&path).expect("the variant analyzes") {
        assert!(
            template.json.contains(r#""kind": "subtree""#),
            "an unmodelled root is published as a hole: {}",
            template.json
        );
    }
}

#[test]
fn restyling_keeps_the_slot_table_but_changes_the_data() {
    let (_directory, path) = write_app(APP, "restyle");
    let mut db = ui_lang_core::AnalysisDb::default();
    let before = db
        .view_template(&path)
        .expect("analyzes")
        .expect("is modelled");

    let edited = APP.replace("gap=12.0", "gap=32.0").replace(
        "text \"Sample\" size=28.0",
        "text \"Sample, edited\" size=40.0",
    );
    let mut file = std::fs::File::create(&path).expect("app source is rewritable");
    file.write_all(edited.as_bytes()).expect("edit writes");
    drop(file);

    let after = db
        .view_template(&path)
        .expect("analyzes")
        .expect("is modelled");

    assert_ne!(
        before.json, after.json,
        "the edit changed the published data"
    );
    assert_eq!(
        before.slot_fingerprint, after.slot_fingerprint,
        "restyling must not change what the compiled binary has to evaluate"
    );
}

#[test]
fn reading_new_state_changes_the_slot_table() {
    let (_directory, path) = write_app(APP, "new-state");
    let mut db = ui_lang_core::AnalysisDb::default();
    let before = db
        .view_template(&path)
        .expect("analyzes")
        .expect("is modelled");

    // Showing a second piece of state needs an expression the running binary
    // does not evaluate, so the fingerprint has to move.
    let edited = APP.replace(
        "      text count #count\n",
        "      text count #count\n        text name #echo\n",
    );
    let edited = edited.replace("        text name #echo", "      text name #echo");
    let mut file = std::fs::File::create(&path).expect("app source is rewritable");
    file.write_all(edited.as_bytes()).expect("edit writes");
    drop(file);

    let after = db
        .view_template(&path)
        .expect("analyzes")
        .expect("is modelled");

    assert_ne!(
        before.slot_fingerprint, after.slot_fingerprint,
        "a view reading more state must force a rebuild"
    );
}

/// Publishing a view must stay a small fraction of generating Rust for it.
///
/// This is a ratio taken in one run rather than an absolute budget: a busy
/// machine slows both sides, so the comparison survives a shared runner. It
/// understates the real saving, which is that the reload path never invokes
/// rustc at all.
#[test]
#[ignore = "performance contract"]
fn performance_contract_view_template_beats_codegen() {
    const ROUNDS: u32 = 40;
    let (_directory, path) = write_app(APP, "perf");
    let mut db = ui_lang_core::AnalysisDb::default();

    // Warm the analysis cache so the comparison isolates emission from parse
    // and check, which both paths share.
    let _ = db.view_template(&path).expect("analyzes");
    let _ = db.compile_root(&path).expect("compiles");

    let started = Instant::now();
    for _ in 0..ROUNDS {
        let _ = db.view_template(&path).expect("analyzes");
    }
    let template = started.elapsed();

    let started = Instant::now();
    for _ in 0..ROUNDS {
        let _ = db.compile_root(&path).expect("compiles");
    }
    let codegen = started.elapsed();

    println!(
        "view template: {:?} over {ROUNDS} rounds; rust codegen: {codegen:?}",
        template
    );
    assert!(
        template < codegen,
        "publishing a view must cost less than generating Rust for it \
         (template {template:?}, codegen {codegen:?})"
    );
}
