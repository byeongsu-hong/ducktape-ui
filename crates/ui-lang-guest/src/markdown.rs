//! Markdown state keeps its source for host rendering and the native parser
//! for incremental updates and the pure `markdown_images` query.
use iced::widget::markdown::Content;
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct Markdown {
    source: String,
    parsed: Content,
}

impl Markdown {
    pub fn parse(source: &str) -> Self {
        Self {
            source: source.to_owned(),
            parsed: Content::parse(source),
        }
    }

    pub fn surface_value(
        &self,
        settings: iced::widget::markdown::Settings,
        palette: iced::theme::Palette,
    ) -> crate::wire::SurfaceValue {
        let style = settings.style;
        assert!(
            [style.font, style.inline_code_font, style.code_block_font]
                .into_iter()
                .all(|font| font == iced::Font::DEFAULT || font == iced::Font::MONOSPACE),
            "the tree emitter rejects named markdown fonts"
        );
        let color = |c: iced::Color| [c.r, c.g, c.b, c.a];
        let iced::Background::Color(background) = style.inline_code_highlight.background else {
            unreachable!("the tree emitter rejects markdown gradients");
        };
        let border = style.inline_code_highlight.border;
        crate::wire::MarkdownDocument {
            source: self.source.clone(),
            palette: [
                palette.background,
                palette.text,
                palette.primary,
                palette.success,
                palette.warning,
                palette.danger,
            ]
            .map(color),
            metrics: [
                settings.text_size.0,
                settings.h1_size.0,
                settings.h2_size.0,
                settings.h3_size.0,
                settings.h4_size.0,
                settings.h5_size.0,
                settings.h6_size.0,
                settings.code_size.0,
                settings.spacing.0,
            ],
            monospace: [style.font, style.inline_code_font, style.code_block_font]
                .map(|font| font.family == iced::font::Family::Monospace),
            colors: [
                background,
                style.inline_code_color,
                style.link_color,
                border.color,
            ]
            .map(color),
            padding: [
                style.inline_code_padding.top,
                style.inline_code_padding.right,
                style.inline_code_padding.bottom,
                style.inline_code_padding.left,
            ],
            border_width: border.width,
            radii: [
                border.radius.top_left,
                border.radius.top_right,
                border.radius.bottom_right,
                border.radius.bottom_left,
            ],
        }
        .into_surface_value()
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn push_str(&mut self, text: &str) {
        self.source.push_str(text);
        self.parsed.push_str(text);
    }

    pub fn images(&self) -> &HashSet<String> {
        self.parsed.images()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appending_markdown_preserves_source_and_image_queries() {
        let first = "# Title\n\n![first](asset:first)\n";
        let next = "\n![second](asset:second)\n";
        let mut content = Markdown::parse(first);
        assert_eq!(content.source(), first);
        assert!(content.images().contains("asset:first"));
        content.push_str(next);
        assert_eq!(
            content.source(),
            format!("{first}{next}"),
            "appended source must reach the host"
        );
        assert_eq!(
            content.images(),
            &HashSet::from(["asset:first".into(), "asset:second".into()])
        );
        let replacement = Markdown::parse("replacement");
        assert_eq!(replacement.source(), "replacement");
        assert!(replacement.images().is_empty());
    }
}
