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
use iced::{Color, Element, Font, Padding, border};

/// Mirrors the `code` font declared in `src/ui/app.ice`.
const MONO: &str = "Geist Mono";

/// Link and inline-code colours, following `src/ui/theme.ice`. They are here
/// rather than passed per row because a colour is a property of the palette,
/// not of an answer, and the adapter is told only which palette is on.
const LINK_LIGHT: Color = Color::from_rgb(0.42, 0.29, 0.19);
const LINK_DARK: Color = Color::from_rgb(0.87, 0.67, 0.47);
const CODE_BG_LIGHT: Color = Color::from_rgb(0.94, 0.93, 0.91);
const CODE_BG_DARK: Color = Color::from_rgb(0.17, 0.16, 0.15);
const CODE_FG_LIGHT: Color = Color::from_rgb(0.34, 0.22, 0.14);
const CODE_FG_DARK: Color = Color::from_rgb(0.91, 0.78, 0.64);

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
        inline_code_padding: Padding::from([1.0, 4.0]),
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

/// One settled Markdown row. The message is the URL of a clicked link.
pub fn markdown_body(id: i64, source: String, size: f64, dark: bool) -> Element<'static, String> {
    let size = size as f32;
    let mut settings = markdown::Settings::with_text_size(size, style(dark));
    // The heading scale `with_text_size` derives is built for a document.
    // Inside a chat row a level-one heading twice the body size reads as a
    // banner, so the ladder is tightened and the code block left alone.
    settings.h1_size = (size * 1.45).into();
    settings.h2_size = (size * 1.3).into();
    settings.h3_size = (size * 1.15).into();
    settings.h4_size = size.into();
    settings.h5_size = size.into();
    settings.h6_size = size.into();
    settings.code_size = (size * 0.9).into();
    settings.spacing = (size * 0.8).into();

    markdown::view(items(id, &source), settings).map(|url| url.to_string())
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
