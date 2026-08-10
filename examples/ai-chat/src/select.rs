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

use std::cell::OnceCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::advanced::text::{Hit, LineHeight, Paragraph, Shaping, Span, Wrapping};
use iced::advanced::widget::{Tree, text as widget_text, tree};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer, text,
};
use iced::alignment;
use iced::widget::markdown;
use iced::{Element, Event, Font, Length, Pixels, Point, Rectangle, Size, Vector, keyboard};

/// Which selection the window is currently showing.
///
/// One at a time, as everywhere else: starting a drag anywhere takes the
/// highlight away from wherever it was, and every other block goes quiet
/// without having to be told.
static NEXT_SELECTION: AtomicU64 = AtomicU64::new(1);
static ACTIVE_SELECTION: AtomicU64 = AtomicU64::new(0);

type Renderer = iced::Renderer;
type Para = <Renderer as text::Renderer>::Paragraph;
type Line = Span<'static, markdown::Uri, Font>;

/// How much of the text colour the highlight keeps. Enough to see under either
/// palette, little enough to read the words through.
const HIGHLIGHT_ALPHA: f32 = 0.28;

/// One block of an answer: its spans, drawn and selectable.
pub struct Selectable {
    spans: Arc<[Line]>,
    /// The block as one string, for the clipboard and for the byte offsets a
    /// selection is kept in. Built on demand: this widget is constructed again
    /// for every call the runtime makes into the row, and all but the few that
    /// touch a live selection never need it.
    plain: OnceCell<String>,
    size: Pixels,
}

/// One block of rendered Markdown, as an element that can be dragged across.
pub fn selectable(spans: Arc<[Line]>, size: Pixels) -> Element<'static, String> {
    Element::new(Selectable {
        spans,
        plain: OnceCell::new(),
        size,
    })
}

struct State {
    paragraph: Para,
    shaped: Vec<Line>,
    token: u64,
    anchor: usize,
    cursor: usize,
    dragging: bool,
    hovered_link: Option<usize>,
    pressed_link: Option<usize>,
}

impl State {
    fn is_active(&self) -> bool {
        self.token != 0 && ACTIVE_SELECTION.load(Ordering::Relaxed) == self.token
    }

    /// The selected byte range, or nothing when this block is not the one
    /// holding the window's selection.
    fn range(&self, text: &str) -> Option<std::ops::Range<usize>> {
        if !self.is_active() {
            return None;
        }

        let start = self.anchor.min(self.cursor);
        let end = self.anchor.max(self.cursor);
        (start != end && text.get(start..end).is_some()).then_some(start..end)
    }
}

impl Selectable {
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
            token: 0,
            anchor: 0,
            cursor: 0,
            dragging: false,
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
                state.token = NEXT_SELECTION.fetch_add(1, Ordering::Relaxed);
                ACTIVE_SELECTION.store(state.token, Ordering::Relaxed);
                state.anchor = offset;
                state.cursor = offset;
                state.dragging = true;
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.dragging => {
                let Some(position) = cursor.position_from(layout.position()) else {
                    return;
                };
                if let Some(offset) = hit(&state.paragraph, position, true)
                    && state.cursor != offset
                {
                    // A drag is a selection, not a click on whatever it started
                    // over.
                    state.pressed_link = None;
                    state.cursor = offset;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging = false;

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
            }) if state.is_active() && modifiers.command() => match key.to_latin(*physical_key) {
                Some('a') => {
                    state.anchor = 0;
                    state.cursor = self.plain().len();
                    shell.capture_event();
                    shell.request_redraw();
                }
                // Out through the app rather than straight to the clipboard,
                // because this window has one route for text leaving it: the
                // same handler a clicked link and a Copy button use, which is
                // also what puts "Copied" under the composer.
                Some('c') => {
                    if let Some(range) = state.range(self.plain()) {
                        shell.publish(self.plain()[range].to_owned());
                        shell.capture_event();
                    }
                }
                _ => {}
            },
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) if state.is_active() => {
                ACTIVE_SELECTION.store(0, Ordering::Relaxed);
                state.dragging = false;
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
        if let Some(range) = state.range(self.plain()) {
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
