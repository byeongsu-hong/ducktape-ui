//! Rendered Markdown a pointer can be dragged across.
//!
//! `text` in this workspace is already drag-selectable — `ui-lang-runtime`
//! wraps it, reads the paragraph iced keeps in the widget's own tree state, and
//! hit-tests that. Rich text cannot be wrapped the same way: its state, and so
//! its paragraph, is private to iced. An answer is rich text from end to end —
//! bold, links, inline code — which is why it was the one thing in this window
//! that could not be selected.
//!
//! So this owns the paragraph instead of borrowing one. It is iced's own rich
//! text with the selection iced does not have: the same shaping inputs, the
//! same span highlights and links, plus an anchor, a cursor, and the platform's
//! copy and select-all.

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use ui_lang_runtime::selection;

use iced::advanced::text::{Hit, LineHeight, Paragraph, Shaping, Span, Wrapping};
use iced::advanced::widget::{Tree, text as widget_text, tree};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer, text,
};
use iced::alignment;
use iced::widget::markdown;
use iced::{Element, Event, Font, Length, Pixels, Point, Rectangle, Size, Vector, keyboard};

// The one selection the window is showing, wherever it is.
//
// It has to live outside the widgets because it does not live inside one: an
// answer is drawn as a column of blocks, each its own widget with its own
// paragraph, and dragging from a sentence through a code block into a list is
// one selection across several of them. Each block reads this to find its own
// share of it, and only the block under the pointer writes to it.
//
// One at a time, everywhere — including outside these blocks. Plain text in
// this window is selectable too, and it and this agree on nothing except the
// runtime's token: whoever claimed last has the selection, and the other goes
// quiet without having to be told.
thread_local! {
    static SELECTION: RefCell<Option<Selection>> = const { RefCell::new(None) };
    /// What each block contributed to the copy being assembled. Cleared by the
    /// block that finishes it.
    static PARTS: RefCell<BTreeMap<usize, String>> = const {
        RefCell::new(BTreeMap::new())
    };
}

/// Groups are handed out per answer. The lowest few are reserved for the reply
/// still being written, which is rebuilt from nothing on every frame and so
/// cannot be given one.
const LANES: u64 = 8;
static NEXT_GROUP: AtomicU64 = AtomicU64::new(LANES);

/// Where one end of a selection sits: which block, and how far into it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Place {
    block: usize,
    offset: usize,
}

#[derive(Debug)]
struct Selection {
    /// What holds the window's one selection. Plain text takes the same kind
    /// of token, so a drag over a prompt puts this one out.
    token: u64,
    group: u64,
    anchor: Place,
    focus: Place,
    dragging: bool,
}

impl Selection {
    fn ends(&self) -> (Place, Place) {
        (self.anchor.min(self.focus), self.anchor.max(self.focus))
    }

    /// This block's share of the selection: all of it in the middle, part of it
    /// at either end, and nothing at all outside.
    fn within(&self, group: u64, block: usize, len: usize) -> Option<Range<usize>> {
        if self.group != group || !selection::holds(self.token) {
            return None;
        }

        let (start, end) = self.ends();
        if block < start.block || block > end.block {
            return None;
        }

        let from = if block == start.block {
            start.offset
        } else {
            0
        };
        let to = if block == end.block { end.offset } else { len };
        (from < to && to <= len).then_some(from..to)
    }
}

/// Read this block's share of whatever is selected.
fn selected(group: u64, block: usize, len: usize) -> Option<Range<usize>> {
    SELECTION.with_borrow(|selection| selection.as_ref()?.within(group, block, len))
}

type Renderer = iced::Renderer;
type Para = <Renderer as text::Renderer>::Paragraph;
/// One styled run of an answer, as `iced`'s Markdown hands it over.
pub type Line = Span<'static, markdown::Uri, Font>;

/// How much of the text colour the highlight keeps. Enough to see under either
/// palette, little enough to read the words through.
const HIGHLIGHT_ALPHA: f32 = 0.28;

/// One block of an answer: its spans, drawn and selectable.
pub struct Selectable {
    /// Which answer this block belongs to, and where it sits in it. A selection
    /// is one range across the group, so a block needs both to know its share.
    group: u64,
    block: usize,
    /// How many blocks the answer has, shared with every one of them. Only the
    /// first and the last need it, to answer for a drag that has left the
    /// answer at the top or the bottom.
    blocks: Rc<Cell<usize>>,
    spans: Arc<[Line]>,
    /// The block as one string, for the clipboard and for the byte offsets a
    /// selection is kept in. Built on demand: this widget is constructed again
    /// for every call the runtime makes into the row, and all but the few that
    /// touch a live selection never need it.
    plain: OnceCell<String>,
    size: Pixels,
}

/// One block of rendered Markdown, as an element that can be dragged across.
pub fn selectable(
    group: u64,
    block: usize,
    blocks: Rc<Cell<usize>>,
    spans: Arc<[Line]>,
    size: Pixels,
) -> Element<'static, String> {
    Element::new(Selectable {
        group,
        block,
        blocks,
        spans,
        plain: OnceCell::new(),
        size,
    })
}

/// A fresh group, for an answer that will be drawn again as it stands.
pub fn group() -> u64 {
    NEXT_GROUP.fetch_add(1, Ordering::Relaxed)
}

/// The group of one surface of the reply still being written.
///
/// A live reply is drawn by more than one surface — what the model is working
/// out, and what it is answering with — and each is rebuilt from nothing every
/// frame, so none can hold an identity of its own. The lane names them apart
/// instead: there is only ever one of each, so its number is identity enough.
/// Two surfaces sharing a lane share a selection, and a drag in one lands in
/// the other.
pub fn live(lane: i64) -> u64 {
    lane.rem_euclid(LANES as i64) as u64
}

struct State {
    paragraph: Para,
    shaped: Vec<Line>,
    hovered_link: Option<usize>,
    pressed_link: Option<usize>,
}

impl Selectable {
    /// This block's share of the selection.
    fn selected(&self) -> Option<Range<usize>> {
        selected(self.group, self.block, self.plain().len())
    }

    /// Whether the answer this block belongs to is the one holding the
    /// selection — true for every block of it, selected or not.
    fn holds_selection(&self) -> bool {
        SELECTION.with_borrow(|held| {
            held.as_ref()
                .is_some_and(|held| held.group == self.group && selection::holds(held.token))
        })
    }

    /// Whether this is the last block the selection reaches, and so the one
    /// that sends an assembled copy.
    fn last_of_selection(&self) -> bool {
        SELECTION.with_borrow(|selection| {
            selection
                .as_ref()
                .is_some_and(|selection| selection.ends().1.block == self.block)
        })
    }

    fn plain(&self) -> &str {
        self.plain
            .get_or_init(|| self.spans.iter().map(|span| span.text.as_ref()).collect())
    }

    /// The shaping inputs, kept in one place because the paragraph laid out
    /// here and the one re-split to draw a highlight must agree exactly — a
    /// highlight shaped even slightly differently sits beside its own words.
    fn text<'a>(
        &self,
        spans: &'a [Line],
        bounds: Size,
        font: Font,
    ) -> text::Text<&'a [Line], Font> {
        text::Text {
            content: spans,
            bounds,
            size: self.size,
            line_height: LineHeight::default(),
            font,
            align_x: text::Alignment::Default,
            align_y: alignment::Vertical::Top,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::default(),
        }
    }
}

impl Widget<String, iced::Theme, Renderer> for Selectable {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            paragraph: Para::default(),
            shaped: Vec::new(),
            hovered_link: None,
            pressed_link: None,
        })
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state: &mut State = tree.state.downcast_mut();
        // The window's own face, the same one iced's rich text would have
        // reached for. Naming a font here instead would set an answer in
        // something the rest of the transcript is not.
        let font = text::Renderer::default_font(renderer);

        // Shrink on both axes, as iced's own rich text is. A code block is laid
        // out inside a horizontal scroll, where the width on offer is infinite:
        // filling it means a block shaped into infinity, which draws nothing.
        layout::sized(limits, Length::Shrink, Length::Shrink, |limits| {
            let bounds = limits.max();

            if state.shaped != *self.spans {
                state.paragraph = Para::with_spans(self.text(&self.spans, bounds, font));
                state.shaped = self.spans.to_vec();
            } else {
                match state
                    .paragraph
                    .compare(self.text(&[], bounds, font).with_content(()))
                {
                    text::Difference::None => {}
                    text::Difference::Bounds => state.paragraph.resize(bounds),
                    text::Difference::Shape => {
                        state.paragraph = Para::with_spans(self.text(&self.spans, bounds, font));
                    }
                }
            }

            state.paragraph.min_bounds()
        })
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, String>,
        _viewport: &Rectangle,
    ) {
        let state: &mut State = tree.state.downcast_mut();

        let hovered = cursor.position_in(layout.bounds()).and_then(|position| {
            let span = state.paragraph.hit_span(position)?;
            self.spans.get(span)?.link.as_ref().map(|_| span)
        });
        if state.hovered_link != hovered {
            state.hovered_link = hovered;
            shell.request_redraw();
        }

        match event {
            // A link is a click, and the rest of a block is a drag. Pressing on
            // a link still starts a selection, because a press that turns out
            // to be a drag has to have selected from somewhere.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_in(layout.bounds()) else {
                    return;
                };
                state.pressed_link = state.hovered_link;

                let Some(offset) = hit(&state.paragraph, position, false) else {
                    return;
                };
                let at = Place {
                    block: self.block,
                    offset,
                };
                SELECTION.replace(Some(Selection {
                    token: selection::claim(),
                    group: self.group,
                    anchor: at,
                    focus: at,
                    dragging: true,
                }));
                shell.request_redraw();
            }
            // Every block of the answer sees the drag, and the one the pointer
            // is level with claims it — by row rather than by bounds, because a
            // block is only as wide as its own text and a drag down the margin
            // beside it is still a drag through it.
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(position) = cursor.position() else {
                    return;
                };
                let bounds = layout.bounds();
                // Above the answer belongs to its first block and below it to
                // its last, so a drag that overshoots the whole answer still
                // takes it to the end rather than stopping where the pointer
                // left the words.
                let above = position.y < bounds.y;
                let below = position.y >= bounds.y + bounds.height;
                let mine = (!above && !below)
                    || (above && self.block == 0)
                    || (below && self.block + 1 == self.blocks.get());

                let moved = SELECTION.with_borrow_mut(|selection| {
                    let Some(selection) = selection.as_mut() else {
                        return false;
                    };
                    if !selection.dragging || selection.group != self.group || !mine {
                        return false;
                    }

                    let offset = if above {
                        0
                    } else if below {
                        self.plain().len()
                    } else {
                        let inside = position - (layout.position() - Point::ORIGIN);
                        let Some(offset) =
                            hit(&state.paragraph, Point::new(inside.x, inside.y), true)
                        else {
                            return false;
                        };
                        offset
                    };
                    let at = Place {
                        block: self.block,
                        offset,
                    };
                    (selection.focus != at)
                        .then(|| selection.focus = at)
                        .is_some()
                });

                if moved {
                    // A drag is a selection, not a click on whatever it started
                    // over.
                    state.pressed_link = None;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                SELECTION.with_borrow_mut(|selection| {
                    if let Some(selection) = selection.as_mut() {
                        selection.dragging = false;
                    }
                });

                if let Some(span) = state.pressed_link.take()
                    && Some(span) == state.hovered_link
                    && let Some(link) = self.spans.get(span).and_then(|span| span.link.clone())
                {
                    shell.publish(link.to_string());
                    shell.capture_event();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                ..
            }) if modifiers.command() => match key.to_latin(*physical_key) {
                // Every block extends the selection to itself, so the last one
                // of the answer to be handed the key leaves it covering all of
                // them. Only the group already holding a selection answers, so
                // this reaches the answer being read rather than the transcript.
                Some('a') if self.holds_selection() => {
                    SELECTION.with_borrow_mut(|selection| {
                        if let Some(selection) = selection.as_mut() {
                            selection.anchor = Place {
                                block: 0,
                                offset: 0,
                            };
                            selection.focus = Place {
                                block: self.block,
                                offset: self.plain().len(),
                            };
                        }
                    });
                    shell.capture_event();
                    shell.request_redraw();
                }
                // A copy is assembled the same way: each block leaves its own
                // share behind, and the last block of the selection — reached
                // last, because a column hands its children the event in the
                // order it draws them — sends the whole of it.
                //
                // Out through the app rather than straight to the clipboard,
                // because this window has one route for text leaving it: the
                // same handler a clicked link and a Copy button use, which is
                // also what puts "Copied" under the composer.
                Some('c') => {
                    let Some(range) = self.selected() else {
                        return;
                    };
                    PARTS.with_borrow_mut(|parts| {
                        parts.insert(self.block, self.plain()[range].to_owned());
                    });
                    shell.capture_event();

                    if self.last_of_selection() {
                        let whole = PARTS.with_borrow_mut(|parts| {
                            let whole = parts.values().cloned().collect::<Vec<_>>().join("\n\n");
                            parts.clear();
                            whole
                        });
                        shell.publish(whole);
                    }
                }
                _ => {}
            },
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) if self.holds_selection() => {
                selection::clear();
                SELECTION.replace(None);
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state: &State = tree.state.downcast_ref();

        if state.hovered_link.is_some() {
            mouse::Interaction::Pointer
        } else if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &iced::Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if !layout.bounds().intersects(viewport) {
            return;
        }

        let state: &State = tree.state.downcast_ref();
        let style =
            widget_text::Catalog::style(theme, &<iced::Theme as widget_text::Catalog>::default());
        let translation = layout.position() - Point::ORIGIN;

        // Inline code's ground, a strikethrough, and the underline a link
        // grows under the pointer. iced paints these for its own rich text and
        // nothing else does; without them an answer loses every mark that is
        // not a glyph.
        for (index, span) in self.spans.iter().enumerate() {
            let hovered = Some(index) == state.hovered_link;
            if span.highlight.is_none() && !span.underline && !span.strikethrough && !hovered {
                continue;
            }

            let regions = state.paragraph.span_bounds(index);

            if let Some(highlight) = span.highlight {
                for bounds in &regions {
                    let bounds = Rectangle::new(
                        bounds.position() - Vector::new(span.padding.left, span.padding.top),
                        bounds.size() + Size::new(span.padding.x(), span.padding.y()),
                    );

                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: bounds + translation,
                            border: highlight.border,
                            ..Default::default()
                        },
                        highlight.background,
                    );
                }
            }

            if !span.underline && !span.strikethrough && !hovered {
                continue;
            }

            let size = span.size.unwrap_or(self.size);
            let line_height = span.line_height.unwrap_or_default().to_absolute(size);
            let color = span.color.or(style.color).unwrap_or(defaults.text_color);
            let baseline = translation + Vector::new(0.0, size.0 + (line_height.0 - size.0) / 2.0);

            for bounds in &regions {
                if span.underline || hovered {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle::new(
                                bounds.position() + baseline - Vector::new(0.0, size.0 * 0.08),
                                Size::new(bounds.width, 1.0),
                            ),
                            ..Default::default()
                        },
                        color,
                    );
                }

                if span.strikethrough {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle::new(
                                bounds.position() + baseline - Vector::new(0.0, size.0 / 2.0),
                                Size::new(bounds.width, 1.0),
                            ),
                            ..Default::default()
                        },
                        color,
                    );
                }
            }
        }

        // The selection is drawn from a second paragraph, split at the two ends
        // of the range and shaped from the same inputs, because a span's bounds
        // are the only geometry a paragraph will give up — there is no asking
        // one where a byte landed.
        if let Some(range) = self.selected() {
            let font = text::Renderer::default_font(renderer);
            let (split, selected) = split(&self.spans, range);
            let paragraph = Para::with_spans(self.text(&split, state.paragraph.bounds(), font));
            let color = defaults.text_color.scale_alpha(HIGHLIGHT_ALPHA);

            for index in selected {
                for bounds in paragraph.span_bounds(index) {
                    let bounds = bounds + translation;
                    if bounds.intersects(viewport) {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds,
                                ..Default::default()
                            },
                            color,
                        );
                    }
                }
            }
        }

        widget_text::draw(
            renderer,
            defaults,
            layout.bounds(),
            &state.paragraph,
            style,
            viewport,
        );
    }
}

fn hit(paragraph: &Para, mut point: Point, clamp: bool) -> Option<usize> {
    if clamp {
        // The measured text, not the box it was laid out in. A block is laid
        // out into whatever width is on offer, so clamping to that lets a drag
        // sit in empty space to the right of the last line and hit nothing —
        // the selection stops moving the moment the pointer leaves the words.
        // Half a pixel inside, because the far edge is not part of the text:
        // a drag that leaves the block at its bottom-right corner hit-tests
        // exactly on the boundary and comes back with nothing.
        let bounds = paragraph.min_bounds();
        point.x = point.x.clamp(0.0, (bounds.width - 0.5).max(0.0));
        point.y = point.y.clamp(0.0, (bounds.height - 0.5).max(0.0));
    }

    paragraph.hit_test(point).map(Hit::cursor)
}

/// The same spans cut at both ends of a byte range, and which of the pieces
/// fall inside it.
///
/// A span keeps everything but its text, so the cut paragraph shapes to the
/// same width as the one on screen and its span bounds are the range's own
/// geometry.
fn split(spans: &[Line], range: std::ops::Range<usize>) -> (Vec<Line>, Vec<usize>) {
    let mut split = Vec::with_capacity(spans.len() + 2);
    let mut selected = Vec::new();
    let mut at = 0;

    for span in spans {
        let text = span.text.as_ref();
        for piece in cuts(at, text.len(), &range) {
            if piece.is_empty() {
                continue;
            }
            if at + piece.start >= range.start && at + piece.end <= range.end {
                selected.push(split.len());
            }
            split.push(Span {
                text: text[piece].to_owned().into(),
                ..span.clone()
            });
        }
        at += text.len();
    }

    (split, selected)
}

/// One span's text cut into the pieces before, inside, and after the range.
fn cuts(at: usize, len: usize, range: &std::ops::Range<usize>) -> [std::ops::Range<usize>; 3] {
    let start = range.start.saturating_sub(at).min(len);
    let end = range.end.saturating_sub(at).min(len);

    [0..start, start..end, end..len]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(pieces: &[&str]) -> Arc<[Line]> {
        pieces
            .iter()
            .map(|piece| Span::new((*piece).to_owned()))
            .collect()
    }

    fn texts(spans: &[Line]) -> Vec<&str> {
        spans.iter().map(|span| span.text.as_ref()).collect()
    }

    /// A selection is kept as one byte range over the whole block, but the
    /// block is drawn as spans and a span's bounds are the only geometry a
    /// paragraph will give up. Cutting at both ends is what turns the one into
    /// the other; cutting in the wrong place highlights the wrong words.
    #[test]
    fn a_range_cuts_the_spans_it_crosses_and_names_the_pieces_inside_it() {
        let block = spans(&["Hold ", "the parsed", " document"]);

        // "the parsed" exactly: one cut on each side, and only the middle span
        // is inside.
        let (pieces, selected) = split(&block, 5..15);
        assert_eq!(texts(&pieces), ["Hold ", "the parsed", " document"]);
        assert_eq!(selected, [1]);

        // Across a boundary: the two spans it crosses are each cut, and both
        // inner pieces are named.
        let (pieces, selected) = split(&block, 8..17);
        assert_eq!(texts(&pieces), ["Hold ", "the", " parsed", " d", "ocument"]);
        assert_eq!(selected, [2, 3]);
        let inside: String = selected.iter().map(|i| pieces[*i].text.as_ref()).collect();
        assert_eq!(
            inside, " parsed d",
            "the pieces inside are the range itself"
        );
    }

    /// Both ends of one selection, under a token the caller already holds:
    /// taking a fresh one here would put out the selection built beside it.
    fn across(token: u64, anchor: (usize, usize), focus: (usize, usize)) -> Selection {
        Selection {
            token,
            group: 7,
            anchor: Place {
                block: anchor.0,
                offset: anchor.1,
            },
            focus: Place {
                block: focus.0,
                offset: focus.1,
            },
            dragging: false,
        }
    }

    /// A selection belongs to the answer, not to the block it started in: the
    /// block it started in keeps its tail, the blocks between are taken whole,
    /// and the block it ended in keeps its head. Getting this wrong is a drag
    /// that stops at the first paragraph — which is what it used to do.
    #[test]
    fn a_selection_across_blocks_gives_each_one_its_own_share() {
        let selection = across(selection::claim(), (1, 4), (3, 2));

        assert_eq!(selection.within(7, 0, 10), None, "above it, nothing");
        assert_eq!(
            selection.within(7, 1, 10),
            Some(4..10),
            "from where it began"
        );
        assert_eq!(selection.within(7, 2, 10), Some(0..10), "then whole blocks");
        assert_eq!(selection.within(7, 3, 10), Some(0..2), "to where it ended");
        assert_eq!(selection.within(7, 4, 10), None, "below it, nothing");
        assert_eq!(selection.within(8, 2, 10), None, "and never another answer");
    }

    /// Dragged upwards it is the same selection, because which end is the
    /// anchor is not something the highlight can see.
    #[test]
    fn a_selection_dragged_upwards_covers_what_one_dragged_down_would() {
        let token = selection::claim();
        let down = across(token, (1, 4), (3, 2));
        let up = across(token, (3, 2), (1, 4));

        for block in 0..5 {
            assert_eq!(
                up.within(7, block, 10),
                down.within(7, block, 10),
                "block {block} is the same either way round"
            );
        }
    }

    /// The cut keeps everything about a span except its text, because the
    /// paragraph built from the cut spans has to shape exactly as the one on
    /// screen — a highlight shaped differently sits beside its own words.
    #[test]
    fn cutting_a_span_keeps_what_it_was_drawn_with() {
        let code = Span::new("push_str".to_owned())
            .size(Pixels(11.0))
            .font(Font::MONOSPACE);
        let block: Arc<[Line]> = vec![Span::new("with ".to_owned()), code].into();

        let (pieces, selected) = split(&block, 5..9);
        let piece = &pieces[selected[0]];

        assert_eq!(piece.text.as_ref(), "push");
        assert_eq!(piece.size, Some(Pixels(11.0)));
        assert_eq!(piece.font, Some(Font::MONOSPACE));
    }
}
