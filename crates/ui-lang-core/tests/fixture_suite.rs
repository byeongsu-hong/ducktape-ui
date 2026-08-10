mod support;

use support::{assert_contains, cases};
use ui_lang_core::{analyze, compile, format_fragment, format_source};

#[test]
fn format_cases() {
    for case in cases("format") {
        let source = case.read("as-is.ice");
        // The formatter also runs on sources the parser rejects and must
        // preserve what it cannot represent.
        let formatted = format_source(&source).unwrap_or_else(|_| format_fragment(&source));
        assert_eq!(formatted, case.read("to-be.ice"), "{}", case.name());
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
