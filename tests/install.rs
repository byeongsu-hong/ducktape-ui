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
        ],
        project.path(),
    )
    .unwrap();
    assert!(output.contains("theme.rs"));
    assert!(output.contains("button.rs"));
    assert!(output.contains("surface.rs"));
    assert!(output.contains("field.rs"));
    assert!(output.contains("segmented_control.rs"));

    fs::write(
        project.path().join("src/main.rs"),
        r#"mod ui;

#[derive(Debug, Clone)]
enum Message {
    EmailChanged(String),
    SectionSelected(Section),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    General,
    Advanced,
}

fn view() -> iced::Element<'static, Message> {
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

    iced::widget::column![
        ui::card::card(field, &theme).max_width(480),
        ui::badge::badge("Operational", ui::badge::BadgeVariant::Success, &theme)
            .size(ui::badge::BadgeSize::Small)
            .dot(),
        sections,
    ]
    .into()
}

fn main() {}
"#,
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(project.path())
        .env("CARGO_TARGET_DIR", project.path().join("target"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "installed source did not compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
