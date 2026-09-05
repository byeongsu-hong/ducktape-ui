use iced::advanced::renderer::Headless;
use iced::advanced::{Widget, layout, text, widget};
use iced::widget::text_editor::Content;
use iced::{Color, Font, Length, Pixels, Size, Theme};
use std::ops::Range;
use ui_lang_runtime::{ContentVersion, RichTextEditor, rich_text_editor::Format};

mod common;
use common::clean_window;

#[derive(Default)]
struct WholeLine {
    current_line: usize,
}

impl text::Highlighter for WholeLine {
    type Settings = ();
    type Highlight = ();
    type Iterator<'a> = std::iter::Once<(Range<usize>, ())>;

    fn new(_: &Self::Settings) -> Self {
        Self::default()
    }

    fn update(&mut self, _: &Self::Settings) {}

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

#[test]
#[ignore = "rich-text editor allocation contract run explicitly in CI"]
fn performance_contract_format_relayout_reuses_line_segment_storage() {
    const LINES: usize = 4_096;
    const ALLOCATIONS: usize = 1_528;
    const ALLOCATED_BYTES: usize = 425_554;

    let source = (0..LINES)
        .map(|index| format!("line {index}\n"))
        .collect::<String>();
    let content = Content::with_text(&source);
    let renderer = iced_test::futures::futures::executor::block_on(
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")),
    )
    .expect("headless renderer");
    let limits = layout::Limits::new(Size::ZERO, Size::new(800.0, 600.0));
    let version = ContentVersion::new(1, 0);
    let mut initial = RichTextEditor::<_, ()>::new(&content, version)
        .width(Length::Fixed(800.0))
        .height(Length::Fixed(600.0))
        .wrapping(text::Wrapping::None)
        .highlight_with::<WholeLine>((), 0, |_| Format::default());
    let mut tree = widget::Tree::new(&initial as &dyn Widget<_, Theme, iced::Renderer>);
    initial.layout(&mut tree, &renderer, &limits);
    drop(initial);

    let mut changed = RichTextEditor::<_, ()>::new(&content, version)
        .width(Length::Fixed(800.0))
        .height(Length::Fixed(600.0))
        .wrapping(text::Wrapping::None)
        .highlight_with::<WholeLine>((), 1, |_| Format {
            color: Some(Color::BLACK),
            ..Format::default()
        });
    let stats = clean_window((ALLOCATIONS, ALLOCATED_BYTES), || {
        let node = std::hint::black_box(&mut changed).layout(&mut tree, &renderer, &limits);
        assert_eq!(node.size(), Size::new(800.0, 600.0));
    });

    eprintln!(
        "{LINES}-line format relayout: allocations={} bytes={}",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, ALLOCATIONS);
    assert_eq!(stats.bytes_allocated, ALLOCATED_BYTES);
}
