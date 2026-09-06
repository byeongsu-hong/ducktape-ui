//! The macOS counterpart of `linux_native_atspi_exports_tree_and_routes_action`.
//!
//! Builds a real `NSWindow`, attaches the bridge's NSAccessibility subclass to
//! its content view, publishes a hand-built tree, and then asks the view the
//! questions VoiceOver asks: its children, the child's role, label and
//! frame, and a press. Everything stays in one process — the subclassed view
//! answers directly — so unlike an `AXUIElement` client this needs no
//! Accessibility permission and runs on a CI runner.
//!
//! `harness = false`: `main` runs on the main thread, which is the only
//! thread `accesskit_macos` attaches on. On every other target `main` is a
//! no-op, so `cargo test --workspace` neither skips nor fails it.
#![cfg_attr(
    target_os = "macos",
    expect(
        unsafe_code,
        reason = "AppKit and the AccessKit platform nodes are only reachable through Objective-C messages"
    )
)]

fn main() {
    #[cfg(target_os = "macos")]
    macos::run();
    #[cfg(not(target_os = "macos"))]
    println!("macos_native_smoke: NSAccessibility exists only on macOS; nothing to run here");
}

#[cfg(target_os = "macos")]
mod macos {
    use accesskit::{Action, Node, NodeId, Rect, Role, Tree, TreeId, TreeUpdate};
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{MainThreadMarker, MainThreadOnly, msg_send};
    use objc2_app_kit::{NSApplication, NSBackingStoreType, NSWindow, NSWindowStyleMask};
    use objc2_foundation::{NSArray, NSPoint, NSRange, NSRect, NSSize, NSString};
    use ui_lang_runtime::{Bridge, NativeWindow, Snapshot, is_refresh_request};

    const ROOT: NodeId = NodeId(0);
    const BUTTON: NodeId = NodeId(1);

    /// The button's layout-unit bounds, as iced would report them.
    const BUTTON_BOUNDS: Rect = Rect {
        x0: 10.0,
        y0: 20.0,
        x1: 110.0,
        y1: 60.0,
    };

    /// Depth-first through `accessibilityChildren`, returning the first node
    /// whose `accessibilityRole` is `role`.
    fn descendant_with_role(children: &NSArray, role: &str) -> Option<Retained<AnyObject>> {
        for index in 0..children.count() {
            let child: Retained<AnyObject> = children.objectAtIndex(index);
            // SAFETY: NSAccessibility protocol messages on platform nodes the
            // adapter handed out, with the return types AppKit declares.
            let child_role: Option<Retained<NSString>> =
                unsafe { msg_send![&*child, accessibilityRole] };
            if child_role.is_some_and(|child_role| child_role.to_string() == role) {
                return Some(child);
            }
            let grandchildren: Option<Retained<NSArray>> =
                unsafe { msg_send![&*child, accessibilityChildren] };
            if let Some(found) = grandchildren
                .as_deref()
                .and_then(|grandchildren| descendant_with_role(grandchildren, role))
            {
                return Some(found);
            }
        }
        None
    }

    pub fn run() {
        let mtm =
            MainThreadMarker::new().expect("a harness-free test binary starts on the main thread");
        let _app = NSApplication::sharedApplication(mtm);
        let content = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 300.0));
        // SAFETY: a plain AppKit initializer on a freshly allocated window, on
        // the main thread the marker proves; the window is never shown.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                content,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        let view = window.contentView().expect("a window has a content view");
        let scale = window.backingScaleFactor() as f32;

        let label = format!("ui-lang-macos-smoke-{}", std::process::id());
        let mut root = Node::new(Role::Window);
        root.set_label(label.clone());
        root.set_children(vec![BUTTON]);
        let mut button = Node::new(Role::Button);
        button.set_label(label.clone());
        button.add_action(Action::Click);
        button.set_bounds(BUTTON_BOUNDS);
        let update = TreeUpdate {
            nodes: vec![(ROOT, root), (BUTTON, button)],
            tree: Some(Tree {
                root: ROOT,
                toolkit_name: Some("Ice macOS smoke".into()),
                toolkit_version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            tree_id: TreeId::ROOT,
            focus: ROOT,
        };

        let mut bridge = Bridge::<()>::new();
        let id = iced::window::Id::unique();
        let ns_view = Retained::as_ptr(&view) as usize;
        assert!(
            bridge.attach_window(NativeWindow::for_view(id, ns_view)),
            "the subclass attaches on the main thread"
        );
        assert!(bridge.is_attached());
        // What generated code delivers after an attach: the backing scale the
        // bridge multiplies into every published tree, then the focus.
        bridge.window_event(id, iced::window::Event::Rescaled(scale));
        bridge.update(Snapshot::from_update(update));
        bridge.window_event(id, iced::window::Event::Focused);
        let mut actions = bridge
            .take_action_receiver()
            .expect("the bridge still holds its action receiver");

        // Asking the view for its children is the first thing VoiceOver does;
        // it is what activates the tree. The adapter exposes the tree root as
        // a group under the view, so the button is one level further down —
        // a walk finds it wherever the adapter chooses to place it.
        // SAFETY: NSAccessibility protocol messages on a live view and on the
        // platform nodes it hands out, with the return types AppKit declares.
        let children: Option<Retained<NSArray>> =
            unsafe { msg_send![&*view, accessibilityChildren] };
        let children = children.expect("an attached view exports children");
        let child = descendant_with_role(&children, "AXButton")
            .expect("the tree exports one AXButton under the view");
        // AppKit's name for a button's label is its title; the adapter maps
        // AccessKit's `label` there, and `accessibilityLabel` stays nil.
        let exported: Option<Retained<NSString>> =
            unsafe { msg_send![&*child, accessibilityTitle] };
        assert_eq!(exported.expect("a title").to_string(), label);

        // The frame comes back in points. The node went in as layout units,
        // the bridge multiplied by the backing scale, and the adapter divided
        // it out again — so the size must round-trip. Before the bridge
        // scaled, a Retina display answered half of it.
        let frame: NSRect = unsafe { msg_send![&*child, accessibilityFrame] };
        let expected = (
            BUTTON_BOUNDS.x1 - BUTTON_BOUNDS.x0,
            BUTTON_BOUNDS.y1 - BUTTON_BOUNDS.y0,
        );
        assert!(
            (frame.size.width - expected.0).abs() < 0.01
                && (frame.size.height - expected.1).abs() < 0.01,
            "frame {frame:?} does not round-trip {expected:?} layout units at scale {scale}"
        );

        // Activation asked the program for a fresh tree; that is not a user
        // action and the generated code treats it as none.
        let refresh = actions.try_recv().expect("activation queued a refresh");
        assert!(is_refresh_request(&refresh));

        let pressed: bool = unsafe { msg_send![&*child, accessibilityPerformPress] };
        assert!(pressed, "a button with a click action accepts a press");
        let request = actions.try_recv().expect("the press reached the bridge");
        assert_eq!(request.action, Action::Click);
        assert_eq!(request.target_node, BUTTON);

        // The display preferences come from the same `NSWorkspace` a shipped
        // app asks; the values depend on the machine, reaching them does not.
        let settings = ui_lang_runtime::accessibility_settings();
        assert!(
            settings.screen_reader,
            "the tree was just activated, so a screen reader counts as running"
        );
        println!(
            "macos_native_smoke: exported {label} at scale {scale}, frame {frame:?}, press routed, settings {settings:?}"
        );

        // The rest of VoiceOver's questions, each answered from the same
        // AppKit the shipped app talks to: help text, a slider's number and
        // its increment, a heading, and a text field's characters and caret.
        const SLIDER: NodeId = NodeId(2);
        const HEADING: NodeId = NodeId(3);
        const INPUT: NodeId = NodeId(4);
        const RUN: NodeId = NodeId(5);
        let rect = |y0: f64| Rect {
            x0: 10.0,
            y0,
            x1: 210.0,
            y1: y0 + 30.0,
        };

        let mut root2 = Node::new(Role::Window);
        root2.set_label(label.clone());
        root2.set_children(vec![BUTTON, SLIDER, HEADING, INPUT]);
        let mut button2 = Node::new(Role::Button);
        button2.set_label(label.clone());
        button2.set_description("Saves the document");
        button2.add_action(Action::Click);
        button2.set_bounds(BUTTON_BOUNDS);
        let mut slider = Node::new(Role::Slider);
        slider.set_label("Volume");
        slider.set_numeric_value(4.0);
        slider.set_min_numeric_value(0.0);
        slider.set_max_numeric_value(10.0);
        slider.set_numeric_value_step(1.0);
        slider.add_action(Action::Increment);
        slider.add_action(Action::Decrement);
        slider.set_bounds(rect(80.0));
        let mut heading = Node::new(Role::Heading);
        heading.set_label("Title");
        heading.set_value("Title");
        heading.set_level(2);
        heading.set_bounds(rect(120.0));
        let mut input = Node::new(Role::TextInput);
        input.set_label("Greeting");
        input.set_value("h\u{e9}llo");
        input.set_children(vec![RUN]);
        input.add_action(Action::SetTextSelection);
        input.set_text_selection(accesskit::TextSelection {
            anchor: accesskit::TextPosition {
                node: RUN,
                character_index: 3,
            },
            focus: accesskit::TextPosition {
                node: RUN,
                character_index: 3,
            },
        });
        input.set_bounds(rect(160.0));
        let mut run = Node::new(Role::TextRun);
        run.set_value("h\u{e9}llo");
        run.set_character_lengths(vec![1u8, 2, 1, 1, 1].into_boxed_slice());
        run.set_bounds(rect(160.0));
        let update2 = TreeUpdate {
            nodes: vec![
                (ROOT, root2),
                (BUTTON, button2),
                (SLIDER, slider),
                (HEADING, heading),
                (INPUT, input),
                (RUN, run),
            ],
            tree: None,
            tree_id: TreeId::ROOT,
            focus: ROOT,
        };
        bridge.update(Snapshot::from_update(update2));
        while actions.try_recv().is_ok() {}

        let children: Option<Retained<NSArray>> =
            unsafe { msg_send![&*view, accessibilityChildren] };
        let children = children.expect("children after the second update");
        let describe = |object: &AnyObject| -> String {
            let text: Option<Retained<NSString>> = unsafe { msg_send![object, description] };
            text.map(|text| text.to_string()).unwrap_or_default()
        };
        let string_attr = |object: &AnyObject, selector: &str| -> String {
            let value: Option<Retained<AnyObject>> = match selector {
                "help" => unsafe { msg_send![object, accessibilityHelp] },
                "value" => unsafe { msg_send![object, accessibilityValue] },
                "role" => unsafe { msg_send![object, accessibilityRole] },
                "subrole" => unsafe { msg_send![object, accessibilitySubrole] },
                _ => None,
            };
            value
                .as_deref()
                .map(describe)
                .unwrap_or_else(|| "<nil>".into())
        };

        let button = descendant_with_role(&children, "AXButton").expect("AXButton");
        let help = string_attr(&button, "help");
        println!("probe button: help={help:?}");
        assert_eq!(
            help, "Saves the document",
            "description reaches accessibilityHelp"
        );

        let slider = descendant_with_role(&children, "AXSlider").expect("AXSlider");
        let value = string_attr(&slider, "value");
        println!("probe slider: value={value:?}");
        assert_eq!(value, "4", "numeric value reaches accessibilityValue");
        let incremented: bool = unsafe { msg_send![&*slider, accessibilityPerformIncrement] };
        let request = actions.try_recv().expect("increment reached the bridge");
        println!(
            "probe slider: performIncrement={incremented} request={:?}",
            request.action
        );
        assert_eq!(request.action, Action::Increment);
        assert_eq!(request.target_node, SLIDER);

        // accesskit_macos 0.26 answers the non-standard role string
        // "Heading" for a heading rather than AppKit's AXHeading, so
        // VoiceOver's heading rotor does not see it; the level is not
        // exported at all. Pinned here so a bump that fixes it is noticed.
        let heading = descendant_with_role(&children, "Heading")
            .expect("the adapter exports the heading under its own role string");
        println!(
            "probe heading: role={:?} value={:?}",
            string_attr(&heading, "role"),
            string_attr(&heading, "value")
        );
        assert_eq!(string_attr(&heading, "value"), "Title");

        let field = descendant_with_role(&children, "AXTextField").expect("AXTextField");
        let count: isize = unsafe { msg_send![&*field, accessibilityNumberOfCharacters] };
        let range: NSRange = unsafe { msg_send![&*field, accessibilitySelectedTextRange] };
        println!(
            "probe text field: value={:?} characters={count} selected={{{}, {}}}",
            string_attr(&field, "value"),
            range.location,
            range.length
        );
        assert_eq!(count, 5, "five graphemes, not six bytes");
        assert_eq!(
            (range.location, range.length),
            (3, 0),
            "the caret sits after the third grapheme"
        );
        let _: () = unsafe {
            msg_send![&*field, setAccessibilitySelectedTextRange: NSRange { location: 1, length: 0 }]
        };
        let request = actions
            .try_recv()
            .expect("the caret move reached the bridge");
        println!(
            "probe text field: set range -> {:?} {:?}",
            request.action, request.data
        );
        assert_eq!(request.action, Action::SetTextSelection);
        match request.data {
            Some(accesskit::ActionData::SetTextSelection(selection)) => {
                assert_eq!(selection.focus.node, RUN);
                assert_eq!(selection.focus.character_index, 1);
            }
            other => panic!("expected a text selection, got {other:?}"),
        }
        println!("probe: every question answered");
    }
}
