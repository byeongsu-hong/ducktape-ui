//! Uses the real guest loader, capability check, queue and redraw path.
use super::*;
use iced::advanced::Clipboard;
use iced::advanced::clipboard::Kind;

#[derive(Default)]
struct MemoryClipboard {
    standard: Option<String>,
    primary: Option<String>,
    writes: usize,
    reads: std::cell::Cell<usize>,
    delay: Duration,
}
impl Clipboard for MemoryClipboard {
    fn read(&self, kind: Kind) -> Option<String> {
        self.reads.set(self.reads.get() + 1);
        if !self.delay.is_zero() {
            std::thread::sleep(self.delay);
        }
        match kind {
            Kind::Standard => self.standard.clone(),
            Kind::Primary => self.primary.clone(),
        }
    }
    fn write(&mut self, kind: Kind, value: String) {
        match kind {
            Kind::Standard => self.standard = Some(value),
            Kind::Primary => self.primary = Some(value),
        }
        self.writes += 1;
    }
}

fn load(allowed: bool) -> Guest {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/clipboard-fixture/app_store_clipboard_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle clipboard fixture first");
    Guest::load(&CatalogEntry {
        id: "clipboard-fixture".into(),
        name: "Clipboard fixture".into(),
        description: String::new(),
        capabilities: if allowed {
            vec![Capability {
                name: "clipboard".into(),
            }]
        } else {
            vec![]
        },
        path: path.to_string_lossy().into_owned(),
        mark: "C".into(),
        hash: sha256_hex(&bytes),
    })
    .unwrap()
}

fn press(guest: &mut Guest, label: &str) {
    fn find(node: &wire::Node, label: &str) -> Option<u32> {
        if let wire::Node::Button {
            content: wire::ButtonContent::Label(value),
            on_press,
            ..
        } = node
            && value == label
        {
            return *on_press;
        }
        node.children().iter().find_map(|node| find(node, label))
    }
    let id = find(guest.frame.root.as_ref().unwrap(), label).expect("fixture button");
    guest.deliver(ui_lang_runtime::view_tree::Output::Activate(id));
}
fn has_text(guest: &Guest, text: &str) -> bool {
    fn find(node: &wire::Node, text: &str) -> bool {
        matches!(node, wire::Node::Text { content, .. } if content == text)
            || node.children().iter().any(|node| find(node, text))
    }
    find(guest.frame.root.as_ref().unwrap(), text)
}
fn step(guest: &mut Guest, clipboard: &mut MemoryClipboard, now: &mut Instant) {
    *now += Duration::from_secs(1);
    guest.redraw(*now, clipboard);
    assert!(guest.fault.is_none(), "{:?}", guest.fault);
}

#[test]
#[ignore = "requires the bundled clipboard fixture"]
fn bundled_clipboard_tasks_use_the_platform_and_enforce_instance_permissions() {
    let mut platform = MemoryClipboard::default();
    let mut now = Instant::now();
    let mut guest = load(true);
    step(&mut guest, &mut platform, &mut now);
    for (copy, read, expected) in [
        ("Copy", "Read", "module copy"),
        ("Copy primary", "Read primary", "module selection"),
    ] {
        press(&mut guest, copy);
        press(&mut guest, read);
        step(&mut guest, &mut platform, &mut now);
        step(&mut guest, &mut platform, &mut now);
        assert!(
            has_text(&guest, expected),
            "clipboard read must resume the guest with {expected}"
        );
    }
    assert_eq!(platform.writes, 2);
    assert!(has_text(&guest, "2"));

    let mut denied = load(false);
    step(&mut denied, &mut platform, &mut now);
    press(&mut denied, "Copy");
    press(&mut denied, "Read");
    step(&mut denied, &mut platform, &mut now);
    step(&mut denied, &mut platform, &mut now);
    assert_eq!(
        platform.writes, 2,
        "undeclared capability must never reach the platform"
    );
    assert!(has_text(&denied, "standard empty"));
    assert!(
        has_text(&denied, "1"),
        "denied read must complete with None"
    );

    let write = |id| wire::Request {
        id,
        kind: "clipboard.write".into(),
        payload: wire::encode(&(wire::ClipboardTarget::Standard, "canceled".to_string())),
    };
    guest.answer(now, write(900));
    let id = guest.clipboard[0].0;
    guest.cancel(id);
    guest.execute_clipboard(now, &mut platform);
    assert!(guest.clipboard.is_empty());
    assert_eq!(platform.writes, 2, "canceled write must not run");
    guest.answer(now, write(901));
    assert!(!guest.clipboard.is_empty());
    guest.fault = Some("test fault".into());
    guest.execute_clipboard(now, &mut platform);
    assert!(guest.clipboard.is_empty());
    assert_eq!(platform.writes, 2, "faulted instance must not write");
}

#[test]
#[ignore = "requires the bundled clipboard fixture"]
fn bundled_clipboard_work_obeys_time_and_byte_governors() {
    let mut guest = load(true);
    let mut platform = MemoryClipboard::default();
    let mut now = Instant::now();
    step(&mut guest, &mut platform, &mut now);
    platform.delay = Duration::from_millis(50);
    press(&mut guest, "Read");
    step(&mut guest, &mut platform, &mut now);
    let minimum =
        rest_after(platform.delay).expect("the deliberate slow clipboard overruns the tick budget");
    assert!(
        guest
            .resting_until
            .is_some_and(|until| until >= now + minimum),
        "clipboard time must contribute to the host rest governor"
    );

    let mut guest = load(true);
    platform = MemoryClipboard {
        standard: Some("x".repeat(wire::MAX_STRING_BYTES)),
        ..MemoryClipboard::default()
    };
    step(&mut guest, &mut platform, &mut now);
    for id in 0..70 {
        guest.answer(
            now,
            wire::Request {
                id,
                kind: "clipboard.read".into(),
                payload: wire::encode(&wire::ClipboardTarget::Standard),
            },
        );
    }
    guest.answer(
        now,
        wire::Request {
            id: 70,
            kind: "clipboard.write".into(),
            payload: wire::encode(&(
                wire::ClipboardTarget::Standard,
                "must not write".to_string(),
            )),
        },
    );
    guest.execute_clipboard(now, &mut platform);
    assert_eq!(
        platform.writes, 0,
        "no platform effect after the reply budget is exhausted"
    );
    assert!(platform.reads.get() <= MAX_REPLY_BYTES_PER_TICK / wire::MAX_STRING_BYTES + 1);
}

#[test]
fn clipboard_commands_preserve_empty_values_and_bound_text() {
    use wire::ClipboardTarget as Target;
    let mut platform = MemoryClipboard::default();
    let read = |target, platform: &mut MemoryClipboard| {
        let command = clipboard::decode("read", &wire::encode(&target)).unwrap();
        wire::decode::<Option<String>>(&clipboard::execute(command, platform)).unwrap()
    };
    assert_eq!(read(Target::Standard, &mut platform), None);
    let write =
        clipboard::decode("write", &wire::encode(&(Target::Primary, String::new()))).unwrap();
    clipboard::execute(write, &mut platform);
    assert_eq!(read(Target::Primary, &mut platform), Some(String::new()));
    assert_eq!(read(Target::Standard, &mut platform), None);
    platform.standard = Some("é".repeat(wire::MAX_STRING_BYTES));
    let text = read(Target::Standard, &mut platform).unwrap();
    assert_eq!(text.len(), wire::MAX_STRING_BYTES);
    assert!(text.ends_with('é'));
    assert!(
        clipboard::decode(
            "write",
            &wire::encode(&(Target::Standard, "x".repeat(wire::MAX_STRING_BYTES + 1)))
        )
        .is_err()
    );
    assert!(clipboard::decode("read", &[255]).is_err());
    assert!(clipboard::decode("unknown", &[]).is_err());
    assert_eq!(platform.writes, 1);
}
