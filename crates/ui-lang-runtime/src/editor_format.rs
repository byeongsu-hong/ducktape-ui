//! Graphics-independent editor span formatting.

use iced::advanced::text;
use iced::{Color, Font, Padding, Pixels};

/// Visual formatting for a highlighted source range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Format {
    /// Text color override.
    pub color: Option<Color>,
    /// Font override.
    pub font: Option<Font>,
    /// Font size override.
    pub size: Option<Pixels>,
    /// Line height override.
    pub line_height: Option<text::LineHeight>,
    /// Background drawn around the formatted span.
    pub highlight: Option<text::Highlight>,
    /// Background drawn across every visual line containing the range.
    pub line_highlight: Option<text::Highlight>,
    /// Layout padding inside [`Self::line_highlight`].
    pub line_padding: Padding,
    /// A full-width horizontal rule painted across the line's vertical center
    /// — what a markdown divider renders as when its `---` glyphs are hidden.
    pub line_rule: Option<Color>,
    /// Strikethrough color.
    pub strikethrough: Option<Color>,
    /// Extra paint-only padding around [`Self::highlight`].
    pub padding: Padding,
}

impl Default for Format {
    fn default() -> Self {
        Self {
            color: None,
            font: None,
            size: None,
            line_height: None,
            highlight: None,
            line_highlight: None,
            line_padding: Padding::ZERO,
            line_rule: None,
            strikethrough: None,
            padding: Padding::ZERO,
        }
    }
}

#[cfg(any(feature = "tiny-skia", feature = "wgpu"))]
impl Format {
    pub(crate) fn overlay(self, overlay: Self) -> Self {
        Self {
            color: overlay.color.or(self.color),
            font: overlay.font.or(self.font),
            size: overlay.size.or(self.size),
            line_height: overlay.line_height.or(self.line_height),
            highlight: overlay.highlight.or(self.highlight),
            line_highlight: overlay.line_highlight.or(self.line_highlight),
            line_padding: if overlay.line_padding == Padding::ZERO {
                self.line_padding
            } else {
                overlay.line_padding
            },
            line_rule: overlay.line_rule.or(self.line_rule),
            strikethrough: overlay.strikethrough.or(self.strikethrough),
            padding: if overlay.padding == Padding::ZERO {
                self.padding
            } else {
                overlay.padding
            },
        }
    }
}
