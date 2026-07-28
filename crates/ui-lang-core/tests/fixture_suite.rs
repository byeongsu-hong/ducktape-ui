mod support;

use support::{assert_contains, cases};
use ui_lang_core::{analyze, compile, format_source};

#[test]
fn format_cases() {
    for case in cases("format") {
        assert_eq!(
            format_source(&case.read("as-is.ice")).unwrap(),
            case.read("to-be.ice"),
            "{}",
            case.name()
        );
    }
}

#[test]
fn diagnostic_cases() {
    for case in cases("diagnostic") {
        let error = analyze(&case.read("as-is.ice")).unwrap_err();
        assert_contains(&case, &format!("{}\n{}", error.code, error.message));
    }
}

#[test]
fn warning_cases() {
    for case in cases("warning") {
        let document = analyze(&case.read("as-is.ice")).unwrap();
        let warnings = document
            .warnings()
            .iter()
            .map(|warning| format!("{} {}", warning.code, warning.message))
            .collect::<Vec<_>>()
            .join("\n");
        assert_contains(&case, &warnings);
    }
}

#[test]
fn compile_cases() {
    for case in cases("compile") {
        let generated = compile(&case.read("as-is.ice"), "as-is.ice").unwrap();
        assert_contains(&case, &generated);
    }
}
