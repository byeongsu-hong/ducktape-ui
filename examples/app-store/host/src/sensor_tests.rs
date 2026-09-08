use super::*;

#[test]
#[ignore = "requires bundled sensor-fixture wasm"]
fn text_wasm_sensor_reset_remeasures_only_when_the_key_changes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/sensor-fixture");
    let entry = crate::catalog::scan_dir(&path)
        .into_iter()
        .next()
        .expect("bundle sensor fixture first");
    let guest = Arc::new(Mutex::new(Guest::load(&entry).unwrap()));
    fn text<'a>(node: &'a wire::Node, suffix: &str) -> Option<&'a str> {
        if let wire::Node::Text { key, content, .. } = node
            && key.ends_with(suffix)
        {
            return Some(content);
        }
        node.children().iter().find_map(|child| text(child, suffix))
    }
    let mut renderer = renderer();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        600.0,
    );
    let mut now = std::time::Instant::now();
    for expected in ["1", "2", "3"] {
        if expected != "1" {
            click(&mut ui, &mut renderer, "Rearm");
        }
        for _ in 0..5 {
            ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
        }
        let guard = guest.lock().unwrap();
        let root = guard.frame.root.as_ref().unwrap();
        assert_eq!(
            text(root, "/shows"),
            Some(expected),
            "changed keys must remeasure once, stable keys must stay quiet"
        );
        assert_eq!(text(root, "/width"), Some("20"));
        assert_eq!(text(root, "/height"), Some("10"));
    }
}
