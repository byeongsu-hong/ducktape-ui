//! Settled Markdown, parsed once and kept.
//!
//! Ice holds only cloneable values in component state, and a parsed Markdown
//! document is not one — so a per-row document cannot live in the language,
//! and this is the typed adapter that answers for it.
//!
//! Keeping it is the point. A transcript redraws on every frame and on every
//! token of the reply still being written; reparsing each settled answer that
//! often is the cost this window exists to avoid. Parsed once, a settled row
//! costs a borrow.

use std::cell::RefCell;
use std::collections::HashMap;

use iced::widget::markdown;
use iced::widget::{column, container, rich_text, scrollable};
use iced::{Color, Element, Font, Length, Padding, border};

/// Mirrors the `code` font declared in `src/ui/app.ice`.
const MONO: &str = "Monoplex KR";

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

// Parsed answers, keyed by the id of the row that owns them.
//
// `markdown::view` borrows its items for as long as the element lives, and a
// generated extern component hands back a `'static` element — so a parse is
// interned for the life of the process. The map is thread-local because a
// parsed item caches its own layout in a `Cell` and is therefore not `Sync`;
// drawing happens on one thread, and this belongs to it.
//
// The ceiling is one parsed copy per answer ever drawn in this window: clearing
// a chat orphans its entries rather than freeing them. Row ids are unique per
// process rather than per chat, so an orphan is never handed to a later answer.
thread_local! {
    static CACHE: RefCell<HashMap<i64, &'static [markdown::Item]>> = RefCell::default();
}

fn items(id: i64, source: &str) -> &'static [markdown::Item] {
    CACHE.with_borrow_mut(|cache| {
        *cache.entry(id).or_insert_with(|| {
            let parsed: Vec<markdown::Item> = markdown::parse(source).collect();
            Box::leak(parsed.into_boxed_slice())
        })
    })
}

fn style(dark: bool) -> markdown::Style {
    let mono = Font::with_name(MONO);
    markdown::Style {
        font: Font::default(),
        // The highlight is painted around the span but the line is laid out
        // from the glyphs alone, so horizontal padding here is drawn over
        // whatever sits beside it. Keep it to the hair iced itself uses.
        inline_code_padding: Padding::from([0.0, 3.0]),
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
struct Blocks {
    dark: bool,
}

impl<'a> markdown::Viewer<'a, String> for Blocks {
    fn on_link_click(url: markdown::Uri) -> String {
        url.to_string()
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
                    rich_text(line.spans(settings.style))
                        .on_link_click(Self::on_link_click)
                        .font(settings.style.code_block_font)
                        .size(settings.code_size)
                        .into()
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

/// One settled Markdown row. The message is the URL of a clicked link.
pub fn markdown_body(id: i64, source: String, size: f64, dark: bool) -> Element<'static, String> {
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

    markdown::view_with(items(id, &source), settings, &Blocks { dark })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cache is the whole point: a second draw of the same row must reuse
    /// the first parse, or every frame pays for the answer again.
    #[test]
    fn the_same_row_is_parsed_once() {
        let first = items(-1, "# One");
        let second = items(-1, "# One");
        assert!(
            std::ptr::eq(first, second),
            "a second draw must reuse the interned parse"
        );
    }

    /// Ids are process-unique so that a cleared chat cannot hand its layout to
    /// a later answer. This is the property that makes interning safe.
    #[test]
    fn different_rows_keep_different_parses() {
        let one = items(-2, "# One");
        let two = items(-3, "# Two\n\n- and a list");
        assert!(!std::ptr::eq(one, two));
        assert_ne!(one.len(), two.len(), "each row keeps its own document");
    }
}
