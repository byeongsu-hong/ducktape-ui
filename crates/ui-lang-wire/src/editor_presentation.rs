//! Declarative editor formatting. Document identity is the enclosing Editor node's reference.
use crate::{Border, Edges, LineHeight, NamedFont, Rgba};
use serde::{Deserialize, Serialize};

/// Presentation is bounded independently of the canonical document bytes.
pub const MAX_EDITOR_FORMATS: usize = 256;
pub const MAX_EDITOR_SPANS: usize = 32_768;

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationError {
    Limit,
    Format,
    Range,
}

impl EditorPresentation {
    /// Validate against the exact resident document before any source is hidden.
    pub fn validate(&self, text: &str) -> Result<(), PresentationError> {
        if self.formats.len() > MAX_EDITOR_FORMATS || self.spans.len() > MAX_EDITOR_SPANS {
            return Err(PresentationError::Limit);
        }
        let mut lines = crate::editor_lines(text).enumerate();
        let mut current = lines.next();
        let mut previous: Option<(u32, u32)> = None;
        for span in &self.spans {
            if usize::from(span.format) >= self.formats.len() {
                return Err(PresentationError::Format);
            }
            if span.start > span.end
                || previous.is_some_and(|(line, end)| {
                    span.line < line || (span.line == line && span.start < end)
                })
            {
                return Err(PresentationError::Range);
            }
            while current.is_some_and(|(line, _)| line < span.line as usize) {
                current = lines.next();
            }
            let Some((line, source)) = current else {
                return Err(PresentationError::Range);
            };
            if line != span.line as usize
                || !source.is_char_boundary(span.start as usize)
                || !source.is_char_boundary(span.end as usize)
            {
                return Err(PresentationError::Range);
            }
            previous = Some((span.line, span.end));
        }
        Ok(())
    }
}

fn decode_bounded<'de, D, T, const LIMIT: usize>(d: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Bounded<T, const LIMIT: usize>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const LIMIT: usize> serde::de::Visitor<'de> for Bounded<T, LIMIT> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a bounded editor presentation collection")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            if seq.size_hint().is_some_and(|count| count > LIMIT) {
                return Err(serde::de::Error::custom("editor presentation count limit"));
            }
            let mut result = Vec::new();
            while let Some(value) = seq.next_element()? {
                if result.len() == LIMIT {
                    return Err(serde::de::Error::custom("editor presentation count limit"));
                }
                result.push(value);
            }
            Ok(result)
        }
    }
    d.deserialize_seq(Bounded::<T, LIMIT>(std::marker::PhantomData))
}

fn decode_formats<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<EditorFormat>, D::Error> {
    decode_bounded::<D, _, MAX_EDITOR_FORMATS>(d)
}

fn decode_spans<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<EditorSpan>, D::Error> {
    decode_bounded::<D, _, MAX_EDITOR_SPANS>(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn presentation(spans: Vec<EditorSpan>) -> EditorPresentation {
        EditorPresentation {
            formats: vec![EditorFormat::default()],
            spans,
        }
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
        };
        assert_eq!(oversized.validate(&source), Err(PresentationError::Limit));
    }

    #[test]
    fn decoder_rejects_oversized_collections_before_host_validation() {
        let oversized = EditorPresentation {
            formats: vec![EditorFormat::default(); MAX_EDITOR_FORMATS + 1],
            spans: vec![],
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
