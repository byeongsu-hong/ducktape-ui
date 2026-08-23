use super::*;

#[test]
fn lowers_native_canvas_geometry_cache_and_events() {
    let source = r#"app Drawing
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #0f172a
  fg #f8fafc
  primary #7c3aed
  danger #dc2626
state
  cached = true
  picture = rgba(1, 1, bytes(ff 00 ff ff))
on pressed(x, y)
on released(x, y)
on moved(x, y)
on scrolled(x, y, pixels)
on entered
on exited
view
  canvas w=fill h=240.0 cache=cached cache-group=drawings capture=true cursor=crosshair press=pressed release=released right-press=pressed right-release=released middle-press=pressed middle-release=released move=moved scroll=scrolled enter=entered exit=exited
    rect x=0.0 y=0.0 w=canvas_width h=canvas_height fill=linear(1.57, bg@0.0, primary@1.0) stroke=fg
    rect x=8.0 y=8.0 w=72.0 h=40.0 r=8.0 r-tl=4.0 stroke=fg stroke-w=2.0 dash=(4.0, 2.0) dash-offset=1 cap=round join=bevel
    circle x=120.0 y=60.0 r=24.0 fill=primary fill-rule=even-odd stroke=fg
    line x1=16.0 y1=120.0 x2=200.0 y2=120.0 stroke=fg stroke-w=3.0 cap=square
    text "Canvas" x=16.0 y=150.0 max-w=180.0 color=fg size=18.0 line-h=1.2 font=default align-x=left align-y=top shape=advanced
    image picture x=8.0 y=160.0 w=32.0 h=24.0 filter=nearest rotate=0.2 opacity=0.8 snap=true r=4.0 r-tl=2.0
    svg "<svg/>" memory x=48.0 y=160.0 w=24.0 h=24.0 color=fg rotate=0.1 opacity=0.9
    path fill=primary stroke=fg stroke-w=1.0
      move x=220.0 y=20.0
      line x=260.0 y=20.0
      arc x=260.0 y=40.0 r=20.0 start=0.0 end=3.14
      arc-to ax=280.0 ay=60.0 bx=300.0 by=40.0 r=8.0
      ellipse x=320.0 y=40.0 r-x=20.0 r-y=10.0 rotate=0.2 start=0.0 end=6.28
      bezier ax=340.0 ay=10.0 bx=360.0 by=70.0 x=380.0 y=40.0
      quadratic cx=400.0 cy=10.0 x=420.0 y=40.0
      rect x=220.0 y=80.0 w=30.0 h=20.0
      rounded x=260.0 y=80.0 w=30.0 h=20.0 r=4.0
      circle x=320.0 y=90.0 r=10.0
      close
    group x=10.0 y=10.0 rotate=0.1 scale=1.1 scale-x=1.0 scale-y=0.9 clip=(0.0, 0.0, 100.0, 100.0)
      circle x=20.0 y=20.0 r=10.0 fill=fg
    if cached
      circle x=360.0 y=180.0 r=12.0 fill=primary
    for value in [12.0, 24.0]
      circle x=value y=210.0 r=4.0 fill=fg
"#;
    let generated = compile(source, "drawing.ice").unwrap();
    assert!(generated.contains("offset: usize::try_from(1).unwrap_or(0)"));
    assert!(generated.contains("Hash::hash(&(__ice_palette.name, self.cached)"));
    for expected in [
        "impl<State, Message, Draw, Update, Interaction> ::iced::widget::canvas::Program<Message>",
        "__state.cache.get_or_init",
        "Cache::with_group",
        "__ICE_CANVAS_GROUP_DRAWINGS",
        "::std::hash::Hash::hash",
        "::iced::widget::canvas::Path::rounded_rectangle",
        "::iced::Size::new(((72.0) as f32).max(0.0).min(f32::MAX), ((40.0) as f32).max(0.0).min(f32::MAX))",
        "__frame.fill_rectangle",
        "__frame.stroke_rectangle",
        "__frame.fill_text",
        "max_width: ((180.0) as f32).max(0.0).min(f32::MAX)",
        "size: ::iced::Pixels(((18.0) as f32).max(f32::EPSILON).min(f32::MAX))",
        "LineHeight::Relative(((1.2) as f32).max(f32::EPSILON).min(f32::MAX))",
        "__frame.draw_image",
        // The non-left buttons are spelled kebab-case like every other Ice
        // attribute, and each reaches its own native mouse button.
        "::iced::mouse::Event::ButtonPressed(::iced::mouse::Button::Right)",
        "::iced::mouse::Event::ButtonReleased(::iced::mouse::Button::Right)",
        "::iced::mouse::Event::ButtonPressed(::iced::mouse::Button::Middle)",
        "::iced::mouse::Event::ButtonReleased(::iced::mouse::Button::Middle)",
        "__frame.draw_svg",
        "opacity: ((0.8) as f32).max(0.0).min(1.0)",
        "opacity: ((0.9) as f32).max(0.0).min(1.0)",
        "::iced::advanced::svg::Svg",
        "__path.arc(",
        "radius: ((20.0) as f32).max(0.0).min(f32::MAX)",
        "__path.arc_to(",
        "__path.ellipse(",
        "radii: ::iced::Vector::new(((20.0) as f32).max(0.0).min(f32::MAX), ((10.0) as f32).max(0.0).min(f32::MAX))",
        "width: ((2.0) as f32).max(0.0).min(f32::MAX)",
        "segments: &[((4.0) as f32).max(0.0).min(f32::MAX), ((2.0) as f32).max(0.0).min(f32::MAX)]",
        "Path::circle(::iced::Point::new(120.0 as f32, 60.0 as f32), ((24.0) as f32).max(0.0).min(f32::MAX))",
        "__path.bezier_curve_to(",
        "__path.quadratic_curve_to(",
        "__frame.with_save",
        "__frame.with_clip",
        "__frame.scale(((1.1) as f32).max(f32::EPSILON).min(f32::MAX))",
        "__frame.scale_nonuniform(::iced::Vector::new(((1.0) as f32).max(f32::EPSILON).min(f32::MAX), ((0.9) as f32).max(f32::EPSILON).min(f32::MAX)))",
        "::iced::mouse::Interaction::Crosshair",
        "::iced::widget::canvas::Action::publish",
        ".and_capture()",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
}

#[test]
fn lowers_every_canvas_event_and_redraw_action() {
    let source = example!("canvas_events.ice");
    let generated = compile(source, "canvas_events.ice").unwrap();
    assert!(
        generated.contains(
            "#[allow(dead_code)]\n#[derive(Debug, Clone)]\npub(crate) struct __IceKeyPress"
        )
    );
    for expected in [
        "Event::InputMethod",
        "Event::Keyboard",
        "Event::Mouse",
        "Event::Touch",
        "Event::Window",
        "struct __IceKeyPress",
        "::iced::mouse::Button",
        "KeyPressed",
        "KeyReleased",
        "ModifiersChanged",
        "CursorEntered",
        "CursorLeft",
        "CursorMoved",
        "ButtonPressed",
        "ButtonReleased",
        "WheelScrolled",
        "FingerPressed",
        "FingerMoved",
        "FingerLifted",
        "FingerLost",
        "RedrawRequested",
        "CloseRequested",
        "FileHovered",
        "FileDropped",
        "FilesHoveredLeft",
        "Action::publish",
        "Action::capture",
        "Action::request_redraw()",
        "Action::request_redraw_at",
        "Duration::from_millis(16)",
        "__now.checked_add(::iced::time::Duration::from_millis(16))",
        ".and_capture()",
        "move_count: i64",
        "__state.move_count =",
        "fn __ice_canvas_interaction",
        "__ice_canvas_interaction(__interaction.as_str())",
        "__cursor.is_over(__bounds)",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
    assert!(!generated.contains("if (true) || __cursor.is_over(__bounds)"));
}

#[test]
fn lowers_media_tooltip_and_pointer_events() {
    let source = r#"app Media
extern crate::backend
  svg-style dynamic_svg(active:bool)
  box-style dynamic_tooltip(active:bool)
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
  encoded_image = encoded(bytes(50 36 0a))
  rgba_image = rgba(1, 1, bytes(ff 00 00 ff))
on entered
on exited
on pressed
on moved(x, y)
on scrolled(x, y, pixels)
view
  col
    image "photo.ppm" w=fill h=64.0 fit=cover filter=nearest rotate=rotation.solid(radians(0.5)) opacity=0.8 scale=1.2 expand=true r=4.0 r-tl=1.0 r-br=2.0 crop=(1, 2, 30, 40)
    image encoded_image
    image rgba_image
    viewer encoded_image w=fill(2) h=120.0 fit=contain filter=linear p=8.0 min-scale=0.5 max-scale=4.0 scale-step=0.25
    viewer "photo.ppm" w=64.0 h=64.0
    svg "icon.svg" w=48.0 h=shrink fit=scale-down rotate=rotation.floating(radians(0.1)) opacity=0.9 color=fg hover=primary style=dynamic_svg(active)
    svg "<svg/>" memory w=16.0 color=fg hover=none
    svg bytes(3c 73 76 67 2f 3e) memory w=16.0
    tooltip position=cursor gap=2.0 p=5.0 delay=100 snap=false style=dynamic_tooltip(active) bg=linear(1.57, bg@0.0, primary/25@1.0) text=fg border=primary/75 border-w=1.0 r=5.0 r-tl=2.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=8.0 px-snap=true
      mouse enter=entered exit=exited press=pressed move=moved scroll=scrolled cursor=pointer
        text "Hover"
      text "Tip"
"#;
    let generated = compile(source, "media.ice").unwrap();
    assert!(generated.contains(
        "::iced::widget::image(::iced::widget::image::Handle::from_bytes(include_bytes!(\"photo.ppm\").as_slice()))"
    ));
    assert!(
        generated.contains(".rotation(::iced::Rotation::Solid(::iced::Radians((0.5) as f32)))")
    );
    assert!(generated.contains(".border_radius(::iced::border::Radius { top_left: ((1.0) as f32).max(0.0).min(f32::MAX), top_right: ((4.0) as f32).max(0.0).min(f32::MAX), bottom_right: ((2.0) as f32).max(0.0).min(f32::MAX), bottom_left: ((4.0) as f32).max(0.0).min(f32::MAX) })"));
    assert!(generated.contains("image::Handle::from_bytes(::std::vec![0x50u8, 0x36u8, 0x0au8])"));
    assert!(generated.contains("image::Handle::from_rgba((1).clamp(0, u32::MAX as i64) as u32, (1).clamp(0, u32::MAX as i64) as u32, ::std::vec![0xffu8, 0x00u8, 0x00u8, 0xffu8])"));
    for expected in [
        ".opacity(((0.8) as f32).max(0.0).min(1.0))",
        ".scale(((1.2) as f32).max(f32::EPSILON).min(f32::MAX))",
        ".padding(((8.0) as f32).max(0.0).min(f32::MAX))",
        "::ui_lang_runtime::viewer_scale_bounds(0.5, 4.0)",
        ".min_scale(__viewer_min_scale).max_scale(__viewer_max_scale)",
        ".scale_step(((0.25) as f32).max(f32::EPSILON).min(f32::MAX))",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
    assert!(generated.contains(
        "::iced::widget::image::viewer(::iced::widget::image::Handle::from_bytes(include_bytes!(\"photo.ppm\").as_slice()))"
    ));
    assert!(generated.contains(".crop(::iced::Rectangle { x: (1).clamp(0, u32::MAX as i64) as u32, y: (2).clamp(0, u32::MAX as i64) as u32, width: (30).clamp(0, u32::MAX as i64) as u32, height: (40).clamp(0, u32::MAX as i64) as u32 })"));
    assert!(generated.contains(".filter_method(::iced::widget::image::FilterMethod::Nearest)"));
    assert!(generated.contains(
        "::iced::widget::svg(::iced::widget::svg::Handle::from_memory(include_bytes!(\"icon.svg\").as_slice()))"
    ));
    assert!(generated.contains("svg::Handle::from_memory((\"<svg/>\").as_bytes().to_vec())"));
    assert!(generated.contains(
        "svg::Handle::from_memory(::std::vec![0x3cu8, 0x73u8, 0x76u8, 0x67u8, 0x2fu8, 0x3eu8])"
    ));
    assert!(generated.contains("crate::backend::dynamic_svg(__theme, __status, self.active)"));
    assert!(generated.contains("fn __ui_lang_check_svg_style_dynamic_svg"));
    assert!(generated.contains("svg::Status::Idle => __style.color = Some(__ice_palette.colors"));
    assert!(
        generated.contains("svg::Status::Hovered => __style.color = Some(__ice_palette.colors")
    );
    assert!(generated.contains("svg::Status::Hovered => __style.color = None"));
    assert!(!generated.contains("svg::Status::Hovered => __style.color = None, _ => {}"));
    let default_svg = compile(
        &source.replace(" style=dynamic_svg(active)", ""),
        "media.ice",
    )
    .unwrap();
    assert!(default_svg.contains("let mut __style = ::iced::widget::svg::Style::default()"));
    assert!(generated.contains("tooltip::Position::FollowCursor"));
    assert!(generated.contains(
        ".gap(::ui_lang_runtime::bounded_table_metric(2.0, 1)).padding(::ui_lang_runtime::bounded_table_metric(5.0, 1))"
    ));
    assert!(
        generated.contains(
            ".delay(::std::time::Duration::from_millis(u64::try_from(100).unwrap_or(0)))"
        )
    );
    assert!(generated.contains("crate::backend::dynamic_tooltip(__theme, self.active)"));
    let preset_tooltip = compile(
        &source.replace("style=dynamic_tooltip(active)", "style=success"),
        "media.ice",
    )
    .unwrap();
    assert!(preset_tooltip.contains("container::success(__theme)"));
    assert!(generated.contains("__style.background = Some("));
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("__style.border.radius"));
    assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
    assert!(generated.contains("__style.shadow.blur_radius = 8.0 as f32"));
    assert!(generated.contains("__style.snap = true"));
    assert!(generated.contains(".on_enter(__MediaMessage::Entered)"));
    assert!(generated.contains(
        ".on_move(move |__point| __MediaMessage::Moved(__point.x as f64, __point.y as f64))"
    ));
    assert!(generated.contains("::iced::mouse::ScrollDelta::Lines"));
    assert!(generated.contains("__MediaMessage::Scrolled(__x as f64, __y as f64, true)"));
    assert!(generated.contains(".interaction(::iced::mouse::Interaction::Pointer)"));
}

#[test]
fn lowers_normalized_interaction_component_outputs_and_named_events() {
    let source = r#"app InteractionComponents
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
component PointerSurface(active:bool) -> bool
  emits
    moved(f64, f64)
  col
    mouse press=emit(active) move=emit(moved, _, _) cursor=pointer
      text "Pointer"
    resize-handle drag=emit(moved, _, _) cursor=resize-horizontal
      text "Resize"
on changed(next)
on moved(x, y)
view
  PointerSurface active=active -> changed _
    events
      moved -> moved _ _
"#;
    let generated = compile(source, "interaction-components.ice").unwrap();
    assert!(generated.contains("__InteractionComponentsMessage::Changed(__value))(self.active)"));
    // Named-event consumptions go through the hoisted callback alias so a
    // `move` closure never moves an outlined method's callback parameter.
    assert!(generated.contains(
        "let __route_callback = (move |__event_0, __event_1| __InteractionComponentsMessage::Moved(__event_0, __event_1)).clone()"
    ));
    assert!(generated.contains("(__route_callback)(__point.x as f64, __point.y as f64)"));
    assert!(generated.contains("(__route_callback)(__dx, __dy)"));
}

#[test]
fn lowers_borrowed_component_svg_memory_source() {
    let source = r#"app Media
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
component MemorySvg(source:str)
  svg source memory
view
  MemorySvg source="<svg/>"
"#;

    let generated = compile(source, "media.ice").unwrap();
    assert!(generated.contains("svg::Handle::from_memory((\"<svg/>\").as_bytes().to_vec())"));
    assert!(!generated.contains(".into_bytes()"));
}

#[test]
fn embeds_relative_media_files_from_the_ice_source_directory() {
    let source = r#"app Assets
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
  dynamic_path = "runtime/photo.ppm"
view
  col
    image "assets/photo.ppm"
    image dynamic_path
    image "/opt/share/photo.ppm"
    viewer "assets/photo.ppm"
    viewer "/opt/share/photo.ppm"
    svg "assets/icon.svg"
    svg "/opt/share/icon.svg"
    canvas w=fill h=64.0
      image "assets/photo.ppm" x=0.0 y=0.0 w=16.0 h=16.0
      svg "assets/icon.svg" x=16.0 y=0.0 w=16.0 h=16.0
"#;

    let generated = compile(source, "src/ui/assets.ice").unwrap();
    for expected in [
        "::iced::widget::image(::iced::widget::image::Handle::from_bytes(include_bytes!(\"src/ui/assets/photo.ppm\").as_slice()))",
        "::iced::widget::image::viewer(::iced::widget::image::Handle::from_bytes(include_bytes!(\"src/ui/assets/photo.ppm\").as_slice()))",
        "::iced::widget::svg(::iced::widget::svg::Handle::from_memory(include_bytes!(\"src/ui/assets/icon.svg\").as_slice()))",
        "::iced::widget::image::Handle::from_bytes(include_bytes!(\"src/ui/assets/photo.ppm\").as_slice())",
        "::iced::advanced::svg::Handle::from_memory(include_bytes!(\"src/ui/assets/icon.svg\").as_slice())",
    ] {
        assert!(generated.contains(expected), "missing {expected}");
    }
    for runtime in [
        "::iced::widget::image(self.dynamic_path.to_owned())",
        "::iced::widget::image(\"/opt/share/photo.ppm\".to_owned())",
        "::iced::widget::image::viewer(::iced::widget::image::Handle::from_path(\"/opt/share/photo.ppm\".to_owned()))",
        "::iced::widget::svg(\"/opt/share/icon.svg\".to_owned())",
    ] {
        assert!(generated.contains(runtime), "missing {runtime}");
    }
    assert!(!generated.contains("__ICE_ASSET_PATH_"));
    assert!(!generated.contains("from_path(\"src/ui/assets"));
}
