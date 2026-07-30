use iced::keyboard::{Key, Modifiers, key};
use pulldown_cmark::{Event, Options, Parser, Tag};
use rfd::AsyncFileDialog;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug)]
pub struct DocumentFile {
    pub path: String,
    pub name: String,
    pub source: String,
}

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

pub async fn open_document() -> Result<DocumentFile, EditorError> {
    let Some(file) = AsyncFileDialog::new()
        .set_title("Open Markdown")
        .add_filter("Markdown", &["md", "markdown", "mdown", "mkd"])
        .add_filter("Text", &["txt"])
        .pick_file()
        .await
    else {
        return Ok(cancelled_file());
    };
    let source = String::from_utf8(file.read().await).map_err(|error| EditorError {
        message: format!("The selected file is not valid UTF-8: {error}"),
    })?;
    Ok(document_file(file.path(), source))
}

async fn save_document(
    path: String,
    source: String,
    revision: i64,
) -> Result<DocumentFile, EditorError> {
    std::fs::write(&path, &source).map_err(|error| EditorError {
        message: format!("Could not save {path}: {error}"),
    })?;
    crate::editor::mark_saved(revision);
    Ok(document_file(Path::new(&path), source))
}

pub async fn save_document_as(
    suggested_name: String,
    source: String,
    revision: i64,
) -> Result<DocumentFile, EditorError> {
    let Some(file) = AsyncFileDialog::new()
        .set_title("Save Markdown")
        .set_file_name(if suggested_name.is_empty() {
            "Untitled.md"
        } else {
            &suggested_name
        })
        .add_filter("Markdown", &["md"])
        .save_file()
        .await
    else {
        return Ok(cancelled_file());
    };
    file.write(source.as_bytes())
        .await
        .map_err(|error| EditorError {
            message: format!("Could not save {}: {error}", file.path().display()),
        })?;
    crate::editor::mark_saved(revision);
    Ok(document_file(file.path(), source))
}

pub async fn save_current(
    path: String,
    suggested_name: String,
    source: String,
    revision: i64,
) -> Result<DocumentFile, EditorError> {
    if path.is_empty() {
        save_document_as(suggested_name, source, revision).await
    } else {
        save_document(path, source, revision).await
    }
}

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
    Open,
    Save,
    SaveAs,
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
        ('o', _) => Shortcut::Open,
        ('s', true) => Shortcut::SaveAs,
        ('s', false) => Shortcut::Save,
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
    open_shortcut => Open,
    save_shortcut => Save,
    save_as_shortcut => SaveAs,
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

fn document_file(path: &Path, source: String) -> DocumentFile {
    let path = normalize_path(path);
    let name = Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled.md")
        .to_owned();
    DocumentFile { path, name, source }
}

fn cancelled_file() -> DocumentFile {
    DocumentFile {
        path: String::new(),
        name: String::new(),
        source: String::new(),
    }
}

fn normalize_path(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .into_owned()
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
