use super::*;

#[test]
fn rejects_state_capture_in_subscription_routes() {
    let source = r#"app Demo
extern crate::backend
  subscription events() -> bool
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  count = 1
on event(count, next)
subscribe
  events() -> event(count, _)
view
  text count
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E127");
}

#[test]
fn checks_native_keyboard_payload_fields() {
    let source = r#"app Shortcuts
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  key:key = key.unidentified()
  physical:physical-key = key.native_unidentified()
  location:key-location = key.location("standard")
  modifiers:key-modifiers = key.modifiers(false, false, false, false)
  label = ""
  latin:str? = none
  matched = false
  typed:str? = none
  repeat = false
  command = false
on pressed(event)
  key = event.key
  physical = event.physical_key
  location = event.location
  modifiers = event.modifiers
  label = event.key.kind
  latin = key.latin(event.key, event.physical_key)
  matched = event.key == key.named("Enter")
  typed = event.text
  repeat = event.repeat
  command = event.modifiers.command
on released(event)
  physical = event.physical_key
  command = event.modifiers.jump
on modifiers_changed(modifiers)
  command = modifiers.macos_command
subscribe
  keyboard press -> pressed _
  keyboard release -> released _
  keyboard modifiers -> modifiers_changed _
view
  text label
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[0].params[0]
            .ty
            .display(),
        "key-press"
    );
    assert_eq!(
        document.source_document().handlers[1].params[0]
            .ty
            .display(),
        "key-release"
    );
    assert_eq!(
        document.source_document().handlers[2].params[0]
            .ty
            .display(),
        "key-modifiers"
    );

    let error = analyze(&source.replace(
        "on released(event)\n  physical = event.physical_key",
        "on released(event)\n  physical = event.repeat",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E151");
    assert!(error.message.contains("key-release"));

    for (from, to, message) in [
        (
            "key.named(\"Enter\")",
            "key.named(\"enter\")",
            "exact iced Rust variant",
        ),
        (
            "key.named(\"Enter\")",
            "key.named(\"Self\")",
            "exact iced Rust variant",
        ),
        (
            "key.location(\"standard\")",
            "key.location(\"middle\")",
            "standard, left, right, or numpad",
        ),
        (
            "key.native_unidentified()",
            "key.native(\"windows\", 65536)",
            "0..=65535",
        ),
        (
            "key.latin(event.key, event.physical_key)",
            "key.latin(event.key, event.location)",
            "expected `physical-key`",
        ),
    ] {
        let error = analyze(&source.replace(from, to)).unwrap_err();
        assert!(error.message.contains(message), "{}", error.message);
    }
}

#[test]
fn checks_native_system_tasks_and_theme_subscription() {
    let source = r#"app Diagnostics
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  cpu = ""
  memory = 0
  used:i64? = none
  mode = "none"
on inspect
  task system info -> inspected _
on inspected(info)
  cpu = info.cpu_brand
  memory = info.memory_total
  used = info.memory_used
on read_theme
  task system theme -> theme_changed _
on theme_changed(next)
  mode = next
subscribe
  system theme -> theme_changed _
view
  text cpu
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[1].params[0]
            .ty
            .display(),
        "system-info"
    );
    assert_eq!(
        document.source_document().handlers[3].params[0]
            .ty
            .display(),
        "str"
    );

    let error = analyze(&source.replace("info.cpu_brand", "info.unknown")).unwrap_err();
    assert_eq!(error.code, "E151");
    assert!(error.message.contains("system-info"));

    let error = analyze(&source.replace(
        "task system theme -> theme_changed _",
        "task system theme -> theme_changed _ | theme_changed _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E131");
}

#[test]
fn checks_native_clipboard_tasks() {
    let source = r#"app Clipboard
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  standard:str? = none
  primary:str? = none
on read
  task clipboard read -> standard_read _
on standard_read(value)
  standard = value
on read_primary
  task clipboard read-primary -> primary_read _
on primary_read(value)
  primary = value
on write
  task clipboard write "copied"
on write_primary
  task clipboard write-primary "selected"
view
  text "Clipboard"
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[1].params[0]
            .ty
            .display(),
        "str?"
    );
    assert_eq!(
        document.source_document().handlers[3].params[0]
            .ty
            .display(),
        "str?"
    );

    let error = analyze(&source.replace(
        "task clipboard write \"copied\"",
        "task clipboard write true",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `str`"));
}

#[test]
fn checks_native_runtime_font_loading() {
    let source = r#"app Fonts
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  font_bytes:bytes = bytes(00 01)
on load
  task font load font_bytes -> loaded _
on loaded(result)
view
  text "Fonts"
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[1].params[0]
            .ty
            .display(),
        "unit"
    );

    let error = analyze(&source.replace("font load font_bytes", "font load true")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `bytes`"));

    let error = analyze(&source.replace(" -> loaded _", " -> loaded _ | loaded _")).unwrap_err();
    assert_eq!(error.code, "E131");
    assert!(error.message.contains("infallible"));
}

#[test]
fn checks_all_static_widget_operations() {
    let source = r#"app Operations
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
  focused = false
on previous
  task widget focus-prev
on next
  task widget focus-next
on focus
  task widget focus #field
on check
  task widget focused #field -> checked _
on checked(value)
  focused = value
on front
  task widget cursor-front #field
on end
  task widget cursor-end #field
on cursor
  task widget cursor #field 2
on all
  task widget select-all #field
on range
  task widget select #field 1 3
on snap
  task widget snap #list 0.0 1.0
on snap_end
  task widget snap-end #list
on scroll_to
  task widget scroll-to #list 0.0 24.0
on scroll_by
  task widget scroll-by #list -4.0 8.0
view
  col
    input "Value" #field <-> value
    scroll #list
      text "Content"
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[4].params[0]
            .ty
            .display(),
        "bool"
    );

    let error = analyze(&source.replace("focus #field", "focus #missing")).unwrap_err();
    assert_eq!(error.code, "E172");
    assert!(error.message.contains("#missing"));

    let error = analyze(&source.replace("snap #list 0.0 1.0", "snap #list 0.0 1.1")).unwrap_err();
    assert_eq!(error.code, "E128");
}

#[test]
fn checks_all_dynamic_widget_operations() {
    let source = r#"app DynamicOperations
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  ids = [1, 2]
  selected = 1
  value = ""
  focused = false
on focus
  task widget focus #field(selected)
on check
  task widget focused #field(selected) -> checked _
on checked(value)
  focused = value
on front
  task widget cursor-front #field(selected)
on end
  task widget cursor-end #field(selected)
on cursor
  task widget cursor #field(selected) 2
on all
  task widget select-all #field(selected)
on range
  task widget select #field(selected) 1 3
on snap
  task widget snap #list(selected) 0.0 1.0
on snap_end
  task widget snap-end #list(selected)
on scroll_to
  task widget scroll-to #list(selected) 0.0 24.0
on scroll_by
  task widget scroll-by #list(selected) -4.0 8.0
view
  col
    for id in ids
      input "Value" #field(id) <-> value
      scroll #list(id)
        text id
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[2].params[0].ty,
        Type::Bool
    );

    let error = analyze(&source.replacen("focus #field(selected)", "focus #missing(selected)", 1))
        .unwrap_err();
    assert_eq!(error.code, "E172");
    assert!(error.message.contains("#missing(key)"));

    let error = analyze(&source.replace("selected = 1", "selected = \"one\"")).unwrap_err();
    assert_eq!(error.code, "E172");
    assert!(error.message.contains("expects key type `i64`, got `str`"));

    let error =
        analyze(&source.replacen("focus #field(selected)", "focus #field(true)", 1)).unwrap_err();
    assert_eq!(error.code, "E172");
    assert!(error.message.contains("expects key type `i64`, got `bool`"));
}

#[test]
fn checks_scoped_widget_operations() {
    let source = example!("scoped_widget_operations.ice");
    analyze(source).unwrap();

    let error = analyze(&source.replacen("/inner/field", "/inner/missing", 1)).unwrap_err();
    assert_eq!(error.code, "E172");
    assert!(error.message.contains("#outer(key)/inner/missing"));
    assert!(
        error
            .hint
            .as_deref()
            .is_some_and(|hint| hint.contains("#outer(key)/inner/field")),
        "{:?}",
        error.hint
    );

    let error = analyze(&source.replacen("#outer(selected)", "#outer(value)", 1)).unwrap_err();
    assert_eq!(error.code, "E172");
    assert!(
        error
            .message
            .contains("segment `outer` expects key type `i64`, got `str`")
    );

    let error = analyze(&source.replacen(
        "#row(row_index)/col(column_index)/cell",
        "#col(column_index)/row(row_index)/cell",
        1,
    ))
    .unwrap_err();
    assert_eq!(error.code, "E172");
    assert!(error.message.contains("unknown app widget target"));
}

#[test]
fn checks_widget_selectors() {
    let source = example!("widget_selectors.ice");
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[6].params[0].ty,
        Type::Option(Box::new(Type::WidgetTarget))
    );
    assert_eq!(
        document.source_document().handlers[7].params[0].ty,
        Type::List(Box::new(Type::WidgetTarget))
    );
    assert_eq!(
        document.source_document().handlers[8].params[0].ty,
        Type::List(Box::new(Type::Str))
    );

    for (before, after, message) in [
        ("find text \"Search\"", "find text 1", "expected `str`"),
        (
            "find point 12.0 24.0",
            "find point true 24.0",
            "expected `f64`",
        ),
        (
            "find id #root/field",
            "find id #root/missing",
            "unknown app widget target",
        ),
        (
            "find-all by_kind(\"text\")",
            "find-all by_kind(1)",
            "expected `str`",
        ),
    ] {
        let error = analyze(&source.replacen(before, after, 1)).unwrap_err();
        assert!(error.message.contains(message), "{}", error.message);
    }

    let error = analyze(&source.replacen(" -> found_one _", "", 1)).unwrap_err();
    assert_eq!(error.code, "E172");
    assert!(error.message.contains("selector requires"));

    let error = analyze(&source.replacen("value.kind", "value.missing", 1)).unwrap_err();
    assert_eq!(error.code, "E151");
    assert!(error.message.contains("has no field `missing`"));
}

#[test]
fn rejects_comparisons_of_widget_selector_targets() {
    let source = example!("widget_selectors.ice")
        .replace(
            "  found_x:f64? = none",
            "  found_x:f64? = none\n  same = false",
        )
        .replace(
            "on find_id",
            "on compare_targets\n  same = found == found\n\non find_id",
        );
    let failure = analyze(&source).unwrap_err();

    assert_eq!(failure.code, "E153");
    assert!(failure.message.contains("target values"));
    assert!(failure.message.contains("explicit field"));
}

#[test]
fn rejects_events_routed_to_mount() {
    let source = r#"app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
on mount
view
  button "Invalid" -> mount
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E135");
}

#[test]
fn rejects_invalid_media_options() {
    let source = r#"app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
view
  image "photo.ppm" opacity=1.5
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("opacity"));

    let valid = source.replace(
        "image \"photo.ppm\" opacity=1.5",
        "image rgba(1, 1, bytes(ff 00 00 ff)) crop=(0, 0, 1, 1)",
    );
    analyze(&valid).unwrap();

    let error = analyze(&valid.replace("bytes(ff 00 00 ff)", "bytes(ff 00 00)")).unwrap_err();
    assert_eq!(error.code, "E152");
    assert!(error.message.contains("width × height × 4"));

    let error = analyze(&valid.replace("crop=(0, 0, 1, 1)", "crop=(-1, 0, 1, 1)")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("crop"));

    let viewer = source.replace(
        "image \"photo.ppm\" opacity=1.5",
        "viewer \"photo.ppm\" p=8.0 min-scale=0.5 max-scale=4.0 scale-step=0.25",
    );
    analyze(&viewer).unwrap();
    let error = analyze(&viewer.replace("min-scale=0.5", "min-scale=5.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("minimum scale"));

    let source = source.replace(
        "image \"photo.ppm\" opacity=1.5",
        "svg \"icon.svg\" color=missing",
    );
    let error = analyze(&source).unwrap_err();
    assert_eq!(error.code, "E129");
    assert!(error.message.contains("missing"));

    let source = source.replace(
        "svg \"icon.svg\" color=missing",
        "image \"photo.ppm\" memory",
    );
    let error = analyze(&source).unwrap_err();
    assert_eq!(error.code, "E085");
    assert!(error.message.contains("only available on svg"));
}

#[test]
fn checks_svg_style_calls() {
    let source = r#"app Demo
extern crate::backend
  svg-style dynamic_svg(active:bool)
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  active = true
view
  svg "icon.svg" style=dynamic_svg(active)
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("dynamic_svg(active)", "missing(active)")).unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("svg style"));

    let error = analyze(&source.replace("dynamic_svg(active)", "dynamic_svg(1)")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("style=dynamic_svg(active)", "style=primary")).unwrap_err();
    assert_eq!(error.code, "E085");
    assert!(error.message.contains("declared style call"));

    let error = analyze(&source.replace(
        "svg \"icon.svg\" style=dynamic_svg(active)",
        "image \"icon.svg\" style=dynamic_svg(active)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E085");
    assert!(error.message.contains("only available on svg"));
}

#[test]
fn rejects_invalid_canvas_programs() {
    let source = r#"app Drawing
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  cached = true
  picture = rgba(1, 1, bytes(ff 00 ff ff))
on pressed(x, y)
on key(value)
view
  canvas w=fill h=120.0 cache=cached cache-group=drawings press=pressed
    event keyboard press -> key _
    redraw window frame after=16ms
    capture touch lost
    circle x=60.0 y=60.0 r=24.0 fill=primary
    image picture x=4.0 y=4.0 w=16.0 h=16.0 opacity=0.8 snap=true
    svg "<svg/>" memory x=24.0 y=4.0 w=16.0 h=16.0 color=fg opacity=0.9
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("fill=primary", "fill=missing")).unwrap_err();
    assert_eq!(error.code, "E190");
    assert!(error.message.contains("canvas fill"));

    let error = analyze(&source.replace("cache=cached", "cache=1.0")).unwrap_err();
    assert_eq!(error.code, "E190");
    assert!(error.message.contains("stable hashing"));

    let error = analyze(&source.replace("cache=cached ", "")).unwrap_err();
    assert_eq!(error.code, "E190");
    assert!(error.message.contains("cache-group requires"));

    let error = analyze(&source.replace("opacity=0.8", "opacity=1.1")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("image opacity"));

    for value in ["3.5e38", "-3.5e38"] {
        let error = analyze(&source.replace("x=60.0", &format!("x={value}"))).unwrap_err();
        assert_eq!(error.code, "E128");
        assert!(error.message.contains("circle x"));
    }

    let error = analyze(&source.replace("color=fg", "color=missing")).unwrap_err();
    assert_eq!(error.code, "E190");
    assert!(error.message.contains("svg color"));

    let error = analyze(&source.replace(" r=24.0", "")).unwrap_err();
    assert_eq!(error.code, "E190");
    assert!(error.message.contains("requires `r=`"));

    let error = analyze(&source.replace(
        "event keyboard press -> key _",
        "event keyboard press -> key _\n    event keyboard press -> key _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E190");
    assert!(error.message.contains("duplicate canvas event"));

    let error =
        analyze(&source.replace("event keyboard press -> key _", "event every 1s -> key _"))
            .unwrap_err();
    assert_eq!(error.code, "E190");
    assert!(error.message.contains("canvas events accept"));

    let error = analyze(&source.replace(
        "event keyboard press -> key _",
        "event window focused with-id -> key _",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E190");
    assert!(error.message.contains("`with-id` options"));

    let error = analyze(&source.replace("after=16ms", "after=0ms")).unwrap_err();
    assert_eq!(error.code, "E084");
    assert!(error.message.contains("positive"));
}

#[test]
fn checks_canvas_local_state_and_event_blocks() {
    let source = r#"app Drawing
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
on released(button)
view
  canvas w=fill h=120.0 cursor=(cursor_state) cursor-outside=outside
    state
      cursor_state = "grab"
      outside = false
      hits = 0
    event mouse pressed as button
      set cursor_state = "grabbing"
      set hits = hits + 1
      redraw
      capture
    event mouse released as button
      set cursor_state = "grab"
      emit released button
    text hits x=8.0 y=20.0 color=fg size=14.0
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("hits = 0", "cache = 0")).unwrap_err();
    assert!(error.message.contains("reserved"));

    let error = analyze(&source.replace("outside = false", "hits = 1")).unwrap_err();
    assert!(error.message.contains("duplicate canvas state"));

    let captured = source.replace(
        "  danger #ff0000\n",
        "  danger #ff0000\nstate\n  initial_cursor = \"grab\"\n",
    );
    let error = analyze(&captured.replacen(
        "cursor_state = \"grab\"",
        "cursor_state:str = initial_cursor",
        1,
    ))
    .unwrap_err();
    assert!(error.message.contains("initial_cursor"));

    let error = analyze(&source.replace("set hits = hits + 1", "set hits = \"many\"")).unwrap_err();
    assert!(error.message.contains("expected `i64`"));

    let error = analyze(&source.replace("set hits = hits + 1", "set missing = 1")).unwrap_err();
    assert!(error.message.contains("unknown canvas state `missing`"));

    let error = analyze(&source.replace(
        "event mouse released as button",
        "event mouse released as button, extra",
    ))
    .unwrap_err();
    assert!(error.message.contains("exposes 1 values"));

    let error = analyze(&source.replace(
        "      redraw\n      capture",
        "      redraw\n      emit released button\n      capture",
    ))
    .unwrap_err();
    assert!(error.message.contains("one `emit` or `redraw`"));

    let error = analyze(&source.replace("emit released button", "emit released _")).unwrap_err();
    assert!(error.message.contains("named bindings"));

    let error = analyze(&source.replace("cursor=(cursor_state)", "cursor=(hits)")).unwrap_err();
    assert!(error.message.contains("expected `str`"));

    let error = analyze(&source.replace("cursor=(cursor_state) ", "")).unwrap_err();
    assert!(error.message.contains("cursor-outside requires"));

    let error =
        analyze(&source.replace("cursor=(cursor_state)", "cursor=(\"bogus\")")).unwrap_err();
    assert!(error.message.contains("unknown canvas cursor"));
}

#[test]
fn accepts_a_full_tray_declaration() {
    let source = r#"daemon Demo
  tray
    icon-rgba "assets/tray.rgba" 2 2
    icon-template true
    label describe(count)
    tooltip "Demo"
    popover status
  window status
    size 320 240
extern crate::backend
  sync describe(value:i64) -> str
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  count = 1
view
  text count
"#;
    analyze(source).unwrap();
}

#[test]
fn rejects_a_non_str_tray_label() {
    let source = r#"app Demo
  tray
    icon-rgba "assets/tray.rgba" 2 2
    label count
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  count = 1
view
  text count
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `str`"));
}

#[test]
fn rejects_a_non_str_tray_tooltip() {
    let source = r#"app Demo
  tray
    icon-rgba "assets/tray.rgba" 2 2
    tooltip count
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  count = 1
view
  text count
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `str`"));
}

#[test]
fn rejects_a_tray_popover_without_a_matching_window() {
    let source = r#"daemon Demo
  tray
    icon-rgba "assets/tray.rgba" 2 2
    popover missing
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
view
  text "hi"
"#;
    let error = analyze(source).unwrap_err();
    assert_eq!(error.code, "E173");
    assert!(error.message.contains("unknown app window `missing`"));
}
