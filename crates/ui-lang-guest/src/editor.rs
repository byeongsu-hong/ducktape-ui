//! Plain document state; the host retains its native editor between observations.
use crate::wire;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Editor(wire::EditorState);
impl Editor {
    pub fn new(text: impl Into<String>) -> Self {
        let mut state = wire::EditorState {
            text: text.into(),
            ..Default::default()
        };
        state.sanitize();
        Self(state)
    }
    pub fn text(&self) -> String {
        self.0.text.clone()
    }
    pub fn cursor(&self) -> wire::EditorCursor {
        self.0.cursor
    }
    pub fn observation_revision(&self) -> u64 {
        self.0.revision
    }
    pub fn reset_revision(&self) -> u64 {
        self.0.reset
    }
    pub fn line_count(&self) -> usize {
        wire::editor_lines(&self.0.text).count()
    }
    pub fn line(&self, line: usize) -> Option<String> {
        wire::editor_lines(&self.0.text)
            .nth(line)
            .map(str::to_owned)
    }
    /// An authoritative assignment, including an identical-text document replacement.
    pub fn replace(&mut self, mut next: Self, previous_reset: u64) {
        next.0.reset = previous_reset
            .checked_add(1)
            .expect("editor reset revisions exhausted");
        next.0.sanitize();
        *self = next;
    }
    /// Observations from a previous document cannot overwrite a replacement.
    pub fn accept(&mut self, mut state: wire::EditorState) {
        if state.reset == self.0.reset && state.revision > self.0.revision {
            state.sanitize();
            self.0 = state;
        }
    }
    pub fn move_to(&mut self, mut cursor: wire::EditorCursor) {
        cursor.clamp(&self.0.text);
        self.0.cursor = cursor;
        self.0.reset = self
            .0
            .reset
            .checked_add(1)
            .expect("editor reset revisions exhausted");
    }
    pub fn snapshot(&self) -> Vec<u8> {
        wire::encode(&self.0)
    }
    pub fn restore(bytes: &[u8]) -> Option<Self> {
        let mut state: wire::EditorState = wire::decode(bytes).ok()?;
        let original = state.clone();
        state.sanitize();
        (state == original).then_some(Self(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observations_do_not_reset_and_old_document_events_cannot_replace_new_state() {
        let mut editor = Editor::new("a");
        let observed = wire::EditorState {
            text: "한글".into(),
            cursor: wire::EditorCursor {
                position: wire::EditorPosition { line: 0, column: 6 },
                selection: Some(wire::EditorPosition { line: 0, column: 0 }),
            },
            reset: 0,
            revision: 100,
        };
        editor.accept(observed.clone());
        assert_eq!(editor.text(), "한글");
        assert_eq!(editor.reset_revision(), 0);
        assert_eq!(editor.cursor().selection.unwrap().column, 0);
        let mut stale = observed.clone();
        stale.revision = 99;
        stale.text = "old".into();
        editor.accept(stale);
        assert_eq!(editor.text(), "한글");
        assert_eq!(Editor::restore(&editor.snapshot()), Some(editor.clone()));
        editor.replace(Editor::new("한글"), editor.reset_revision());
        assert_eq!(editor.reset_revision(), 1);
        assert_eq!(editor.cursor().selection, None);
        editor.accept(observed);
        assert_eq!(
            editor.cursor().position.column,
            0,
            "late old-document cursor stays rejected"
        );
    }
}
