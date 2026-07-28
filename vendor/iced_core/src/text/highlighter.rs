//! Highlight text.
use crate::text::{Highlight, LineHeight};
use crate::{Color, Padding, Pixels};

use std::ops::Range;

/// A type capable of highlighting text.
///
/// A [`Highlighter`] highlights lines in sequence. When a line changes,
/// it must be notified and the lines after the changed one must be fed
/// again to the [`Highlighter`].
pub trait Highlighter: 'static {
    /// The settings to configure the [`Highlighter`].
    type Settings: PartialEq + Clone;

    /// The output of the [`Highlighter`].
    type Highlight;

    /// The highlight iterator type.
    type Iterator<'a>: Iterator<Item = (Range<usize>, Self::Highlight)>
    where
        Self: 'a;

    /// Creates a new [`Highlighter`] from its [`Self::Settings`].
    fn new(settings: &Self::Settings) -> Self;

    /// Updates the [`Highlighter`] with some new [`Self::Settings`].
    fn update(&mut self, new_settings: &Self::Settings);

    /// Notifies the [`Highlighter`] that the line at the given index has changed.
    fn change_line(&mut self, line: usize);

    /// Highlights the given line.
    ///
    /// If a line changed prior to this, the first line provided here will be the
    /// line that changed.
    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_>;

    /// Returns the current line of the [`Highlighter`].
    ///
    /// If `change_line` has been called, this will normally be the least index
    /// that changed.
    fn current_line(&self) -> usize;
}

/// A highlighter that highlights nothing.
#[derive(Debug, Clone, Copy)]
pub struct PlainText;

impl Highlighter for PlainText {
    type Settings = ();
    type Highlight = ();

    type Iterator<'a> = std::iter::Empty<(Range<usize>, Self::Highlight)>;

    fn new(_settings: &Self::Settings) -> Self {
        Self
    }

    fn update(&mut self, _new_settings: &Self::Settings) {}

    fn change_line(&mut self, _line: usize) {}

    fn highlight_line(&mut self, _line: &str) -> Self::Iterator<'_> {
        std::iter::empty()
    }

    fn current_line(&self) -> usize {
        usize::MAX
    }
}

/// The format of some text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Format<Font> {
    /// The [`Color`] of the text.
    pub color: Option<Color>,
    /// The `Font` of the text.
    pub font: Option<Font>,
    /// The size of the text.
    pub size: Option<Pixels>,
    /// The line height of the text.
    pub line_height: Option<LineHeight>,
    /// The background highlight of the text.
    pub highlight: Option<Highlight>,
    /// The background highlight of the whole visual line.
    pub line_highlight: Option<Highlight>,
    /// The color of a strikethrough decoration.
    pub strikethrough: Option<Color>,
    /// The padding of the text background highlight.
    pub padding: Padding,
}

impl<Font> Default for Format<Font> {
    fn default() -> Self {
        Self {
            color: None,
            font: None,
            size: None,
            line_height: None,
            highlight: None,
            line_highlight: None,
            strikethrough: None,
            padding: Padding::ZERO,
        }
    }
}
