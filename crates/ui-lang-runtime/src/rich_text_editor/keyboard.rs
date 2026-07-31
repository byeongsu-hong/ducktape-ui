use super::movement::{move_cursor, uses_rich_geometry};
use super::{Action, State};
use iced::advanced::{Clipboard, Shell, text};
use iced::keyboard::{self, key};
use iced::widget::text_editor::{self, Binding, Content, Edit, Motion};
use std::sync::Arc;

pub(super) fn rich_binding(press: &text_editor::KeyPress) -> Option<Binding<Edit>> {
    match press.modified_key.as_ref() {
        keyboard::Key::Named(key::Named::Tab) if press.modifiers.shift() => {
            Some(Binding::Custom(Edit::Unindent))
        }
        keyboard::Key::Named(key::Named::Tab) => Some(Binding::Custom(Edit::Indent)),
        keyboard::Key::Named(key::Named::Backspace) if press.modifiers.jump() => {
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::WordLeft),
                Binding::Backspace,
            ]))
        }
        keyboard::Key::Named(key::Named::Backspace) if press.modifiers.macos_command() => {
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::Home),
                Binding::Backspace,
            ]))
        }
        keyboard::Key::Named(key::Named::Delete)
            if press.modifiers.jump()
                && (press.text.is_none() || press.text.as_deref() == Some("\u{7f}")) =>
        {
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::WordRight),
                Binding::Delete,
            ]))
        }
        keyboard::Key::Named(key::Named::Delete)
            if press.modifiers.macos_command()
                && (press.text.is_none() || press.text.as_deref() == Some("\u{7f}")) =>
        {
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::End),
                Binding::Delete,
            ]))
        }
        _ => None,
    }
}

pub(super) fn editor_binding(press: &text_editor::KeyPress) -> Option<Binding<Edit>> {
    if command_shortcut_bubbles(press) {
        return None;
    }

    rich_binding(press).or_else(|| Binding::<Edit>::from_key_press(press.clone()))
}

fn command_shortcut_bubbles(press: &text_editor::KeyPress) -> bool {
    if !press.modifiers.command() {
        return false;
    }

    match press.key.to_latin(press.physical_key) {
        Some('a' | 'c' | 'x') => false,
        Some('v') => press.modifiers.alt(),
        Some(_) => true,
        None => false,
    }
}

#[derive(Debug, Default)]
pub(super) struct PendingImeCommit {
    content: Option<String>,
}

impl PendingImeCommit {
    pub(super) fn clear(&mut self) {
        self.content = None;
    }

    pub(super) fn is_pending(&self) -> bool {
        self.content.is_some()
    }

    pub(super) fn on_preedit(&mut self, content: &str) {
        // The built-in macOS Korean IME emits an additional empty preedit
        // after Commit. It is still part of the same key event, so only a new
        // non-empty composition supersedes the pending boundary.
        if !content.is_empty() {
            self.clear();
        }
    }

    pub(super) fn on_commit(&mut self, content: &str) {
        self.content = Some(content.to_owned());
    }

    pub(super) fn resolve(&mut self, character: Option<char>) -> ImeBoundary {
        let Some(character) = character else {
            return ImeBoundary::Unrelated;
        };
        let Some(committed) = self.content.take() else {
            return ImeBoundary::Unrelated;
        };

        if committed.ends_with(character) {
            ImeBoundary::Duplicate
        } else {
            ImeBoundary::Missing(character)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ImeBoundary {
    Missing(char),
    Duplicate,
    Unrelated,
}

pub(super) fn single_printable_ascii(text: &str) -> Option<char> {
    let mut characters = text.chars();
    let character = characters.next()?;
    (characters.next().is_none() && character.is_ascii() && !character.is_ascii_control())
        .then_some(character)
}

fn logical_ascii_character(key: &keyboard::Key) -> Option<char> {
    match key.as_ref() {
        keyboard::Key::Character(text) => single_printable_ascii(text),
        keyboard::Key::Named(key::Named::Space) => Some(' '),
        _ => None,
    }
}

fn physical_ime_boundary_fallback(code: key::Code, shift: bool) -> Option<char> {
    Some(match (code, shift) {
        (key::Code::Comma, false) => ',',
        (key::Code::Comma, true) => '<',
        (key::Code::Period, false) => '.',
        (key::Code::Period, true) => '>',
        (key::Code::Space, _) => ' ',
        _ => return None,
    })
}

pub(super) fn ime_boundary_character(
    key: &keyboard::Key,
    modified_key: &keyboard::Key,
    physical_key: key::Physical,
    modifiers: keyboard::Modifiers,
) -> Option<char> {
    if modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }

    logical_ascii_character(modified_key)
        .or_else(|| {
            if modifiers.shift() {
                None
            } else {
                logical_ascii_character(key)
            }
        })
        .or_else(|| {
            let key::Physical::Code(code) = physical_key else {
                return None;
            };
            physical_ime_boundary_fallback(code, modifiers.shift())
        })
}

pub(super) fn apply_binding<H, Message>(
    binding: Binding<Edit>,
    content: &Content,
    state: &mut State<H>,
    on_action: &dyn Fn(Action) -> Message,
    clipboard: &mut dyn Clipboard,
    shell: &mut Shell<'_, Message>,
) where
    H: text::Highlighter,
{
    let publish = |shell: &mut Shell<'_, Message>, action| {
        shell.publish(on_action(action));
    };

    match binding {
        Binding::Unfocus => {
            state.focus = None;
            state.drag_anchor = None;
            state.drag_moved = false;
            state.release_bubbles = None;
        }
        Binding::Copy => {
            if let Some(selection) = content.selection() {
                clipboard.write(iced::advanced::clipboard::Kind::Standard, selection);
            }
        }
        Binding::Cut => {
            if let Some(selection) = content.selection() {
                clipboard.write(iced::advanced::clipboard::Kind::Standard, selection);
                publish(shell, Action::Edit(text_editor::Action::Edit(Edit::Delete)));
            }
            state.preferred_x = None;
        }
        Binding::Paste => {
            if let Some(source) = clipboard.read(iced::advanced::clipboard::Kind::Standard) {
                publish(
                    shell,
                    Action::Edit(text_editor::Action::Edit(Edit::Paste(Arc::new(source)))),
                );
            }
            state.preferred_x = None;
        }
        Binding::Move(motion) => {
            if uses_rich_geometry(motion) {
                let cursor = move_cursor(state, content.cursor(), motion, false);
                publish(shell, Action::MoveTo(cursor));
            } else {
                publish(shell, Action::Edit(text_editor::Action::Move(motion)));
                state.preferred_x = None;
            }
        }
        Binding::Select(motion) => {
            if uses_rich_geometry(motion) {
                let cursor = move_cursor(state, content.cursor(), motion, true);
                publish(shell, Action::MoveTo(cursor));
            } else {
                publish(shell, Action::Edit(text_editor::Action::Select(motion)));
                state.preferred_x = None;
            }
        }
        Binding::SelectWord => {
            publish(shell, Action::Edit(text_editor::Action::SelectWord));
            state.preferred_x = None;
        }
        Binding::SelectLine => {
            publish(shell, Action::Edit(text_editor::Action::SelectLine));
            state.preferred_x = None;
        }
        Binding::SelectAll => {
            publish(shell, Action::Edit(text_editor::Action::SelectAll));
            state.preferred_x = None;
        }
        Binding::Insert(character) => {
            publish(
                shell,
                Action::Edit(text_editor::Action::Edit(Edit::Insert(character))),
            );
            state.preferred_x = None;
        }
        Binding::Enter => {
            publish(shell, Action::Edit(text_editor::Action::Edit(Edit::Enter)));
            state.preferred_x = None;
        }
        Binding::Backspace => {
            publish(
                shell,
                Action::Edit(text_editor::Action::Edit(Edit::Backspace)),
            );
            state.preferred_x = None;
        }
        Binding::Delete => {
            publish(shell, Action::Edit(text_editor::Action::Edit(Edit::Delete)));
            state.preferred_x = None;
        }
        Binding::Sequence(bindings) => {
            for binding in bindings {
                apply_binding(binding, content, state, on_action, clipboard, shell);
            }
        }
        Binding::Custom(edit) => {
            publish(shell, Action::Edit(text_editor::Action::Edit(edit)));
            state.preferred_x = None;
        }
    }
}
