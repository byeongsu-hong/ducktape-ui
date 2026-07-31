use serde_json::Value;
use std::fs;
use std::process::{Command, Output};
use tempfile::TempDir;

fn run(project: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-ice"))
        .arg("ice")
        .args(args)
        .current_dir(project.path())
        .output()
        .unwrap()
}

fn write_project() -> TempDir {
    let project = TempDir::new().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        "[package]\nname = \"api-fixture\"\nversion = \"3.2.1\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(project.path().join("src/lib.rs"), "").unwrap();
    project
}

fn interface(component: &str) -> String {
    format!(
        r#"theme contract AppTheme
  bg
  fg
  primary
  danger
palette light for AppTheme
  bg #000000
  fg #ffffff
  primary #112233
  danger #ff0000
{component}
"#
    )
}

#[test]
fn api_diff_is_machine_readable_and_breaking_changes_exit_nonzero() {
    let project = write_project();
    let api = project.path().join("api.ice");
    fs::write(
        &api,
        interface("component Card(title:str=\"Draft\")\n  space"),
    )
    .unwrap();

    let baseline = run(&project, &["api", "api.ice"]);
    assert!(
        baseline.status.success(),
        "{}",
        String::from_utf8_lossy(&baseline.stderr)
    );
    let baseline_json: Value = serde_json::from_slice(&baseline.stdout).unwrap();
    assert_eq!(baseline_json["schema_version"], 1);
    assert_eq!(baseline_json["language_revision"], "2.0");
    assert_eq!(baseline_json["package"]["name"], "api-fixture");
    assert_eq!(baseline_json["package"]["version"], "3.2.1");
    fs::write(project.path().join("baseline.json"), &baseline.stdout).unwrap();

    fs::write(
        &api,
        interface("component Card(count:i64, title:str=\"Draft\")\n  space"),
    )
    .unwrap();
    let current = run(&project, &["api", "api.ice"]);
    assert!(current.status.success());
    fs::write(project.path().join("current.json"), &current.stdout).unwrap();

    let machine = run(
        &project,
        &[
            "api",
            "diff",
            "baseline.json",
            "current.json",
            "--format",
            "json",
        ],
    );
    assert!(!machine.status.success());
    let report: Value = serde_json::from_slice(&machine.stdout).unwrap();
    assert_eq!(report["summary"]["breaking"], 1);
    assert_eq!(report["changes"][0]["classification"], "breaking");
    assert_eq!(report["changes"][0]["code"], "required_prop_added");
    assert!(String::from_utf8_lossy(&machine.stderr).contains("breaking change"));

    let human = run(&project, &["api", "diff", "baseline.json", "current.json"]);
    assert!(!human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("1 breaking"));
    assert!(human.contains("[BREAKING] components.Card.props.count"));
}
