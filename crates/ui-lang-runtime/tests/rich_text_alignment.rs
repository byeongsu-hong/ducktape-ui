//! A rich text's decorations and links land on the glyphs the reader sees.
//!
//! `iced` paints a paragraph at `bounds.anchor(min_bounds, align_x, align_y)`,
//! so a centred, right- or bottom-aligned label sits somewhere inside its
//! widget box rather than at its top-left corner. `rich_text` translated span
//! decorations by `layout.position()` and hit-tested links against it, so an
//! aligned label underlined empty space and routed clicks from empty space:
//! `text ... align-x=center` with a `link` span — what Ice lowers in
//! `view_tree::rich_text` and `codegen::view::text` — was unclickable where it
//! was drawn and clickable where it was not.
//!
//! The oracle is the paint. The glyph box and the underline quad are both read
//! back out of the renderer, so an assertion is "decoration agrees with paint"
//! rather than a repeat of the anchor arithmetic. The label sits in a padded
//! container, so a passing run also says the origin is not the window's.

use iced::advanced::clipboard;
use iced::advanced::renderer::{self, Renderer as _};
use iced::alignment::Vertical;
use iced::widget::{container, rich_text, span, text};
use iced::{Element, Event, Font, Length, Pixels, Point, Rectangle, Size, Theme, mouse};
use iced_test::runtime::{UserInterface, user_interface};

type Renderer = iced_tiny_skia::Renderer;

const CONTENT: &str = "Handbook";
/// Wider and taller than the label, so both axes have room to align in.
const BOX: Size = Size::new(300.0, 120.0);
const VIEWPORT: Size = Size::new(400.0, 200.0);
/// The label's box is not the window's: a correct origin has to survive it.
const INSET: f32 = 24.0;
/// Just inside the widget box's top-left corner — where the paragraph used to
/// be assumed to start.
const CORNER: Point = Point {
    x: INSET + 2.0,
    y: INSET + 2.0,
};

fn view(align_x: text::Alignment, align_y: Vertical) -> Element<'static, (), Theme, Renderer> {
    container(
        rich_text([span(CONTENT).link(()).underline(true)])
            .size(20.0)
            .width(Length::Fixed(BOX.width))
            .height(Length::Fixed(BOX.height))
            .align_x(align_x)
            .align_y(align_y)
            .on_link_click(|(): ()| ()),
    )
    .padding(INSET)
    .into()
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

/// The underline. Nothing else in this view fills a quad.
fn underline(renderer: &mut Renderer) -> Rectangle {
    let mut union: Option<Rectangle> = None;

    for layer in renderer.layers() {
        for (quad, _) in &layer.quads {
            union = Some(match union {
                Some(union) => union.union(&quad.bounds),
                None => quad.bounds,
            });
        }
    }

    union.expect("an underlined span fills a quad")
}

/// Presses and releases at `at`, and reports how many link messages that
/// produced.
fn click(
    ui: &mut UserInterface<'_, (), Theme, Renderer>,
    renderer: &mut Renderer,
    at: Point,
) -> usize {
    let mut messages = Vec::new();

    for event in [
        Event::Mouse(mouse::Event::CursorMoved { position: at }),
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ] {
        let _ = ui.update(
            &[event],
            mouse::Cursor::Available(at),
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
    }

    messages.len()
}

struct Outcome {
    glyphs: Rectangle,
    underline: Rectangle,
    /// Clicks at the middle of the painted glyphs.
    clicks_on_the_glyphs: usize,
    /// Clicks just inside the widget box's top-left corner.
    clicks_at_the_corner: usize,
}

fn probe(align_x: text::Alignment, align_y: Vertical) -> Outcome {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    let mut ui = UserInterface::build(
        view(align_x, align_y),
        VIEWPORT,
        user_interface::Cache::default(),
        &mut renderer,
    );

    paint(&mut ui, &mut renderer);
    let glyphs = glyph_box(&mut renderer);
    let underline = underline(&mut renderer);

    let middle = Point::new(
        glyphs.x + glyphs.width / 2.0,
        glyphs.y + glyphs.height / 2.0,
    );

    Outcome {
        glyphs,
        underline,
        clicks_on_the_glyphs: click(&mut ui, &mut renderer, middle),
        clicks_at_the_corner: click(&mut ui, &mut renderer, CORNER),
    }
}

/// The underline has to sit under the glyphs, not merely somewhere on screen.
fn assert_underline_follows_the_glyphs(outcome: &Outcome, label: &str) {
    let (line, glyphs) = (outcome.underline, outcome.glyphs);

    assert!(
        (line.x - glyphs.x).abs() <= 3.0,
        "{label}: the underline must start where the glyphs are painted: \
         expected ~{}, got {} (glyphs {glyphs:?}, underline {line:?})",
        glyphs.x,
        line.x
    );
    assert!(
        (line.width - glyphs.width).abs() <= 6.0,
        "{label}: the underline must span the glyphs it underlines: \
         expected ~{}, got {}",
        glyphs.width,
        line.width
    );
    assert!(
        line.y >= glyphs.y && line.y <= glyphs.y + glyphs.height + 6.0,
        "{label}: the underline must lie at the foot of the glyphs: \
         expected between {} and {}, got {}",
        glyphs.y,
        glyphs.y + glyphs.height + 6.0,
        line.y
    );
}

/// Horizontal: a centred label is drawn well right of its box's left edge, so
/// a decoration or hit test against that edge lands beside the line entirely.
#[test]
fn a_centred_link_is_decorated_and_clicked_where_it_is_painted() {
    let outcome = probe(text::Alignment::Center, Vertical::Top);

    assert!(
        outcome.glyphs.x > INSET + 1.0,
        "the fixture only means anything if centring moved the glyphs: \
         box starts at {INSET}, glyphs at {}",
        outcome.glyphs.x
    );
    assert_eq!(
        (outcome.clicks_on_the_glyphs, outcome.clicks_at_the_corner),
        (1, 0),
        "centred: the painted link is the link: clicking its glyphs must follow \
         it and the corner holds no glyphs, so it is not the link \
         (clicks on the glyphs, clicks at the box's corner)"
    );
    assert_underline_follows_the_glyphs(&outcome, "centred");
}

#[test]
fn a_right_aligned_link_is_decorated_and_clicked_where_it_is_painted() {
    let outcome = probe(text::Alignment::Right, Vertical::Top);

    assert!(
        outcome.glyphs.x + outcome.glyphs.width > INSET + BOX.width - 4.0,
        "the fixture only means anything if the label reaches its box's right \
         edge: box ends at {}, glyphs at {}",
        INSET + BOX.width,
        outcome.glyphs.x + outcome.glyphs.width
    );
    assert_eq!(
        (outcome.clicks_on_the_glyphs, outcome.clicks_at_the_corner),
        (1, 0),
        "right-aligned: the painted link is the link: clicking its glyphs must follow \
         it and the corner holds no glyphs, so it is not the link \
         (clicks on the glyphs, clicks at the box's corner)"
    );
    assert_underline_follows_the_glyphs(&outcome, "right-aligned");
}

/// Vertical: a bottom-aligned label is drawn a whole box below its top edge.
#[test]
fn a_bottom_aligned_link_is_decorated_and_clicked_where_it_is_painted() {
    let outcome = probe(text::Alignment::Default, Vertical::Bottom);

    assert!(
        outcome.glyphs.y > INSET + BOX.height / 2.0,
        "the fixture only means anything if bottom alignment moved the glyphs: \
         box starts at {INSET}, glyphs at {}",
        outcome.glyphs.y
    );
    assert_eq!(
        (outcome.clicks_on_the_glyphs, outcome.clicks_at_the_corner),
        (1, 0),
        "bottom-aligned: the painted link is the link: clicking its glyphs must follow \
         it and the corner holds no glyphs, so it is not the link \
         (clicks on the glyphs, clicks at the box's corner)"
    );
    assert_underline_follows_the_glyphs(&outcome, "bottom-aligned");
}

#[test]
fn a_vertically_centred_link_is_decorated_and_clicked_where_it_is_painted() {
    let outcome = probe(text::Alignment::Default, Vertical::Center);

    assert!(
        outcome.glyphs.y > INSET + 1.0,
        "the fixture only means anything if centring moved the glyphs: \
         box starts at {INSET}, glyphs at {}",
        outcome.glyphs.y
    );
    assert_eq!(
        (outcome.clicks_on_the_glyphs, outcome.clicks_at_the_corner),
        (1, 0),
        "vertically centred: the painted link is the link: clicking its glyphs must follow \
         it and the corner holds no glyphs, so it is not the link \
         (clicks on the glyphs, clicks at the box's corner)"
    );
    assert_underline_follows_the_glyphs(&outcome, "vertically centred");
}

/// The control: with the default alignment the paragraph does start at the
/// box's corner, so the corner is on the link and everything above still holds.
#[test]
fn a_top_left_link_is_decorated_and_clicked_at_its_box_corner() {
    let outcome = probe(text::Alignment::Default, Vertical::Top);

    assert!(
        outcome.glyphs.x < INSET + 4.0 && outcome.glyphs.y < INSET + 12.0,
        "the default alignment paints at the box's corner: box at \
         ({INSET}, {INSET}), glyphs at ({}, {})",
        outcome.glyphs.x,
        outcome.glyphs.y
    );
    assert_eq!(
        (outcome.clicks_on_the_glyphs, outcome.clicks_at_the_corner),
        (1, 1),
        "top-left: the painted link is the link: clicking its glyphs must follow \
         it and the glyphs are at the corner, so the corner is the link too \
         (clicks on the glyphs, clicks at the box's corner)"
    );
    assert_underline_follows_the_glyphs(&outcome, "top-left");
}
