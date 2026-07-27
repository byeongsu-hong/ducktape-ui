use super::*;

#[test]
fn lowers_typed_iced_extern_boundaries() {
    let source = r#"app Interop
  renderer crate::backend::Renderer
extern crate::backend
  Failure(code:i64)
  component native_meter(value:f64) -> f64
  component borrowed_meter(label:&str, values:&[f64], active:&bool) -> f64
  component passive() -> unit
  shader native_shader(value:f64) -> bool
  shader passive_shader() -> unit
  task focus_next() -> unit
  task save() -> i64 ! Failure
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
  amount = 1.0
  label = "Borrowed"
  values:[f64] = [1.0, 2.0]
  count = 0
  seen = false
on changed(next)
  amount = next
on focused
on focus
  task focus_next() -> focused
on save
  task save() -> saved _ | failed _
on saved(next)
  count = next
on failed(error)
  count = error.code
on event(next)
  seen = next
on shaded(next)
  seen = next
subscribe
  events() -> event _
view
  col
    extern native_meter(amount) -> changed _
    extern borrowed_meter(label, values, seen) -> changed _
    extern passive()
    shader native_shader(amount) w=fill h=64.0 -> shaded _
    shader passive_shader()
    button "Focus" -> focus
    button "Save" -> save
"#;
    let generated = compile(source, "interop.ice").unwrap();
    assert!(generated.contains("type __IceRenderer = crate::backend::Renderer"));
    assert!(generated.contains("__IceElement<'static, f64>"));
    assert!(generated.contains("fn __ui_lang_check_component_borrowed_meter<'a>(arg0: &'a str, arg1: &'a [f64], arg2: &'a bool)"));
    assert!(generated.contains("let _: __IceElement<'a, f64>"));
    assert!(generated.contains("::iced::Task<()>"));
    assert!(generated.contains("::iced::Subscription<bool>"));
    assert!(generated.contains("fn __ui_lang_check_shader_native_shader"));
    assert!(generated.contains("::iced::widget::shader::Program<bool>"));
    assert!(
        generated
            .contains("::iced::widget::Shader::new(crate::backend::native_shader(self.amount))")
    );
    assert!(generated.contains(".width(::iced::Fill).height(64.0 as f32)"));
    assert!(generated.contains(".subscription(Self::__subscription)"));
    assert!(generated.contains("native_meter(self.amount).map"));
    assert!(generated.contains("borrowed_meter(::std::convert::AsRef::as_ref(&(self.label)), ::std::convert::AsRef::as_ref(&(self.values)), ::std::borrow::Borrow::borrow(&(self.seen))).map"));
    assert!(generated.contains("passive().map(move |__value| __InteropMessage::__ExternNoop)"));
    assert!(generated.contains("focus_next().map(|value| __InteropMessage::Focused)"));
    assert!(generated.contains("save().map(|result| match result"));
    assert!(generated.contains("Result::Err(error) => __InteropMessage::Failed(error)"));
}

#[test]
fn lowers_native_keyboard_subscriptions() {
    let source = example!("keyboard_values.ice");
    let generated = compile(source, "keyboard_values.ice").unwrap();
    assert!(generated.contains("struct __IceKeyPress"));
    assert!(generated.contains("struct __IceKeyRelease"));
    assert!(generated.contains("key: ::iced::keyboard::Key"));
    assert!(generated.contains("physical_key: ::iced::keyboard::key::Physical"));
    assert!(generated.contains("modifiers: ::iced::keyboard::Modifiers"));
    assert!(generated.contains("::iced::event::listen_with"));
    assert!(generated.contains("::iced::keyboard::Event::KeyPressed"));
    assert!(generated.contains("::iced::keyboard::Event::KeyReleased"));
    assert!(generated.contains("::iced::keyboard::Event::ModifiersChanged"));
    assert!(generated.contains("::iced::keyboard::key::Named::Enter"));
    assert!(generated.contains("::iced::keyboard::key::NativeCode::Windows(42u16)"));
    assert!(generated.contains("<u32>::try_from(42).ok().map"));
    assert!(generated.contains("::iced::keyboard::Location::Standard"));
    assert!(generated.contains("::iced::keyboard::Modifiers::SHIFT"));
    assert!(generated.contains("::iced::keyboard::Modifiers::COMMAND"));
    assert!(generated.contains(".to_latin(event.physical_key)"));
    assert!(generated.contains("::iced::keyboard::Key::Character(value)"));
    assert!(generated.contains("::iced::keyboard::key::Physical::Code(value)"));
    assert!(generated.contains("fn __ui_lang_check_sync_keyboard_value"));
}

#[test]
fn lowers_native_timer_subscription() {
    let source = example!("timer.ice");
    let generated = compile(source, "timer.ice").unwrap();
    assert!(generated.contains("::iced::time::every(::std::time::Duration::from_millis(250))"));
    assert!(generated.contains("if self.auto_refresh { ::iced::Subscription::batch(["));
    assert!(generated.contains("]) } else { ::iced::Subscription::none() }"));
    assert!(generated.contains("::iced::time::now().map"));
    assert!(generated.contains("__TimerEventsMessage::Tick(__value)"));
    assert!(generated.contains(
            "::iced::time::repeat(crate::backend::refresh_time, ::std::time::Duration::from_millis(1000))"
        ));
    assert!(generated.contains(
        ".filter_map(|__value| crate::backend::even_refresh(__value)).with(self.generation)"
    ));
    assert!(generated.contains(
            ".filter_map(|__value| crate::backend::visible_pointer(__value.0, __value.1)).with(self.generation)"
        ));
    assert!(generated.contains(".filter_map(|_| crate::backend::allow_frame())"));
    assert!(generated.contains("__TimerEventsMessage::Refreshed(__value.0, __value.1)"));
}

#[test]
fn lowers_generic_event_values_to_all_native_listeners() {
    let source = r#"app Events
extern crate::backend
  sync event_name(value:event) -> str
  sync event_label(value:event) -> str?
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
on received(value)
on labeled(value)
on identified(id, value)
subscribe
  event -> received _
  event filter=event_label status=any -> labeled _
  event with-id status=ignored -> identified _ _
  event raw status=captured -> received _
  event raw with-id -> identified _ _
view
  text "Events"
"#;
    let generated = compile(source, "events.ice").unwrap();
    assert!(generated.contains("fn __ui_lang_check_sync_event_name"));
    assert!(generated.contains("arg0: ::iced::Event"));
    assert!(generated.contains("::iced::event::listen_with"));
    assert!(generated.contains("::iced::event::listen_raw"));
    assert!(generated.contains("::iced::event::Status::Ignored"));
    assert!(generated.contains("::iced::event::Status::Captured"));
    assert!(generated.contains("Some((__id, __event))"));
    assert!(generated.contains("filter_map(|__value| crate::backend::event_label(__value))"));
    assert!(generated.contains("__EventsMessage::Received(__value)"));
    assert!(generated.contains("__EventsMessage::Identified(__value.0, __value.1)"));
}

#[test]
fn lowers_a_condition_around_window_frames() {
    let source = example!("window_events.ice");
    let generated = compile(source, "window_events.ice").unwrap();
    assert!(
        generated.contains(
            "if self.listen_frames { ::iced::Subscription::batch([::iced::window::frames()"
        )
    );
    assert!(generated.contains("]) } else { ::iced::Subscription::none() }"));
    assert!(generated.contains("::iced::Event::Window(__event)"));
    assert!(generated.contains("::iced::event::Status::Captured"));
    assert!(generated.contains("::iced::event::listen_with(|__event, __status, __id|"));
    assert!(generated.contains("(__id, __value.0, __value.1, __value.2, __value.3)"));
    assert!(generated.contains(".map(|_| __id)"));
    assert!(generated.contains(".map(|__value| (__id, __value))"));
    assert!(generated.contains(
        "__WindowEventsMessage::Opened(__value.0, __value.1, __value.2, __value.3, __value.4)"
    ));

    let defaults = source
        .replace("on focused(id)\n  last_window = some(id)", "on focused")
        .replace(
            "window focused with-id -> focused _",
            "window focused -> focused",
        );
    let generated = compile(&defaults, "window_events.ice").unwrap();
    assert!(generated.contains("map(move |__value| __WindowEventsMessage::Focused)"));
}

#[test]
fn lowers_all_native_input_method_subscriptions() {
    let source = example!("input_method_events.ice");
    let generated = compile(source, "input_method_events.ice").unwrap();
    assert!(generated.contains("::iced::advanced::input_method::Event::Opened"));
    assert!(generated.contains("::iced::advanced::input_method::Event::Preedit"));
    assert!(generated.contains("::iced::advanced::input_method::Event::Commit"));
    assert!(generated.contains("::iced::advanced::input_method::Event::Closed"));
    assert!(generated.contains("i64::try_from(range.start)"));
    assert!(generated.contains("|__event, _, _|"));
}

#[test]
fn lowers_all_native_mouse_subscriptions() {
    let source = example!("mouse_events.ice");
    let generated = compile(source, "mouse_events.ice").unwrap();
    assert!(generated.contains("::iced::event::listen_with"));
    assert!(generated.contains("::iced::mouse::Event::CursorEntered"));
    assert!(generated.contains("::iced::mouse::Event::CursorLeft"));
    assert!(generated.contains("::iced::mouse::Event::CursorMoved"));
    assert!(generated.contains("::iced::mouse::Event::ButtonPressed"));
    assert!(generated.contains("::iced::mouse::Event::ButtonReleased"));
    assert!(generated.contains("::iced::mouse::Event::WheelScrolled"));
    assert!(generated.contains("::iced::mouse::ScrollDelta::Pixels"));
    assert!(generated.contains("::std::option::Option::Some(button)"));
    assert!(generated.contains("::iced::event::Status::Captured"));
}

#[test]
fn lowers_all_native_touch_subscriptions() {
    let source = example!("touch_events.ice");
    let generated = compile(source, "touch_events.ice").unwrap();
    assert!(generated.contains("::iced::touch::Event::FingerPressed"));
    assert!(generated.contains("::iced::touch::Event::FingerMoved"));
    assert!(generated.contains("::iced::touch::Event::FingerLifted"));
    assert!(generated.contains("::iced::touch::Event::FingerLost"));
    assert!(generated.contains("::std::option::Option::Some((id, position.x as f64"));
    assert!(generated.contains("::iced::event::Status::Ignored"));
}

#[test]
fn lowers_typed_pointer_values() {
    let source = example!("pointer_values.ice");
    let generated = compile(source, "pointer_values.ice").unwrap();
    for expected in [
        "Pressed(::iced::mouse::Button)",
        "Touched(::iced::touch::Finger, f64, f64)",
        "::iced::advanced::mouse::Click::new",
        "::iced::mouse::Cursor::Available",
        "::iced::mouse::Button::Other(9u16)",
        "::iced::touch::Finger(18446744073709551615u64)",
        ".position_over(self.bounds)",
        "fn __ui_lang_check_sync_pointer_click",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_native_transformations() {
    let source = example!("transformation_values.ice");
    let generated = compile(source, "transformation_values.ice").unwrap();
    for expected in [
        "identity: ::iced::Transformation",
        "translation: ::iced::Vector",
        "size_value: ::iced::Size",
        "::iced::Transformation::orthographic(640u32, 480u32)",
        "<u32>::try_from((-1))",
        "::iced::Transformation::translate",
        "::iced::Transformation::scale",
        ".inverse()",
        "::std::convert::Into::<[f32; 16]>::into",
        "fn __ui_lang_check_sync_transformation_round_trip",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_native_geometry_values() {
    let source = example!("geometry_values.ice");
    let generated = compile(source, "geometry_values.ice").unwrap();
    for expected in [
        "snapped_point: ::iced::Point<u32>",
        "exact_bounds: ::iced::Rectangle<u32>",
        "snapped_bounds: ::std::option::Option<::iced::Rectangle<u32>>",
        "::iced::Point::ORIGIN",
        ".distance(::iced::Point::new",
        ".snap()",
        "::iced::Vector::ZERO",
        "::iced::Size::INFINITE",
        ".rotate(::iced::Radians",
        "::iced::Size::from((640u32, 480u32))",
        "<u32>::try_from((-1))",
        "::iced::Rectangle::with_vertices",
        ".intersection(&(::iced::Rectangle",
        "::iced::Padding { top:",
        "(self.bounds).anchor(::iced::Size::new",
        "::iced::alignment::Horizontal::Right",
        "::iced::alignment::Vertical::Bottom",
        "(2.0) as f32",
        "fn __ui_lang_check_sync_geometry_round_trip",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_native_padding_and_angles() {
    let source = example!("padding_angles.ice");
    let generated = compile(source, "padding_angles.ice").unwrap();
    for expected in [
        "pixel_value: ::iced::Pixels",
        "direct_padding: ::iced::Padding",
        "degree_value: ::iced::Degrees",
        "radians_value: ::iced::Radians",
        "::iced::Pixels::from(4294967295u32)",
        ".ok().map(::iced::Pixels::from)",
        "::iced::padding::all((5.0) as f32)",
        "::iced::padding::right(::iced::Pixels",
        "::iced::Padding::from([",
        ".fit(::iced::Size::new",
        "::iced::Degrees::RANGE.contains",
        "::iced::Radians::RANGE.contains",
        "::iced::Radians::from(::iced::Degrees",
        ".to_distance(&(::iced::Rectangle",
        " % ",
        "(2.0) as f32 * ::iced::Radians",
        ".rotate(self.radians_value)",
        "fn __ui_lang_check_sync_unit_round_trip",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_native_system_tasks_and_subscription() {
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
  mode = "none"
on inspect
  task system info -> inspected _
on inspected(info)
  cpu = info.cpu_brand
on read_theme
  task system theme -> theme_changed _
on theme_changed(next)
  mode = next
subscribe
  system theme -> theme_changed _
view
  text cpu
"#;
    let generated = compile(source, "diagnostics.ice").unwrap();
    assert!(generated.contains("struct __IceSystemInfo"));
    assert!(generated.contains("fn __ice_system_info(value: ::iced::system::Information)"));
    assert!(generated.contains("::iced::system::information().map(__ice_system_info)"));
    assert!(generated.contains("::iced::system::theme().map(__ice_system_theme)"));
    assert!(generated.contains("::iced::system::theme_changes().map(__ice_system_theme)"));
    assert!(generated.contains("self.cpu = info.cpu_brand.clone()"));
}

#[test]
fn lowers_system_tasks_used_only_by_presets() {
    let source = r#"app Diagnostics
preset inspect
  boot
    task system info -> inspected _
on inspected(info)
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
  text "Ready"
"#;

    let generated = compile(source, "diagnostics.ice").unwrap();

    assert!(generated.contains("struct __IceSystemInfo"));
}

#[test]
fn emits_widget_target_type_for_declared_state() {
    let app_state = r#"app SelectorState
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
  found:widget-target? = none
view
  text "Ready"
"#;
    let canvas_state = r#"app SelectorCanvas
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
  canvas
    state
      found:widget-target? = none
"#;

    for source in [app_state, canvas_state] {
        let generated = compile(source, "selector_state.ice").unwrap();
        assert!(generated.contains("struct __IceWidgetTarget"));
    }
}

#[test]
fn lowers_native_clipboard_tasks() {
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
  value:str? = none
on read
  task clipboard read -> read_done _
on read_done(next)
  value = next
on read_primary
  task clipboard read-primary -> read_done _
on write
  task clipboard write "copied"
on write_primary
  task clipboard write-primary "selected"
view
  text "Clipboard"
"#;
    let generated = compile(source, "clipboard.ice").unwrap();
    assert!(generated.contains("::iced::clipboard::read().map"));
    assert!(generated.contains("::iced::clipboard::read_primary().map"));
    assert!(generated.contains("::iced::clipboard::write::<__ClipboardMessage>"));
    assert!(generated.contains("::iced::clipboard::write_primary::<__ClipboardMessage>"));
}

#[test]
fn lowers_native_runtime_font_loading() {
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
    let generated = compile(source, "fonts.ice").unwrap();
    assert!(generated.contains("::iced::font::load(self.font_bytes.clone()).map"));
    assert!(generated.contains("Result::Ok(value) => __FontsMessage::Loaded(value)"));
    assert!(generated.contains("Result::Err(error) => match error {}"));
}

#[test]
fn lowers_all_static_widget_operations() {
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
on checked(value)
on previous
  task widget focus-prev
on next
  task widget focus-next
on focus
  task widget focus #field
on check
  task widget focused #field -> checked _
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
    let generated = compile(source, "operations.ice").unwrap();
    for function in [
        "focus_previous",
        "focus_next",
        "focus::<",
        "is_focused",
        "move_cursor_to_front",
        "move_cursor_to_end",
        "move_cursor_to::<",
        "select_all",
        "select_range",
        "snap_to::<",
        "snap_to_end",
        "scroll_to::<",
        "scroll_by::<",
    ] {
        assert!(generated.contains(function), "missing {function}");
    }
    assert!(generated.contains("Id::new(\"Operations/field\")"));
    assert!(generated.contains("Id::new(\"Operations/list\")"));
    assert!(generated.contains("usize::try_from(2).unwrap_or(0)"));
    assert!(generated.contains("usize::try_from(1).unwrap_or(0)"));
    assert!(generated.contains("usize::try_from(3).unwrap_or(0)"));
    assert!(generated.contains(
        "RelativeOffset { x: ((0.0) as f32).max(0.0).min(1.0), y: ((1.0) as f32).max(0.0).min(1.0) }"
    ));
    assert!(generated.contains("AbsoluteOffset"));
    assert!(generated.contains("(-4.0)"));
}

#[test]
fn lowers_all_dynamic_widget_operations() {
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
on checked(value)
on focus
  task widget focus #field(selected)
on check
  task widget focused #field(selected) -> checked _
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
    let generated = compile(source, "dynamic_operations.ice").unwrap();
    for function in [
        "focus::<",
        "is_focused",
        "move_cursor_to_front",
        "move_cursor_to_end",
        "move_cursor_to::<",
        "select_all",
        "select_range",
        "snap_to::<",
        "snap_to_end",
        "scroll_to::<",
        "scroll_by::<",
    ] {
        assert!(generated.contains(function), "missing {function}");
    }
    assert!(
        generated
            .contains("Id::from(format!(\"{}/field({})\", \"DynamicOperations\", self.selected))")
    );
    assert!(
        generated
            .contains("Id::from(format!(\"{}/list({})\", \"DynamicOperations\", self.selected))")
    );
    assert!(
        generated.contains("let __a11y_key = format!(\"{}/field({})\", \"DynamicOperations\", id)")
    );
    assert!(generated.contains(".id(::iced::widget::Id::from(__a11y_key.clone()))"));
    assert!(generated.contains(
        ".id(::iced::widget::Id::from(format!(\"{}/list({})\", \"DynamicOperations\", id)))"
    ));
}

#[test]
fn lowers_scoped_widget_operations() {
    let source = example!("scoped_widget_operations.ice");
    let generated = compile(source, "scoped_widget_operations.ice").unwrap();

    for id in [
        "Id::new(\"ScopedOperations/default/field\")",
        "Id::new(\"ScopedOperations/frame/inner-frame/slot-field\")",
        "Id::new(\"ScopedOperations/workspace/details/list\")",
    ] {
        assert!(generated.contains(id), "missing {id}");
    }
    for path in [
        "format!(\"{}/field\", format!(\"{}/inner\", format!(\"{}/outer({})\", \"ScopedOperations\", self.selected)))",
        "format!(\"{}/field\", format!(\"{}/key({})\", \"ScopedOperations\", self.selected))",
        "format!(\"{}/filter\", format!(\"{}/header({})\", \"ScopedOperations\", self.column_index))",
        "format!(\"{}/cell\", format!(\"{}/col({})\", format!(\"{}/row({})\", \"ScopedOperations\", self.row_index), self.column_index))",
    ] {
        assert!(generated.contains(path), "missing {path}");
    }
}

#[test]
fn lowers_widget_selectors() {
    let source = example!("widget_selectors.ice");
    let generated = compile(source, "widget_selectors.ice").unwrap();

    for expected in [
        "struct __IceWidgetTarget",
        "fn __ice_widget_target_from_target",
        "fn __ice_widget_target_from_text",
        "::iced::widget::selector::find(::iced::widget::selector::id(",
        "::iced::widget::selector::find(\"Search\".to_owned())",
        "::iced::widget::selector::find(::iced::Point::new(",
        "::iced::widget::selector::is_focused()",
        "::iced::widget::selector::find_all(\"Search\".to_owned())",
        "::iced::widget::selector::find_all(crate::backend::by_kind(",
        "fn __ui_lang_check_selector_by_kind",
        ".as_ref().map(|value| value.kind.clone())",
        ".as_ref().map(|value| value.x.clone())",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn checks_and_lowers_main_window_tasks() {
    let source = r#"app WindowTasks
  window child
    size 640 480
    position centered
extern crate::backend
  window describe_window(prefix:str) -> str
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
on closed
on size_read(width, height)
on bool_read(value)
on optional_bool_read(value)
on optional_pair_read(x, y)
on scale_read(value)
on mode_read(value)
on raw_id_read(value)
on text_read(value)
on screenshot_read(value)
on opened(id)
  task window size target=id -> size_read _ _
on selected(id)
on close_target(id)
  task window close target=id
on open_child
  task window open child -> opened _
on open_default
  task window open -> close_target _
on read_oldest
  task window oldest -> selected _
on read_latest
  task window latest -> selected _
on close_window
  task window close
on drag_window
  task window drag
on drag_resize_window
  task window drag-resize north-east
on resize_window
  task window resize 800.0 600.0
on resizable_window
  task window resizable true
on min_size_window
  task window min-size 320.0 240.0
on clear_min_size
  task window min-size none
on max_size_window
  task window max-size 1920.0 1080.0
on resize_increments_window
  task window resize-step 8.0 16.0
on read_size
  task window size -> size_read _ _
on read_maximized
  task window maximized -> bool_read _
on maximize_window
  task window maximize true
on read_minimized
  task window minimized -> optional_bool_read _
on minimize_window
  task window minimize false
on read_position
  task window position -> optional_pair_read _ _
on read_scale
  task window scale -> scale_read _
on move_window
  task window move -10.0 20.0
on read_mode
  task window mode -> mode_read _
on mode_window
  task window set-mode fullscreen
on toggle_maximize_window
  task window toggle-maximize
on toggle_decorations_window
  task window toggle-decorations
on attention_window
  task window attention informational
on clear_attention
  task window attention none
on focus_window
  task window focus
on level_window
  task window level always-on-top
on system_menu_window
  task window system-menu
on read_raw_id
  task window raw-id -> raw_id_read _
on capture_window
  task window screenshot -> screenshot_read _
on passthrough_window
  task window mouse-passthrough false
on read_monitor
  task window monitor-size -> optional_pair_read _ _
on automatic_tabbing
  task window auto-tabs false
on change_icon
  task window icon bytes(ff 00 00 ff 00 ff 00 ff) 2 1
on describe_window
  task window describe_window("main") -> text_read _
view
  text "Window"
"#;
    let generated = compile(source, "window_tasks.ice").unwrap();
    for function in [
        "window::open",
        "window::oldest",
        "window::latest",
        "window::close",
        "window::drag",
        "window::drag_resize",
        "window::resize",
        "window::set_resizable",
        "window::set_min_size",
        "window::set_max_size",
        "window::set_resize_increments",
        "window::size",
        "window::is_maximized",
        "window::maximize",
        "window::is_minimized",
        "window::minimize",
        "window::position",
        "window::scale_factor",
        "window::move_to",
        "window::mode",
        "window::set_mode",
        "window::toggle_maximize",
        "window::toggle_decorations",
        "window::request_user_attention",
        "window::gain_focus",
        "window::set_level",
        "window::show_system_menu",
        "window::raw_id",
        "window::screenshot",
        "window::enable_mouse_passthrough",
        "window::disable_mouse_passthrough",
        "window::monitor_size",
        "window::allow_automatic_tabbing",
        "window::set_icon",
        "window::run",
    ] {
        assert!(generated.contains(function), "missing {function}");
    }
    assert!(generated.contains("fn __window_0() -> ::iced::window::Settings"));
    assert!(generated.contains("size: ::iced::Size::new(640 as f32, 480 as f32)"));
    assert!(generated.contains("::iced::window::open(Self::__window_0())"));
    assert!(generated.contains("::iced::window::open(::std::default::Default::default())"));
    assert!(generated.contains("::iced::window::size(id).map"));
    assert!(generated.contains("::iced::window::close::<__WindowTasksMessage>(id)"));
    assert!(generated.contains(
        "Size::new(((800.0) as f32).max(f32::EPSILON).min(f32::MAX), ((600.0) as f32).max(f32::EPSILON).min(f32::MAX))"
    ));
    assert!(generated.contains("value.to_string()"));
    assert!(generated.contains(
        "::iced::window::screenshot(__window).map(move |value| __WindowTasksMessage::ScreenshotRead(value))"
    ));
    assert!(generated.contains("window::oldest().and_then"));
    assert!(generated.contains("crate::backend::describe_window(__window, \"main\".to_owned())"));
    assert!(generated.contains("fn __ui_lang_check_window_describe_window"));
    assert!(generated.contains("window: &dyn ::iced::window::Window"));
    assert!(generated.contains("__width.checked_mul(__height).is_some()"));

    let error = compile(
        &source.replacen("task window close\n", "task window close -> closed\n", 1),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E173");

    let error = compile(
        &source.replace("resize 800.0 600.0", "resize -1.0 600.0"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E128");

    let error = compile(
        &source.replace("size -> size_read _ _", "size -> size_read _"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E129");

    let error = compile(
        &source.replace("task window open child", "task window open missing"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E173");

    let error = compile(
        &source.replace("task window oldest", "task window oldest target=id"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E173");

    let error = compile(
        &source.replace("task window size target=id", "task window size target=true"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E101");

    let error = compile(
        &source.replace(
            "task window screenshot -> screenshot_read _",
            "task window screenshot -> screenshot_read _ _",
        ),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E133");

    let error = compile(
        &source.replace(
            "bytes(ff 00 00 ff 00 ff 00 ff) 2 1",
            "bytes(ff 00 00 ff) 2 1",
        ),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E173");
    assert!(error.message.contains("width × height × 4"));

    let error = compile(
        &source.replace(
            "bytes(ff 00 00 ff 00 ff 00 ff) 2 1",
            "bytes(ff 00 00 ff 00 ff 00 ff) 4294967295 2",
        ),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E173");
    assert!(error.message.contains("dimensions are too large"));

    let error = compile(
        &source.replace("describe_window(\"main\")", "describe_window(true)"),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E101");

    let error = compile(
        &source.replace(
            "describe_window(\"main\") -> text_read _",
            "missing(\"main\") -> text_read _",
        ),
        "window_tasks.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E130");
}
