//! Native text layout for copied guest values.
use super::*;
use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};

static FAMILIES: OnceLock<RwLock<HashSet<&'static str>>> = OnceLock::new();

/// Register a host-owned family name after arranging for its font data to load.
/// Guest strings only look up this registry; they never become permanent names.
pub fn register_font_family(family: &'static str) {
    FAMILIES
        .get_or_init(Default::default)
        .write()
        .expect("font families")
        .insert(family);
}

pub(super) fn named_font(face: &wire::NamedFont) -> iced::Font {
    use iced::font::{Family, Stretch, Style};
    let family = match &face.family {
        wire::FontFamily::Named(name) => FAMILIES
            .get_or_init(Default::default)
            .read()
            .expect("font families")
            .get(name.as_str())
            .copied()
            .map(Family::Name)
            .unwrap_or(Family::SansSerif),
        wire::FontFamily::Serif => Family::Serif,
        wire::FontFamily::SansSerif => Family::SansSerif,
        wire::FontFamily::Cursive => Family::Cursive,
        wire::FontFamily::Fantasy => Family::Fantasy,
        wire::FontFamily::Monospace => Family::Monospace,
    };
    let stretch = match face.stretch {
        wire::FontStretch::UltraCondensed => Stretch::UltraCondensed,
        wire::FontStretch::ExtraCondensed => Stretch::ExtraCondensed,
        wire::FontStretch::Condensed => Stretch::Condensed,
        wire::FontStretch::SemiCondensed => Stretch::SemiCondensed,
        wire::FontStretch::Normal => Stretch::Normal,
        wire::FontStretch::SemiExpanded => Stretch::SemiExpanded,
        wire::FontStretch::Expanded => Stretch::Expanded,
        wire::FontStretch::ExtraExpanded => Stretch::ExtraExpanded,
        wire::FontStretch::UltraExpanded => Stretch::UltraExpanded,
    };
    let style = match face.style {
        wire::FontStyle::Normal => Style::Normal,
        wire::FontStyle::Italic => Style::Italic,
        wire::FontStyle::Oblique => Style::Oblique,
    };
    iced::Font {
        family,
        stretch,
        style,
        ..font(wire::Font {
            monospace: false,
            weight: face.weight,
        })
    }
}

pub(super) fn render(node: &wire::Node) -> IceElement<'static, Output> {
    let wire::Node::Text {
        key,
        content,
        size,
        color: fg,
        font: face,
        width,
        align_x,
        options,
    } = node
    else {
        unreachable!("text node")
    };
    let face = options
        .font
        .as_ref()
        .map(named_font)
        .unwrap_or_else(|| font(*face));
    let glyph = |value: String| {
        let mut text = widget::text(value).font(face);
        if let Some(size) = size {
            text = text.size(size.max(f32::EPSILON));
        }
        if let Some(fg) = fg {
            text = text.color(color(*fg));
        }
        if let Some(line_height) = options.line_height {
            text = text.line_height(match line_height {
                wire::LineHeight::Relative(value) => widget::text::LineHeight::Relative(value),
                wire::LineHeight::Absolute(value) => {
                    widget::text::LineHeight::Absolute(value.into())
                }
            });
        }
        if let Some(shaping) = options.shaping {
            text = text.shaping(match shaping {
                wire::Shaping::Auto => widget::text::Shaping::Auto,
                wire::Shaping::Basic => widget::text::Shaping::Basic,
                wire::Shaping::Advanced => widget::text::Shaping::Advanced,
            });
        }
        if let Some(wrapping) = options.wrapping {
            text = text.wrapping(match wrapping {
                wire::Wrapping::None => widget::text::Wrapping::None,
                wire::Wrapping::Word => widget::text::Wrapping::Word,
                wire::Wrapping::Glyph => widget::text::Wrapping::Glyph,
                wire::Wrapping::WordOrGlyph => widget::text::Wrapping::WordOrGlyph,
            });
        }
        text
    };
    let content_widget: IceElement<'static, Output> = if options.tracking > 0.0 {
        let children: Vec<IceElement<'static, Output>> = crate::graphemes(content)
            .map(|part| glyph(part.to_owned()).into())
            .collect();
        let spacing = bounded_spacing(f64::from(options.tracking), children.len());
        let mut run = widget::container(widget::row(children).spacing(spacing));
        if let Some(width) = width {
            run = run.width(length(*width));
        }
        if let Some(height) = options.height {
            run = run.height(length(height));
        }
        if let Some(align) = align_x {
            run = run.align_x(horizontal(*align));
        }
        if let Some(align) = options.align_y {
            run = run.align_y(vertical(align));
        }
        run.into()
    } else {
        let mut text = glyph(content.clone());
        if let Some(width) = width {
            text = text.width(length(*width));
        }
        if let Some(height) = options.height {
            text = text.height(length(height));
        }
        if let Some(align) = align_x {
            text = text.align_x(horizontal(*align));
        }
        if let Some(align) = options.align_y {
            text = text.align_y(vertical(align));
        }
        crate::selectable_text(text).into()
    };
    accessible(content_widget, StableId::new(key), Role::Label)
        .logical_id_maybe(cfg!(test).then_some(key.as_str()))
        .value(content.clone())
        .into()
}
