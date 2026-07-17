use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TestProject(PathBuf);

impl TestProject {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "ducktape-ui-install-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("src")).unwrap();
        fs::write(
            path.join("Cargo.toml"),
            r#"[package]
name = "ducktape-ui-install-check"
version = "0.0.0"
edition = "2024"
rust-version = "1.88"
"#,
        )
        .unwrap();
        fs::write(path.join("src/main.rs"), "fn main() {}\n").unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn dry_run_reports_an_inactive_dependency_without_writing() {
    let project = TestProject::new();
    let manifest_path = project.path().join("Cargo.toml");
    let manifest = format!(
        "{}\n[target.'cfg(windows)'.dependencies]\niced = {{ version = \"=0.14.0\", features = [\"advanced\"] }}\n",
        fs::read_to_string(&manifest_path).unwrap(),
    );
    fs::write(&manifest_path, &manifest).unwrap();

    ducktape_ui::execute(["init"], project.path()).unwrap();
    let output = ducktape_ui::execute(["add", "button", "--dry-run"], project.path()).unwrap();

    assert!(output.contains(&format!(
        "would update {}: cargo add iced@=0.14.0 --features advanced",
        manifest_path.display(),
    )));
    assert_eq!(fs::read_to_string(manifest_path).unwrap(), manifest);
    assert!(!project.path().join("src/ui/button.rs").exists());
}

#[test]
fn custom_ui_directory_keeps_component_tests_portable() {
    let project = TestProject::new();

    ducktape_ui::execute(["init", "--ui", "src/widgets"], project.path()).unwrap();
    ducktape_ui::execute(["add", "button"], project.path()).unwrap();
    fs::write(
        project.path().join("src/main.rs"),
        r#"mod widgets;

fn main() {
    let theme = widgets::theme::LIGHT;
    let _: iced::Element<'_, ()> = widgets::button::button("Save", &theme)
        .on_press(())
        .into();
}
"#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(["test", "--quiet"])
        .current_dir(project.path())
        .env("CARGO_TARGET_DIR", project.path().join("target"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "custom UI directory did not test:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn init_add_composed_components_and_compile_owned_source() {
    let project = TestProject::new();

    ducktape_ui::execute(["init"], project.path()).unwrap();
    let output = ducktape_ui::execute(
        [
            "add",
            "card",
            "field",
            "input",
            "badge",
            "segmented-control",
            "separator",
            "alert",
            "checkbox",
            "empty",
            "progress",
            "textarea",
            "aspect-ratio",
            "avatar",
            "breadcrumb",
            "item",
            "kbd",
            "label",
            "pagination",
            "skeleton",
            "typography",
            "attachment",
            "bubble",
            "button-group",
            "direction",
            "input-group",
            "marker",
            "message",
            "message-scroller",
            "scroll-area",
            "spinner",
            "table",
            "accordion",
            "calendar",
            "date-picker",
            "carousel",
            "collapsible",
            "combobox",
            "data-table",
            "focus-control",
            "native-select",
            "input-otp",
            "radio-group",
            "slider",
            "switch",
            "tabs",
            "toggle",
            "toggle-group",
            "resizable",
            "dialog",
            "alert-dialog",
            "toast",
            "sonner",
            "command",
            "chart",
            "popover",
            "tooltip",
            "hover-card",
            "sidebar",
            "dropdown-menu",
            "context-menu",
            "menubar",
            "select",
            "sheet",
            "drawer",
            "navigation-menu",
        ],
        project.path(),
    )
    .unwrap();
    assert!(
        fs::read_to_string(project.path().join("Cargo.toml"))
            .unwrap()
            .contains("\"advanced\"")
    );
    assert!(
        fs::read_to_string(project.path().join("Cargo.toml"))
            .unwrap()
            .contains("\"canvas\"")
    );
    assert!(output.contains("theme.rs"));
    assert!(output.contains("button.rs"));
    assert!(output.contains("surface.rs"));
    assert!(output.contains("field.rs"));
    assert!(output.contains("segmented_control.rs"));
    assert!(output.contains("separator.rs"));
    assert!(output.contains("alert.rs"));
    assert!(output.contains("checkbox.rs"));
    assert!(output.contains("empty_state.rs"));
    assert!(output.contains("progress.rs"));
    assert!(output.contains("textarea.rs"));
    assert!(output.contains("aspect_ratio.rs"));
    assert!(output.contains("avatar.rs"));
    assert!(output.contains("breadcrumb.rs"));
    assert!(output.contains("pagination.rs"));
    assert!(output.contains("typography.rs"));
    assert!(output.contains("attachment.rs"));
    assert!(output.contains("button_group.rs"));
    assert!(output.contains("message_scroller.rs"));
    assert!(output.contains("table.rs"));
    assert!(output.contains("accordion.rs"));
    assert!(output.contains("calendar.rs"));
    assert!(output.contains("date_picker.rs"));
    assert!(output.contains("carousel.rs"));
    assert!(output.contains("collapsible.rs"));
    assert!(output.contains("combobox.rs"));
    assert!(output.contains("data_table.rs"));
    assert!(output.contains("focus_control.rs"));
    assert!(output.contains("native_select.rs"));
    assert!(output.contains("input_otp.rs"));
    assert!(output.contains("radio_group.rs"));
    assert!(output.contains("slider.rs"));
    assert!(output.contains("switch.rs"));
    assert!(output.contains("tabs.rs"));
    assert!(output.contains("toggle.rs"));
    assert!(output.contains("toggle_group.rs"));
    assert!(output.contains("resizable.rs"));
    assert!(output.contains("modal.rs"));
    assert!(output.contains("dialog.rs"));
    assert!(output.contains("alert_dialog.rs"));
    assert!(output.contains("toast.rs"));
    assert!(output.contains("sonner.rs"));
    assert!(output.contains("command.rs"));
    assert!(output.contains("chart.rs"));
    assert!(output.contains("popover.rs"));
    assert!(output.contains("tooltip.rs"));
    assert!(output.contains("hover_card.rs"));
    assert!(output.contains("sidebar.rs"));
    assert!(output.contains("menu.rs"));
    assert!(output.contains("dropdown_menu.rs"));
    assert!(output.contains("context_menu.rs"));
    assert!(output.contains("menubar.rs"));
    assert!(output.contains("select.rs"));
    assert!(output.contains("sheet.rs"));
    assert!(output.contains("drawer.rs"));
    assert!(output.contains("navigation_menu.rs"));

    fs::write(
        project.path().join("src/main.rs"),
        r#"mod ui;

#[derive(Debug, Clone)]
enum Message {
    EmailChanged(String),
    SectionSelected(Section),
    Accepted(bool),
    Notes(iced::widget::text_editor::Action),
    PageSelected(usize),
    TranscriptScroll(ui::message_scroller::MessageScrollerEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    General,
    Advanced,
}

fn update_message_scroller(
    state: &mut ui::message_scroller::MessageScrollerState,
    event: ui::message_scroller::MessageScrollerEvent,
) -> iced::Task<Message> {
    state.update(event).map(Message::TranscriptScroll)
}

fn seed_message_scroller() -> (
    ui::message_scroller::MessageScrollerState,
    iced::Task<Message>,
) {
    let mut state =
        ui::message_scroller::MessageScrollerState::new("transcript").auto_scroll(true);
    let task = update_message_scroller(
        &mut state,
        ui::message_scroller::MessageScrollerEvent::ItemsChanged(vec![
            ui::message_scroller::MessageScrollerItemMeta::new("today").scroll_anchor(true),
            ui::message_scroller::MessageScrollerItemMeta::new("message-1"),
        ]),
    );
    (state, task)
}

fn view<'a>(
    content: &'a iced::widget::text_editor::Content,
    message_scroller: &'a ui::message_scroller::MessageScrollerState,
) -> iced::Element<'a, Message> {
    let theme = ui::theme::LIGHT;
    let field = ui::field::field(
        "Email",
        ui::input::input("name@example.com", "", &theme).on_input(Message::EmailChanged),
        Some(ui::field::FieldHint::Description("Account notifications only.")),
        &theme,
    );
    let sections = ui::segmented_control::segmented_control(
        [
            (Section::General, "General"),
            (Section::Advanced, "Advanced"),
        ],
        Section::General,
        Message::SectionSelected,
        &theme,
    );
    let notes = ui::textarea::textarea(
        content,
        "Write a note",
        Message::Notes,
        ui::textarea::TextareaVariant::Default,
        &theme,
    );
    let crumbs = ui::breadcrumb::breadcrumb(
        [
            ui::breadcrumb::BreadcrumbItem::link(
                ui::button::button("Home", &theme).on_press(Message::PageSelected(1)),
            ),
            ui::breadcrumb::BreadcrumbItem::current(iced::widget::text("Settings")),
        ],
        || ui::breadcrumb::breadcrumb_separator(&theme),
        &theme,
    );
    let pages = ui::pagination::pagination(
        [
            ui::pagination::PaginationItem::Previous(None),
            ui::pagination::PaginationItem::Page { number: 1, current: true },
            ui::pagination::PaginationItem::Ellipsis,
            ui::pagination::PaginationItem::Next(Some(2)),
        ],
        Message::PageSelected,
        &theme,
    );
    let row_item = ui::item::item(
        Some(ui::avatar::avatar_fallback("DU", ui::avatar::AvatarSize::Small, &theme).into()),
        "ducktape-ui",
        Some("Owned source"),
        Some(ui::kbd::kbd("K", &theme).into()),
        &theme,
    );
    let ratio = ui::aspect_ratio::aspect_ratio(1.0, || iced::widget::text("Square").into())
        .width(100)
        .height(100);
    let grouped_buttons = ui::button_group::button_group(
        [
            ui::button::button("One", &theme).on_press(Message::PageSelected(1)).into(),
            ui::button::button("Two", &theme).on_press(Message::PageSelected(2)).into(),
        ],
        ui::button_group::ButtonGroupOrientation::Horizontal,
        &theme,
    );
    let grouped_input = ui::input_group::input_group(
        Some(iced::widget::text("@").into()),
        ui::input_group::group_input("username", "", &theme).on_input(Message::EmailChanged),
        None,
        ui::input::InputVariant::Default,
        &theme,
    );
    let chat_message = ui::message::message(
        ui::message::MessageSide::Incoming,
        Some(ui::avatar::avatar_fallback("D", ui::avatar::AvatarSize::Small, &theme).into()),
        None,
        ui::bubble::bubble(
            iced::widget::text("Hello"),
            ui::bubble::BubbleVariant::Incoming,
            &theme,
        ),
        None,
        &theme,
    );
    let transcript = ui::message_scroller::controlled_message_scroller(
        message_scroller,
        [
            ui::message_scroller::message_scroller_item(
                "today",
                ui::marker::marker(
                    None,
                    "Today",
                    ui::marker::MarkerVariant::Separator,
                    &theme,
                ),
            )
            .scroll_anchor(true),
            ui::message_scroller::message_scroller_item("message-1", chat_message),
        ],
        Message::TranscriptScroll,
        &theme,
    )
    .height(160);
    let rows = [("Button", "Shipped"), ("Dialog", "Planned")];
    let table_theme_a = theme;
    let table_theme_b = theme;
    let table = ui::table::table(
        [
            ui::table::column(ui::table::header("Name", &theme), move |row: (&'static str, &'static str)| {
                ui::table::cell(iced::widget::text(row.0), &table_theme_a)
            }),
            ui::table::column(ui::table::header("Status", &theme), move |row: (&'static str, &'static str)| {
                ui::table::cell(iced::widget::text(row.1), &table_theme_b)
            }),
        ],
        rows,
        &theme,
    );
    let rtl = ui::direction::directed_row(
        [iced::widget::text("one").into(), iced::widget::text("two").into()],
        ui::direction::Direction::RightToLeft,
    );

    iced::widget::column![
        ui::card::card(field, &theme).max_width(480),
        ui::badge::badge("Operational", ui::badge::BadgeVariant::Success, &theme)
            .size(ui::badge::BadgeSize::Small)
            .dot(),
        ui::alert::alert(
            iced::widget::text("Saved"),
            ui::alert::AlertVariant::Success,
            &theme,
        ),
        ui::checkbox::checkbox("Accepted", false, &theme).on_toggle(Message::Accepted),
        ui::progress::progress(42.0, ui::progress::ProgressVariant::Default, &theme),
        ui::empty_state::empty_state(
            Some(ui::badge::badge("New", ui::badge::BadgeVariant::Default, &theme).into()),
            "Nothing here",
            "Create the first item.",
            &theme,
        ),
        notes,
        sections,
        crumbs,
        pages,
        row_item,
        ratio,
        ui::label::label("Visible label", &theme),
        ui::skeleton::skeleton(&theme).width(100).height(12),
        ui::typography::typography("Heading", ui::typography::TextRole::H2, &theme),
        ui::typography::inline_code("cargo check", &theme),
        grouped_buttons,
        grouped_input,
        ui::attachment::attachment(None, "notes.txt", Some("1 KB"), None, &theme),
        transcript,
        ui::table::caption("Components", &theme),
        ui::table::frame(table, &theme),
        ui::scroll_area::scroll_area(iced::widget::text("Scrollable"), &theme).height(40),
        ui::spinner::spinner(ui::spinner::next_frame(0, false), false, &theme),
        rtl,
    ]
    .into()
}

fn main() {
    let _ = seed_message_scroller();
}
"#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(["test", "--quiet", "--no-run"])
        .current_dir(project.path())
        .env("CARGO_TARGET_DIR", project.path().join("target"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "installed source and its tests did not compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
