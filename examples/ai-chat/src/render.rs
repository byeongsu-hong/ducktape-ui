//! Settled Markdown, parsed once per lazy row and owned by that row.
//!
//! Ice holds only cloneable values in component state, and a parsed Markdown
//! document is not one — so a per-row document cannot live in the language,
//! and this is the typed adapter that answers for it.
//!
//! Keeping it with the row is the point. A transcript redraws on every frame
//! and on every token of the reply still being written; reparsing each settled
//! answer that often is the cost this window exists to avoid. The surrounding
//! `lazy` owns this adapter until the row leaves its bounded parking lot.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer};
use iced::widget::{column, container, markdown, scrollable};
use iced::{Color, Element, Event, Font, Length, Padding, Pixels, Rectangle, Size, border};

use crate::select;

/// Mirrors the `code` font declared in `src/ui/app.ice`.
const MONO: &str = "JetBrains Mono";

/// The palette's `brand` and `accent`/`accent_fg`, following `src/ui/theme.ice`
/// and matching the `style` block the live answer is drawn with in
/// `src/ui/app.ice`. They are duplicated here because this adapter builds its
/// own settings and cannot read a palette — a settled answer and the one still
/// being written must not disagree about what inline code looks like.
const LINK_LIGHT: Color = Color::from_rgb(0.627, 0.353, 0.235);
const LINK_DARK: Color = Color::from_rgb(0.871, 0.667, 0.502);
const CODE_BG_LIGHT: Color = Color::from_rgb(0.953, 0.949, 0.937);
const CODE_BG_DARK: Color = Color::from_rgb(0.149, 0.145, 0.137);
const CODE_FG_LIGHT: Color = Color::from_rgb(0.247, 0.243, 0.224);
const CODE_FG_DARK: Color = Color::from_rgb(0.847, 0.835, 0.804);
/// A code block's own ground, dark under either palette — it is the one part
/// of an answer that should be findable without reading.
/// On paper the block is ink, and needs no edge. On ink it is a deeper well
/// than the page, which alone is too small a step to see — so it carries an
/// edge as well.
const BLOCK_BG_LIGHT: Color = Color::from_rgb(0.149, 0.145, 0.122);
const BLOCK_BG_DARK: Color = Color::from_rgb(0.047, 0.043, 0.039);
const BLOCK_FG: Color = Color::from_rgb(0.953, 0.949, 0.937);
const BLOCK_EDGE: Color = Color::from_rgb(0.227, 0.216, 0.200);

fn style(dark: bool) -> markdown::Style {
    let mono = Font::with_name(MONO);
    markdown::Style {
        // Every span carries its own font, so this is the answer's prose face
        // rather than a fallback. iced's default here is the generic sans, and
        // an answer set in it was the one thing in the window not in the
        // window's own face.
        font: mono,
        // The highlight is painted around the span but the line is laid out
        // from the glyphs alone, so horizontal padding here is drawn over
        // whatever sits beside it: anything wider than a hair swallows the
        // spaces on both sides and the code reads as a gap in the sentence.
        inline_code_padding: Padding::from([0.0, 1.0]),
        inline_code_highlight: markdown::Highlight {
            background: if dark { CODE_BG_DARK } else { CODE_BG_LIGHT }.into(),
            border: border::rounded(4),
        },
        inline_code_color: if dark { CODE_FG_DARK } else { CODE_FG_LIGHT },
        inline_code_font: mono,
        code_block_font: mono,
        link_color: if dark { LINK_DARK } else { LINK_LIGHT },
    }
}

/// The default Markdown view, with the code block given its own ground.
///
/// iced derives a code block's container from the theme, which on a dark
/// palette leaves it the same value as the page behind it — the block stops
/// reading as a block at all. Everything else here is iced's own default.
pub struct Blocks {
    dark: bool,
    /// The answer these blocks belong to, and the count that numbers them
    /// within it. A selection is one range across the whole answer, so every
    /// block has to know which answer it is part of and where in it it sits.
    ///
    /// The count is reset before each walk of the document rather than kept
    /// between them: this viewer is asked for the same blocks again on every
    /// call the runtime makes into the row, and a block has to come back with
    /// the same number every time.
    group: u64,
    next: Cell<usize>,
    /// How many blocks the walk produced, shared with each of them so the
    /// first and the last can answer for a drag that has left the answer.
    blocks: Rc<Cell<usize>>,
}

impl Blocks {
    fn new(dark: bool, group: u64) -> Self {
        Self {
            dark,
            group,
            next: Cell::new(0),
            blocks: Rc::new(Cell::new(0)),
        }
    }

    fn restart(&self) {
        self.next.set(0);
    }

    fn selectable(&self, spans: Arc<[select::Line]>, size: Pixels) -> Element<'static, String> {
        let block = self.next.get();
        self.next.set(block + 1);
        self.blocks.set(block + 1);

        select::selectable(self.group, block, self.blocks.clone(), spans, size)
    }
}

impl<'a> markdown::Viewer<'a, String> for Blocks {
    fn on_link_click(url: markdown::Uri) -> String {
        url.to_string()
    }

    fn paragraph(
        &self,
        settings: markdown::Settings,
        text: &markdown::Text,
    ) -> Element<'a, String> {
        self.selectable(text.spans(settings.style), settings.text_size)
    }

    fn heading(
        &self,
        settings: markdown::Settings,
        level: &'a markdown::HeadingLevel,
        text: &'a markdown::Text,
        index: usize,
    ) -> Element<'a, String> {
        let size = match level {
            markdown::HeadingLevel::H1 => settings.h1_size,
            markdown::HeadingLevel::H2 => settings.h2_size,
            markdown::HeadingLevel::H3 => settings.h3_size,
            markdown::HeadingLevel::H4 => settings.h4_size,
            markdown::HeadingLevel::H5 => settings.h5_size,
            markdown::HeadingLevel::H6 => settings.h6_size,
        };

        container(self.selectable(text.spans(settings.style), size))
            .padding(iced::padding::top(if index > 0 {
                settings.text_size / 2.0
            } else {
                iced::Pixels::ZERO
            }))
            .into()
    }

    fn code_block(
        &self,
        settings: markdown::Settings,
        _language: Option<&'a str>,
        _code: &'a str,
        lines: &'a [markdown::Text],
    ) -> Element<'a, String> {
        let dark = self.dark;
        container(
            scrollable(
                container(column(lines.iter().map(|line| {
                    self.selectable(line.spans(settings.style), settings.code_size)
                })))
                .padding(settings.code_size),
            )
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default()
                    .width(settings.code_size / 2)
                    .scroller_width(settings.code_size / 2),
            )),
        )
        .width(Length::Fill)
        .padding(settings.code_size / 4)
        .style(move |_theme| container::Style {
            text_color: Some(BLOCK_FG),
            background: Some(if dark { BLOCK_BG_DARK } else { BLOCK_BG_LIGHT }.into()),
            border: border::rounded(8)
                .color(BLOCK_EDGE)
                .width(if dark { 1.0 } else { 0.0 }),
            ..container::Style::default()
        })
        .into()
    }
}

/// The same viewer, for the reply still being written.
///
/// Without it the live reply is drawn by iced's default and a code block
/// changes appearance the moment its turn settles — popping onto the ground
/// `Blocks` gives it. The answer should not move when it stops arriving.
pub fn answer_viewer(dark: bool) -> Blocks {
    Blocks::new(dark, select::live())
}

/// Parsed Markdown whose items live exactly as long as its owning lazy row.
///
/// iced's Markdown view borrows its parsed items. This transparent widget owns
/// those items and creates the borrowed native view only for each widget call,
/// so the generated extern can still return a `'static` element without
/// leaking the parse behind it.
struct MarkdownBody {
    items: Rc<[markdown::Item]>,
    settings: markdown::Settings,
    viewer: Blocks,
}

impl MarkdownBody {
    fn new(source: &str, size: f64, dark: bool) -> Self {
        Self {
            items: markdown::parse(source).collect::<Vec<_>>().into(),
            settings: settings(size, dark),
            viewer: Blocks::new(dark, select::group()),
        }
    }

    fn view(&self) -> Element<'_, String> {
        self.viewer.restart();
        markdown::view_with(self.items.iter(), self.settings, &self.viewer)
    }
}

impl Widget<String, iced::Theme, iced::Renderer> for MarkdownBody {
    fn tag(&self) -> tree::Tag {
        self.view().as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.view().as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.view().as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.view().as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.view().as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.view().as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.view().as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.view()
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, String>,
        viewport: &Rectangle,
    ) {
        self.view().as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.view()
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.view()
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }
}

fn settings(size: f64, dark: bool) -> markdown::Settings {
    let size = size as f32;
    let mut settings = markdown::Settings::with_text_size(size, style(dark));
    // The ladder `with_text_size` derives is built for a document: a level-one
    // heading at twice the body size reads as a banner inside a chat row. This
    // one is tightened, but every step still differs from the one below it —
    // three levels that all render at body size are not a hierarchy.
    settings.h1_size = (size * 1.34).into();
    settings.h2_size = (size * 1.22).into();
    settings.h3_size = (size * 1.12).into();
    settings.h4_size = (size * 1.05).into();
    settings.h5_size = size.into();
    settings.h6_size = (size * 0.94).into();
    settings.code_size = (size * 0.9).into();
    settings.spacing = (size * 0.8).into();

    settings
}

/// One settled Markdown row. The message is the URL of a clicked link.
pub fn markdown_body(source: String, size: f64, dark: bool) -> Element<'static, String> {
    Element::new(MarkdownBody::new(&source, size, dark))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Weak;

    type Lazy = ui_lang_runtime::MemoLazy<
        'static,
        String,
        iced::Theme,
        iced::Renderer,
        u16,
        Element<'static, String>,
    >;

    fn filler(dependency: u16) -> Lazy {
        ui_lang_runtime::memo_lazy(
            dependency,
            |value: &u16| Element::from(iced::widget::text(value.to_string())),
            u64::MAX - 1,
        )
    }

    /// A removed lazy row may stay in the runtime's bounded parking lot for a
    /// cheap remount. Once that lot evicts the row, nothing else may keep its
    /// parsed Markdown alive.
    #[test]
    fn evicting_a_lazy_row_reclaims_its_parsed_markdown() {
        let observed = Rc::new(RefCell::new(None::<Weak<[markdown::Item]>>));
        let probe = observed.clone();
        let row: Lazy = ui_lang_runtime::memo_lazy(
            0,
            move |_: &u16| {
                let body = MarkdownBody::new("# One\n\n- and a list", 13.5, false);
                *probe.borrow_mut() = Some(Rc::downgrade(&body.items));
                Element::new(body)
            },
            u64::MAX,
        );
        let tree = Tree::new(&row as &dyn Widget<String, iced::Theme, iced::Renderer>);
        let parsed = observed
            .borrow()
            .clone()
            .expect("the lazy row built its Markdown body");
        assert!(parsed.upgrade().is_some(), "the mounted row owns its parse");

        drop(tree);
        assert!(
            parsed.upgrade().is_some(),
            "an unmounted row stays available in the bounded parking lot"
        );

        // The parking lot holds 1024 entries. More distinct rows force this
        // oldest one out; the assertion observes the owned parsed allocation,
        // not just a cache key disappearing.
        for dependency in 0..1100 {
            let row = filler(dependency);
            drop(Tree::new(
                &row as &dyn Widget<String, iced::Theme, iced::Renderer>,
            ));
        }

        assert!(
            parsed.upgrade().is_none(),
            "eviction must drop the parsed Markdown with its row"
        );
    }
}
