//! A selection lands on the glyphs the reader can see.
//!
//! `iced` draws a text at `bounds.anchor(min_bounds, align_x, align_y)`, so a
//! centred or bottom-aligned label sits somewhere inside its widget box rather
//! than at its top-left corner. `selectable_text` used to hit-test and
//! highlight against the corner, which put both a whole glyph box away from
//! the letters: dragging across visible text selected nothing, so nothing was
//! copied and nothing was highlighted.
//!
//! The oracle here is the paint. The glyph box is read back out of the
//! renderer — the same `visible_bounds` the Ice test harness reports as
//! `text_x`/`text_y` — and the drag is aimed at it, so the assertion is
//! "selection agrees with paint" rather than a repeat of the anchor
//! arithmetic. The text sits inside a padded container, so a passing run also
//! says the origin is not the window's.

use iced::advanced::clipboard::{self, Clipboard};
use iced::advanced::renderer::{self, Renderer as _};
use iced::alignment::Vertical;
use iced::keyboard::{self, Modifiers, key};
use iced::widget::{container, text};
use iced::{Element, Event, Font, Length, Pixels, Point, Rectangle, Size, Theme, mouse};
use iced_test::runtime::{UserInterface, user_interface};
use ui_lang_runtime::selectable_text;

type Renderer = iced_tiny_skia::Renderer;

const CONTENT: &str = "Aligned";
/// Wider and taller than the label, so both axes have room to align in.
const BOX: Size = Size::new(300.0, 120.0);
const VIEWPORT: Size = Size::new(400.0, 200.0);
/// The label's box is not the window's: a correct origin has to survive it.
const INSET: f32 = 24.0;

fn view(
    content: &'static str,
    align_x: text::Alignment,
    align_y: Vertical,
) -> Element<'static, (), Theme, Renderer> {
    container(selectable_text(
        text(content)
            .size(20.0)
            .width(Length::Fixed(BOX.width))
            .height(Length::Fixed(BOX.height))
            .align_x(align_x)
            .align_y(align_y),
    ))
    .padding(INSET)
    .into()
}

#[derive(Default)]
struct Recorder(Option<String>);

impl Clipboard for Recorder {
    fn read(&self, _kind: clipboard::Kind) -> Option<String> {
        self.0.clone()
    }

    fn write(&mut self, _kind: clipboard::Kind, contents: String) {
        self.0 = Some(contents);
    }
}

fn window() -> Rectangle {
    Rectangle::new(Point::ORIGIN, VIEWPORT)
}

fn paint(ui: &mut UserInterface<'_, (), Theme, Renderer>, renderer: &mut Renderer) {
    renderer.reset(window());
    ui.draw(
        renderer,
        &Theme::Light,
        &renderer::Style::default(),
        mouse::Cursor::Unavailable,
    );
}

/// Where the glyphs actually ended up, straight out of the renderer.
fn glyph_box(renderer: &mut Renderer) -> Rectangle {
    let mut painted: Option<Rectangle> = None;

    for layer in renderer.layers() {
        for group in &layer.text {
            let transformation = group.transformation();
            for text in group.as_slice() {
                let Some(bounds) = text.visible_bounds().map(|bounds| bounds * transformation)
                else {
                    continue;
                };
                painted = Some(match painted {
                    Some(union) => union.union(&bounds),
                    None => bounds,
                });
            }
        }
    }

    painted.expect("the label is painted")
}

/// The highlight. Nothing else in this view fills a quad.
fn highlight(renderer: &mut Renderer) -> Option<Rectangle> {
    let mut union: Option<Rectangle> = None;

    for layer in renderer.layers() {
        for (quad, _) in &layer.quads {
            union = Some(match union {
                Some(union) => union.union(&quad.bounds),
                None => quad.bounds,
            });
        }
    }

    union
}

struct Outcome {
    glyphs: Rectangle,
    copied: Option<String>,
    highlight: Option<Rectangle>,
}

/// Drags the pointer straight across the painted label, copies, and reports
/// what the reader would have got.
fn drag_across_the_label(align_x: text::Alignment, align_y: Vertical) -> Outcome {
    drag_across_text(CONTENT, align_x, align_y)
}

fn drag_across_text(content: &'static str, align_x: text::Alignment, align_y: Vertical) -> Outcome {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    let mut clipboard = Recorder::default();
    let mut messages = Vec::new();
    let mut ui = UserInterface::build(
        view(content, align_x, align_y),
        VIEWPORT,
        user_interface::Cache::default(),
        &mut renderer,
    );

    paint(&mut ui, &mut renderer);
    let glyphs = glyph_box(&mut renderer);
    assert!(
        highlight(&mut renderer).is_none(),
        "nothing is selected yet, so nothing may be highlighted"
    );

    // From the leading edge of the first line to beyond the final line:
    // the whole label lies under the drag. The
    // press has to land inside the widget's own box, so it starts on the
    // glyphs rather than beside them.
    let from = Point::new(glyphs.x + 1.0, glyphs.y + 1.0);
    let to = Point::new(
        glyphs.x + glyphs.width + 4.0,
        glyphs.y + glyphs.height + 4.0,
    );

    for (event, at) in [
        (
            Event::Mouse(mouse::Event::CursorMoved { position: from }),
            from,
        ),
        (
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            from,
        ),
        (Event::Mouse(mouse::Event::CursorMoved { position: to }), to),
        (
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            to,
        ),
        (
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character("c".into()),
                modified_key: keyboard::Key::Character("c".into()),
                physical_key: key::Physical::Code(key::Code::KeyC),
                location: keyboard::Location::Standard,
                modifiers: Modifiers::COMMAND,
                text: None,
                repeat: false,
            }),
            to,
        ),
    ] {
        let _ = ui.update(
            &[event],
            mouse::Cursor::Available(at),
            &mut renderer,
            &mut clipboard,
            &mut messages,
        );
    }

    paint(&mut ui, &mut renderer);

    Outcome {
        glyphs,
        copied: clipboard.0.clone(),
        highlight: highlight(&mut renderer),
    }
}

/// The highlight has to sit on the glyphs, not merely somewhere on screen.
fn assert_highlight_covers_the_glyphs(outcome: &Outcome, label: &str) {
    let highlight = outcome.highlight.unwrap_or_else(|| {
        panic!("{label}: the whole label is selected, so it must be highlighted")
    });
    let glyphs = outcome.glyphs;

    assert!(
        (highlight.x - glyphs.x).abs() <= 1.0 && (highlight.y - glyphs.y).abs() <= 1.0,
        "{label}: the highlight must start where the glyphs are painted: \
         expected ~({}, {}), got ({}, {})",
        glyphs.x,
        glyphs.y,
        highlight.x,
        highlight.y
    );
    assert!(
        highlight.width >= glyphs.width - 2.0 && highlight.height >= glyphs.height - 2.0,
        "{label}: the highlight must cover the glyphs it selects: \
         expected at least {} x {}, got {} x {}",
        glyphs.width,
        glyphs.height,
        highlight.width,
        highlight.height
    );
}

/// Horizontal: a centred label is drawn well right of its box's left edge, so
/// a hit test against that edge lands past the end of the line.
#[test]
fn a_centred_label_is_selected_where_it_is_painted() {
    let outcome = drag_across_the_label(text::Alignment::Center, Vertical::Top);

    assert!(
        outcome.glyphs.x > INSET + 1.0,
        "the fixture only means anything if centring moved the glyphs: \
         box starts at {INSET}, glyphs at {}",
        outcome.glyphs.x
    );
    assert_eq!(
        outcome.copied.as_deref(),
        Some(CONTENT),
        "dragging across the painted label must copy it"
    );
    assert_highlight_covers_the_glyphs(&outcome, "centred");
}

/// Vertical: a bottom-aligned label is drawn a whole box below its top edge,
/// so a hit test against that edge lands above the only line there is.
#[test]
fn a_bottom_right_label_is_selected_where_it_is_painted() {
    let outcome = drag_across_the_label(text::Alignment::Right, Vertical::Bottom);

    assert!(
        outcome.glyphs.y > INSET + 1.0 && outcome.glyphs.x > INSET + 1.0,
        "the fixture only means anything if the alignment moved the glyphs: \
         box starts at ({INSET}, {INSET}), glyphs at ({}, {})",
        outcome.glyphs.x,
        outcome.glyphs.y
    );
    assert_eq!(
        outcome.copied.as_deref(),
        Some(CONTENT),
        "dragging across the painted label must copy it"
    );
    assert_highlight_covers_the_glyphs(&outcome, "bottom-right");
}

/// A left/top label is the case that already worked, and it has to keep
/// working: the fix must not trade one origin for another.
#[test]
fn a_top_left_label_is_still_selected_where_it_is_painted() {
    let outcome = drag_across_the_label(text::Alignment::Left, Vertical::Top);

    assert_eq!(
        outcome.copied.as_deref(),
        Some(CONTENT),
        "dragging across the painted label must copy it"
    );
    assert_highlight_covers_the_glyphs(&outcome, "top-left");
}

#[test]
fn dragging_past_multiline_text_copies_the_last_words() {
    for content in [
        "First line\nlast words",
        "첫 번째 줄\r\n\r\n마지막 단어",
        "A longer line wraps softly before reaching its last words",
        "A longer line wraps softly before its end\nlast words",
    ] {
        let outcome = drag_across_text(content, text::Alignment::Left, Vertical::Top);
        assert_eq!(outcome.copied.as_deref(), Some(content));
        assert_highlight_covers_the_glyphs(&outcome, "multiline");
    }
}
