ui_lang::include_app!("tests/cases/ui/wrapped_text_alignment.ice");

use ui_lang_runtime::testing::{Capture, Config, Driver, Location, Platform, Target};

const HERE: Location = Location::new("wrapped_text_alignment.rs", 1, 1, "wrapped glyph alignment");

// Read the rendered ink, not the paragraph rectangle: a paragraph can be in
// the right place while its shorter lines still use the wrong alignment.
fn ink_lines(capture: &Capture, target: &Target) -> Vec<(f64, f64, f64)> {
    let mut lines: Vec<(f64, f64, f64)> = Vec::new();
    let mut previous_y = None;
    for y in target.top() as u32..target.bottom() as u32 {
        let mut xs = (target.left() as u32..target.right() as u32).filter(|x| {
            let i = ((y * capture.width + x) * 4) as usize;
            capture.rgba[i] > 100 && capture.rgba[i + 1] > 100 && capture.rgba[i + 2] > 100
        });
        let Some(first) = xs.next() else { continue };
        let last = xs.next_back().unwrap_or(first);
        let left = f64::from(first) - target.left();
        let right = f64::from(last + 1) - target.left();
        if previous_y == Some(y - 1) {
            let line = lines.last_mut().unwrap();
            line.0 = line.0.min(left);
            line.1 = line.1.max(right);
        } else {
            lines.push((left, right, f64::from(y) - target.top()));
        }
        previous_y = Some(y);
    }
    lines
}

#[test]
fn plain_soft_wrapped_alignment() {
    check_wrapped_alignment("plain");
}

#[test]
fn rich_soft_wrapped_alignment() {
    check_wrapped_alignment("rich");
}

fn check_wrapped_alignment(kind: &str) {
    let mut driver = Driver::new(
        WrappedTextAlignment::__program(),
        Config::new(if kind == "plain" {
            "plain_soft_wrapped_alignment"
        } else {
            "rich_soft_wrapped_alignment"
        })
        .viewport(576.0, 480.0)
        .scale_factor(1.0)
        .locale("en-US")
        .platform(Platform::Linux)
        .reduced_motion(true),
    );
    let capture = driver.capture("wrapped_alignment", HERE);
    let natural = driver.target(
        &format!("WrappedTextAlignment/page/natural/{kind}-natural"),
        HERE,
    );
    let justified = driver.target(
        &format!("WrappedTextAlignment/page/natural/{kind}-natural-justified"),
        HERE,
    );
    assert!(
        (natural.width() - justified.width()).abs() < 0.01,
        "{kind}: hard-newline shrink text expanded from {} to {}",
        natural.width(),
        justified.width()
    );
    assert!(natural.width() < 60.0 && natural.text_height() >= 40.0);

    let left = driver.target(
        &format!("WrappedTextAlignment/page/{kind}/{kind}-left"),
        HERE,
    );
    let reference = ink_lines(&capture, &left);
    assert!(
        reference.len() >= 3,
        "{kind}: fixture must soft-wrap: {reference:?}"
    );
    for (name, vertical) in [("center", 0.5), ("right", 1.0), ("justified", 0.5)] {
        let target = driver.target(
            &format!("WrappedTextAlignment/page/{kind}/{kind}-{name}"),
            HERE,
        );
        let lines = ink_lines(&capture, &target);
        assert_eq!(lines.len(), reference.len(), "{kind}/{name}: line count");
        let shift = (target.height() - target.text_height()) * vertical;
        for (index, (line, original)) in lines.iter().zip(&reference).enumerate() {
            assert!(
                (line.2 - original.2 - shift).abs() <= 1.0,
                "{kind}/{name} line {index}: vertical ink {line:?}, reference {original:?}, shift {shift}"
            );
            match name {
                "center" => assert!(
                    ((line.0 + line.1) / 2.0 - target.width() / 2.0).abs() <= 2.0,
                    "{kind}: centered line {index} ink {line:?}"
                ),
                "right" => assert!(
                    (line.1 - target.width()).abs() <= 2.0,
                    "{kind}: right-aligned line {index} ink {line:?}"
                ),
                "justified" if index + 1 < lines.len() => {
                    assert!(
                        line.0 <= 2.0 && (line.1 - target.width()).abs() <= 2.0,
                        "{kind}: justified line {index} ink {line:?}"
                    );
                }
                "justified" => assert!(
                    (line.0 - original.0).abs() <= 1.0 && (line.1 - original.1).abs() <= 1.0,
                    "{kind}: last justified line must retain natural width: {line:?} vs {original:?}"
                ),
                _ => unreachable!(),
            }
        }
    }
}
