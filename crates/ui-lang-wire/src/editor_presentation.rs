//! Declarative editor formatting. Document identity is the enclosing Editor node's reference.
use crate::{Border, Edges, LineHeight, NamedFont, Rgba};
use serde::{Deserialize, Serialize};

/// Presentation is bounded independently of the canonical document bytes.
pub const MAX_EDITOR_FORMATS: usize = 256;
pub const MAX_EDITOR_SPANS: usize = 32_768;
pub const MAX_EDITOR_MENU_ITEMS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorMenuAnchor {
    Caret,
    Line(u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorMenuItem {
    #[serde(deserialize_with = "crate::editor_transaction::decode_document")]
    pub tag: String,
    #[serde(deserialize_with = "crate::editor_transaction::decode_document")]
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorMenu {
    pub anchor: EditorMenuAnchor,
    #[serde(deserialize_with = "decode_menu_items")]
    pub items: Vec<EditorMenuItem>,
    pub selected: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorGutterButton {
    Plus,
    Handle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorGutter {
    pub line: u32,
    pub plus: bool,
    pub handle: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorMargin {
    pub line: u32,
    pub count: u32,
}

/// Only these source ranges consume a line press; all other clicks remain native.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorHit {
    pub line: u32,
    pub start: u32,
    pub end: u32,
    pub tag: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorAffordances {
    pub menu: Option<EditorMenu>,
    #[serde(deserialize_with = "decode_gutters")]
    pub gutters: Vec<EditorGutter>,
    #[serde(deserialize_with = "decode_boundaries")]
    pub drop_boundaries: Vec<u32>,
    #[serde(deserialize_with = "decode_margins")]
    pub margins: Vec<EditorMargin>,
    #[serde(deserialize_with = "crate::editor_transaction::decode_document")]
    pub margin_label: String,
    #[serde(deserialize_with = "decode_hits")]
    pub hits: Vec<EditorHit>,
}

/// A presentation interaction is not an edit or a history commit. The editor
/// event envelope supplies the instance and canonical document reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorInteraction {
    LinePress {
        tag: u32,
        position: crate::EditorPosition,
    },
    Gutter {
        line: u32,
        button: EditorGutterButton,
    },
    GutterDrop {
        from: u32,
        boundary: u32,
    },
    Margin {
        line: u32,
    },
    MenuSelect {
        index: u32,
    },
    MenuPick {
        #[serde(deserialize_with = "crate::editor_transaction::decode_document")]
        tag: String,
    },
    MenuDismiss,
}

impl EditorAffordances {
    pub fn hit(&self, position: crate::EditorPosition) -> Option<EditorInteraction> {
        self.hits
            .iter()
            .find(|hit| {
                hit.line == position.line
                    && hit.start <= position.column
                    && position.column < hit.end
            })
            .map(|hit| EditorInteraction::LinePress {
                tag: hit.tag,
                position,
            })
    }

    fn validate(&self, source: &str) -> Result<(), PresentationError> {
        if [
            self.gutters.len(),
            self.drop_boundaries.len(),
            self.margins.len(),
            self.hits.len(),
        ]
        .into_iter()
        .try_fold(0usize, |count, next| count.checked_add(next))
        .is_none_or(|count| count > MAX_EDITOR_SPANS)
            || self.margin_label.len() > 1024
        {
            return Err(PresentationError::Limit);
        }
        if self.menu.is_none()
            && self.gutters.is_empty()
            && self.drop_boundaries.is_empty()
            && self.margins.is_empty()
            && self.hits.is_empty()
        {
            return Ok(());
        }
        let line_count = crate::editor_lines(source).count();
        if let Some(menu) = &self.menu {
            if menu.items.len() > MAX_EDITOR_MENU_ITEMS
                || menu
                    .items
                    .iter()
                    .any(|item| item.tag.len() > 1024 || item.label.len() > 1024)
                || menu
                    .items
                    .iter()
                    .map(|item| item.tag.len() + item.label.len())
                    .sum::<usize>()
                    > crate::MAX_STRING_BYTES
            {
                return Err(PresentationError::Limit);
            }
            if menu.selected as usize >= menu.items.len()
                || matches!(menu.anchor, EditorMenuAnchor::Line(line) if line as usize >= line_count)
                || menu.items.iter().enumerate().any(|(index, item)| {
                    item.tag.is_empty()
                        || menu.items[..index]
                            .iter()
                            .any(|earlier| earlier.tag == item.tag)
                })
            {
                return Err(PresentationError::Range);
            }
        }
        if self.gutters.iter().any(|g| g.line as usize >= line_count)
            || self
                .gutters
                .windows(2)
                .any(|pair| pair[0].line >= pair[1].line)
            || self.margins.iter().any(|m| m.line as usize >= line_count)
            || self
                .margins
                .windows(2)
                .any(|pair| pair[0].line >= pair[1].line)
        {
            return Err(PresentationError::Range);
        }
        if self
            .drop_boundaries
            .iter()
            .any(|line| *line as usize > line_count)
            || self
                .drop_boundaries
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self.hits.iter().any(|hit| hit.start == hit.end)
        {
            return Err(PresentationError::Range);
        }
        validate_ranges(
            source,
            self.hits.iter().map(|hit| (hit.line, hit.start, hit.end)),
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EditorFormat {
    pub color: Option<Rgba>,
    pub font: Option<NamedFont>,
    pub size: Option<f32>,
    pub line_height: Option<LineHeight>,
    pub background: Option<Rgba>,
    pub border: Option<Border>,
    pub line_background: Option<Rgba>,
    pub line_border: Option<Border>,
    pub line_padding: Edges,
    pub line_rule: Option<Rgba>,
    pub strikethrough: Option<Rgba>,
    pub padding: Edges,
}

/// A source range, in UTF-8 byte offsets within a logical line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorSpan {
    pub line: u32,
    pub start: u32,
    pub end: u32,
    pub format: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EditorPresentation {
    #[serde(deserialize_with = "decode_formats")]
    pub formats: Vec<EditorFormat>,
    /// Ordered by line, then start; overlapping ranges are rejected.
    #[serde(deserialize_with = "decode_spans")]
    pub spans: Vec<EditorSpan>,
    /// Local editor insets, including any gutter and margin space.
    pub padding: Option<Edges>,
    pub affordances: EditorAffordances,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationError {
    Limit,
    Format,
    Range,
}

impl EditorPresentation {
    pub(super) fn sanitize(&mut self, text_budget: &mut usize) {
        crate::bound_edges(&mut self.padding);
        for format in &mut self.formats {
            for color in [
                &mut format.color,
                &mut format.background,
                &mut format.line_background,
                &mut format.line_rule,
                &mut format.strikethrough,
            ] {
                crate::bound_color(color);
            }
            crate::bound_border(&mut format.border);
            crate::bound_border(&mut format.line_border);
            if let Some(size) = &mut format.size {
                *size = crate::bounded(*size).clamp(f32::EPSILON, crate::MAX_TEXT_PIXELS);
            }
            if let Some(height) = &mut format.line_height {
                height.sanitize();
            }
            if let Some(font) = &mut format.font {
                font.sanitize(text_budget);
            }
            for edges in [&mut format.padding, &mut format.line_padding] {
                edges.top = crate::bounded(edges.top);
                edges.right = crate::bounded(edges.right);
                edges.bottom = crate::bounded(edges.bottom);
                edges.left = crate::bounded(edges.left);
            }
        }
        // Interaction tags/ranges are semantic data: never shorten them.
        // Over-limit metadata is rejected by the bounded decoder, preserving
        // the previous healthy frame instead of publishing partial controls.
    }

    /// Validate against the exact resident document before any source is hidden.
    pub fn validate(&self, text: &str) -> Result<(), PresentationError> {
        self.affordances.validate(text)?;
        if self.formats.len() > MAX_EDITOR_FORMATS || self.spans.len() > MAX_EDITOR_SPANS {
            return Err(PresentationError::Limit);
        }
        for span in &self.spans {
            if usize::from(span.format) >= self.formats.len() {
                return Err(PresentationError::Format);
            }
        }
        validate_ranges(
            text,
            self.spans
                .iter()
                .map(|span| (span.line, span.start, span.end)),
        )
    }
}

fn validate_ranges(
    text: &str,
    ranges: impl Iterator<Item = (u32, u32, u32)>,
) -> Result<(), PresentationError> {
    let mut lines = crate::editor_lines(text).enumerate();
    let mut current = None;
    let mut previous: Option<(u32, u32)> = None;
    for (span_line, start, end) in ranges {
        if start > end
            || previous
                .is_some_and(|(line, end)| span_line < line || (span_line == line && start < end))
        {
            return Err(PresentationError::Range);
        }
        if current.is_none() {
            current = lines.next();
        }
        while current.is_some_and(|(line, _)| line < span_line as usize) {
            current = lines.next();
        }
        let Some((line, source)) = current else {
            return Err(PresentationError::Range);
        };
        if line != span_line as usize
            || !source.is_char_boundary(start as usize)
            || !source.is_char_boundary(end as usize)
        {
            return Err(PresentationError::Range);
        }
        previous = Some((span_line, end));
    }
    Ok(())
}

fn decode_bounded<'de, D, T, const LIMIT: usize>(d: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let values = crate::editor_transaction::decode_bounded::<D, T, LIMIT>(d)?;
    crate::budget::spend(values.len()).map_err(serde::de::Error::custom)?;
    Ok(values)
}

fn decode_formats<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<EditorFormat>, D::Error> {
    decode_bounded::<D, _, MAX_EDITOR_FORMATS>(d)
}

fn decode_spans<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<EditorSpan>, D::Error> {
    decode_bounded::<D, _, MAX_EDITOR_SPANS>(d)
}

fn decode_menu_items<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<EditorMenuItem>, D::Error> {
    decode_bounded::<D, _, MAX_EDITOR_MENU_ITEMS>(d)
}
fn decode_gutters<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<EditorGutter>, D::Error> {
    decode_bounded::<D, _, MAX_EDITOR_SPANS>(d)
}
fn decode_boundaries<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u32>, D::Error> {
    decode_bounded::<D, _, MAX_EDITOR_SPANS>(d)
}
fn decode_margins<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<EditorMargin>, D::Error> {
    decode_bounded::<D, _, MAX_EDITOR_SPANS>(d)
}
fn decode_hits<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<EditorHit>, D::Error> {
    decode_bounded::<D, _, MAX_EDITOR_SPANS>(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_is_rejected_instead_of_silently_truncating_interactions() {
        let mut value = EditorPresentation::default();
        value.affordances.menu = Some(EditorMenu {
            anchor: EditorMenuAnchor::Caret,
            items: (0..=MAX_EDITOR_MENU_ITEMS)
                .map(|index| EditorMenuItem {
                    tag: index.to_string(),
                    label: format!("Action {index}"),
                })
                .collect(),
            selected: 0,
        });
        let mut budget = crate::MAX_STRING_BYTES;
        value.sanitize(&mut budget);
        assert!(
            crate::decode::<EditorPresentation>(&crate::encode(&value)).is_err(),
            "over-budget action lists must reject the frame, not publish a different menu"
        );
    }

    fn presentation(spans: Vec<EditorSpan>) -> EditorPresentation {
        EditorPresentation {
            formats: vec![EditorFormat::default()],
            spans,
            ..Default::default()
        }
    }

    #[test]
    fn only_explicit_hit_ranges_consume_a_press() {
        let affordances = EditorAffordances {
            hits: vec![EditorHit {
                line: 1,
                start: 2,
                end: 5,
                tag: 7,
            }],
            ..Default::default()
        };
        let position = crate::EditorPosition { line: 1, column: 3 };
        assert_eq!(
            affordances.hit(position),
            Some(EditorInteraction::LinePress { tag: 7, position })
        );
        for position in [
            crate::EditorPosition { line: 0, column: 3 },
            crate::EditorPosition { line: 1, column: 1 },
            crate::EditorPosition { line: 1, column: 5 },
        ] {
            assert_eq!(affordances.hit(position), None);
        }
    }

    #[test]
    fn affordances_cannot_address_missing_lines_or_invalid_menu_items() {
        let menu = EditorMenu {
            anchor: EditorMenuAnchor::Caret,
            items: vec![EditorMenuItem {
                tag: "heading".into(),
                label: "Heading".into(),
            }],
            selected: 0,
        };
        let mut value = EditorPresentation::default();
        value.affordances.menu = Some(menu.clone());
        assert_eq!(value.validate("Title\nbody"), Ok(()));
        value.affordances.menu.as_mut().unwrap().selected = 1;
        assert!(value.validate("Title\nbody").is_err());
        value.affordances.menu = Some(EditorMenu {
            anchor: EditorMenuAnchor::Line(2),
            ..menu
        });
        assert!(value.validate("Title\nbody").is_err());
        value.affordances.menu = None;
        value.affordances.gutters.push(EditorGutter {
            line: 2,
            plus: true,
            handle: true,
        });
        assert!(value.validate("Title\nbody").is_err());
        value.affordances.gutters.clear();
        value.affordances.hits.push(EditorHit {
            line: 1,
            start: 1,
            end: 3,
            tag: 0,
        });
        assert!(value.validate("Title\n한글").is_err());
    }

    #[test]
    fn validates_byte_ranges_against_actual_logical_lines() {
        let source = "Title\n- 한글\n";
        let good = EditorSpan {
            line: 1,
            start: 2,
            end: 8,
            format: 0,
        };
        assert_eq!(presentation(vec![good]).validate(source), Ok(()));
        for bad in [
            EditorSpan { start: 3, ..good },
            EditorSpan { end: 9, ..good },
            EditorSpan {
                start: 8,
                end: 2,
                ..good
            },
            EditorSpan { line: 3, ..good },
        ] {
            assert_eq!(
                presentation(vec![bad]).validate(source),
                Err(PresentationError::Range)
            );
        }
        let empty = EditorSpan {
            line: 2,
            start: 0,
            end: 0,
            format: 0,
        };
        assert_eq!(presentation(vec![good, empty]).validate(source), Ok(()));
    }

    #[test]
    fn rejects_ambiguous_order_overlap_and_missing_formats() {
        let span = EditorSpan {
            line: 0,
            start: 0,
            end: 2,
            format: 0,
        };
        assert_eq!(
            presentation(vec![span, span]).validate("abc"),
            Err(PresentationError::Range)
        );
        assert_eq!(
            presentation(vec![EditorSpan { format: 1, ..span }]).validate("abc"),
            Err(PresentationError::Format)
        );
        assert_eq!(
            presentation(vec![EditorSpan { line: 1, ..span }, span]).validate("abc\ndef"),
            Err(PresentationError::Range)
        );
    }

    #[test]
    fn presentation_limits_do_not_consume_or_truncate_document_text() {
        let source = "a".repeat(1024 * 1024);
        assert_eq!(EditorPresentation::default().validate(&source), Ok(()));
        let span = EditorSpan {
            line: 0,
            start: 0,
            end: 0,
            format: 0,
        };
        assert_eq!(
            presentation(vec![span; MAX_EDITOR_SPANS + 1]).validate(&source),
            Err(PresentationError::Limit)
        );
        let oversized = EditorPresentation {
            formats: vec![EditorFormat::default(); MAX_EDITOR_FORMATS + 1],
            spans: vec![],
            ..Default::default()
        };
        assert_eq!(oversized.validate(&source), Err(PresentationError::Limit));
    }

    #[test]
    fn decoder_rejects_oversized_collections_before_host_validation() {
        let oversized = EditorPresentation {
            formats: vec![EditorFormat::default(); MAX_EDITOR_FORMATS + 1],
            spans: vec![],
            ..Default::default()
        };
        let bytes = bincode::serialize(&oversized).unwrap();
        assert!(bincode::deserialize::<EditorPresentation>(&bytes).is_err());
        let span = EditorSpan {
            line: 0,
            start: 0,
            end: 0,
            format: 0,
        };
        let bytes = bincode::serialize(&presentation(vec![span; MAX_EDITOR_SPANS + 1])).unwrap();
        assert!(bincode::deserialize::<EditorPresentation>(&bytes).is_err());
    }
}
