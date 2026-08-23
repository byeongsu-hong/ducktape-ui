//! The notes library: one folder of Markdown files, listed newest first.
//!
//! Every note is a plain `.md` file whose name follows its title, so the
//! folder stays readable by any other Markdown tool. Saves are atomic
//! (write to a sibling temp file, then rename) and a note is renamed on disk
//! when its first line changes.

use crate::document::EditorError;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SNIPPET_CHARS: usize = 96;
const FILE_NAME_CHARS: usize = 60;
const WELCOME: &str = include_str!("welcome.md");

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub stamp: String,
    pub search: String,
}

#[derive(Clone, Debug)]
pub struct Library {
    pub home: String,
    pub notes: Vec<Note>,
    pub path: String,
    pub source: String,
}

#[derive(Clone, Debug)]
pub struct Saved {
    pub path: String,
    pub saved_revision: i64,
    pub notes: Vec<Note>,
}

fn error(message: String) -> EditorError {
    EditorError { message }
}

/// The folder the app reads and writes: the one the app already opened, else
/// `$MARKDOWN_NOTES_DIR`, else `~/Documents/Markdown Notes`.
#[cfg(not(test))]
fn resolve_home(home: &str) -> PathBuf {
    if !home.is_empty() {
        return PathBuf::from(home);
    }
    match std::env::var_os("MARKDOWN_NOTES_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => std::env::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Documents")
            .join("Markdown Notes"),
    }
}

/// Every booted test app opens its own scratch folder, so parallel tests never
/// see each other's notes.
#[cfg(test)]
fn resolve_home(home: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static BOOTS: AtomicU64 = AtomicU64::new(0);
    if !home.is_empty() {
        return PathBuf::from(home);
    }
    let boot = BOOTS.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("markdown-notes-test-{}-{boot}", std::process::id()))
}

/// Lists the library and opens its most recent note, seeding a welcome note
/// into an empty folder so the first launch has something to read.
pub async fn open_library(home: String) -> Result<Library, EditorError> {
    let dir = resolve_home(&home);
    fs::create_dir_all(&dir).map_err(|cause| {
        error(format!(
            "Could not create the notes folder {}: {cause}",
            dir.display()
        ))
    })?;
    let notes = list_notes(&dir)?;
    match notes.first() {
        Some(note) => open(&dir, &note.path),
        None => create(&dir, WELCOME),
    }
}

/// Flushes the current note when it has unsaved edits, then opens `next` —
/// or a fresh "Untitled" note when `next` is empty.
pub async fn switch_note(
    home: String,
    path: String,
    source: String,
    revision: i64,
    dirty: bool,
    next: String,
) -> Result<Library, EditorError> {
    flush_note(home.clone(), path, source, revision, dirty).await?;
    let dir = PathBuf::from(home);
    if next.is_empty() {
        create(&dir, "")
    } else {
        open(&dir, &next)
    }
}

/// Saves the current note only when it has unsaved edits.
pub async fn flush_note(
    home: String,
    path: String,
    source: String,
    revision: i64,
    dirty: bool,
) -> Result<(), EditorError> {
    if dirty && !path.is_empty() {
        save_note(home, path, source, revision).await?;
    }
    Ok(())
}

/// Saves `source` into `path`, renaming the file when the title changed, and
/// returns the refreshed library order.
pub async fn save_note(
    home: String,
    path: String,
    source: String,
    revision: i64,
) -> Result<Saved, EditorError> {
    let dir = PathBuf::from(home);
    let current = PathBuf::from(&path);
    let path = rename_for_title(&dir, &current, &source)?;
    write_atomically(&path, &source)?;
    Ok(Saved {
        path: path.to_string_lossy().into_owned(),
        saved_revision: revision,
        notes: list_notes(&dir)?,
    })
}

/// Deletes a note and opens the most recent remaining one.
pub async fn delete_note(home: String, path: String) -> Result<Library, EditorError> {
    fs::remove_file(&path).map_err(|cause| error(format!("Could not delete {path}: {cause}")))?;
    open_library(home).await
}

pub fn filter_notes(notes: Vec<Note>, query: String) -> Vec<Note> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return notes;
    }
    notes
        .into_iter()
        .filter(|note| note.search.contains(&query))
        .collect()
}

pub fn selected_title(notes: Vec<Note>, path: String) -> String {
    notes
        .into_iter()
        .find(|note| note.path == path)
        .map(|note| note.title)
        .unwrap_or_default()
}

fn open(dir: &Path, path: &str) -> Result<Library, EditorError> {
    let source = fs::read_to_string(path)
        .map_err(|cause| error(format!("Could not open {path}: {cause}")))?;
    Ok(Library {
        home: dir.to_string_lossy().into_owned(),
        notes: list_notes(dir)?,
        path: path.to_owned(),
        source,
    })
}

fn create(dir: &Path, source: &str) -> Result<Library, EditorError> {
    let path = unique_path(dir, &file_stem(title(source)), None);
    write_atomically(&path, source)?;
    open(dir, &path.to_string_lossy())
}

fn list_notes(dir: &Path) -> Result<Vec<Note>, EditorError> {
    let entries = fs::read_dir(dir)
        .map_err(|cause| error(format!("Could not read {}: {cause}", dir.display())))?;
    let now = SystemTime::now();
    let mut notes = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "md") {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH);
        notes.push((modified, note(&path, &source, modified, now)));
    }
    notes.sort_by(|(left, a), (right, b)| right.cmp(left).then_with(|| a.title.cmp(&b.title)));
    Ok(notes.into_iter().map(|(_, note)| note).collect())
}

fn note(path: &Path, source: &str, modified: SystemTime, now: SystemTime) -> Note {
    let title = title(source);
    let snippet = snippet(source);
    Note {
        path: path.to_string_lossy().into_owned(),
        search: format!("{}\n{}", title, source).to_lowercase(),
        title,
        snippet,
        stamp: stamp(modified, now),
    }
}

fn rename_for_title(dir: &Path, current: &Path, source: &str) -> Result<PathBuf, EditorError> {
    let desired = file_stem(title(source));
    let stem = current
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    if base_stem(&stem) == desired {
        return Ok(current.to_path_buf());
    }
    let next = unique_path(dir, &desired, Some(current));
    if current.exists() {
        fs::rename(current, &next).map_err(|cause| {
            error(format!(
                "Could not rename {} to {}: {cause}",
                current.display(),
                next.display()
            ))
        })?;
    }
    Ok(next)
}

/// `Meeting 2` and `Meeting` share the base `Meeting`, so a note that had to
/// be numbered keeps its number instead of fighting its neighbour on every
/// save.
fn base_stem(stem: &str) -> &str {
    match stem.rsplit_once(' ') {
        Some((base, suffix)) if suffix.bytes().all(|byte| byte.is_ascii_digit()) => base,
        _ => stem,
    }
}

fn unique_path(dir: &Path, stem: &str, keep: Option<&Path>) -> PathBuf {
    let mut number = 1;
    loop {
        let name = if number == 1 {
            format!("{stem}.md")
        } else {
            format!("{stem} {number}.md")
        };
        let candidate = dir.join(name);
        if keep == Some(candidate.as_path()) || !candidate.exists() {
            return candidate;
        }
        number += 1;
    }
}

fn write_atomically(path: &Path, source: &str) -> Result<(), EditorError> {
    let temp = path.with_extension("md.tmp");
    fs::write(&temp, source)
        .and_then(|()| fs::rename(&temp, path))
        .map_err(|cause| error(format!("Could not save {}: {cause}", path.display())))
}

/// The first non-blank line without its Markdown markers.
pub fn title(source: &str) -> String {
    source
        .lines()
        .map(plain_line)
        .find(|line| !line.is_empty())
        .unwrap_or_else(|| "Untitled".to_owned())
}

fn snippet(source: &str) -> String {
    let mut lines = source
        .lines()
        .map(plain_line)
        .filter(|line| !line.is_empty());
    let _title = lines.next();
    let mut snippet = String::new();
    for line in lines {
        if !snippet.is_empty() {
            snippet.push(' ');
        }
        snippet.push_str(&line);
        if snippet.chars().count() > SNIPPET_CHARS {
            break;
        }
    }
    if snippet.chars().count() > SNIPPET_CHARS {
        snippet = snippet.chars().take(SNIPPET_CHARS - 1).collect();
        snippet.push('…');
    }
    snippet
}

fn plain_line(line: &str) -> String {
    let mut rest = line.trim();
    loop {
        let trimmed = rest
            .trim_start_matches(['#', '>', '-', '*', '+'])
            .trim_start();
        let trimmed = match trimmed.split_once(' ') {
            Some((marker, tail))
                if marker.len() > 1
                    && marker.ends_with(['.', ')'])
                    && marker[..marker.len() - 1]
                        .bytes()
                        .all(|byte| byte.is_ascii_digit()) =>
            {
                tail.trim_start()
            }
            _ => trimmed,
        };
        let trimmed = trimmed
            .strip_prefix("[ ]")
            .or_else(|| trimmed.strip_prefix("[x]"))
            .or_else(|| trimmed.strip_prefix("[X]"))
            .map_or(trimmed, str::trim_start);
        if trimmed == rest {
            break;
        }
        rest = trimmed;
    }
    rest.replace(['*', '`', '~'], "")
        .trim_matches('_')
        .trim()
        .to_owned()
}

fn file_stem(title: String) -> String {
    let cleaned = title
        .chars()
        .map(|character| {
            if character.is_control() || "/\\:*?\"<>|".contains(character) {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    let stem = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(FILE_NAME_CHARS)
        .collect::<String>()
        .trim_end_matches(['.', ' '])
        .to_owned();
    if stem.is_empty() {
        "Untitled".to_owned()
    } else {
        stem
    }
}

fn stamp(modified: SystemTime, now: SystemTime) -> String {
    let age = now.duration_since(modified).unwrap_or(Duration::ZERO);
    let minutes = age.as_secs() / 60;
    match minutes {
        0 => "Just now".to_owned(),
        1..60 => format!("{minutes} min ago"),
        60..1_440 => format!("{} h ago", minutes / 60),
        1_440..10_080 => format!("{} d ago", minutes / 1_440),
        _ => {
            let days = modified
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_secs()
                / 86_400;
            let (year, month, day) = civil_from_days(days as i64);
            const MONTHS: [&str; 12] = [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ];
            format!("{} {day}, {year}", MONTHS[month as usize - 1])
        }
    }
}

/// Howard Hinnant's `civil_from_days`, for the one place that needs a date.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::{Note, base_stem, civil_from_days, file_stem, filter_notes, snippet, stamp, title};
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn titles_drop_markdown_markers_and_fall_back_to_untitled() {
        assert_eq!(title("# Hello **world**\n\nbody"), "Hello world");
        assert_eq!(title("\n\n- [ ] buy milk"), "buy milk");
        assert_eq!(title("12. twelfth"), "twelfth");
        assert_eq!(title("> *quoted*"), "quoted");
        assert_eq!(title("   \n\n"), "Untitled");
    }

    #[test]
    fn snippets_skip_the_title_and_stay_short() {
        assert_eq!(snippet("# Title\n\n- one\n- two"), "one two");
        let long = format!("# T\n\n{}", "word ".repeat(60));
        let snippet = snippet(&long);
        assert!(snippet.ends_with('…'));
        assert_eq!(snippet.chars().count(), 96);
    }

    #[test]
    fn file_stems_are_safe_and_numbered_stems_keep_their_base() {
        assert_eq!(file_stem("a/b: c?".into()), "a b c");
        assert_eq!(file_stem("...".into()), "Untitled");
        assert_eq!(base_stem("Meeting 2"), "Meeting");
        assert_eq!(base_stem("Meeting"), "Meeting");
        assert_eq!(base_stem("Room 101 notes"), "Room 101 notes");
    }

    #[test]
    fn stamps_are_relative_until_a_week_then_a_date() {
        let now = UNIX_EPOCH + Duration::from_secs(1_756_000_000);
        assert_eq!(stamp(now, now), "Just now");
        assert_eq!(stamp(now - Duration::from_secs(5 * 60), now), "5 min ago");
        assert_eq!(stamp(now - Duration::from_secs(3 * 3_600), now), "3 h ago");
        assert_eq!(stamp(now - Duration::from_secs(2 * 86_400), now), "2 d ago");
        assert_eq!(
            stamp(UNIX_EPOCH + Duration::from_secs(0), now),
            "Jan 1, 1970"
        );
        assert_eq!(civil_from_days(20_323), (2025, 8, 23));
    }

    #[test]
    fn filtering_matches_title_and_body_case_insensitively() {
        let note = |title: &str, body: &str| Note {
            path: format!("{title}.md"),
            title: title.to_owned(),
            snippet: body.to_owned(),
            stamp: String::new(),
            search: format!("{title}\n{body}").to_lowercase(),
        };
        let notes = vec![note("Groceries", "milk"), note("Ideas", "Native editor")];
        assert_eq!(filter_notes(notes.clone(), "  ".into()).len(), 2);
        assert_eq!(
            filter_notes(notes.clone(), "EDITOR".into())[0].title,
            "Ideas"
        );
        assert!(filter_notes(notes, "missing".into()).is_empty());
    }
}
