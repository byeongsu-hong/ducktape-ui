//! Data-backed formatting for an editor whose syntax policy lives elsewhere.
use super::Format;
use iced::advanced::text::Highlighter;
use std::{ops::Range, sync::Arc};

/// Formatting ranges use UTF-8 byte offsets within one logical source line.
#[derive(Clone, Debug, PartialEq)]
pub struct PresentedLine {
    pub line: usize,
    pub spans: Vec<(Range<usize>, Format)>,
}

/// A sparse, ordered presentation of logical lines. Unlisted lines use the
/// editor's base format. Changing the presentation restarts highlighting.
pub struct PresentationHighlighter {
    lines: Arc<[PresentedLine]>,
    line: usize,
}

impl Highlighter for PresentationHighlighter {
    type Settings = Arc<[PresentedLine]>;
    type Highlight = Format;
    type Iterator<'a> = std::iter::Cloned<std::slice::Iter<'a, (Range<usize>, Format)>>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            lines: settings.clone(),
            line: 0,
        }
    }

    fn update(&mut self, settings: &Self::Settings) {
        self.lines = settings.clone();
        self.line = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.line = self.line.min(line);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let spans = self
            .lines
            .binary_search_by_key(&self.line, |entry| entry.line)
            .ok()
            .map(|index| self.lines[index].spans.as_slice())
            .unwrap_or(&[]);
        self.line += 1;
        // Never hand the shaper a partial UTF-8 scalar, an inverted range or
        // stale offsets extending beyond this line. Reject the whole line so
        // malformed hiding and visible spans cannot disagree about geometry.
        if spans.iter().all(|(range, _)| {
            range.start <= range.end
                && line.is_char_boundary(range.start)
                && line.is_char_boundary(range.end)
        }) {
            spans.iter().cloned()
        } else {
            [].iter().cloned()
        }
    }

    fn current_line(&self) -> usize {
        self.line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{Color, Pixels};

    fn hidden() -> Format {
        Format {
            size: Some(Pixels(0.01)),
            color: Some(Color::TRANSPARENT),
            ..Format::default()
        }
    }

    #[test]
    fn sparse_lines_hide_source_markers_without_changing_source_offsets() {
        let lines = Arc::from([PresentedLine {
            line: 1,
            spans: vec![(0..2, hidden())],
        }]);
        let mut highlighter = PresentationHighlighter::new(&lines);
        assert_eq!(highlighter.highlight_line("Title").count(), 0);
        assert_eq!(
            highlighter.highlight_line("- 한글").collect::<Vec<_>>(),
            vec![(0..2, hidden())]
        );
        assert_eq!(highlighter.highlight_line("ordinary").count(), 0);
        assert_eq!(highlighter.current_line(), 3);
        highlighter.change_line(1);
        assert_eq!(highlighter.highlight_line("- 한글").count(), 1);
    }

    #[test]
    fn replacement_settings_restart_from_the_first_line() {
        let mut highlighter = PresentationHighlighter::new(&Arc::from([]));
        highlighter.highlight_line("old").for_each(drop);
        highlighter.update(&Arc::from([PresentedLine {
            line: 0,
            spans: vec![(0..0, hidden())],
        }]));
        assert_eq!(highlighter.current_line(), 0);
        assert_eq!(
            highlighter.highlight_line("").collect::<Vec<_>>(),
            vec![(0..0, hidden())]
        );
    }

    #[test]
    fn malformed_ranges_never_hide_unrelated_source() {
        for range in [1..3, 0..10, Range { start: 3, end: 0 }] {
            let lines = Arc::from([PresentedLine {
                line: 0,
                spans: vec![(range, hidden())],
            }]);
            let mut highlighter = PresentationHighlighter::new(&lines);
            assert_eq!(highlighter.highlight_line("한글").count(), 0);
        }
    }
}
