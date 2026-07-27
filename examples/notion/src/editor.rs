use iced::advanced::Renderer as _;
use iced::advanced::text::{Paragraph, Renderer as TextRenderer, Span};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::widget::{
    Column, button, column, container, row, scrollable, text, text_editor, text_input,
};
use iced::{
    Background, Border, Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Size, Task,
    Theme, Vector, font, keyboard,
};
use pulldown_cmark::{Event as MarkdownEvent, HeadingLevel, Options, Parser, Tag, TagEnd};
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
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    editor_active: bool,
    focus_requested: bool,
    search_focus_requested: bool,
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
            undo: self.undo.clone(),
            redo: self.redo.clone(),
            editor_active: self.editor_active,
            focus_requested: self.focus_requested,
            search_focus_requested: self.search_focus_requested,
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
    Select(usize, Option<usize>),
    Format(MarkdownFormat),
    Undo,
    Redo,
    MoveBlock(i8),
    SmartEnter(bool),
    SmartBackspace,
    Indent(bool),
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
        undo: Vec::new(),
        redo: Vec::new(),
        editor_active: false,
        focus_requested: false,
        search_focus_requested: false,
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
        BlockEditorEvent::Select(caret, anchor) => {
            let source = state.source();
            state.content.move_to(text_editor::Cursor {
                position: byte_to_position(&source, caret),
                selection: anchor.map(|anchor| byte_to_position(&source, anchor)),
            });
            state.editor_active = true;
            state.focus_requested = true;
        }
        BlockEditorEvent::Format(format) => state.format(format),
        BlockEditorEvent::Undo => state.undo(),
        BlockEditorEvent::Redo => state.redo(),
        BlockEditorEvent::MoveBlock(direction) => state.move_block(direction),
        BlockEditorEvent::SmartEnter(hard_break) => state.smart_enter(hard_break),
        BlockEditorEvent::SmartBackspace => state.smart_backspace(),
        BlockEditorEvent::Indent(outdent) => state.perform(text_editor::Action::Edit(if outdent {
            text_editor::Edit::Unindent
        } else {
            text_editor::Edit::Indent
        })),
        BlockEditorEvent::ToggleFormats => state.formats_open = !state.formats_open,
        BlockEditorEvent::ToggleSearch => {
            state.search_open = !state.search_open;
            state.editor_active = !state.search_open;
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
        self.focus_requested = true;
    }

    fn perform(&mut self, action: text_editor::Action) {
        self.editor_active = true;
        let before = action.is_edit().then(|| self.snapshot());
        let previous = before.as_ref().map(|snapshot| snapshot.source.as_str());
        self.content.perform(action);
        let source = self.source();
        if previous.is_some_and(|previous| previous != source) {
            self.remember(before.expect("editing actions capture history"));
        }
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

    fn smart_backspace(&mut self) {
        let source = self.source();
        let selection = self.selection_bytes();
        if !selection.is_empty() || selection.start == 0 {
            self.perform(text_editor::Action::Edit(text_editor::Edit::Backspace));
            return;
        }
        let line = selected_line_range(&source, selection.clone());
        if !source[line.clone()].is_empty() {
            self.perform(text_editor::Action::Edit(text_editor::Edit::Backspace));
            return;
        }
        let start = line.start.saturating_sub(1);
        let mut next = source;
        next.replace_range(start..line.start, "");
        self.replace_source(next, start, None, true);
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

    fn format(&mut self, format: MarkdownFormat) {
        self.formats_open = false;
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct RenderStyle {
    size: f32,
    font: Font,
    color: Color,
    background: Option<Color>,
    underline: bool,
    strikethrough: bool,
}

impl RenderStyle {
    fn body() -> Self {
        Self {
            size: 16.0,
            font: INTER,
            color: FG,
            background: None,
            underline: false,
            strikethrough: false,
        }
    }

    fn syntax() -> Self {
        Self {
            color: FAINT,
            ..Self::body()
        }
    }
}

#[derive(Debug, Clone)]
struct RenderRun {
    range: Range<usize>,
    style: RenderStyle,
}

#[derive(Debug, Clone)]
struct RenderedMarkdown {
    text: String,
    runs: Vec<RenderRun>,
    visual_to_source: Vec<usize>,
}

impl RenderedMarkdown {
    fn new(source_start: usize) -> Self {
        Self {
            text: String::new(),
            runs: Vec::new(),
            visual_to_source: vec![source_start],
        }
    }

    fn push(&mut self, value: &str, source: &str, range: Range<usize>, style: RenderStyle) {
        if value.is_empty() {
            return;
        }
        let visual_start = self.text.len();
        self.text.push_str(value);
        let exact = source
            .get(range.clone())
            .and_then(|slice| slice.find(value).map(|offset| range.start + offset));
        for byte in 1..=value.len() {
            self.visual_to_source.push(exact.map_or_else(
                || {
                    if byte == value.len() {
                        range.end
                    } else {
                        range.start
                    }
                },
                |start| start + byte,
            ));
        }
        let visual_end = self.text.len();
        if let Some(last) = self.runs.last_mut()
            && last.range.end == visual_start
            && last.style == style
        {
            last.range.end = visual_end;
        } else {
            self.runs.push(RenderRun {
                range: visual_start..visual_end,
                style,
            });
        }
    }

    fn push_synthetic(&mut self, value: &str, source: usize, style: RenderStyle) {
        self.push(value, "", source..source, style);
    }

    fn source_to_visual(&self, source: usize) -> usize {
        self.text
            .char_indices()
            .map(|(visual, _)| visual)
            .chain(std::iter::once(self.text.len()))
            .min_by_key(|visual| self.visual_to_source[*visual].abs_diff(source))
            .unwrap_or(0)
    }

    fn source_at(&self, mut visual: usize) -> usize {
        visual = visual.min(self.text.len());
        while visual > 0 && !self.text.is_char_boundary(visual) {
            visual -= 1;
        }
        self.visual_to_source[visual]
    }

    fn spans(&self, selection: Range<usize>) -> Vec<Span<'static, (), Font>> {
        let mut spans = Vec::new();
        for run in &self.runs {
            let mut cuts = vec![run.range.start, run.range.end];
            if run.range.contains(&selection.start) {
                cuts.push(selection.start);
            }
            if run.range.contains(&selection.end) {
                cuts.push(selection.end);
            }
            cuts.sort_unstable();
            cuts.dedup();
            for range in cuts.windows(2).map(|cut| cut[0]..cut[1]) {
                if range.is_empty() {
                    continue;
                }
                let style = run.style;
                let mut span = Span::new(self.text[range.clone()].to_owned())
                    .size(style.size)
                    .font(style.font)
                    .color(style.color)
                    .underline(style.underline)
                    .strikethrough(style.strikethrough);
                if range.start < selection.end && selection.start < range.end {
                    span = span.background(SELECTION);
                } else if let Some(background) = style.background {
                    span = span.background(background);
                }
                spans.push(span);
            }
        }
        spans
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct StyleContext {
    heading: Option<HeadingLevel>,
    strong: usize,
    emphasis: usize,
    strikethrough: usize,
    code: usize,
    link: usize,
    quote: usize,
}

impl StyleContext {
    fn style(self) -> RenderStyle {
        let mut style = RenderStyle::body();
        if let Some(level) = self.heading {
            style.size = heading_size(level);
            style.font.weight = font::Weight::Bold;
        }
        if self.strong > 0 {
            style.font.weight = font::Weight::Bold;
        }
        if self.emphasis > 0 || self.quote > 0 {
            style.font.style = font::Style::Italic;
        }
        if self.strikethrough > 0 {
            style.strikethrough = true;
        }
        if self.code > 0 {
            style.font = Font::MONOSPACE;
            style.color = CODE;
            style.background = Some(CODE_SOFT);
        } else if self.link > 0 {
            style.color = PRIMARY;
            style.underline = true;
        } else if self.quote > 0 {
            style.color = MUTED;
        }
        style
    }

    fn start(&mut self, tag: &Tag<'_>) {
        match tag {
            Tag::Heading { level, .. } => self.heading = Some(*level),
            Tag::Strong => self.strong += 1,
            Tag::Emphasis => self.emphasis += 1,
            Tag::Strikethrough => self.strikethrough += 1,
            Tag::CodeBlock(_) => self.code += 1,
            Tag::Link { .. } => self.link += 1,
            Tag::BlockQuote(_) => self.quote += 1,
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => self.heading = None,
            TagEnd::Strong => self.strong = self.strong.saturating_sub(1),
            TagEnd::Emphasis => self.emphasis = self.emphasis.saturating_sub(1),
            TagEnd::Strikethrough => self.strikethrough = self.strikethrough.saturating_sub(1),
            TagEnd::CodeBlock => self.code = self.code.saturating_sub(1),
            TagEnd::Link => self.link = self.link.saturating_sub(1),
            TagEnd::BlockQuote(_) => self.quote = self.quote.saturating_sub(1),
            _ => {}
        }
    }
}

fn heading_size(level: HeadingLevel) -> f32 {
    match level {
        HeadingLevel::H1 => 31.0,
        HeadingLevel::H2 => 25.0,
        HeadingLevel::H3 => 21.0,
        HeadingLevel::H4 => 18.0,
        HeadingLevel::H5 => 17.0,
        HeadingLevel::H6 => 16.0,
    }
}

fn event_text_range(source: &str, range: Range<usize>, text: &str) -> Range<usize> {
    source
        .get(range.clone())
        .and_then(|slice| slice.find(text).map(|offset| range.start + offset))
        .map_or(range, |start| start..start + text.len())
}

fn push_pending_list(rendered: &mut RenderedMarkdown, pending: &mut Option<(String, usize)>) {
    if let Some((prefix, source)) = pending.take() {
        rendered.push_synthetic(&prefix, source, RenderStyle::syntax());
    }
}

fn render_semantic_block(rendered: &mut RenderedMarkdown, source: &str, block: Range<usize>) {
    let markdown = &source[block.clone()];
    let mut context = StyleContext::default();
    let mut lists = Vec::<Option<u64>>::new();
    let mut pending_list = None;
    let mut table_cell = 0usize;

    for (event, local) in Parser::new_ext(markdown, markdown_options()).into_offset_iter() {
        let range = block.start + local.start..block.start + local.end;
        match event {
            MarkdownEvent::Start(tag) => {
                match &tag {
                    Tag::List(start) => lists.push(*start),
                    Tag::Item => {
                        let prefix = match lists.last_mut() {
                            Some(Some(number)) => {
                                let prefix = format!("{number}. ");
                                *number += 1;
                                prefix
                            }
                            _ => "•  ".to_owned(),
                        };
                        pending_list = Some((prefix, range.start));
                    }
                    Tag::BlockQuote(_) => {
                        push_pending_list(rendered, &mut pending_list);
                        rendered.push_synthetic("▎  ", range.start, RenderStyle::syntax());
                    }
                    Tag::Image { .. } => {
                        push_pending_list(rendered, &mut pending_list);
                        rendered.push_synthetic("▧  ", range.start, RenderStyle::syntax());
                    }
                    Tag::FootnoteDefinition(label) => {
                        push_pending_list(rendered, &mut pending_list);
                        rendered.push_synthetic(
                            &format!("{label}  "),
                            range.start,
                            RenderStyle::syntax(),
                        );
                    }
                    Tag::DefinitionListDefinition => {
                        rendered.push_synthetic("  —  ", range.start, RenderStyle::syntax());
                    }
                    Tag::TableHead | Tag::TableRow => table_cell = 0,
                    Tag::TableCell if table_cell > 0 => {
                        rendered.push_synthetic("    ", range.start, RenderStyle::syntax());
                        table_cell += 1;
                    }
                    Tag::TableCell => table_cell = 1,
                    _ => {}
                }
                context.start(&tag);
            }
            MarkdownEvent::End(tag) => {
                match tag {
                    TagEnd::Item => {
                        push_pending_list(rendered, &mut pending_list);
                        rendered.push_synthetic("\n", range.end, RenderStyle::body());
                    }
                    TagEnd::List(_) => {
                        lists.pop();
                    }
                    TagEnd::TableHead | TagEnd::TableRow => {
                        rendered.push_synthetic("\n", range.end, RenderStyle::body());
                    }
                    _ => {}
                }
                context.end(tag);
            }
            MarkdownEvent::Text(value) => {
                push_pending_list(rendered, &mut pending_list);
                let range = event_text_range(source, range, &value);
                rendered.push(&value, source, range, context.style());
            }
            MarkdownEvent::Code(value) => {
                push_pending_list(rendered, &mut pending_list);
                let range = event_text_range(source, range, &value);
                let mut style = context.style();
                style.font = Font::MONOSPACE;
                style.color = CODE;
                style.background = Some(CODE_SOFT);
                rendered.push(&value, source, range, style);
            }
            MarkdownEvent::InlineMath(value) | MarkdownEvent::DisplayMath(value) => {
                push_pending_list(rendered, &mut pending_list);
                let range = event_text_range(source, range, &value);
                let mut style = context.style();
                style.font = Font::MONOSPACE;
                style.color = CODE;
                rendered.push(&value, source, range, style);
            }
            MarkdownEvent::Html(value) | MarkdownEvent::InlineHtml(value) => {
                push_pending_list(rendered, &mut pending_list);
                let mut style = context.style();
                style.font = Font::MONOSPACE;
                style.color = MUTED;
                rendered.push(&value, source, range, style);
            }
            MarkdownEvent::FootnoteReference(value) => {
                push_pending_list(rendered, &mut pending_list);
                let label = format!("{value}¹");
                let mut style = context.style();
                style.color = PRIMARY;
                rendered.push(&label, source, range, style);
            }
            MarkdownEvent::SoftBreak | MarkdownEvent::HardBreak => {
                rendered.push("\n", source, range, context.style());
            }
            MarkdownEvent::Rule => {
                rendered.push_synthetic("────────────────────", range.start, RenderStyle::syntax());
            }
            MarkdownEvent::TaskListMarker(checked) => {
                pending_list = None;
                rendered.push_synthetic(
                    if checked { "☑  " } else { "☐  " },
                    range.start,
                    RenderStyle::syntax(),
                );
            }
        }
    }
    push_pending_list(rendered, &mut pending_list);
}

fn render_raw_block(rendered: &mut RenderedMarkdown, source: &str, block: Range<usize>) {
    let markdown = &source[block.clone()];
    let mut context = StyleContext::default();
    let mut styled = Vec::<(Range<usize>, RenderStyle)>::new();
    for (event, local) in Parser::new_ext(markdown, markdown_options()).into_offset_iter() {
        let range = block.start + local.start..block.start + local.end;
        match event {
            MarkdownEvent::Start(tag) => context.start(&tag),
            MarkdownEvent::End(tag) => context.end(tag),
            MarkdownEvent::Text(value) => {
                styled.push((event_text_range(source, range, &value), context.style()))
            }
            MarkdownEvent::Code(value)
            | MarkdownEvent::InlineMath(value)
            | MarkdownEvent::DisplayMath(value) => {
                let mut style = context.style();
                style.font = Font::MONOSPACE;
                style.color = CODE;
                style.background = Some(CODE_SOFT);
                styled.push((event_text_range(source, range, &value), style));
            }
            MarkdownEvent::Html(_) | MarkdownEvent::InlineHtml(_) => {
                let mut style = context.style();
                style.font = Font::MONOSPACE;
                style.color = MUTED;
                styled.push((range, style));
            }
            MarkdownEvent::FootnoteReference(_) => {
                let mut style = context.style();
                style.color = PRIMARY;
                styled.push((range, style));
            }
            MarkdownEvent::SoftBreak
            | MarkdownEvent::HardBreak
            | MarkdownEvent::Rule
            | MarkdownEvent::TaskListMarker(_) => {}
        }
    }
    styled.sort_by_key(|(range, _)| range.start);
    let mut cursor = block.start;
    for (range, style) in styled {
        let range = range.start.max(cursor)..range.end.min(block.end);
        if cursor < range.start {
            rendered.push(
                &source[cursor..range.start],
                source,
                cursor..range.start,
                RenderStyle::syntax(),
            );
        }
        if range.start < range.end {
            rendered.push(&source[range.clone()], source, range.clone(), style);
            cursor = range.end;
        }
    }
    if cursor < block.end {
        rendered.push(
            &source[cursor..block.end],
            source,
            cursor..block.end,
            RenderStyle::syntax(),
        );
    }
}

fn render_markdown(
    source: &str,
    caret: usize,
    selection: Range<usize>,
    active: bool,
) -> RenderedMarkdown {
    let mut rendered = RenderedMarkdown::new(0);
    if source.is_empty() {
        rendered.push_synthetic(
            "Start writing…",
            0,
            RenderStyle {
                color: FAINT,
                ..RenderStyle::body()
            },
        );
        return rendered;
    }

    let blocks = block_ranges(source);
    let active_block = (active && selection.is_empty())
        .then(|| {
            blocks
                .iter()
                .position(|range| range.contains(&caret) || caret == range.end)
        })
        .flatten();
    let mut previous_end = 0;
    for (index, block) in blocks.into_iter().enumerate() {
        if !rendered.text.is_empty() {
            rendered.push(
                "\n\n",
                source,
                previous_end..block.start,
                RenderStyle::body(),
            );
        }
        if active_block == Some(index) {
            render_raw_block(&mut rendered, source, block.clone());
        } else {
            render_semantic_block(&mut rendered, source, block.clone());
        }
        previous_end = block.end;
    }
    rendered
}

#[derive(Default)]
struct BearEditorState {
    paragraph: <iced::Renderer as TextRenderer>::Paragraph,
    dragging: bool,
    anchor: usize,
    modifiers: keyboard::Modifiers,
}

struct BearEditor<'a> {
    input: Element<'a, BlockEditorEvent>,
    rendered: RenderedMarkdown,
    spans: Vec<Span<'static, (), Font>>,
    caret: usize,
    anchor: Option<usize>,
    active: bool,
}

impl BearEditor<'_> {
    fn hit(&self, state: &BearEditorState, layout: Layout<'_>, cursor: mouse::Cursor) -> usize {
        let bounds = layout.bounds();
        let Some(position) = cursor.position() else {
            return self.caret;
        };
        let point = Point::new(
            (position.x - bounds.x - 24.0).clamp(0.0, (bounds.width - 48.0).max(0.0)),
            (position.y - bounds.y - 20.0).clamp(0.0, (bounds.height - 40.0).max(0.0)),
        );
        let visual = state.paragraph.hit_test(point).map_or_else(
            || {
                if point.y <= 0.0 {
                    0
                } else {
                    self.rendered.text.len()
                }
            },
            iced::advanced::text::Hit::cursor,
        );
        self.rendered.source_at(visual)
    }
}

impl Widget<BlockEditorEvent, Theme, iced::Renderer> for BearEditor<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<BearEditorState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(BearEditorState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.input)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.children[0].diff(&self.input);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let width = limits.max().width.max(48.0);
        let text_bounds = Size::new((width - 48.0).max(1.0), f32::INFINITY);
        let state = tree.state.downcast_mut::<BearEditorState>();
        state.paragraph =
            <iced::Renderer as TextRenderer>::Paragraph::with_spans(iced::advanced::text::Text {
                content: self.spans.as_slice(),
                bounds: text_bounds,
                size: Pixels(16.0),
                line_height: iced::advanced::text::LineHeight::Relative(1.55),
                font: INTER,
                align_x: iced::advanced::text::Alignment::Default,
                align_y: iced::Alignment::Start.into(),
                shaping: iced::advanced::text::Shaping::Advanced,
                wrapping: iced::advanced::text::Wrapping::WordOrGlyph,
            });
        let intrinsic = Size::new(width, state.paragraph.min_height().max(28.0) + 40.0);
        let size = limits.resolve(Length::Fill, Length::Shrink, intrinsic);
        state.paragraph.resize(Size::new(
            (size.width - 48.0).max(1.0),
            (size.height - 40.0).max(1.0),
        ));

        let child_limits = layout::Limits::new(Size::ZERO, Size::new(size.width, 1.0))
            .width(Length::Fill)
            .height(Length::Fixed(1.0));
        let child =
            self.input
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &child_limits);
        layout::Node::with_children(size, vec![child])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(child) = layout.children().next() {
            self.input
                .as_widget_mut()
                .operate(&mut tree.children[0], child, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, BlockEditorEvent>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<BearEditorState>();
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(layout.bounds()) =>
            {
                let source = self.hit(state, layout, cursor);
                state.anchor = if state.modifiers.shift() {
                    self.anchor.unwrap_or(self.caret)
                } else {
                    source
                };
                state.dragging = true;
                shell.publish(BlockEditorEvent::Select(
                    source,
                    state.modifiers.shift().then_some(state.anchor),
                ));
                shell.capture_event();
                return;
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.dragging => {
                let source = self.hit(state, layout, cursor);
                shell.publish(BlockEditorEvent::Select(source, Some(state.anchor)));
                shell.capture_event();
                return;
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
                state.dragging = false;
                shell.capture_event();
                return;
            }
            _ => {}
        }

        if let Some(child) = layout.children().next() {
            self.input.as_widget_mut().update(
                &mut tree.children[0],
                event,
                child,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if !bounds.intersects(viewport) {
            return;
        }
        let state = tree.state.downcast_ref::<BearEditorState>();
        let origin = bounds.position() + Vector::new(24.0, 20.0);

        for (index, span) in self.spans.iter().enumerate() {
            let regions = state.paragraph.span_bounds(index);
            if let Some(highlight) = span.highlight {
                for region in &regions {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: *region + (origin - Point::ORIGIN),
                            border: highlight.border,
                            ..renderer::Quad::default()
                        },
                        highlight.background,
                    );
                }
            }
        }
        renderer.fill_paragraph(&state.paragraph, origin, defaults.text_color, *viewport);

        for (index, span) in self.spans.iter().enumerate() {
            if !span.underline && !span.strikethrough {
                continue;
            }
            let color = span.color.unwrap_or(defaults.text_color);
            let size = span.size.unwrap_or(Pixels(16.0)).0;
            for region in state.paragraph.span_bounds(index) {
                let y = if span.strikethrough {
                    region.y + region.height / 2.0
                } else {
                    region.y + region.height - size * 0.12
                };
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(
                            Point::new(origin.x + region.x, origin.y + y),
                            Size::new(region.width, 1.0),
                        ),
                        ..renderer::Quad::default()
                    },
                    color,
                );
            }
        }

        if self.active && self.anchor.is_none() {
            let visual = self.rendered.source_to_visual(self.caret);
            let prefix = &self.rendered.text[..visual];
            let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
            let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
            let column = text_input::Value::new(&prefix[line_start..]).len();
            if let Some(position) = state.paragraph.grapheme_position(line, column) {
                let height = self
                    .rendered
                    .runs
                    .iter()
                    .find(|run| run.range.contains(&visual))
                    .map_or(22.0, |run| run.style.size * 1.28);
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(
                            Point::new(origin.x + position.x, origin.y + position.y),
                            Size::new(1.5, height),
                        ),
                        ..renderer::Quad::default()
                    },
                    PRIMARY,
                );
            }
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, BlockEditorEvent, Theme, iced::Renderer>> {
        let child = layout.children().next()?;
        self.input.as_widget_mut().overlay(
            &mut tree.children[0],
            child,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a> From<BearEditor<'a>> for Element<'a, BlockEditorEvent> {
    fn from(editor: BearEditor<'a>) -> Self {
        Self::new(editor)
    }
}

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

fn document(state: &BlockEditorState, _width: f32) -> Element<'_, BlockEditorEvent> {
    let toolbar = toolbar(state);
    let search = state.search_open.then(|| search_bar(state));
    let source = state.source();
    let words = source.split_whitespace().count();
    let blocks = state.block_count();
    let status = row![
        text(format!("{words} words · {blocks} blocks"))
            .size(10)
            .color(MUTED),
        iced::widget::space().width(Length::Fill),
        text("Markdown").size(10).color(FAINT),
    ];

    let mut editor = column![toolbar].spacing(6).width(Length::Fill);
    if let Some(search) = search {
        editor = editor.push(search);
    }
    if state.composer_for.is_some() {
        editor = editor.push(comment_composer(state));
    }
    editor = editor.push(source_editor(state)).push(status);

    container(editor)
        .padding([8, 10])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn source_editor(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    let source = state.source();
    let (caret, anchor) = state.cursor_bytes();
    let selection = anchor
        .map(|anchor| anchor.min(caret)..anchor.max(caret))
        .unwrap_or(caret..caret);
    let rendered = render_markdown(&source, caret, selection.clone(), state.editor_active);
    let visual_selection =
        rendered.source_to_visual(selection.start)..rendered.source_to_visual(selection.end);
    let spans = rendered.spans(visual_selection);
    let search_open = state.search_open;
    let cursor = state.content.cursor();
    let empty_line = anchor.is_none()
        && cursor.position.line > 0
        && cursor.position.column == 0
        && state
            .content
            .line(cursor.position.line)
            .is_some_and(|line| line.text.is_empty());
    let input = text_editor(&state.content)
        .id(iced::widget::Id::new(EDITOR_ID))
        .on_action(BlockEditorEvent::Edit)
        .key_binding(move |event| markdown_binding(event, search_open, empty_line))
        .font(INTER)
        .size(16)
        .padding(0)
        .height(1)
        .style(hidden_editor_style);
    let input = accessible(
        input,
        StableId::new("notion-markdown-document"),
        Role::TextInput,
    )
    .logical_id("notion-markdown-document")
    .focus_id(iced::widget::Id::new(EDITOR_ID))
    .label("Markdown document")
    .value(source)
    .into();
    let editor = BearEditor {
        input,
        rendered,
        spans,
        caret,
        anchor,
        active: state.editor_active && !state.search_open,
    };

    scrollable(
        container(Element::from(editor))
            .max_width(840)
            .center_x(Length::Fill),
    )
    .height(Length::Fill)
    .into()
}
fn toolbar(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    let current = state.current_block();
    let mut controls = column![
        row![
            tool_button(
                "Styles ▾",
                "Text and block styles",
                BlockEditorEvent::ToggleFormats,
                true,
            ),
            format_button("B", "Bold (Ctrl+B)", MarkdownFormat::Bold),
            format_button("I", "Italic (Ctrl+I)", MarkdownFormat::Italic),
            format_button("S", "Strikethrough", MarkdownFormat::Strikethrough),
            format_button("Code", "Inline code", MarkdownFormat::InlineCode),
            format_button("Link", "Link (Ctrl+K)", MarkdownFormat::Link),
            iced::widget::space().width(Length::Fill),
            tool_button(
                "Find",
                "Find and replace (Ctrl+F)",
                BlockEditorEvent::ToggleSearch,
                true,
            ),
            tool_button(
                "Undo",
                "Undo (Ctrl+Z)",
                BlockEditorEvent::Undo,
                !state.undo.is_empty(),
            ),
            tool_button(
                "Redo",
                "Redo (Ctrl+Shift+Z)",
                BlockEditorEvent::Redo,
                !state.redo.is_empty(),
            ),
            tool_button(
                "Comment",
                "Comment on current block",
                BlockEditorEvent::OpenCommentComposer(current),
                true,
            ),
        ]
        .spacing(3)
        .align_y(iced::Alignment::Center),
    ];
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
                format_button("Text", "Paragraph", MarkdownFormat::Paragraph),
                format_button("H1", "Heading 1", MarkdownFormat::Heading(1)),
                format_button("H2", "Heading 2", MarkdownFormat::Heading(2)),
                format_button("H3", "Heading 3", MarkdownFormat::Heading(3)),
                format_button("H4", "Heading 4", MarkdownFormat::Heading(4)),
                format_button("H5", "Heading 5", MarkdownFormat::Heading(5)),
                format_button("H6", "Heading 6", MarkdownFormat::Heading(6)),
            ]
            .spacing(2),
            row![
                format_button("Bullet", "Bulleted list", MarkdownFormat::Bullet),
                format_button("Number", "Numbered list", MarkdownFormat::Ordered),
                format_button("Task", "Task list", MarkdownFormat::Task),
                format_button("Quote", "Quote", MarkdownFormat::Quote),
                format_button("Code block", "Fenced code block", MarkdownFormat::CodeBlock),
                format_button("Table", "GFM table", MarkdownFormat::Table),
                format_button("Divider", "Thematic break", MarkdownFormat::Divider),
            ]
            .spacing(2),
            row![
                format_button("Callout", "GFM callout", MarkdownFormat::Callout),
                format_button("Image", "Image with alt text", MarkdownFormat::Image),
                format_button("Footnote", "Footnote", MarkdownFormat::Footnote),
                format_button("Math", "Display math", MarkdownFormat::Math),
                format_button("Definition", "Definition list", MarkdownFormat::Definition),
                format_button(
                    "Frontmatter",
                    "YAML frontmatter",
                    MarkdownFormat::Frontmatter
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
fn markdown_binding(
    event: text_editor::KeyPress,
    search_open: bool,
    empty_line: bool,
) -> Option<text_editor::Binding<BlockEditorEvent>> {
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
        keyboard::Key::Named(keyboard::key::Named::Backspace) if empty_line => {
            return Some(text_editor::Binding::Custom(
                BlockEditorEvent::SmartBackspace,
            ));
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

fn hidden_editor_style(_: &Theme, _: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        placeholder: Color::TRANSPARENT,
        value: Color::TRANSPARENT,
        selection: Color::TRANSPARENT,
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
const CODE: Color = Color::from_rgb8(130, 80, 170);
const CODE_SOFT: Color = Color::from_rgb8(247, 242, 250);
const SELECTION: Color = Color::from_rgba8(47, 128, 237, 0.2);

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
    fn bear_render_hides_markers_styles_text_and_reveals_only_the_active_block() {
        let source = "# Heading\n\nBody with **bold**, *italic*, ~~strike~~, `code`, and [link](https://example.com).";
        let rendered = render_markdown(source, 0, 0..0, false);
        assert!(rendered.text.contains("Heading"));
        assert!(
            rendered
                .text
                .contains("Body with bold, italic, strike, code, and link.")
        );
        assert!(!rendered.text.contains("**"));
        assert!(!rendered.text.contains("https://"));
        assert!(rendered.runs.iter().any(|run| run.style.size == 31.0));
        assert!(rendered.runs.iter().any(|run| run.style.strikethrough));
        assert!(
            rendered
                .runs
                .iter()
                .any(|run| run.style.font == Font::MONOSPACE)
        );

        let caret = source.find("bold").expect("bold source");
        let active = render_markdown(source, caret, caret..caret, true);
        assert!(active.text.starts_with("Heading\n\n"));
        assert!(active.text.contains("**bold**"));
        assert!(!active.text.starts_with("# "));
        let visual = active.source_to_visual(caret);
        assert_eq!(active.source_at(visual), caret);
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
    fn backspace_on_an_empty_line_removes_it_and_moves_to_the_previous_end() {
        let mut state = block_editor_state("untitled".into());
        state.replace_source("Previous\n\nNext".into(), 9, None, false);
        state = apply(state, BlockEditorEvent::SmartBackspace);
        assert_eq!(state.markdown(), "Previous\nNext");
        assert_eq!(
            state.content.cursor().position,
            text_editor::Position { line: 0, column: 8 }
        );
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
    fn rendered_editor_is_one_inline_surface_with_organized_markdown_tools() {
        let state = block_editor_state("home".into());
        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(920.0, 620.0),
            block_editor(&state),
        );
        assert!(screen.find("Write").is_err());
        assert!(screen.find("Preview").is_err());
        screen.find("Find").expect("find and replace action");
        screen.find("Styles ▾").expect("organized format menu");
        assert!(
            screen
                .find("Can we link the customer research notes here?")
                .is_err(),
            "comments do not interrupt writing"
        );
        screen.click("Styles ▾").expect("format menu");
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
        screen
            .snapshot(&Theme::Light)
            .expect("inline Markdown editor");
    }

    #[test]
    fn clicking_rendered_text_places_the_source_cursor_and_requests_native_focus() {
        let state = block_editor_state("home".into());
        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(920.0, 620.0),
            block_editor(&state),
        );
        screen.point_at(Point::new(220.0, 115.0));
        screen.simulate(iced_test::simulator::click());
        let event = screen
            .into_messages()
            .find(|event| matches!(event, BlockEditorEvent::Select(_, _)))
            .expect("the rendered document owns hit testing");
        let state = apply(state, event);
        assert!(state.editor_active);
        assert!(block_editor_should_focus(state));
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
