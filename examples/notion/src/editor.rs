use iced::advanced::text::{Highlighter, highlighter};
use iced::widget::{
    Column, button, column, container, markdown, row, scrollable, text, text_editor, text_input,
};
use iced::{
    Background, Border, Color, Element, Font, Length, Shadow, Task, Theme, Vector, font, keyboard,
};
use pulldown_cmark::{Event as MarkdownEvent, Options, Parser};
use std::ops::Range;
use ui_lang_runtime::{Role, StableId, accessible};

const EDITOR_ID: &str = "notion-markdown-editor";
const SEARCH_ID: &str = "notion-markdown-search";
const HISTORY_LIMIT: usize = 200;
const INTER: Font = Font {
    family: font::Family::Name("Inter"),
    ..Font::DEFAULT
};
const INTER_BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..INTER
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Write,
    Split,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownFormat {
    Paragraph,
    Heading(u8),
    Bold,
    Italic,
    Strikethrough,
    InlineCode,
    Link,
    Image,
    Bullet,
    Ordered,
    Task,
    Quote,
    CodeBlock,
    Table,
    Divider,
    Callout,
    Footnote,
    Math,
    Definition,
    Frontmatter,
    Html,
}

#[derive(Debug, Clone)]
struct Snapshot {
    source: String,
    cursor: text_editor::Cursor,
}

#[derive(Debug, Clone)]
struct CommentMessage {
    author: &'static str,
    body: String,
    time: &'static str,
}

#[derive(Debug, Clone)]
struct CommentThread {
    id: u64,
    block_id: usize,
    messages: Vec<CommentMessage>,
    resolved: bool,
}

#[derive(Debug)]
pub struct BlockEditorState {
    content: text_editor::Content,
    preview: markdown::Content,
    mode: EditorMode,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    focus_requested: bool,
    search_focus_requested: bool,
    slash_selected: usize,
    formats_open: bool,
    search_open: bool,
    search_query: String,
    replace_query: String,
    search_match: usize,
    threads: Vec<CommentThread>,
    next_thread_id: u64,
    composer_for: Option<usize>,
    comment_draft: String,
    replying_to: Option<u64>,
    reply_draft: String,
    comments_open: bool,
    show_resolved: bool,
}

impl Clone for BlockEditorState {
    fn clone(&self) -> Self {
        let source = self.source();
        let mut content = text_editor::Content::with_text(&source);
        content.move_to(self.content.cursor());
        Self {
            content,
            preview: markdown::Content::parse(&source),
            mode: self.mode,
            undo: self.undo.clone(),
            redo: self.redo.clone(),
            focus_requested: self.focus_requested,
            search_focus_requested: self.search_focus_requested,
            slash_selected: self.slash_selected,
            formats_open: self.formats_open,
            search_open: self.search_open,
            search_query: self.search_query.clone(),
            replace_query: self.replace_query.clone(),
            search_match: self.search_match,
            threads: self.threads.clone(),
            next_thread_id: self.next_thread_id,
            composer_for: self.composer_for,
            comment_draft: self.comment_draft.clone(),
            replying_to: self.replying_to,
            reply_draft: self.reply_draft.clone(),
            comments_open: self.comments_open,
            show_resolved: self.show_resolved,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BlockEditorEvent {
    Edit(text_editor::Action),
    Format(MarkdownFormat),
    Undo,
    Redo,
    SetMode(EditorMode),
    MoveBlock(i8),
    SmartEnter(bool),
    Indent(bool),
    SlashMoved(i8),
    ApplySlash(MarkdownFormat),
    DismissSlash,
    ToggleFormats,
    ToggleSearch,
    SearchChanged(String),
    ReplaceChanged(String),
    SearchMoved(i8),
    ReplaceMatch,
    ReplaceAll,
    OpenLink(String),
    OpenCommentComposer(usize),
    CloseCommentComposer,
    CommentDraftChanged(String),
    SubmitComment,
    ReplyDraftChanged(u64, String),
    SubmitReply(u64),
    Resolve(u64),
    Reopen(u64),
    ToggleComments,
    ToggleResolved,
}

pub fn block_editor_state(template: String) -> BlockEditorState {
    let source = template_source(&template);
    let threads = (template == "home")
        .then(|| CommentThread {
            id: 1,
            block_id: 2,
            messages: vec![CommentMessage {
                author: "Mina",
                body: "Can we link the customer research notes here?".into(),
                time: "18m",
            }],
            resolved: false,
        })
        .into_iter()
        .collect();

    BlockEditorState {
        content: text_editor::Content::with_text(source),
        preview: markdown::Content::parse(source),
        mode: EditorMode::Write,
        undo: Vec::new(),
        redo: Vec::new(),
        focus_requested: false,
        search_focus_requested: false,
        slash_selected: 0,
        formats_open: false,
        search_open: false,
        search_query: String::new(),
        replace_query: String::new(),
        search_match: 0,
        threads,
        next_thread_id: 2,
        composer_for: None,
        comment_draft: String::new(),
        replying_to: None,
        reply_draft: String::new(),
        comments_open: false,
        show_resolved: false,
    }
}

fn template_source(template: &str) -> &'static str {
    match template {
        "roadmap" => {
            "# Product roadmap

## Now

- [x] Editor foundations
- [ ] Keyboard-first navigation
- [ ] Workspace search

## Next

1. Reusable templates
2. Offline drafts
3. Team permissions

| Area | Status |
| :--- | ---: |
| Editor | In progress |
| Search | Planned |"
        }
        "launch" => {
            "# Launch plan

## Goal

Give every team one clear place to prepare for launch.

## Checklist

- [ ] Finalize announcement
- [ ] Review onboarding
- [ ] Schedule customer emails
- [ ] Open the feedback channel

## Timeline

| Day | Milestone |
| --- | --- |
| Monday | Internal preview |
| Wednesday | Early access |
| Friday | Public launch |"
        }
        "meeting" => {
            "# Weekly meeting

**July 27, 2026**

## Agenda

- Wins from last week
- What is blocked
- Decisions we need today

## Notes

"
        }
        "untitled" => "",
        _ => {
            "# Build a calmer place to work.

Keep decisions, plans, and their context in one clear place.

This editor keeps **Markdown** as its source of truth, with *inline formatting*, [links](https://commonmark.org), and `code`.

## Principles

- Show the next useful action, not every possible action
- Keep decisions beside the work that informed them
- Make collaboration visible without interrupting writing

## This quarter

- [x] Validate the editor with five product teams
- [ ] Connect customer research to the roadmap

> Clarity beats more surface area."
        }
    }
}

pub fn block_editor_apply(
    mut state: BlockEditorState,
    event: BlockEditorEvent,
) -> BlockEditorState {
    match event {
        BlockEditorEvent::Edit(action) => state.perform(action),
        BlockEditorEvent::Format(format) => state.format(format, false),
        BlockEditorEvent::Undo => state.undo(),
        BlockEditorEvent::Redo => state.redo(),
        BlockEditorEvent::SetMode(mode) => {
            state.mode = mode;
            state.focus_requested = mode != EditorMode::Preview;
        }
        BlockEditorEvent::MoveBlock(direction) => state.move_block(direction),
        BlockEditorEvent::SmartEnter(hard_break) => state.smart_enter(hard_break),
        BlockEditorEvent::Indent(outdent) => state.perform(text_editor::Action::Edit(if outdent {
            text_editor::Edit::Unindent
        } else {
            text_editor::Edit::Indent
        })),
        BlockEditorEvent::SlashMoved(direction) => {
            let count = state.slash_commands().count();
            if count > 0 {
                state.slash_selected = if direction.is_negative() {
                    state.slash_selected.checked_sub(1).unwrap_or(count - 1)
                } else {
                    (state.slash_selected + 1) % count
                };
            }
        }
        BlockEditorEvent::ApplySlash(format) => state.format(format, true),
        BlockEditorEvent::DismissSlash => state.dismiss_slash(),
        BlockEditorEvent::ToggleFormats => state.formats_open = !state.formats_open,
        BlockEditorEvent::ToggleSearch => {
            state.search_open = !state.search_open;
            state.search_match = 0;
            state.search_focus_requested = state.search_open;
            state.focus_requested = !state.search_open;
        }
        BlockEditorEvent::SearchChanged(query) => {
            state.search_query = query;
            state.search_match = 0;
            state.select_search_match();
        }
        BlockEditorEvent::ReplaceChanged(replacement) => state.replace_query = replacement,
        BlockEditorEvent::SearchMoved(direction) => state.move_search(direction),
        BlockEditorEvent::ReplaceMatch => state.replace_match(),
        BlockEditorEvent::ReplaceAll => state.replace_all(),
        BlockEditorEvent::OpenLink(_uri) => {}
        BlockEditorEvent::OpenCommentComposer(block_id) => {
            state.composer_for = Some(block_id);
            state.comment_draft.clear();
        }
        BlockEditorEvent::CloseCommentComposer => {
            state.composer_for = None;
            state.comment_draft.clear();
        }
        BlockEditorEvent::CommentDraftChanged(value) => state.comment_draft = value,
        BlockEditorEvent::SubmitComment => {
            if let Some(block_id) = state.composer_for
                && !state.comment_draft.trim().is_empty()
            {
                state.threads.push(CommentThread {
                    id: state.next_thread_id,
                    block_id,
                    messages: vec![CommentMessage {
                        author: "You",
                        body: state.comment_draft.trim().to_owned(),
                        time: "now",
                    }],
                    resolved: false,
                });
                state.next_thread_id += 1;
                state.comment_draft.clear();
                state.composer_for = None;
            }
        }
        BlockEditorEvent::ReplyDraftChanged(id, value) => {
            if state.replying_to != Some(id) {
                state.replying_to = Some(id);
                state.reply_draft.clear();
            }
            state.reply_draft = value;
        }
        BlockEditorEvent::SubmitReply(id) => {
            let reply = state.reply_draft.trim().to_owned();
            if !reply.is_empty()
                && let Some(thread) = state.threads.iter_mut().find(|thread| thread.id == id)
            {
                thread.messages.push(CommentMessage {
                    author: "You",
                    body: reply,
                    time: "now",
                });
                state.reply_draft.clear();
                state.replying_to = None;
            }
        }
        BlockEditorEvent::Resolve(id) => {
            if let Some(thread) = state.threads.iter_mut().find(|thread| thread.id == id) {
                thread.resolved = true;
            }
        }
        BlockEditorEvent::Reopen(id) => {
            if let Some(thread) = state.threads.iter_mut().find(|thread| thread.id == id) {
                thread.resolved = false;
            }
        }
        BlockEditorEvent::ToggleComments => state.comments_open = !state.comments_open,
        BlockEditorEvent::ToggleResolved => state.show_resolved = !state.show_resolved,
    }
    state
}

pub fn block_editor_should_focus(state: BlockEditorState) -> bool {
    state.focus_requested || state.search_focus_requested
}

pub fn block_editor_should_focus_search(state: BlockEditorState) -> bool {
    state.search_focus_requested
}

pub fn block_editor_clear_focus(mut state: BlockEditorState) -> BlockEditorState {
    state.focus_requested = false;
    state.search_focus_requested = false;
    state
}

pub fn block_editor_toggle_comments(mut state: BlockEditorState) -> BlockEditorState {
    state.comments_open = !state.comments_open;
    state
}

pub fn block_editor_comments_open(state: BlockEditorState) -> bool {
    state.comments_open
}

pub fn block_editor_focus(search: bool) -> Task<bool> {
    let id = if search { SEARCH_ID } else { EDITOR_ID };
    iced::widget::operation::focus(iced::widget::Id::new(id)).chain(Task::done(true))
}

impl BlockEditorState {
    fn source(&self) -> String {
        self.content.text()
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            source: self.source(),
            cursor: self.content.cursor(),
        }
    }

    fn remember(&mut self, snapshot: Snapshot) {
        // ponytail: a small Vec is simplest; use VecDeque only if 200-entry history profiles poorly.
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.undo.push(snapshot);
        self.redo.clear();
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.content = text_editor::Content::with_text(&snapshot.source);
        self.content.move_to(snapshot.cursor);
        self.preview = markdown::Content::parse(&snapshot.source);
        self.slash_selected = 0;
        self.focus_requested = true;
    }

    fn perform(&mut self, action: text_editor::Action) {
        let before = action.is_edit().then(|| self.snapshot());
        let previous = before.as_ref().map(|snapshot| snapshot.source.as_str());
        self.content.perform(action);
        let source = self.source();
        if previous.is_some_and(|previous| previous != source) {
            self.remember(before.expect("editing actions capture history"));
            self.preview = markdown::Content::parse(&source);
        }
        self.slash_selected = 0;
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(previous);
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let current = self.snapshot();
            if self.undo.len() == HISTORY_LIMIT {
                self.undo.remove(0);
            }
            self.undo.push(current);
            self.restore(next);
        }
    }

    fn smart_enter(&mut self, hard_break: bool) {
        let source = self.source();
        let selection = self.selection_bytes();
        if hard_break {
            let mut next = source;
            next.replace_range(selection.clone(), "  \n");
            self.replace_source(next, selection.start + 3, None, true);
            return;
        }
        if !selection.is_empty() {
            self.perform(text_editor::Action::Edit(text_editor::Edit::Enter));
            return;
        }

        let line_range = selected_line_range(&source, selection.clone());
        let line = &source[line_range.clone()];
        let Some((prefix_len, next_prefix)) = continuation_prefix(line) else {
            self.perform(text_editor::Action::Edit(text_editor::Edit::Enter));
            return;
        };
        let (indent, _) = split_indent(line);
        let indent = indent.to_owned();
        if line[prefix_len..].trim().is_empty() {
            let mut next = source;
            next.replace_range(line_range.clone(), &indent);
            self.replace_source(next, line_range.start + indent.len(), None, true);
            return;
        }

        let mut next = source;
        let insertion = format!("\n{next_prefix}");
        next.insert_str(selection.start, &insertion);
        self.replace_source(next, selection.start + insertion.len(), None, true);
    }

    fn search_matches(&self) -> Vec<Range<usize>> {
        if self.search_query.is_empty() {
            return Vec::new();
        }
        self.source()
            .match_indices(&self.search_query)
            .map(|(start, value)| start..start + value.len())
            .collect()
    }

    fn select_search_match(&mut self) {
        let source = self.source();
        let matches = self.search_matches();
        if matches.is_empty() {
            self.search_match = 0;
            return;
        }
        self.search_match %= matches.len();
        let range = &matches[self.search_match];
        self.content.move_to(text_editor::Cursor {
            position: byte_to_position(&source, range.end),
            selection: Some(byte_to_position(&source, range.start)),
        });
    }

    fn move_search(&mut self, direction: i8) {
        let count = self.search_matches().len();
        if count == 0 {
            self.search_match = 0;
            return;
        }
        self.search_match = if direction.is_negative() {
            self.search_match.checked_sub(1).unwrap_or(count - 1)
        } else {
            (self.search_match + 1) % count
        };
        self.select_search_match();
    }

    fn replace_match(&mut self) {
        let matches = self.search_matches();
        let Some(range) = matches
            .get(self.search_match % matches.len().max(1))
            .cloned()
        else {
            return;
        };
        let mut source = self.source();
        source.replace_range(range.clone(), &self.replace_query);
        let caret = range.start + self.replace_query.len();
        self.replace_source(source, caret, None, true);
        self.focus_requested = false;
        self.search_match = self
            .search_match
            .min(self.search_matches().len().saturating_sub(1));
        self.select_search_match();
    }

    fn replace_all(&mut self) {
        if self.search_query.is_empty() || !self.source().contains(&self.search_query) {
            return;
        }
        let source = self
            .source()
            .replace(&self.search_query, &self.replace_query);
        self.replace_source(source, 0, None, true);
        self.focus_requested = false;
        self.search_match = 0;
        self.select_search_match();
    }

    fn replace_source(
        &mut self,
        source: String,
        caret: usize,
        selection: Option<usize>,
        remember: bool,
    ) {
        if source == self.source() {
            return;
        }
        if remember {
            self.remember(self.snapshot());
        }
        let cursor = text_editor::Cursor {
            position: byte_to_position(&source, caret),
            selection: selection.map(|selection| byte_to_position(&source, selection)),
        };
        self.content = text_editor::Content::with_text(&source);
        self.content.move_to(cursor);
        self.preview = markdown::Content::parse(&source);
        self.slash_selected = 0;
        self.focus_requested = true;
    }

    fn cursor_bytes(&self) -> (usize, Option<usize>) {
        let cursor = self.content.cursor();
        (
            position_to_byte(&self.content, cursor.position),
            cursor
                .selection
                .map(|selection| position_to_byte(&self.content, selection)),
        )
    }

    fn selection_bytes(&self) -> Range<usize> {
        let (caret, selection) = self.cursor_bytes();
        let anchor = selection.unwrap_or(caret);
        anchor.min(caret)..anchor.max(caret)
    }

    fn current_line_range(&self) -> Range<usize> {
        let source = self.source();
        let selection = self.selection_bytes();
        selected_line_range(&source, selection)
    }

    fn format(&mut self, format: MarkdownFormat, slash: bool) {
        self.formats_open = false;
        if slash {
            self.replace_slash(format);
            return;
        }
        match format {
            MarkdownFormat::Bold => self.wrap_inline("**", "**", "bold text"),
            MarkdownFormat::Italic => self.wrap_inline("*", "*", "italic text"),
            MarkdownFormat::Strikethrough => self.wrap_inline("~~", "~~", "struck text"),
            MarkdownFormat::InlineCode => self.wrap_inline("`", "`", "code"),
            MarkdownFormat::Link => self.insert_link(false),
            MarkdownFormat::Image => self.insert_link(true),
            MarkdownFormat::CodeBlock
            | MarkdownFormat::Table
            | MarkdownFormat::Divider
            | MarkdownFormat::Callout
            | MarkdownFormat::Footnote
            | MarkdownFormat::Math
            | MarkdownFormat::Definition
            | MarkdownFormat::Frontmatter
            | MarkdownFormat::Html => self.insert_block_template(format),
            _ => self.format_lines(format),
        }
    }

    fn wrap_inline(&mut self, open: &str, close: &str, placeholder: &str) {
        let source = self.source();
        let mut range = self.selection_bytes();
        if range.is_empty() {
            range = word_range(&source, range.start);
        }
        let inner = if range.is_empty() {
            placeholder.to_owned()
        } else {
            source[range.clone()].to_owned()
        };

        if range.start >= open.len()
            && range.end + close.len() <= source.len()
            && &source[range.start - open.len()..range.start] == open
            && &source[range.end..range.end + close.len()] == close
        {
            let mut next = source;
            next.replace_range(range.end..range.end + close.len(), "");
            next.replace_range(range.start - open.len()..range.start, "");
            let start = range.start - open.len();
            self.replace_source(next, start + inner.len(), Some(start), true);
            return;
        }

        let mut next = source;
        let replacement = format!("{open}{inner}{close}");
        next.replace_range(range.clone(), &replacement);
        let start = range.start + open.len();
        self.replace_source(next, start + inner.len(), Some(start), true);
    }

    fn insert_link(&mut self, image: bool) {
        let source = self.source();
        let mut range = self.selection_bytes();
        if range.is_empty() {
            range = word_range(&source, range.start);
        }
        let label = if range.is_empty() {
            if image { "alt text" } else { "link text" }
        } else {
            &source[range.clone()]
        };
        let prefix = if image { "![" } else { "[" };
        let replacement = format!("{prefix}{label}](https://)");
        let url_start = range.start + prefix.len() + label.len() + 2;
        let mut next = source;
        next.replace_range(range, &replacement);
        self.replace_source(next, url_start + 8, Some(url_start), true);
    }

    fn format_lines(&mut self, format: MarkdownFormat) {
        let source = self.source();
        let range = self.current_line_range();
        let selected = &source[range.clone()];
        let lines = selected.split('\n').collect::<Vec<_>>();
        let target = block_prefix(format);
        let toggle_off = target.is_some()
            && lines
                .iter()
                .filter(|line| !line.trim().is_empty())
                .all(|line| line_has_prefix(line, format));
        let replacement = lines
            .iter()
            .map(|line| {
                let (indent, body) = split_indent(line);
                let body = strip_block_prefix(body);
                if toggle_off || matches!(format, MarkdownFormat::Paragraph) {
                    format!("{indent}{body}")
                } else {
                    format!("{indent}{}{body}", target.unwrap_or_default())
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut next = source;
        next.replace_range(range.clone(), &replacement);
        self.replace_source(
            next,
            range.start + replacement.len(),
            Some(range.start),
            true,
        );
    }

    fn insert_block_template(&mut self, format: MarkdownFormat) {
        let source = self.source();
        let range = self.current_line_range();
        let selected = source[range.clone()].trim_matches(['\r', '\n']);
        let (replacement, select) = block_template(format, selected);
        let mut next = source;
        next.replace_range(range.clone(), &replacement);
        let selection = select
            .as_ref()
            .map(|selection| range.start + selection.start);
        let caret = select
            .map(|selection| range.start + selection.end)
            .unwrap_or(range.start + replacement.len());
        self.replace_source(next, caret, selection, true);
    }

    fn replace_slash(&mut self, format: MarkdownFormat) {
        let source = self.source();
        let range = self.current_line_range();
        let (replacement, select) = block_template(format, "");
        let mut next = source;
        next.replace_range(range.clone(), &replacement);
        let selection = select
            .as_ref()
            .map(|selection| range.start + selection.start);
        let caret = select
            .map(|selection| range.start + selection.end)
            .unwrap_or(range.start + replacement.len());
        self.replace_source(next, caret, selection, true);
    }

    fn dismiss_slash(&mut self) {
        if self.slash_query().is_some() {
            let source = self.source();
            let range = self.current_line_range();
            let mut next = source;
            next.replace_range(range.clone(), "");
            self.replace_source(next, range.start, None, true);
        }
    }

    fn slash_query(&self) -> Option<String> {
        let source = self.source();
        let range = self.current_line_range();
        source[range]
            .trim()
            .strip_prefix('/')
            .map(|query| query.to_lowercase())
    }

    fn slash_commands(&self) -> impl Iterator<Item = &'static SlashCommand> + '_ {
        let query = self.slash_query();
        SLASH_COMMANDS.iter().filter(move |command| {
            query.as_ref().is_some_and(|query| {
                query.is_empty()
                    || command.label.to_lowercase().contains(query)
                    || command.keywords.contains(query)
            })
        })
    }

    fn current_block(&self) -> usize {
        let source = self.source();
        let caret = self.selection_bytes().start;
        block_ranges(&source)
            .iter()
            .position(|range| range.contains(&caret) || caret == range.end)
            .map_or(1, |index| index + 1)
    }

    fn move_block(&mut self, direction: i8) {
        let source = self.source();
        let ranges = block_ranges(&source);
        if ranges.len() < 2 {
            return;
        }
        let from = self.current_block().saturating_sub(1).min(ranges.len() - 1);
        let to = if direction.is_negative() {
            from.saturating_sub(1)
        } else {
            (from + 1).min(ranges.len() - 1)
        };
        if from == to {
            return;
        }
        let mut blocks = ranges
            .iter()
            .map(|range| source[range.clone()].trim_matches(['\r', '\n']).to_owned())
            .collect::<Vec<_>>();
        blocks.swap(from, to);
        let next = blocks.join("\n\n");
        let caret = blocks.iter().take(to).map(|block| block.len() + 2).sum();
        for thread in &mut self.threads {
            if thread.block_id == from + 1 {
                thread.block_id = to + 1;
            } else if thread.block_id == to + 1 {
                thread.block_id = from + 1;
            }
        }
        self.replace_source(next, caret, None, true);
    }

    fn block_text(&self, id: usize) -> Option<String> {
        let source = self.source();
        block_ranges(&source)
            .get(id.checked_sub(1)?)
            .map(|range| source[range.clone()].trim().to_owned())
    }

    pub fn block_count(&self) -> usize {
        block_ranges(&self.source()).len()
    }

    #[cfg(test)]
    pub fn markdown(&self) -> String {
        self.source()
    }

    #[cfg(test)]
    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }

    #[cfg(test)]
    pub fn thread_message_count(&self, id: u64) -> usize {
        self.threads
            .iter()
            .find(|thread| thread.id == id)
            .map_or(0, |thread| thread.messages.len())
    }

    #[cfg(test)]
    pub fn thread_resolved(&self, id: u64) -> bool {
        self.threads
            .iter()
            .find(|thread| thread.id == id)
            .is_some_and(|thread| thread.resolved)
    }
}

fn markdown_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_MATH
        | Options::ENABLE_GFM
        | Options::ENABLE_DEFINITION_LIST
}

fn block_ranges(source: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (event, range) in Parser::new_ext(source, markdown_options()).into_offset_iter() {
        match event {
            MarkdownEvent::Start(_) => {
                if depth == 0 {
                    start = range.start;
                }
                depth += 1;
            }
            MarkdownEvent::End(_) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    ranges.push(start..range.end);
                }
            }
            MarkdownEvent::Rule if depth == 0 => ranges.push(range),
            _ => {}
        }
    }
    ranges.sort_by_key(|range| range.start);
    ranges.dedup_by(|next, previous| next.start < previous.end);

    let mut complete = Vec::new();
    let mut cursor = 0;
    for range in ranges {
        push_gap(source, cursor..range.start, &mut complete);
        complete.push(trim_newlines(source, range));
        cursor = complete.last().map_or(cursor, |range| range.end);
    }
    push_gap(source, cursor..source.len(), &mut complete);
    complete.retain(|range| !source[range.clone()].trim().is_empty());
    complete
}

fn push_gap(source: &str, gap: Range<usize>, ranges: &mut Vec<Range<usize>>) {
    if gap.start >= gap.end || source[gap.clone()].trim().is_empty() {
        return;
    }
    let mut start = gap.start;
    for separator in ["\n\n", "\r\n\r\n"] {
        if let Some(found) = source[start..gap.end].find(separator) {
            let end = start + found;
            if !source[start..end].trim().is_empty() {
                ranges.push(trim_newlines(source, start..end));
            }
            start = end + separator.len();
        }
    }
    if start < gap.end && !source[start..gap.end].trim().is_empty() {
        ranges.push(trim_newlines(source, start..gap.end));
    }
}

fn trim_newlines(source: &str, mut range: Range<usize>) -> Range<usize> {
    while range.start < range.end && matches!(source.as_bytes()[range.start], b'\n' | b'\r') {
        range.start += 1;
    }
    while range.end > range.start && matches!(source.as_bytes()[range.end - 1], b'\n' | b'\r') {
        range.end -= 1;
    }
    range
}

fn position_to_byte(content: &text_editor::Content, position: text_editor::Position) -> usize {
    let mut byte = 0;
    for index in 0..position.line {
        let Some(line) = content.line(index) else {
            return content.text().len();
        };
        byte += line.text.len() + line.ending.as_str().len();
    }
    let Some(line) = content.line(position.line) else {
        return byte;
    };
    byte + text_input::Value::new(&line.text)
        .until(position.column)
        .to_string()
        .len()
}

fn byte_to_position(source: &str, byte: usize) -> text_editor::Position {
    let byte = byte.min(source.len());
    let line_start = source[..byte].rfind('\n').map_or(0, |index| index + 1);
    text_editor::Position {
        line: source[..byte].bytes().filter(|byte| *byte == b'\n').count(),
        column: text_input::Value::new(&source[line_start..byte]).len(),
    }
}

fn selected_line_range(source: &str, selection: Range<usize>) -> Range<usize> {
    let start = source[..selection.start.min(source.len())]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let logical_end = if selection.end > selection.start
        && source.as_bytes().get(selection.end.saturating_sub(1)) == Some(&b'\n')
    {
        selection.end - 1
    } else {
        selection.end
    };
    let end = source[logical_end.min(source.len())..]
        .find('\n')
        .map_or(source.len(), |index| logical_end + index);
    start..end
}

fn word_range(source: &str, caret: usize) -> Range<usize> {
    let mut start = caret.min(source.len());
    let mut end = start;
    while let Some((index, character)) = source[..start].char_indices().next_back() {
        if !character.is_alphanumeric() && character != '_' && character != '-' {
            break;
        }
        start = index;
    }
    while let Some(character) = source[end..].chars().next() {
        if !character.is_alphanumeric() && character != '_' && character != '-' {
            break;
        }
        end += character.len_utf8();
    }
    start..end
}

fn split_indent(line: &str) -> (&str, &str) {
    let indent = line
        .char_indices()
        .find(|(_, character)| !matches!(character, ' ' | '\t'))
        .map_or(line.len(), |(index, _)| index);
    line.split_at(indent)
}

fn continuation_prefix(line: &str) -> Option<(usize, String)> {
    let (indent, body) = split_indent(line);
    let (marker_len, marker) =
        if body.starts_with("- [ ] ") || body.starts_with("- [x] ") || body.starts_with("- [X] ") {
            (6, "- [ ] ".to_owned())
        } else if let Some(marker) = ["- ", "* ", "+ ", "> "]
            .into_iter()
            .find(|marker| body.starts_with(marker))
        {
            (marker.len(), marker.to_owned())
        } else {
            let digits = body.bytes().take_while(u8::is_ascii_digit).count();
            if digits == 0 || body.get(digits..digits + 2) != Some(". ") {
                return None;
            }
            let number = body[..digits].parse::<usize>().ok()?;
            (digits + 2, format!("{}. ", number + 1))
        };
    Some((indent.len() + marker_len, format!("{indent}{marker}")))
}

fn strip_block_prefix(line: &str) -> &str {
    if let Some(rest) = line.strip_prefix("> [!NOTE] ") {
        return rest;
    }
    if let Some(rest) = line.strip_prefix("> ") {
        return rest;
    }
    if let Some(rest) = line
        .strip_prefix("- [ ] ")
        .or_else(|| line.strip_prefix("- [x] "))
        .or_else(|| line.strip_prefix("- [X] "))
    {
        return rest;
    }
    if let Some(rest) = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "))
    {
        return rest;
    }
    let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
    if (1..=6).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ') {
        return &line[hashes + 1..];
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 && line.get(digits..digits + 2) == Some(". ") {
        return &line[digits + 2..];
    }
    line
}

fn block_prefix(format: MarkdownFormat) -> Option<&'static str> {
    match format {
        MarkdownFormat::Paragraph => Some(""),
        MarkdownFormat::Heading(1) => Some("# "),
        MarkdownFormat::Heading(2) => Some("## "),
        MarkdownFormat::Heading(3) => Some("### "),
        MarkdownFormat::Heading(4) => Some("#### "),
        MarkdownFormat::Heading(5) => Some("##### "),
        MarkdownFormat::Heading(6) => Some("###### "),
        MarkdownFormat::Bullet => Some("- "),
        MarkdownFormat::Ordered => Some("1. "),
        MarkdownFormat::Task => Some("- [ ] "),
        MarkdownFormat::Quote => Some("> "),
        _ => None,
    }
}

fn line_has_prefix(line: &str, format: MarkdownFormat) -> bool {
    let (_, line) = split_indent(line);
    block_prefix(format).is_some_and(|prefix| !prefix.is_empty() && line.starts_with(prefix))
}

fn block_template(format: MarkdownFormat, selected: &str) -> (String, Option<Range<usize>>) {
    let selected = (!selected.is_empty()).then_some(selected);
    match format {
        MarkdownFormat::Paragraph => (String::new(), None),
        MarkdownFormat::Heading(level @ 1..=6) => {
            (format!("{} ", "#".repeat(level as usize)), None)
        }
        MarkdownFormat::Bullet => ("- ".into(), None),
        MarkdownFormat::Ordered => ("1. ".into(), None),
        MarkdownFormat::Task => ("- [ ] ".into(), None),
        MarkdownFormat::Quote => ("> ".into(), None),
        MarkdownFormat::CodeBlock => {
            let body = selected.unwrap_or("code");
            let value = format!("```text\n{body}\n```");
            let start = 8;
            (value, Some(start..start + body.len()))
        }
        MarkdownFormat::Table => {
            let value = "| Column 1 | Column 2 |\n| :--- | ---: |\n| Value | Value |".to_owned();
            (value, Some(2..10))
        }
        MarkdownFormat::Divider => ("---".into(), None),
        MarkdownFormat::Callout => {
            let body = selected.unwrap_or("Useful context");
            let value = format!("> [!NOTE]\n> {body}");
            let start = 12;
            (value, Some(start..start + body.len()))
        }
        MarkdownFormat::Footnote => {
            let value = "Text with a footnote[^1].\n\n[^1]: Footnote text.".to_owned();
            (value, Some(36..49))
        }
        MarkdownFormat::Math => {
            let body = selected.unwrap_or("E = mc^2");
            let value = format!("$$\n{body}\n$$");
            (value, Some(3..3 + body.len()))
        }
        MarkdownFormat::Definition => {
            let value = "Term\n: Definition".to_owned();
            (value, Some(7..17))
        }
        MarkdownFormat::Frontmatter => {
            let value = "---\ntitle: Untitled\n---".to_owned();
            (value, Some(11..19))
        }
        MarkdownFormat::Html => {
            let value = "<details>\n<summary>Details</summary>\n\nContent\n</details>".to_owned();
            (value, Some(41..48))
        }
        MarkdownFormat::Image => {
            let value = "![alt text](https://)".to_owned();
            (value, Some(2..10))
        }
        MarkdownFormat::Link => {
            let value = "[link text](https://)".to_owned();
            (value, Some(1..10))
        }
        MarkdownFormat::Bold => ("**bold text**".into(), Some(2..11)),
        MarkdownFormat::Italic => ("*italic text*".into(), Some(1..12)),
        MarkdownFormat::Strikethrough => ("~~struck text~~".into(), Some(2..13)),
        MarkdownFormat::InlineCode => ("`code`".into(), Some(1..5)),
        MarkdownFormat::Heading(_) => (String::new(), None),
    }
}

#[derive(Clone, Copy)]
struct SlashCommand {
    icon: &'static str,
    label: &'static str,
    description: &'static str,
    keywords: &'static str,
    format: MarkdownFormat,
}

const SLASH_COMMANDS: [SlashCommand; 18] = [
    SlashCommand {
        icon: "T",
        label: "Text",
        description: "Plain paragraph",
        keywords: "paragraph body",
        format: MarkdownFormat::Paragraph,
    },
    SlashCommand {
        icon: "H1",
        label: "Heading 1",
        description: "Page section title",
        keywords: "title heading one",
        format: MarkdownFormat::Heading(1),
    },
    SlashCommand {
        icon: "H2",
        label: "Heading 2",
        description: "Section heading",
        keywords: "subtitle heading two",
        format: MarkdownFormat::Heading(2),
    },
    SlashCommand {
        icon: "H3",
        label: "Heading 3",
        description: "Subsection heading",
        keywords: "heading three",
        format: MarkdownFormat::Heading(3),
    },
    SlashCommand {
        icon: "•",
        label: "Bulleted list",
        description: "Unordered list item",
        keywords: "bullet list unordered",
        format: MarkdownFormat::Bullet,
    },
    SlashCommand {
        icon: "1.",
        label: "Numbered list",
        description: "Ordered list item",
        keywords: "number ordered list",
        format: MarkdownFormat::Ordered,
    },
    SlashCommand {
        icon: "☐",
        label: "Task",
        description: "GFM task item",
        keywords: "todo checkbox task",
        format: MarkdownFormat::Task,
    },
    SlashCommand {
        icon: "❝",
        label: "Quote",
        description: "Block quotation",
        keywords: "quote citation",
        format: MarkdownFormat::Quote,
    },
    SlashCommand {
        icon: "</>",
        label: "Code block",
        description: "Fenced source code",
        keywords: "code fence syntax",
        format: MarkdownFormat::CodeBlock,
    },
    SlashCommand {
        icon: "▦",
        label: "Table",
        description: "GFM table",
        keywords: "table grid columns",
        format: MarkdownFormat::Table,
    },
    SlashCommand {
        icon: "—",
        label: "Divider",
        description: "Thematic break",
        keywords: "rule separator line",
        format: MarkdownFormat::Divider,
    },
    SlashCommand {
        icon: "!",
        label: "Callout",
        description: "GFM note alert",
        keywords: "alert note tip warning",
        format: MarkdownFormat::Callout,
    },
    SlashCommand {
        icon: "¹",
        label: "Footnote",
        description: "Reference and definition",
        keywords: "footnote reference",
        format: MarkdownFormat::Footnote,
    },
    SlashCommand {
        icon: "∑",
        label: "Math",
        description: "Display math block",
        keywords: "math latex equation",
        format: MarkdownFormat::Math,
    },
    SlashCommand {
        icon: ":",
        label: "Definition",
        description: "Definition list",
        keywords: "definition term list",
        format: MarkdownFormat::Definition,
    },
    SlashCommand {
        icon: "Y",
        label: "Frontmatter",
        description: "YAML metadata",
        keywords: "yaml metadata frontmatter",
        format: MarkdownFormat::Frontmatter,
    },
    SlashCommand {
        icon: "<>",
        label: "HTML",
        description: "Raw HTML block",
        keywords: "html details",
        format: MarkdownFormat::Html,
    },
    SlashCommand {
        icon: "▧",
        label: "Image",
        description: "Image with alt text",
        keywords: "image picture media",
        format: MarkdownFormat::Image,
    },
];

pub fn block_editor(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    iced::widget::responsive(move |size| editor_for_size(state, size.width)).into()
}

fn editor_for_size(state: &BlockEditorState, width: f32) -> Element<'_, BlockEditorEvent> {
    let document = document(state, width);
    if state.comments_open {
        return row![document, comments_panel(state)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    document
}

fn document(state: &BlockEditorState, width: f32) -> Element<'_, BlockEditorEvent> {
    let toolbar = toolbar(state);
    let search = state.search_open.then(|| search_bar(state));
    let slash = slash_menu(state);
    let body: Element<'_, _> = match state.mode {
        EditorMode::Write => source_editor(state),
        EditorMode::Preview => preview(state),
        EditorMode::Split if width >= 680.0 => row![
            container(source_editor(state)).width(Length::FillPortion(1)),
            container(preview(state))
                .width(Length::FillPortion(1))
                .style(|_| container::Style::default().border(Border {
                    color: BORDER,
                    width: 0.0,
                    radius: 0.0.into(),
                }))
        ]
        .spacing(12)
        .height(Length::Fill)
        .into(),
        EditorMode::Split => source_editor(state),
    };
    let source = state.source();
    let words = source.split_whitespace().count();
    let blocks = block_ranges(&source).len();
    let status = row![
        text(format!(
            "{words} words · {blocks} blocks · {} characters",
            source.chars().count()
        ))
        .size(10)
        .color(MUTED),
        iced::widget::space().width(Length::Fill),
        text("CommonMark + GFM").size(10).color(FAINT),
    ];

    let mut editor = column![toolbar].spacing(6).width(Length::Fill);
    if let Some(search) = search {
        editor = editor.push(search);
    }
    if let Some(slash) = slash {
        editor = editor.push(slash);
    }
    if state.composer_for.is_some() {
        editor = editor.push(comment_composer(state));
    }
    editor = editor.push(body).push(status);

    container(editor)
        .padding([8, 10])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn source_editor(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    let slash_choice = state
        .slash_commands()
        .nth(state.slash_selected)
        .map(|command| command.format);
    let search_open = state.search_open;
    let editor = text_editor(&state.content)
        .id(iced::widget::Id::new(EDITOR_ID))
        .placeholder("Write Markdown, or type / for blocks…")
        .on_action(BlockEditorEvent::Edit)
        .key_binding(move |event| markdown_binding(event, slash_choice, search_open))
        .font(INTER)
        .size(15)
        .line_height(iced::advanced::text::LineHeight::Relative(1.65))
        .padding([14, 16])
        .height(Length::Fill)
        .highlight_with::<MarkdownHighlighter>((), markdown_highlight)
        .style(editor_style);
    accessible(
        editor,
        StableId::new("notion-markdown-document"),
        Role::TextInput,
    )
    .logical_id("notion-markdown-document")
    .focus_id(iced::widget::Id::new(EDITOR_ID))
    .label("Markdown document")
    .value(state.source())
    .into()
}

fn preview(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    let mut settings = markdown::Settings::with_text_size(
        16,
        markdown::Style {
            font: INTER,
            inline_code_highlight: iced::advanced::text::Highlight {
                background: SIDEBAR.into(),
                border: Border {
                    color: BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
            },
            inline_code_padding: iced::Padding::from([1, 3]),
            inline_code_color: FG,
            inline_code_font: Font::MONOSPACE,
            code_block_font: Font::MONOSPACE,
            link_color: PRIMARY,
        },
    );
    settings.h1_size = 30.into();
    settings.h2_size = 24.into();
    settings.h3_size = 20.into();
    settings.spacing = 12.into();
    let content = if state.preview.items().is_empty() {
        container(
            column![
                text("Nothing to preview").size(18).font(INTER_BOLD),
                text("Switch to Write and start with Markdown or / commands.")
                    .size(13)
                    .color(MUTED),
            ]
            .spacing(5),
        )
        .padding(20)
        .into()
    } else {
        markdown::view(state.preview.items(), settings).map(BlockEditorEvent::OpenLink)
    };
    scrollable(container(content).padding([18, 20]).width(Length::Fill))
        .height(Length::Fill)
        .into()
}

fn toolbar(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    let current = state.current_block();
    let mut controls = column![
        row![
            text("MARKDOWN").size(10).font(INTER_BOLD).color(MUTED),
            mode_button("Write", EditorMode::Write, state.mode),
            mode_button("Preview", EditorMode::Preview, state.mode),
            mode_button("Split", EditorMode::Split, state.mode),
            iced::widget::space().width(Length::Fill),
            tool_button(
                "Find",
                "Find and replace (Ctrl+F)",
                BlockEditorEvent::ToggleSearch,
                true
            ),
            tool_button(
                "Undo",
                "Undo (Ctrl+Z)",
                BlockEditorEvent::Undo,
                !state.undo.is_empty()
            ),
            tool_button(
                "Redo",
                "Redo (Ctrl+Shift+Z)",
                BlockEditorEvent::Redo,
                !state.redo.is_empty()
            ),
            tool_button(
                "Block ↑",
                "Move current block up",
                BlockEditorEvent::MoveBlock(-1),
                current > 1
            ),
            tool_button(
                "Block ↓",
                "Move current block down",
                BlockEditorEvent::MoveBlock(1),
                current < state.block_count()
            ),
            tool_button(
                "Comment",
                "Comment on current block",
                BlockEditorEvent::OpenCommentComposer(current),
                true
            ),
        ]
        .spacing(3)
        .align_y(iced::Alignment::Center),
        row![
            format_button("Text", "Paragraph", MarkdownFormat::Paragraph),
            format_button("H1", "Heading 1", MarkdownFormat::Heading(1)),
            format_button("H2", "Heading 2", MarkdownFormat::Heading(2)),
            format_button("B", "Bold (Ctrl+B)", MarkdownFormat::Bold),
            format_button("I", "Italic (Ctrl+I)", MarkdownFormat::Italic),
            format_button("S", "Strikethrough", MarkdownFormat::Strikethrough),
            format_button("Code", "Inline code", MarkdownFormat::InlineCode),
            format_button("Link", "Link (Ctrl+K)", MarkdownFormat::Link),
            format_button("Bullet", "Bulleted list", MarkdownFormat::Bullet),
            format_button("Number", "Numbered list", MarkdownFormat::Ordered),
            format_button("Task", "Task list", MarkdownFormat::Task),
            format_button("Quote", "Quote", MarkdownFormat::Quote),
            tool_button(
                "More ▾",
                "More Markdown blocks",
                BlockEditorEvent::ToggleFormats,
                true,
            ),
        ]
        .spacing(2)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(5);
    if state.formats_open {
        controls = controls.push(more_formats());
    }
    container(controls)
        .padding([5, 6])
        .style(|_| {
            container::Style::default()
                .background(SIDEBAR)
                .border(Border {
                    color: BORDER,
                    width: 1.0,
                    radius: 7.0.into(),
                })
        })
        .into()
}

fn search_bar(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    let matches = state.search_matches();
    let count = matches.len();
    let position = if count == 0 {
        "No matches".to_owned()
    } else {
        format!("{} / {count}", state.search_match % count + 1)
    };
    container(
        column![
            row![
                text("FIND").size(10).font(INTER_BOLD).color(MUTED),
                text_input("Find…", &state.search_query)
                    .id(iced::widget::Id::new(SEARCH_ID))
                    .on_input(BlockEditorEvent::SearchChanged)
                    .on_submit(BlockEditorEvent::SearchMoved(1))
                    .padding(6)
                    .width(Length::Fill),
                text(position).size(10).color(MUTED),
                tool_button(
                    "Previous",
                    "Previous match",
                    BlockEditorEvent::SearchMoved(-1),
                    count > 0,
                ),
                tool_button(
                    "Next",
                    "Next match",
                    BlockEditorEvent::SearchMoved(1),
                    count > 0,
                ),
                tool_button(
                    "Close",
                    "Close find and replace",
                    BlockEditorEvent::ToggleSearch,
                    true,
                ),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center),
            row![
                text("REPLACE").size(10).font(INTER_BOLD).color(MUTED),
                text_input("Replace with…", &state.replace_query)
                    .on_input(BlockEditorEvent::ReplaceChanged)
                    .on_submit(BlockEditorEvent::ReplaceMatch)
                    .padding(6)
                    .width(Length::Fill),
                tool_button(
                    "Replace",
                    "Replace current match",
                    BlockEditorEvent::ReplaceMatch,
                    count > 0,
                ),
                tool_button(
                    "Replace all",
                    "Replace all matches",
                    BlockEditorEvent::ReplaceAll,
                    count > 0,
                ),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(5),
    )
    .padding([6, 8])
    .style(|_| soft_surface())
    .into()
}

fn more_formats() -> Element<'static, BlockEditorEvent> {
    container(
        column![
            row![
                format_button("H3", "Heading 3", MarkdownFormat::Heading(3)),
                format_button("H4", "Heading 4", MarkdownFormat::Heading(4)),
                format_button("H5", "Heading 5", MarkdownFormat::Heading(5)),
                format_button("H6", "Heading 6", MarkdownFormat::Heading(6)),
                format_button("Code block", "Fenced code block", MarkdownFormat::CodeBlock,),
                format_button("Table", "GFM table", MarkdownFormat::Table),
                format_button("Image", "Image with alt text", MarkdownFormat::Image),
                format_button("Divider", "Thematic break", MarkdownFormat::Divider),
                format_button("Callout", "GFM callout", MarkdownFormat::Callout),
            ]
            .spacing(2),
            row![
                format_button("Footnote", "Footnote", MarkdownFormat::Footnote),
                format_button("Math", "Display math", MarkdownFormat::Math),
                format_button("Definition", "Definition list", MarkdownFormat::Definition,),
                format_button(
                    "Frontmatter",
                    "YAML frontmatter",
                    MarkdownFormat::Frontmatter,
                ),
                format_button("HTML", "Raw HTML block", MarkdownFormat::Html),
            ]
            .spacing(2),
        ]
        .spacing(3),
    )
    .padding([4, 0])
    .into()
}

fn slash_menu(state: &BlockEditorState) -> Option<Element<'_, BlockEditorEvent>> {
    state.slash_query()?;
    let commands = state.slash_commands().collect::<Vec<_>>();
    let mut list = Column::new().spacing(2);
    for (index, command) in commands.iter().enumerate() {
        let event = BlockEditorEvent::ApplySlash(command.format);
        let selected = index == state.slash_selected;
        list = list.push(semantic_button(
            button(
                row![
                    container(text(command.icon).size(12).font(INTER_BOLD))
                        .center_x(34)
                        .center_y(30)
                        .style(|_| soft_surface()),
                    column![
                        text(command.label).size(12).font(INTER_BOLD),
                        text(command.description).size(10).color(MUTED),
                    ]
                    .spacing(1),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .on_press(event.clone())
            .padding([3, 5])
            .width(Length::Fill)
            .style(move |theme: &Theme, status| {
                let mut style = button::text(theme, status);
                if selected {
                    style.background = Some(Background::Color(BLUE_SOFT));
                }
                style.border.radius = 5.0.into();
                style
            }),
            format!("notion-slash-{}", command.label),
            format!("{}: {}", command.label, command.description),
            Some(event),
        ));
    }
    if commands.is_empty() {
        list = list.push(text("No matching Markdown block").size(12).color(MUTED));
    }
    Some(
        container(
            column![
                row![
                    text("Markdown blocks").size(11).font(INTER_BOLD),
                    iced::widget::space().width(Length::Fill),
                    text("↑↓ choose · ↵ insert · esc close")
                        .size(9)
                        .color(FAINT),
                ],
                scrollable(list).height(Length::Fixed(250.0)),
            ]
            .spacing(6),
        )
        .padding(8)
        .width(390)
        .style(|_| floating_surface())
        .into(),
    )
}

fn markdown_binding(
    event: text_editor::KeyPress,
    slash_choice: Option<MarkdownFormat>,
    search_open: bool,
) -> Option<text_editor::Binding<BlockEditorEvent>> {
    if let Some(format) = slash_choice {
        match event.key.as_ref() {
            keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                return Some(text_editor::Binding::Custom(BlockEditorEvent::SlashMoved(
                    -1,
                )));
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                return Some(text_editor::Binding::Custom(BlockEditorEvent::SlashMoved(
                    1,
                )));
            }
            keyboard::Key::Named(keyboard::key::Named::Enter) => {
                return Some(text_editor::Binding::Custom(BlockEditorEvent::ApplySlash(
                    format,
                )));
            }
            keyboard::Key::Named(keyboard::key::Named::Escape) => {
                return Some(text_editor::Binding::Custom(BlockEditorEvent::DismissSlash));
            }
            _ => {}
        }
    }
    match event.key.as_ref() {
        keyboard::Key::Named(keyboard::key::Named::Enter) => {
            return Some(text_editor::Binding::Custom(BlockEditorEvent::SmartEnter(
                event.modifiers.shift(),
            )));
        }
        keyboard::Key::Named(keyboard::key::Named::Tab) => {
            return Some(text_editor::Binding::Custom(BlockEditorEvent::Indent(
                event.modifiers.shift(),
            )));
        }
        keyboard::Key::Named(keyboard::key::Named::Escape) if search_open => {
            return Some(text_editor::Binding::Custom(BlockEditorEvent::ToggleSearch));
        }
        _ => {}
    }
    if event.modifiers.command() {
        match event.key.to_latin(event.physical_key) {
            Some('f') => return Some(text_editor::Binding::Custom(BlockEditorEvent::ToggleSearch)),
            Some('z') if event.modifiers.shift() => {
                return Some(text_editor::Binding::Custom(BlockEditorEvent::Redo));
            }
            Some('z') => return Some(text_editor::Binding::Custom(BlockEditorEvent::Undo)),
            Some('y') => return Some(text_editor::Binding::Custom(BlockEditorEvent::Redo)),
            Some('b') => {
                return Some(text_editor::Binding::Custom(BlockEditorEvent::Format(
                    MarkdownFormat::Bold,
                )));
            }
            Some('i') => {
                return Some(text_editor::Binding::Custom(BlockEditorEvent::Format(
                    MarkdownFormat::Italic,
                )));
            }
            Some('k') => {
                return Some(text_editor::Binding::Custom(BlockEditorEvent::Format(
                    MarkdownFormat::Link,
                )));
            }
            _ => {}
        }
    }
    text_editor::Binding::from_key_press(event)
}

#[derive(Debug, Clone, Copy)]
enum MarkdownHighlight {
    Heading,
    Syntax,
    Quote,
    Code,
    Link,
}

#[derive(Debug, Default)]
struct MarkdownHighlighter {
    current_line: usize,
}

impl Highlighter for MarkdownHighlighter {
    type Settings = ();
    type Highlight = MarkdownHighlight;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, MarkdownHighlight)>;

    fn new(_: &Self::Settings) -> Self {
        Self::default()
    }

    fn update(&mut self, _: &Self::Settings) {}

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.current_line += 1;
        if line.starts_with('#') {
            return vec![(0..line.len(), MarkdownHighlight::Heading)].into_iter();
        }
        if line.starts_with("    ") || line.starts_with("```") {
            return vec![(0..line.len(), MarkdownHighlight::Code)].into_iter();
        }
        if line.starts_with('>') {
            return vec![(0..line.len(), MarkdownHighlight::Quote)].into_iter();
        }
        let mut highlights = Vec::new();
        if let Some(index) = line.find("http://").or_else(|| line.find("https://")) {
            let end = line[index..]
                .find(|character: char| character.is_whitespace() || matches!(character, ')' | '>'))
                .map_or(line.len(), |length| index + length);
            highlights.push((index..end, MarkdownHighlight::Link));
        }
        let bytes = line.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if b"*~`[]()!_|".contains(&bytes[index]) {
                let start = index;
                index += 1;
                while index < bytes.len() && b"*~`[]()!_|".contains(&bytes[index]) {
                    index += 1;
                }
                if !highlights.iter().any(|(range, _)| range.contains(&start)) {
                    highlights.push((start..index, MarkdownHighlight::Syntax));
                }
            } else {
                index += 1;
            }
        }
        highlights.sort_by_key(|(range, _)| range.start);
        highlights.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

fn markdown_highlight(highlight: &MarkdownHighlight, _theme: &Theme) -> highlighter::Format<Font> {
    match highlight {
        MarkdownHighlight::Heading => highlighter::Format {
            color: Some(FG),
            font: Some(INTER_BOLD),
        },
        MarkdownHighlight::Syntax => highlighter::Format {
            color: Some(FAINT),
            font: Some(INTER),
        },
        MarkdownHighlight::Quote => highlighter::Format {
            color: Some(MUTED),
            font: Some(Font {
                style: font::Style::Italic,
                ..INTER
            }),
        },
        MarkdownHighlight::Code => highlighter::Format {
            color: Some(CODE),
            font: Some(Font::MONOSPACE),
        },
        MarkdownHighlight::Link => highlighter::Format {
            color: Some(PRIMARY),
            font: Some(INTER),
        },
    }
}

fn comment_composer(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    let target = state.composer_for.unwrap_or(0);
    let input = text_input("Write a comment…", &state.comment_draft)
        .on_input(BlockEditorEvent::CommentDraftChanged)
        .on_submit(BlockEditorEvent::SubmitComment)
        .padding(7)
        .width(Length::Fill);
    let submit =
        (!state.comment_draft.trim().is_empty()).then_some(BlockEditorEvent::SubmitComment);
    container(
        row![
            input,
            semantic_button(
                button("Comment")
                    .on_press_maybe(submit.clone())
                    .style(button::primary),
                format!("notion-comment-{target}-submit"),
                "Submit comment",
                submit,
            ),
            semantic_button(
                button("Cancel")
                    .on_press(BlockEditorEvent::CloseCommentComposer)
                    .style(button::text),
                format!("notion-comment-{target}-cancel"),
                "Cancel comment",
                Some(BlockEditorEvent::CloseCommentComposer),
            ),
        ]
        .spacing(5),
    )
    .padding(6)
    .style(|_| soft_surface())
    .into()
}

fn comments_panel(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    let mut threads = Column::new().spacing(9);
    for thread in state
        .threads
        .iter()
        .rev()
        .filter(|thread| thread.resolved == state.show_resolved)
    {
        threads = threads.push(thread_card(state, thread));
    }
    if !state
        .threads
        .iter()
        .any(|thread| thread.resolved == state.show_resolved)
    {
        threads = threads.push(
            text(if state.show_resolved {
                "No resolved comments"
            } else {
                "No open comments"
            })
            .size(13)
            .color(MUTED),
        );
    }
    container(
        column![
            row![
                text("Comments").size(16).font(INTER_BOLD),
                iced::widget::space().width(Length::Fill),
                tool_button(
                    if state.show_resolved {
                        "Resolved ▾"
                    } else {
                        "Open ▾"
                    },
                    "Change comment filter",
                    BlockEditorEvent::ToggleResolved,
                    true,
                ),
                tool_button(
                    "×",
                    "Close comments",
                    BlockEditorEvent::ToggleComments,
                    true
                ),
            ]
            .align_y(iced::Alignment::Center),
            scrollable(threads).height(Length::Fill),
        ]
        .spacing(10),
    )
    .padding(12)
    .width(340)
    .height(Length::Fill)
    .style(|_| {
        container::Style::default()
            .background(SIDEBAR)
            .border(Border {
                color: BORDER,
                width: 1.0,
                radius: 0.0.into(),
            })
    })
    .into()
}

fn thread_card<'a>(
    state: &'a BlockEditorState,
    thread: &'a CommentThread,
) -> Element<'a, BlockEditorEvent> {
    let id = thread.id;
    let context = state
        .block_text(thread.block_id)
        .unwrap_or_else(|| "Deleted block".into());
    let action = if thread.resolved {
        tool_button(
            "Reopen",
            "Reopen comment",
            BlockEditorEvent::Reopen(id),
            true,
        )
    } else {
        tool_button(
            "Resolve",
            "Resolve comment",
            BlockEditorEvent::Resolve(id),
            true,
        )
    };
    let mut messages = Column::new().spacing(7);
    for message in &thread.messages {
        messages = messages.push(
            row![
                container(text(message.author.chars().next().unwrap_or('?')).size(11))
                    .center_x(24)
                    .center_y(24)
                    .style(|_| soft_surface()),
                column![
                    text(format!("{} · {}", message.author, message.time))
                        .size(10)
                        .color(MUTED),
                    text(&message.body).size(12),
                ]
                .spacing(2),
            ]
            .spacing(7),
        );
    }
    let active = state.replying_to == Some(id);
    let reply = if active {
        state.reply_draft.as_str()
    } else {
        ""
    };
    let submit = (active && !state.reply_draft.trim().is_empty())
        .then_some(BlockEditorEvent::SubmitReply(id));
    container(
        column![
            row![
                text(context).size(10).color(MUTED).width(Length::Fill),
                action,
            ]
            .align_y(iced::Alignment::Center),
            messages,
            row![
                text_input("Reply…", reply)
                    .on_input(move |value| BlockEditorEvent::ReplyDraftChanged(id, value))
                    .on_submit(BlockEditorEvent::SubmitReply(id))
                    .padding(6)
                    .width(Length::Fill),
                semantic_button(
                    button("Send")
                        .on_press_maybe(submit.clone())
                        .style(button::text),
                    format!("notion-thread-{id}-submit"),
                    "Send reply",
                    submit,
                ),
            ]
            .spacing(4),
        ]
        .spacing(7),
    )
    .padding(10)
    .width(Length::Fill)
    .style(move |_| comment_surface(thread.resolved))
    .into()
}

fn mode_button(
    label: &'static str,
    mode: EditorMode,
    current: EditorMode,
) -> Element<'static, BlockEditorEvent> {
    let event = BlockEditorEvent::SetMode(mode);
    semantic_button(
        button(text(label).size(11))
            .on_press(event.clone())
            .padding([5, 8])
            .style(move |theme: &Theme, status| {
                let mut style = button::text(theme, status);
                if mode == current {
                    style.background = Some(Background::Color(BLUE_SOFT));
                    style.text_color = PRIMARY;
                }
                style.border.radius = 5.0.into();
                style
            }),
        format!("notion-mode-{label}"),
        format!("{label} Markdown"),
        Some(event),
    )
}

fn format_button(
    visible: &'static str,
    label: &'static str,
    format: MarkdownFormat,
) -> Element<'static, BlockEditorEvent> {
    tool_button(visible, label, BlockEditorEvent::Format(format), true)
}

fn tool_button(
    visible: impl Into<String>,
    label: impl Into<String>,
    event: BlockEditorEvent,
    enabled: bool,
) -> Element<'static, BlockEditorEvent> {
    let visible = visible.into();
    let label = label.into();
    let event = enabled.then_some(event);
    semantic_button(
        button(text(visible).size(11))
            .on_press_maybe(event.clone())
            .padding([4, 7])
            .style(button::text),
        format!("notion-tool-{label}"),
        label,
        event,
    )
}

fn semantic_button<'a>(
    control: iced::widget::Button<'a, BlockEditorEvent>,
    key: impl Into<String>,
    label: impl Into<String>,
    event: Option<BlockEditorEvent>,
) -> Element<'a, BlockEditorEvent> {
    let key = key.into();
    accessible(control, StableId::new(&key), Role::Button)
        .logical_id(key)
        .label(label)
        .disabled(event.is_none())
        .on_activate_maybe(event)
        .into()
}

fn editor_style(_: &Theme, status: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(Color::WHITE),
        border: Border {
            color: if matches!(status, text_editor::Status::Focused { .. }) {
                PRIMARY
            } else {
                BORDER
            },
            width: 1.0,
            radius: 7.0.into(),
        },
        placeholder: FAINT,
        value: FG,
        selection: Color::from_rgb8(194, 218, 247),
    }
}

fn soft_surface() -> container::Style {
    container::Style::default()
        .background(SIDEBAR)
        .border(Border {
            color: BORDER,
            width: 1.0,
            radius: 6.0.into(),
        })
}

fn floating_surface() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::WHITE)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.1),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
}

fn comment_surface(resolved: bool) -> container::Style {
    container::Style::default()
        .background(if resolved { SIDEBAR } else { Color::WHITE })
        .border(Border {
            color: BORDER,
            width: 1.0,
            radius: 8.0.into(),
        })
}

const FG: Color = Color::from_rgb8(55, 53, 47);
const MUTED: Color = Color::from_rgb8(120, 119, 116);
const FAINT: Color = Color::from_rgb8(155, 154, 151);
const BORDER: Color = Color::from_rgb8(233, 233, 231);
const SIDEBAR: Color = Color::from_rgb8(247, 247, 245);
const PRIMARY: Color = Color::from_rgb8(47, 128, 237);
const BLUE_SOFT: Color = Color::from_rgb8(234, 243, 251);
const CODE: Color = Color::from_rgb8(130, 80, 170);

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(state: BlockEditorState, event: BlockEditorEvent) -> BlockEditorState {
        block_editor_apply(state, event)
    }

    fn select(state: &mut BlockEditorState, start: usize, end: usize) {
        let source = state.source();
        state.content.move_to(text_editor::Cursor {
            position: byte_to_position(&source, end),
            selection: Some(byte_to_position(&source, start)),
        });
    }

    #[test]
    fn markdown_source_round_trips_every_supported_block_family() {
        let source = "---\ntitle: Test\n---\n\n# Heading {#id}\n\n**bold** *italic* ~~strike~~ `code` [link](https://example.com)\n\n- [x] task\n1. ordered\n\n> [!NOTE]\n> callout\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\n```rust\nfn main() {}\n```\n\nTerm\n: Definition\n\nMath $x$ and $$y$$\n\nFootnote[^1]\n\n[^1]: note\n\n<div>HTML</div>";
        let mut state = block_editor_state("untitled".into());
        state.replace_source(source.into(), source.len(), None, false);
        assert_eq!(state.markdown(), source);
        assert!(state.block_count() >= 10);
    }

    #[test]
    fn formatting_is_unicode_safe_toggleable_and_undoable() {
        let mut state = block_editor_state("untitled".into());
        state.replace_source("hello 한글".into(), "hello 한글".len(), None, false);
        select(&mut state, 6, "hello 한글".len());
        state = apply(state, BlockEditorEvent::Format(MarkdownFormat::Bold));
        assert_eq!(state.markdown(), "hello **한글**");
        state = apply(state, BlockEditorEvent::Format(MarkdownFormat::Bold));
        assert_eq!(state.markdown(), "hello 한글");
        state = apply(state, BlockEditorEvent::Undo);
        assert_eq!(state.markdown(), "hello **한글**");
        state = apply(state, BlockEditorEvent::Redo);
        assert_eq!(state.markdown(), "hello 한글");
    }

    #[test]
    fn block_formats_and_movement_transform_markdown_source() {
        let mut state = block_editor_state("untitled".into());
        state.replace_source("First\n\nSecond".into(), 0, None, false);
        state = apply(state, BlockEditorEvent::Format(MarkdownFormat::Heading(2)));
        assert_eq!(state.markdown(), "## First\n\nSecond");
        state = apply(state, BlockEditorEvent::MoveBlock(1));
        assert_eq!(state.markdown(), "Second\n\n## First");
    }

    #[test]
    fn slash_menu_covers_extended_markdown_and_inserts_a_table() {
        let mut state = block_editor_state("untitled".into());
        state.replace_source("/tab".into(), 4, None, false);
        let commands = state.slash_commands().collect::<Vec<_>>();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].format, MarkdownFormat::Table);
        state = apply(state, BlockEditorEvent::ApplySlash(MarkdownFormat::Table));
        assert!(state.markdown().contains("| Column 1 | Column 2 |"));
        assert!(state.markdown().contains("| :--- | ---: |"));
    }

    #[test]
    fn native_editor_keeps_document_wide_selection_and_ime_actions() {
        let mut state = block_editor_state("untitled".into());
        state.replace_source("alpha\nbravo".into(), 0, None, false);
        state.content.move_to(text_editor::Cursor {
            position: text_editor::Position { line: 1, column: 3 },
            selection: Some(text_editor::Position { line: 0, column: 2 }),
        });
        assert_eq!(state.content.selection().as_deref(), Some("pha\nbra"));
        state = apply(
            state,
            BlockEditorEvent::Edit(text_editor::Action::Edit(text_editor::Edit::Paste(
                std::sync::Arc::new("한".into()),
            ))),
        );
        assert_eq!(state.markdown(), "al한vo");
    }

    #[test]
    fn enter_replaces_a_cross_line_selection() {
        let mut state = block_editor_state("untitled".into());
        state.replace_source("alpha\nbravo".into(), 0, None, false);
        select(&mut state, 2, 9);
        state = apply(state, BlockEditorEvent::SmartEnter(false));
        assert_eq!(state.markdown(), "al\nvo");
    }

    #[test]
    fn smart_enter_continues_and_exits_markdown_lists() {
        let mut state = block_editor_state("untitled".into());
        state.replace_source("- [x] Ship".into(), 10, None, false);
        state = apply(state, BlockEditorEvent::SmartEnter(false));
        assert_eq!(state.markdown(), "- [x] Ship\n- [ ] ");
        state = apply(state, BlockEditorEvent::SmartEnter(false));
        assert_eq!(state.markdown(), "- [x] Ship\n");

        state.replace_source("9. First".into(), 8, None, false);
        state = apply(state, BlockEditorEvent::SmartEnter(false));
        assert_eq!(state.markdown(), "9. First\n10. ");
    }

    #[test]
    fn tab_indents_and_outdents_a_cross_line_selection() {
        let mut state = block_editor_state("untitled".into());
        state.replace_source("one\ntwo".into(), 0, None, false);
        select(&mut state, 0, 7);
        state = apply(state, BlockEditorEvent::Indent(false));
        let indented = state.markdown();
        assert_ne!(indented, "one\ntwo");
        state = apply(state, BlockEditorEvent::Indent(true));
        assert_eq!(state.markdown(), "one\ntwo");
    }

    #[test]
    fn find_navigation_and_replace_share_document_history() {
        let mut state = block_editor_state("untitled".into());
        state.replace_source("one two one".into(), 0, None, false);
        state = apply(state, BlockEditorEvent::SearchChanged("one".into()));
        assert_eq!(state.content.selection().as_deref(), Some("one"));
        state = apply(state, BlockEditorEvent::SearchMoved(1));
        assert_eq!(state.selection_bytes(), 8..11);
        state = apply(state, BlockEditorEvent::ReplaceChanged("three".into()));
        state = apply(state, BlockEditorEvent::ReplaceMatch);
        assert_eq!(state.markdown(), "one two three");
        state = apply(state, BlockEditorEvent::Undo);
        assert_eq!(state.markdown(), "one two one");
    }

    #[test]
    fn rendered_editor_exposes_write_preview_and_markdown_tools() {
        let state = block_editor_state("home".into());
        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(920.0, 620.0),
            block_editor(&state),
        );
        screen.find("Write").expect("write mode");
        screen.find("Preview").expect("preview mode");
        screen.find("Find").expect("find and replace action");
        screen.find("More ▾").expect("organized extended formats");
        assert!(
            screen
                .find("Can we link the customer research notes here?")
                .is_err(),
            "comments do not interrupt writing"
        );
        screen.click("More ▾").expect("extended format menu");
        screen.click("Find").expect("find and replace action");
        let state = screen.into_messages().fold(state, apply);
        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(920.0, 620.0),
            block_editor(&state),
        );
        screen.find("Table").expect("GFM table action");
        screen.find("Footnote").expect("footnote action");
        screen.find("Find…").expect("find input");
        screen.find("Replace with…").expect("replace input");
        screen.click("Preview").expect("preview control");
        let state = screen.into_messages().fold(state, apply);
        assert_eq!(state.mode, EditorMode::Preview);
        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(920.0, 620.0),
            block_editor(&state),
        );
        assert!(!state.preview.items().is_empty());
        screen
            .snapshot(&Theme::Light)
            .expect("rendered Markdown preview");
    }

    #[test]
    fn comments_support_threads_replies_and_resolution() {
        let mut state = block_editor_state("untitled".into());
        state = apply(state, BlockEditorEvent::OpenCommentComposer(1));
        state = apply(state, BlockEditorEvent::CommentDraftChanged("First".into()));
        state = apply(state, BlockEditorEvent::SubmitComment);
        let id = state.threads[0].id;
        state = apply(
            state,
            BlockEditorEvent::ReplyDraftChanged(id, "Reply".into()),
        );
        state = apply(state, BlockEditorEvent::SubmitReply(id));
        assert_eq!(state.thread_message_count(id), 2);
        state = apply(state, BlockEditorEvent::Resolve(id));
        assert!(state.thread_resolved(id));
    }
}
