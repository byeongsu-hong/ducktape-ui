//! Atomic editor patch validation shared by native and guest transaction lanes.
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

use crate::EditorCursor;

/// A replacement in the pre-transaction UTF-8 document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorPatch {
    pub start_byte: u32,
    pub end_byte: u32,
    #[serde(deserialize_with = "decode_replacement")]
    pub replacement: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorPatchError {
    Limit,
    Range,
    Cursor,
}

pub const MAX_EDITOR_PATCHES: usize = 256;

/// Validate the complete batch before any native Content is mutated.
pub fn patched_editor_text(
    text: &str,
    patches: &[EditorPatch],
    cursor: EditorCursor,
) -> Result<String, EditorPatchError> {
    if text.len() > crate::MAX_STRING_BYTES || patches.len() > MAX_EDITOR_PATCHES {
        return Err(EditorPatchError::Limit);
    }
    // Native selection positions cannot address the middle of a grapheme
    // or either two-byte line terminator accepted by Content.
    let boundaries: Vec<_> = text
        .grapheme_indices(true)
        .map(|(at, _)| at)
        .chain(std::iter::once(text.len()))
        .filter(|at| {
            !(*at > 0
                && *at < text.len()
                && matches!(&text.as_bytes()[at - 1..=*at], b"\r\n" | b"\n\r"))
        })
        .collect();
    let mut previous_end = 0;
    let mut removed = 0;
    let mut inserted = 0usize;
    for patch in patches {
        let start = patch.start_byte as usize;
        let end = patch.end_byte as usize;
        if start < previous_end
            || start > end
            || end > text.len()
            || boundaries.binary_search(&start).is_err()
            || boundaries.binary_search(&end).is_err()
        {
            return Err(EditorPatchError::Range);
        }
        previous_end = end;
        removed += end - start;
        inserted = inserted
            .checked_add(patch.replacement.len())
            .filter(|bytes| *bytes <= crate::MAX_STRING_BYTES)
            .ok_or(EditorPatchError::Limit)?;
    }
    let len = (text.len() - removed)
        .checked_add(inserted)
        .filter(|bytes| *bytes <= crate::MAX_STRING_BYTES)
        .ok_or(EditorPatchError::Limit)?;
    let mut result = String::with_capacity(len);
    let mut offset = 0;
    for patch in patches {
        result.push_str(&text[offset..patch.start_byte as usize]);
        result.push_str(&patch.replacement);
        offset = patch.end_byte as usize;
    }
    result.push_str(&text[offset..]);
    let mut valid = cursor;
    valid.clamp(&result);
    if valid != cursor {
        return Err(EditorPatchError::Cursor);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EditorPosition, MAX_STRING_BYTES};

    fn patch(start: u32, end: u32, replacement: &str) -> EditorPatch {
        EditorPatch {
            start_byte: start,
            end_byte: end,
            replacement: replacement.into(),
        }
    }

    #[test]
    fn a_batch_uses_original_offsets_and_preserves_unicode_cursor() {
        let text = "1. 한글\n2. next";
        let cursor = EditorCursor {
            position: EditorPosition { line: 0, column: 7 },
            selection: Some(EditorPosition { line: 1, column: 4 }),
        };
        assert_eq!(
            patched_editor_text(text, &[patch(0, 1, "10"), patch(10, 11, "11")], cursor),
            Ok("10. 한글\n11. next".into())
        );
        assert_eq!(text, "1. 한글\n2. next");
    }

    #[test]
    fn endpoints_cannot_split_native_graphemes_or_line_terminators() {
        for (text, at) in [("a\r\nb", 2), ("a\n\rb", 2), ("e\u{301}", 1), ("👍🏽", 4)] {
            assert_eq!(
                patched_editor_text(text, &[patch(at, at, "X")], EditorCursor::default()),
                Err(EditorPatchError::Range),
                "{text:?} at {at}"
            );
        }
    }

    #[test]
    fn malformed_late_patch_rejects_the_entire_batch() {
        assert_eq!(
            patched_editor_text(
                "한글",
                &[patch(0, 3, "A"), patch(4, 6, "B")],
                EditorCursor::default()
            ),
            Err(EditorPatchError::Range)
        );
        assert_eq!(
            patched_editor_text(
                "abc",
                &[patch(0, 2, "A"), patch(1, 3, "B")],
                EditorCursor::default()
            ),
            Err(EditorPatchError::Range)
        );
        assert_eq!(
            patched_editor_text(
                "abc",
                &[patch(2, 3, "A"), patch(0, 1, "B")],
                EditorCursor::default()
            ),
            Err(EditorPatchError::Range)
        );
    }

    #[test]
    fn final_cursor_must_be_valid_without_silent_clamping() {
        let cursor = EditorCursor {
            position: EditorPosition { line: 0, column: 1 },
            selection: None,
        };
        assert_eq!(
            patched_editor_text("", &[patch(0, 0, "e\u{301}")], cursor),
            Err(EditorPatchError::Cursor)
        );
    }

    #[test]
    fn transaction_limits_reject_instead_of_truncating() {
        assert_eq!(
            patched_editor_text(
                "",
                &[patch(0, 0, &"x".repeat(MAX_STRING_BYTES + 1))],
                EditorCursor::default()
            ),
            Err(EditorPatchError::Limit)
        );
        assert_eq!(
            patched_editor_text(
                "",
                &vec![patch(0, 0, ""); MAX_EDITOR_PATCHES + 1],
                EditorCursor::default()
            ),
            Err(EditorPatchError::Limit)
        );
    }
}

/// Explicit claims are evaluated by the native host after IME processing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorKeyClaim {
    pub key: crate::keyboard::Key,
    pub modifiers: crate::keyboard::Modifiers,
    /// Add the host platform's command modifier (logo on macOS, control elsewhere).
    pub command: bool,
}
impl EditorKeyClaim {
    pub fn matches(&self, key: &crate::keyboard::KeyState, macos: bool) -> bool {
        let mut modifiers = self.modifiers;
        if self.command {
            if macos {
                modifiers.logo = true;
            } else {
                modifiers.control = true;
            }
        }
        self.key == key.key && modifiers == key.modifiers
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorBinding {
    #[serde(deserialize_with = "decode_claims")]
    pub claims: Vec<EditorKeyClaim>,
    pub on_request: u32,
    pub on_event: u32,
}

/// The response must echo all fields, including the retry attempt and observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorTransactionId {
    pub instance: u64,
    #[serde(deserialize_with = "decode_document")]
    pub document: String,
    pub reset: u64,
    pub sequence: u64,
    pub attempt: u32,
    pub text_revision: u64,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorKeyRequest {
    pub id: EditorTransactionId,
    pub state: crate::EditorState,
    pub key: crate::keyboard::KeyState,
    pub repeat: bool,
    pub input_time_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorHistoryEffect {
    Native,
    NewGroup,
    ExtendPrevious,
    Undo,
    Redo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorEditKind {
    Insert,
    Paste,
    ImeCommit,
    Enter,
    Backspace,
    Delete,
    Indent,
    Unindent,
    Cut,
    Cursor,
    GuestPatch,
    Undo,
    Redo,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorDecision {
    DefaultEditorAction,
    Noop,
    Apply {
        #[serde(deserialize_with = "decode_patches")]
        patches: Vec<EditorPatch>,
        cursor: EditorCursor,
        history: EditorHistoryEffect,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorResponse {
    pub id: EditorTransactionId,
    pub decision: EditorDecision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorFault {
    Overflow,
    Timeout,
    Conflicts,
    InvalidResponse,
    Limit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorTransactionEvent {
    Commit {
        id: EditorTransactionId,
        before: crate::EditorState,
        after: crate::EditorState,
        kind: EditorEditKind,
        history: EditorHistoryEffect,
        input_time_ms: u64,
    },
    Fault {
        id: EditorTransactionId,
        state: crate::EditorState,
        reason: EditorFault,
    },
    Cancelled {
        id: EditorTransactionId,
        state: crate::EditorState,
    },
}

pub const MAX_EDITOR_CLAIMS: usize = 32;
pub const MAX_EDITOR_RESPONSES: usize = 128;

fn decode_bounded<'de, D, T, const LIMIT: usize>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Bounded<T, const LIMIT: usize>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const LIMIT: usize> serde::de::Visitor<'de> for Bounded<T, LIMIT> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a bounded editor transaction sequence")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            if seq.size_hint().is_some_and(|n| n > LIMIT) {
                return Err(serde::de::Error::custom("editor transaction count limit"));
            }
            let mut out = Vec::new();
            while let Some(item) = seq.next_element()? {
                if out.len() == LIMIT {
                    return Err(serde::de::Error::custom("editor transaction count limit"));
                }
                out.push(item);
            }
            Ok(out)
        }
    }
    deserializer.deserialize_seq(Bounded::<T, LIMIT>(std::marker::PhantomData))
}
fn decode_claims<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<EditorKeyClaim>, D::Error> {
    decode_bounded::<D, _, MAX_EDITOR_CLAIMS>(d)
}
fn decode_patches<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<EditorPatch>, D::Error> {
    decode_bounded::<D, _, MAX_EDITOR_PATCHES>(d)
}
pub(crate) fn decode_responses<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<EditorResponse>, D::Error> {
    let responses = decode_bounded::<D, _, MAX_EDITOR_RESPONSES>(d)?;
    let bytes: usize = responses
        .iter()
        .map(|response: &EditorResponse| match &response.decision {
            EditorDecision::Apply { patches, .. } => {
                patches.iter().map(|patch| patch.replacement.len()).sum()
            }
            _ => 0,
        })
        .sum();
    if bytes > crate::MAX_STRING_BYTES {
        return Err(serde::de::Error::custom(
            "editor response aggregate byte limit",
        ));
    }
    Ok(responses)
}

thread_local! {
    static REPLACEMENT_BYTES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
pub(crate) fn reset_decode_budget() {
    REPLACEMENT_BYTES.with(|bytes| bytes.set(0));
}
fn decode_replacement<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    decode_text(d, crate::MAX_STRING_BYTES, true)
}
pub(crate) fn decode_document<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    decode_text(d, 1024, false)
}
fn decode_text<'de, D: serde::Deserializer<'de>>(
    d: D,
    limit: usize,
    replacement: bool,
) -> Result<String, D::Error> {
    struct Text {
        limit: usize,
        replacement: bool,
    }
    impl<'de> serde::de::Visitor<'de> for Text {
        type Value = String;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded editor text")
        }
        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<String, E> {
            if value.len() > self.limit {
                return Err(E::custom("editor text limit"));
            }
            if self.replacement {
                let accepted = REPLACEMENT_BYTES.with(|bytes| {
                    match bytes
                        .get()
                        .checked_add(value.len())
                        .filter(|n| *n <= crate::MAX_STRING_BYTES)
                    {
                        Some(next) => {
                            bytes.set(next);
                            true
                        }
                        None => false,
                    }
                });
                if !accepted {
                    return Err(E::custom("editor aggregate replacement limit"));
                }
            }
            Ok(value.to_owned())
        }
    }
    d.deserialize_str(Text { limit, replacement })
}

#[cfg(test)]
mod protocol_tests {
    use super::*;

    fn response(replacement: String) -> EditorResponse {
        EditorResponse {
            id: EditorTransactionId {
                instance: 1,
                document: "app:draft".into(),
                reset: 0,
                sequence: 1,
                attempt: 1,
                text_revision: 0,
                revision: 0,
            },
            decision: EditorDecision::Apply {
                patches: vec![EditorPatch {
                    start_byte: 0,
                    end_byte: 0,
                    replacement,
                }],
                cursor: crate::EditorCursor::default(),
                history: EditorHistoryEffect::NewGroup,
            },
        }
    }

    #[test]
    fn decoder_rejects_aggregate_patch_bytes_and_resets_budget_after_failure() {
        let mut frame = crate::Frame {
            upstream_sanitization: Default::default(),
            editor_decisions: vec![response("a".repeat(crate::MAX_STRING_BYTES / 2 + 1)); 2],
            ..Default::default()
        };
        assert!(crate::decode::<crate::Frame>(&crate::encode(&frame)).is_err());
        frame.editor_decisions = vec![response("ok".into())];
        assert!(crate::decode::<crate::Frame>(&crate::encode(&frame)).is_ok());
    }

    #[test]
    fn decoder_rejects_excess_claims_and_responses() {
        let binding = EditorBinding {
            claims: vec![
                EditorKeyClaim {
                    key: crate::keyboard::Key::Named(crate::keyboard::Named::Tab),
                    modifiers: crate::keyboard::Modifiers::default(),
                    command: false,
                };
                MAX_EDITOR_CLAIMS + 1
            ],
            on_request: 1,
            on_event: 2,
        };
        assert!(crate::decode::<EditorBinding>(&crate::encode(&binding)).is_err());
        let frame = crate::Frame {
            upstream_sanitization: Default::default(),
            editor_decisions: vec![response(String::new()); MAX_EDITOR_RESPONSES + 1],
            ..Default::default()
        };
        assert!(crate::decode::<crate::Frame>(&crate::encode(&frame)).is_err());
    }
}
