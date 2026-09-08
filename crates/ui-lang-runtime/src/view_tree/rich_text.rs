//! One native paragraph for all copied spans, including link hit testing.
use super::*;

pub(super) fn render(node: &wire::Node) -> IceElement<'static, Output> {
    let wire::Node::RichText {
        key,
        spans,
        size,
        color: fg,
        font: face,
        width,
        align_x,
        options,
        on_link,
    } = node
    else {
        unreachable!("rich text node")
    };
    let values = spans
        .iter()
        .map(|span| {
            let mut native = widget::span(span.content.clone());
            if let Some(size) = span.size {
                native = native.size(size.max(f32::EPSILON));
            }
            if let Some(height) = span.line_height {
                native = native.line_height(line_height(height));
            }
            if let Some(font) = &span.font {
                native = native.font(text::named_font(font));
            }
            if let Some(fg) = span.color {
                native = native.color(color(fg));
            }
            if let Some(link) = &span.link {
                native = native.link(link.clone());
            }
            if let Some(bg) = span.background {
                native = native.background(color(bg));
            }
            if let Some(edge) = span.border {
                native = native.border(border(edge));
            }
            if let Some(edges) = span.padding {
                native = native.padding(padding(edges));
            }
            native
                .underline(span.underline)
                .strikethrough(span.strikethrough)
        })
        .collect::<Vec<widget::text::Span<'static, String>>>();
    let face = options
        .font
        .as_ref()
        .map(text::named_font)
        .unwrap_or_else(|| font(*face));
    let mut rich = widget::rich_text(values).font(face);
    if let Some(size) = size {
        rich = rich.size(size.max(f32::EPSILON));
    }
    if let Some(fg) = fg {
        rich = rich.color(color(*fg));
    }
    if let Some(width) = width {
        rich = rich.width(length(*width));
    }
    if let Some(height) = options.height {
        rich = rich.height(length(height));
    }
    if let Some(align) = align_x {
        rich = rich.align_x(horizontal(*align));
    }
    if let Some(align) = options.align_y {
        rich = rich.align_y(vertical(align));
    }
    if let Some(height) = options.line_height {
        rich = rich.line_height(line_height(height));
    }
    if let Some(wrapping) = options.wrapping {
        rich = rich.wrapping(match wrapping {
            wire::Wrapping::None => widget::text::Wrapping::None,
            wire::Wrapping::Word => widget::text::Wrapping::Word,
            wire::Wrapping::Glyph => widget::text::Wrapping::Glyph,
            wire::Wrapping::WordOrGlyph => widget::text::Wrapping::WordOrGlyph,
        });
    }
    if let Some(handler) = *on_link {
        rich = rich.on_link_click(move |text| Output::Link { handler, text });
    }
    accessible(rich, StableId::new(key), Role::Label)
        .logical_id_maybe(cfg!(test).then_some(key.as_str()))
        .value(
            spans
                .iter()
                .map(|span| span.content.as_str())
                .collect::<String>(),
        )
        .into()
}

fn line_height(height: wire::LineHeight) -> widget::text::LineHeight {
    match height {
        wire::LineHeight::Relative(value) => widget::text::LineHeight::Relative(value),
        wire::LineHeight::Absolute(value) => widget::text::LineHeight::Absolute(value.into()),
    }
}
