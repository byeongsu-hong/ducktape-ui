//! Atomic editor patch validation shared by native and guest transaction lanes.
use serde::{Deserialize, Serialize};

use crate::EditorCursor;

/// A replacement in the pre-transaction UTF-8 document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorPatch {
    pub start_byte: u32,
    pub end_byte: u32,
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
    let mut previous_end = 0;
    let mut removed = 0;
    let mut inserted = 0usize;
    for patch in patches {
        let start = patch.start_byte as usize;
        let end = patch.end_byte as usize;
        if start < previous_end
            || start > end
            || end > text.len()
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
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
