//! What `scroll_anchor` promises, held against a plain scrollable in the same
//! run: a list that grew carries the reader's offset with it, and nothing else
//! moves them.
//!
//! Growth is the whole of the input, because it is the whole of what a wrapper
//! around a scrollable can see — which end the rows landed on is not something
//! a box of pixels with a height can report. So these fixtures grow a column
//! and assert the offset arithmetic; the case that matters, rows landing
//! *above* a reader, is driven end to end against the real terminal by
//! `examples/trading/src/ui/tests/scrolling.ice`, where `push_fills` really
//! does put the new rows on top.
//!
//! The control is the point. Both halves of the first assertion are built,
//! laid out and read in one process against one renderer, so a passing
//! anchored number is only meaningful beside the unanchored one that does not
//! move — which is exactly the reported symptom.

use iced::advanced::renderer;
use iced::advanced::widget::operation::{self, Scrollable};
use iced::advanced::widget::{Id, Operation};
use iced::widget::{column, scrollable, space};
use iced::{Element, Font, Length, Pixels, Rectangle, Size, Theme, Vector};
use iced_test::runtime::{UserInterface, user_interface};
use ui_lang_runtime::scroll_anchor;

type Renderer = iced_test::renderer::Renderer;

const ROW: f32 = 20.0;
const VIEWPORT: Size = Size::new(200.0, 100.0);
const LIST: &str = "list";

fn renderer() -> Renderer {
    iced_test::futures::futures::executor::block_on(<Renderer as renderer::Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        None,
    ))
    .expect("headless renderer")
}

/// A list of fixed-height rows inside a scrollable, optionally anchored.
fn view(rows: usize, anchored: bool) -> Element<'static, (), Theme, Renderer> {
    let list = column((0..rows).map(|_| {
        space::Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(ROW))
            .into()
    }));
    let scroller = scrollable(list)
        .id(Id::new(LIST))
        .width(Length::Fill)
        .height(Length::Fill);
    if anchored {
        scroll_anchor(scroller).into()
    } else {
        scroller.into()
    }
}

/// Reads the scrollable's current vertical translation — what the reader is
/// actually looking at, in content pixels from the top.
fn offset(ui: &mut UserInterface<'_, (), Theme, Renderer>, renderer: &Renderer) -> f32 {
    struct Read(Option<f32>);

    impl Operation for Read {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
            operate(self);
        }

        fn scrollable(
            &mut self,
            id: Option<&Id>,
            _bounds: Rectangle,
            _content_bounds: Rectangle,
            translation: Vector,
            _state: &mut dyn Scrollable,
        ) {
            if id == Some(&Id::new(LIST)) {
                self.0 = Some(translation.y);
            }
        }
    }

    let mut read = Read(None);
    ui.operate(renderer, &mut read);
    read.0.expect("the list is on screen")
}

fn scroll_to(ui: &mut UserInterface<'_, (), Theme, Renderer>, renderer: &Renderer, y: f32) {
    let mut op = operation::scrollable::scroll_to::<()>(
        Id::new(LIST),
        operation::scrollable::AbsoluteOffset {
            x: None,
            y: Some(y),
        },
    );
    ui.operate(renderer, &mut op);
}

/// Drives one variant through: settle, scroll to `start`, then rebuild with
/// `grown` rows prepended, and report where the reader ended up.
fn after_growth(anchored: bool, rows: usize, start: f32, grown: usize) -> f32 {
    let mut renderer = renderer();
    let mut cache = user_interface::Cache::default();

    // Two builds before touching it: the first has no previous height to
    // compare against, so the contract only begins on the second.
    for _ in 0..2 {
        cache =
            UserInterface::build(view(rows, anchored), VIEWPORT, cache, &mut renderer).into_cache();
    }

    let mut ui = UserInterface::build(view(rows, anchored), VIEWPORT, cache, &mut renderer);
    scroll_to(&mut ui, &renderer, start);
    let cache = ui.into_cache();

    let mut ui = UserInterface::build(view(rows + grown, anchored), VIEWPORT, cache, &mut renderer);
    offset(&mut ui, &renderer)
}

/// The reported symptom and its fix, side by side. A reader 60px into a list
/// has it grow by four 20px rows: unanchored the offset holds, so rows
/// inserted above them slide 80px down the screen; anchored the offset follows
/// the growth and the same rows stay under their eye.
#[test]
fn a_grown_list_carries_a_scrolled_readers_offset_with_it() {
    let plain = after_growth(false, 20, 60.0, 4);
    let anchored = after_growth(true, 20, 60.0, 4);

    assert!(
        (plain - 60.0).abs() < 0.5,
        "iced holds an absolute offset across a content change: expected 60, got {plain}"
    );
    assert!(
        (anchored - (60.0 + 4.0 * ROW)).abs() < 0.5,
        "four rows of {ROW}px of growth must carry the offset with them: \
         expected {}, got {anchored}",
        60.0 + 4.0 * ROW
    );
}

/// A reader who has not scrolled is reading the newest row, and following the
/// content is what they want. Anchoring must leave them there — the fix must
/// not turn "resting on the newest row" into "drifting down the list".
#[test]
fn a_reader_at_the_top_is_left_on_the_newest_row() {
    let anchored = after_growth(true, 20, 0.0, 4);
    assert!(
        anchored.abs() < 0.5,
        "a list resting at the top must stay there when rows arrive: got {anchored}"
    );
}

/// Growth is not the only thing that changes a content height, and the other
/// one must not move the reader: nothing was inserted, so nothing should move.
#[test]
fn an_unchanged_list_leaves_the_offset_alone() {
    let anchored = after_growth(true, 20, 60.0, 0);
    assert!(
        (anchored - 60.0).abs() < 0.5,
        "a list that did not grow must not scroll: got {anchored}"
    );
}
