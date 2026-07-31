use super::*;

#[derive(Default)]
struct WholeLine {
    current_line: usize,
}

impl text::Highlighter for WholeLine {
    type Settings = ();
    type Highlight = ();
    type Iterator<'a> = std::iter::Once<(Range<usize>, ())>;

    fn new(_settings: &Self::Settings) -> Self {
        Self::default()
    }

    fn update(&mut self, _new_settings: &Self::Settings) {}

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.current_line += 1;
        std::iter::once((0..line.len(), ()))
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

struct CaretSizedMarker {
    current_line: usize,
    expanded: bool,
}

impl text::Highlighter for CaretSizedMarker {
    type Settings = bool;
    type Highlight = bool;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, bool)>;

    fn new(expanded: &Self::Settings) -> Self {
        Self {
            current_line: 0,
            expanded: *expanded,
        }
    }

    fn update(&mut self, expanded: &Self::Settings) {
        self.expanded = *expanded;
    }

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.current_line += 1;
        (!line.is_empty())
            .then_some((0..1, self.expanded))
            .into_iter()
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

fn test_layout_style(width: f32) -> LineLayoutStyle {
    LineLayoutStyle {
        width,
        font: Font::DEFAULT,
        text_size: Pixels(16.0),
        line_height: text::LineHeight::Relative(1.6),
        wrapping: text::Wrapping::Word,
    }
}

fn content_lines(content: &Content) -> Vec<String> {
    content.lines().map(|line| line.text.into_owned()).collect()
}

fn headless_renderer() -> iced::Renderer {
    use iced::advanced::renderer::Headless;

    iced_test::futures::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer")
}

#[path = "ime.rs"]
mod ime_tests;
#[path = "layout.rs"]
mod layout_tests;
#[path = "performance.rs"]
mod performance_tests;
#[path = "pointer.rs"]
mod pointer_tests;
#[path = "unicode.rs"]
mod unicode_tests;
