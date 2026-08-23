use iced::keyboard::{Key, Modifiers, key};
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::fmt;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct EditorError {
    pub message: String,
}

impl fmt::Display for EditorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for EditorError {}

pub async fn open_url(url: String) -> Result<(), EditorError> {
    if !safe_web_url(&url) {
        return Err(EditorError {
            message: "Only http:// and https:// links can be opened".into(),
        });
    }
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler");
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");
    #[cfg(not(any(unix, target_os = "windows")))]
    return Err(EditorError {
        message: "Opening links is not supported on this platform".into(),
    });

    let mut child = command.arg(&url).spawn().map_err(|error| EditorError {
        message: format!("Could not open {url}: {error}"),
    })?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

pub fn link_at_cursor(line: Option<String>, column: i64) -> String {
    let Some(line) = line else {
        return String::new();
    };
    let Ok(column) = usize::try_from(column) else {
        return String::new();
    };
    link_at(&line, column)
}

pub fn link_at(line: &str, column: usize) -> String {
    Parser::new_ext(line, Options::ENABLE_STRIKETHROUGH)
        .into_offset_iter()
        .find_map(|(event, range)| match event {
            Event::Start(Tag::Link { dest_url, .. })
                if range.start <= column && column <= range.end =>
            {
                let url = dest_url.into_string();
                safe_web_url(&url).then_some(url)
            }
            _ => None,
        })
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shortcut {
    New,
    Save,
    Undo,
    Redo,
    Find,
    Bold,
    Italic,
    Code,
    Link,
    Escape,
}

fn shortcut(key: Key, physical: key::Physical, modifiers: Modifiers) -> Option<Shortcut> {
    if let Key::Named(key::Named::Escape) = key.as_ref() {
        return Some(Shortcut::Escape);
    }
    if !modifiers.command() {
        return None;
    }
    let key = key.to_latin(physical)?.to_ascii_lowercase();
    Some(match (key, modifiers.shift()) {
        ('n', _) => Shortcut::New,
        ('s', _) => Shortcut::Save,
        ('z', true) | ('y', _) => Shortcut::Redo,
        ('z', false) => Shortcut::Undo,
        ('f', _) => Shortcut::Find,
        ('b', _) => Shortcut::Bold,
        ('i', _) => Shortcut::Italic,
        ('k', _) => Shortcut::Link,
        ('`', _) => Shortcut::Code,
        _ => return None,
    })
}

macro_rules! shortcut_filters {
    ($($name:ident => $shortcut:ident),+ $(,)?) => {$(
        pub fn $name(press: crate::__IceKeyPress) -> Option<()> {
            (shortcut(press.key, press.physical_key, press.modifiers) == Some(Shortcut::$shortcut))
                .then_some(())
        }
    )+};
}

shortcut_filters! {
    new_shortcut => New,
    save_shortcut => Save,
    undo_shortcut => Undo,
    redo_shortcut => Redo,
    find_shortcut => Find,
    bold_shortcut => Bold,
    italic_shortcut => Italic,
    code_shortcut => Code,
    link_shortcut => Link,
    escape_shortcut => Escape,
}

pub fn cursor_status(line: i64, column: i64, lines: i64) -> String {
    format!("Ln {}, Col {}  ·  {} lines", line + 1, column + 1, lines)
}

fn safe_web_url(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && !url.chars().any(char::is_control)
        && !url.contains(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::{Shortcut, link_at_cursor, safe_web_url, shortcut};

    #[test]
    fn resolves_only_the_link_under_the_cursor() {
        let line = Some("before [site](https://example.com) after".to_owned());

        assert_eq!(link_at_cursor(line.clone(), 10), "https://example.com");
        assert_eq!(link_at_cursor(line, 1), "");
        assert!(!safe_web_url("file:///tmp/private"));
    }

    #[test]
    fn undo_and_redo_shortcuts_survive_a_non_latin_input_source() {
        use iced::keyboard::key::{Code, Physical};
        use iced::keyboard::{Key, Modifiers};

        let command = if cfg!(target_os = "macos") {
            Modifiers::LOGO
        } else {
            Modifiers::CTRL
        };
        let key = Key::Character("ㅋ".into());
        let physical = Physical::Code(Code::KeyZ);

        assert_eq!(
            shortcut(key.clone(), physical, command),
            Some(Shortcut::Undo)
        );
        assert_eq!(
            shortcut(key, physical, command | Modifiers::SHIFT),
            Some(Shortcut::Redo)
        );
    }
}
