use super::{__TasksMessage, Tasks};

#[test]
fn resolves_application_callbacks_from_state() {
    let (mut app, _) = Tasks::__boot();
    assert_eq!(Tasks::__title(&app), "Ice Tasks");
    assert_eq!(Tasks::__theme(&app), Tasks::__app_theme(app.__palette()));
    let dark_theme = Tasks::__theme(&app);

    app.active_palette = "app".into();
    assert_ne!(Tasks::__theme(&app), dark_theme);

    app.window_title = "Renamed".into();
    app.app_theme = "dark".into();
    app.app_background = "#123456".into();
    app.app_text = "#abcdef".into();
    app.ui_scale = 1.5;
    let style = Tasks::__style(&app, &iced::Theme::Dark);
    assert_eq!(Tasks::__title(&app), "Renamed");
    assert_eq!(Tasks::__theme(&app), iced::Theme::Dark);
    assert_eq!(style.background_color, "#123456".parse().unwrap());
    assert_eq!(style.text_color, "#abcdef".parse().unwrap());
    assert_eq!(Tasks::__scale_factor(&app), 1.5);

    app.app_theme = "unknown".into();
    app.active_palette = "missing".into();
    app.app_background = "invalid".into();
    let base = <iced::Theme as iced::theme::Base>::base(&iced::Theme::Dark);
    assert_eq!(Tasks::__theme(&app), Tasks::__app_theme(app.__palette()));
    assert_eq!(
        Tasks::__style(&app, &iced::Theme::Dark).background_color,
        base.background_color
    );
    app.ui_scale = 0.0;
    assert_eq!(Tasks::__scale_factor(&app), f32::EPSILON);
}

#[test]
fn constructs_structured_boot_preset() {
    let (app, task) = Tasks::__preset_0();
    assert!(!app.loading);
    assert_eq!(task.units(), 1);

    let (app, task) = Tasks::__preset_1();
    assert_eq!(app.draft, "Preset task");
    assert!(app.loading);
    assert_eq!(task.units(), 2);
}

#[test]
fn opens_and_targets_a_named_window() {
    let (mut app, _) = Tasks::__boot();
    assert_eq!(app.__update(__TasksMessage::OpenChild).units(), 2);

    let id = iced::window::Id::unique();
    assert_eq!(app.__update(__TasksMessage::ChildOpened(id)).units(), 2);
    assert_eq!(app.child_window, Some(id));

    assert_eq!(
        app.__update(__TasksMessage::ChildSized(640.0, 480.0))
            .units(),
        1
    );
    assert_eq!((app.child_width, app.child_height), (640.0, 480.0));
}

#[test]
fn constructs_window_capture_queries() {
    let (mut app, _) = Tasks::__boot();
    assert_eq!(app.__update(__TasksMessage::ReadRawWindowId).units(), 2);
    assert_eq!(app.__update(__TasksMessage::CaptureWindow).units(), 2);
    assert_eq!(app.__update(__TasksMessage::SetWindowIcon).units(), 2);
    assert_eq!(app.__update(__TasksMessage::InspectWindowHandle).units(), 2);

    let pixels = vec![255, 0, 0, 255, 0, 255, 0, 255];
    let screenshot = iced::window::Screenshot::new(pixels, iced::Size::new(2, 1), 1.5);
    let _ = app.__update(__TasksMessage::WindowCaptured(screenshot));
    let _ = app.__update(__TasksMessage::RawWindowIdRead("42".into()));
    assert!(app.snapshot_ready);
    assert_eq!((app.snapshot_width, app.snapshot_height), (2, 1));
    assert_eq!(app.snapshot_scale, 1.5);
    assert_eq!(app.raw_window_id, "42");
}

#[test]
fn renders_every_tasks_node_through_the_headless_draw_path() {
    let (mut app, _) = Tasks::__boot();
    app.about_open = true;
    app.tasks = vec![crate::backend::Task {
        id: 1,
        title: "Rendered task".into(),
        done: false,
    }];

    let counts = super::draw_headlessly(
        app.__view(),
        &Tasks::__theme(&app),
        iced::Size::new(1600.0, 1200.0),
    );
    assert!(counts.quads > 0, "surface widgets must draw quads");
    assert!(counts.primitives > 0, "canvas must draw primitives");
    assert!(counts.images > 0, "media widgets must draw images");
    assert!(counts.text > 0, "text widgets must draw glyph runs");
}
