use super::Conformance;
use ducktape_ui::ui::theme::{LIGHT, Theme};
use iced::widget;
use iced_test::Simulator;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

const CONTRACT: &str = include_str!("../../expected/reference.json");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Contract {
    tolerances: Tolerances,
    cases: BTreeMap<String, Case>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tolerances {
    geometry_px: f32,
    color_channel: u8,
    changed_pixel_ratio: f32,
    changed_pixel_channel: u8,
}

#[derive(Deserialize)]
struct Case {
    size: [f32; 2],
    roles: BTreeMap<String, String>,
    style: Style,
    screenshot: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Style {
    background: String,
    foreground: String,
    border_color: String,
}

#[derive(Debug)]
struct Pixels {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[test]
fn ice_matches_the_approved_react_contract() {
    let contract: Contract = serde_json::from_str(CONTRACT).expect("valid web contract");
    let scratch = scratch_dir();
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("clear prior conformance scratch directory");
    }
    fs::create_dir_all(&scratch).expect("create conformance scratch directory");

    let errors = contract
        .cases
        .iter()
        .filter_map(|(id, case)| compare_case(id, case, &contract.tolerances, &scratch).err())
        .collect::<Vec<_>>();
    fs::remove_dir_all(&scratch).expect("remove conformance scratch directory");
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

fn compare_case(
    id: &str,
    case: &Case,
    tolerances: &Tolerances,
    scratch: &Path,
) -> Result<(), String> {
    compare_roles(id, case, tolerances, &LIGHT)?;

    let (mut app, _) = Conformance::__boot();
    app.case_id = id.to_owned();
    app.input_value = if id == "input.placeholder" {
        String::new()
    } else {
        "acme-research".to_owned()
    };
    let theme = app.__theme();
    let mut simulator = Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(800.0, 200.0),
        app.__view(),
    );
    let target_id = match id {
        "button.default" | "button.hover" => "button-primary".to_owned(),
        "typography.machine" => "machine-copy".to_owned(),
        _ => id.replace('.', "-"),
    };
    let widget_id = widget::Id::from(format!("Conformance/{target_id}"));
    let target = simulator
        .find(|candidate: iced_test::selector::Candidate<'_>| {
            let matching_text = match (&candidate, id) {
                (iced_test::selector::Candidate::Text { content, .. }, "typography.display") => {
                    *content == "Welcome to Ducktape"
                }
                (iced_test::selector::Candidate::Text { content, .. }, "typography.machine") => {
                    *content == "127.0.0.1:8844 · height 84,912"
                }
                _ => false,
            };
            (candidate.id() == Some(&widget_id) || matching_text)
                .then(|| iced_test::selector::Target::from(candidate))
        })
        .map_err(|error| format!("{id}: target lookup failed: {error}"))?;
    let bounds = target.bounds();

    if id == "button.hover" {
        simulator.point_at(bounds.center());
    } else if id == "input.focused" {
        simulator
            .click(iced_test::selector::id(widget::Id::from(format!(
                "Conformance/{target_id}"
            ))))
            .map_err(|error| format!("{id}: focus failed: {error}"))?;
    }

    close(
        id,
        "width",
        bounds.width,
        case.size[0],
        tolerances
            .geometry_px
            .max(if id.starts_with("typography.") {
                32.0
            } else {
                5.0
            }),
    )?;
    close(
        id,
        "height",
        bounds.height,
        case.size[1],
        tolerances.geometry_px,
    )?;

    let output = scratch.join(id.replace('.', "-"));
    simulator
        .snapshot(&theme)
        .map_err(|error| format!("{id}: snapshot failed: {error}"))?
        .matches_image(&output)
        .map_err(|error| format!("{id}: writing snapshot failed: {error}"))?;
    let actual_path = fs::read_dir(scratch)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_stem()
                .is_some_and(|stem| stem.to_string_lossy().starts_with(&id.replace('.', "-")))
        })
        .ok_or_else(|| format!("{id}: snapshot output not found"))?;
    let actual = crop(read_png(&actual_path)?, bounds, 2.0);
    let expected = read_png(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../expected")
            .join(&case.screenshot),
    )?;
    compare_pixels(id, &actual, &expected, tolerances)
}

fn compare_roles(
    id: &str,
    case: &Case,
    tolerances: &Tolerances,
    theme: &Theme,
) -> Result<(), String> {
    for (property, role) in &case.roles {
        let expected = match property.as_str() {
            "background" => parse_css_rgb(&case.style.background)?,
            "foreground" => parse_css_rgb(&case.style.foreground)?,
            "border" => parse_css_rgb(&case.style.border_color)?,
            property => return Err(format!("{id}: unknown paint property {property}")),
        };
        let actual =
            role_color(theme, role).ok_or_else(|| format!("{id}: unknown semantic role {role}"))?;
        for (channel, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
            if actual.abs_diff(expected) > tolerances.color_channel {
                return Err(format!(
                    "{id}: {property}/{role} channel {channel} differs: Ice {actual}, React {expected}"
                ));
            }
        }
    }
    Ok(())
}

fn role_color(theme: &Theme, role: &str) -> Option<[u8; 3]> {
    let palette = theme.palette;
    let color = match role {
        "foreground" => palette.foreground,
        "primary" => palette.primary,
        "card" => palette.card,
        "secondary" => palette.secondary,
        "primary_hover" => palette.primary_hover,
        "primary_foreground" => palette.primary_foreground,
        "secondary_foreground" => palette.secondary_foreground,
        "accent_foreground" => palette.accent_foreground,
        "disabled" => palette.disabled,
        "disabled_foreground" => palette.disabled_foreground,
        "border" => palette.border,
        "control_line" => palette.control_line,
        "ring" => palette.ring,
        "brand_background" => palette.brand_background,
        "brand_line" => palette.brand_line,
        _ => return None,
    };
    Some([
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
    ])
}

fn close(id: &str, field: &str, actual: f32, expected: f32, tolerance: f32) -> Result<(), String> {
    if (actual - expected).abs() <= tolerance {
        Ok(())
    } else {
        Err(format!(
            "{id}: {field} differs: Ice {actual:.3}, React {expected:.3}"
        ))
    }
}

fn parse_css_rgb(value: &str) -> Result<[u8; 3], String> {
    let body = value
        .strip_prefix("rgb(")
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| format!("unsupported CSS color {value}"))?;
    let channels = body
        .split(',')
        .map(|part| part.trim().parse::<u8>().map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    channels
        .try_into()
        .map_err(|_| format!("expected three CSS color channels in {value}"))
}

fn read_png(path: &Path) -> Result<Pixels, String> {
    let decoder = png::Decoder::new(BufReader::new(
        fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?,
    ));
    let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
    let mut bytes = vec![0; reader.output_buffer_size().ok_or("PNG is too large")?];
    let info = reader
        .next_frame(&mut bytes)
        .map_err(|error| error.to_string())?;
    let pixels = &bytes[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels
            .chunks_exact(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        color => {
            return Err(format!(
                "{}: unsupported PNG color type {color:?}",
                path.display()
            ));
        }
    };
    Ok(Pixels {
        width: info.width,
        height: info.height,
        rgba,
    })
}

fn crop(source: Pixels, bounds: iced::Rectangle, scale: f32) -> Pixels {
    let x = (bounds.x * scale).floor() as u32;
    let y = (bounds.y * scale).floor() as u32;
    let width = (bounds.width * scale).ceil() as u32;
    let height = (bounds.height * scale).ceil() as u32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for row in y..y + height {
        let start = ((row * source.width + x) * 4) as usize;
        rgba.extend_from_slice(&source.rgba[start..start + (width * 4) as usize]);
    }
    Pixels {
        width,
        height,
        rgba,
    }
}

fn compare_pixels(
    id: &str,
    actual: &Pixels,
    expected: &Pixels,
    tolerances: &Tolerances,
) -> Result<(), String> {
    if id.starts_with("typography.") {
        return compare_ink_coverage(id, actual, expected, tolerances.changed_pixel_channel);
    }
    let width = actual.width.min(expected.width);
    let height = actual.height.min(expected.height);
    let area = actual.width.max(expected.width) * actual.height.max(expected.height);
    let mut changed = area - width * height;
    for y in 0..height {
        for x in 0..width {
            let actual_index = ((y * actual.width + x) * 4) as usize;
            let expected_index = ((y * expected.width + x) * 4) as usize;
            if (0..3).any(|channel| {
                actual.rgba[actual_index + channel]
                    .abs_diff(expected.rgba[expected_index + channel])
                    > tolerances.changed_pixel_channel
            }) {
                changed += 1;
            }
        }
    }
    let ratio = changed as f32 / area as f32;
    if ratio <= tolerances.changed_pixel_ratio {
        Ok(())
    } else {
        Err(format!(
            "{id}: screenshot differs in {:.2}% of pixels (limit {:.2}%)",
            ratio * 100.0,
            tolerances.changed_pixel_ratio * 100.0
        ))
    }
}

fn compare_ink_coverage(
    id: &str,
    actual: &Pixels,
    expected: &Pixels,
    threshold: u8,
) -> Result<(), String> {
    let coverage = |image: &Pixels| {
        let background = [image.rgba[0], image.rgba[1], image.rgba[2]];
        image
            .rgba
            .chunks_exact(4)
            .filter(|pixel| {
                (0..3).any(|channel| pixel[channel].abs_diff(background[channel]) > threshold)
            })
            .count() as f32
            / (image.width * image.height) as f32
    };
    let actual = coverage(actual);
    let expected = coverage(expected);
    let delta = (actual - expected).abs() / expected.max(f32::EPSILON);
    if actual > 0.0 && delta <= 0.35 {
        Ok(())
    } else {
        Err(format!(
            "{id}: text ink coverage differs: Ice {:.2}%, React {:.2}%",
            actual * 100.0,
            expected * 100.0
        ))
    }
}

fn scratch_dir() -> PathBuf {
    std::env::temp_dir().join(format!("ice-ui-conformance-{}", std::process::id()))
}
