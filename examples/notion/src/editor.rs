use iced::widget::{
    Column, Stack, button, column, container, keyed_column, mouse_area, pin, responsive, row,
    scrollable, text, text_input,
};
use iced::{Background, Border, Color, Element, Length, Shadow, Task, Theme, Vector, mouse};
use ui_lang_runtime::resize_handle;

const CARD_WIDTH: f32 = 292.0;
const BLOCK_DRAG_STEP: f32 = 34.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    HeadingOne,
    HeadingTwo,
    Todo,
    Bullet,
    Quote,
    Divider,
}

impl BlockKind {
    fn placeholder(self) -> &'static str {
        match self {
            Self::HeadingOne => "Heading 1",
            Self::HeadingTwo => "Heading 2",
            Self::Todo => "To-do",
            Self::Bullet => "List item",
            Self::Quote => "Quote",
            Self::Divider => "",
            Self::Paragraph => "Type '/' for commands",
        }
    }

    fn text_size(self) -> f32 {
        match self {
            Self::HeadingOne => 28.0,
            Self::HeadingTwo => 22.0,
            _ => 16.0,
        }
    }

    fn height(self) -> f32 {
        match self {
            Self::HeadingOne => 52.0,
            Self::HeadingTwo => 46.0,
            Self::Divider => 34.0,
            _ => 42.0,
        }
    }

    fn after_enter(self) -> Self {
        match self {
            Self::Todo | Self::Bullet => self,
            _ => Self::Paragraph,
        }
    }
}

#[derive(Debug, Clone)]
struct Block {
    id: u64,
    kind: BlockKind,
    text: String,
    checked: bool,
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
    block_id: u64,
    messages: Vec<CommentMessage>,
    resolved: bool,
    offset_x: f32,
    offset_y: f32,
}

#[derive(Debug, Clone)]
pub struct BlockEditorState {
    blocks: Vec<Block>,
    next_block_id: u64,
    threads: Vec<CommentThread>,
    next_thread_id: u64,
    hovered_block: Option<u64>,
    dragged_block: Option<u64>,
    block_drag_y: f32,
    focus_request: Option<u64>,
    composer_for: Option<u64>,
    comment_draft: String,
    replying_to: Option<u64>,
    reply_draft: String,
    comments_open: bool,
    floating_comments: bool,
    show_resolved: bool,
    scroll_y: f32,
}

#[derive(Debug, Clone)]
pub enum BlockEditorEvent {
    Edit(u64, String),
    Hover(Option<u64>),
    Add(BlockKind),
    AddAfter(u64, BlockKind),
    Delete(u64),
    SetKind(u64, BlockKind),
    ToggleTodo(u64),
    BlockDragStarted(u64),
    BlockDragged(u64, f64, f64),
    BlockDragEnded,
    OpenCommentComposer(u64),
    CloseCommentComposer,
    CommentDraftChanged(String),
    SubmitComment,
    Reply(u64),
    ReplyDraftChanged(String),
    SubmitReply(u64),
    Resolve(u64),
    Reopen(u64),
    CommentDragged(u64, f64, f64),
    ToggleComments,
    ToggleFloating,
    ToggleResolved,
    Scrolled(f32),
}

pub fn block_editor_state(template: String) -> BlockEditorState {
    let blocks = match template.as_str() {
        "roadmap" => vec![
            (BlockKind::HeadingOne, "Product roadmap"),
            (BlockKind::HeadingTwo, "Now"),
            (BlockKind::Todo, "Editor foundations"),
            (BlockKind::Todo, "Keyboard-first navigation"),
            (BlockKind::Todo, "Workspace search"),
            (BlockKind::HeadingTwo, "Next"),
            (BlockKind::Bullet, "Reusable templates"),
            (BlockKind::Bullet, "Offline drafts"),
            (BlockKind::Bullet, "Team permissions"),
        ],
        "launch" => vec![
            (BlockKind::HeadingOne, "Launch plan"),
            (BlockKind::HeadingTwo, "Goal"),
            (
                BlockKind::Paragraph,
                "Give every team one clear place to prepare for launch.",
            ),
            (BlockKind::Todo, "Finalize announcement"),
            (BlockKind::Todo, "Review onboarding"),
            (BlockKind::Todo, "Schedule customer emails"),
            (BlockKind::Todo, "Open the feedback channel"),
            (BlockKind::HeadingTwo, "Timeline"),
            (BlockKind::Paragraph, "Monday — internal preview"),
            (BlockKind::Paragraph, "Wednesday — early access"),
            (BlockKind::Paragraph, "Friday — public launch"),
        ],
        "meeting" => vec![
            (BlockKind::HeadingOne, "Weekly meeting"),
            (BlockKind::Paragraph, "July 27, 2026"),
            (BlockKind::HeadingTwo, "Agenda"),
            (BlockKind::Bullet, "Wins from last week"),
            (BlockKind::Bullet, "What is blocked"),
            (BlockKind::Bullet, "Decisions we need today"),
            (BlockKind::HeadingTwo, "Notes"),
            (BlockKind::Paragraph, ""),
        ],
        "untitled" => vec![(BlockKind::Paragraph, "")],
        _ => vec![
            (BlockKind::HeadingOne, "Welcome to your workspace."),
            (
                BlockKind::Paragraph,
                "This is a calm place to write, plan, and keep the details that matter.",
            ),
            (BlockKind::HeadingTwo, "A few things to try"),
            (BlockKind::Todo, "Rename this page"),
            (BlockKind::Todo, "Drag a block by its handle"),
            (BlockKind::Todo, "Add a comment to any block"),
            (
                BlockKind::Quote,
                "Your work stays connected to its context.",
            ),
        ],
    }
    .into_iter()
    .enumerate()
    .map(|(index, (kind, text))| Block {
        id: index as u64 + 1,
        kind,
        text: text.to_owned(),
        checked: false,
    })
    .collect::<Vec<_>>();

    let next_block_id = blocks.len() as u64 + 1;
    let threads = (template == "home")
        .then(|| CommentThread {
            id: 1,
            block_id: 2,
            messages: vec![CommentMessage {
                author: "Mina",
                body: "Can we add the customer research notes here?".into(),
                time: "18m",
            }],
            resolved: false,
            offset_x: 0.0,
            offset_y: 0.0,
        })
        .into_iter()
        .collect();

    BlockEditorState {
        blocks,
        next_block_id,
        threads,
        next_thread_id: 2,
        hovered_block: None,
        dragged_block: None,
        block_drag_y: 0.0,
        focus_request: None,
        composer_for: None,
        comment_draft: String::new(),
        replying_to: None,
        reply_draft: String::new(),
        comments_open: true,
        floating_comments: true,
        show_resolved: false,
        scroll_y: 0.0,
    }
}

pub fn block_editor_apply(state: BlockEditorState, event: BlockEditorEvent) -> BlockEditorState {
    let (state, focus) = reduce(state, event);
    BlockEditorState {
        focus_request: focus,
        ..state
    }
}

pub fn block_editor_pending_focus(state: BlockEditorState) -> i64 {
    state.focus_request.map_or(0, |id| id as i64)
}

pub fn block_editor_clear_focus(mut state: BlockEditorState) -> BlockEditorState {
    state.focus_request = None;
    state
}

pub fn block_editor_focus(block: i64) -> Task<bool> {
    let Ok(block) = u64::try_from(block) else {
        return Task::done(false);
    };
    iced::widget::operation::focus(block_widget_id(block)).chain(Task::done(true))
}

fn reduce(mut state: BlockEditorState, event: BlockEditorEvent) -> (BlockEditorState, Option<u64>) {
    let mut focus = None;
    match event {
        BlockEditorEvent::Edit(id, value) => {
            if let Some(block) = state.block_mut(id) {
                block.text = value;
            }
        }
        BlockEditorEvent::Hover(block) => state.hovered_block = block,
        BlockEditorEvent::Add(kind) => {
            let id = state.push_block(kind, "");
            focus = Some(id);
        }
        BlockEditorEvent::AddAfter(id, kind) => {
            let id = state.insert_after(id, kind, "");
            focus = Some(id);
        }
        BlockEditorEvent::Delete(id) => {
            if state.blocks.len() > 1
                && let Some(index) = state.index_of(id)
            {
                state.blocks.remove(index);
                state.threads.retain(|thread| thread.block_id != id);
                let next = index.saturating_sub(1).min(state.blocks.len() - 1);
                focus = Some(state.blocks[next].id);
            }
        }
        BlockEditorEvent::SetKind(id, kind) => {
            if let Some(block) = state.block_mut(id) {
                block.kind = kind;
                if block.text == "/" {
                    block.text.clear();
                }
            }
            focus = Some(id);
        }
        BlockEditorEvent::ToggleTodo(id) => {
            if let Some(block) = state.block_mut(id) {
                block.checked = !block.checked;
            }
        }
        BlockEditorEvent::BlockDragStarted(id) => {
            state.dragged_block = Some(id);
            state.block_drag_y = 0.0;
        }
        BlockEditorEvent::BlockDragged(id, _dx, dy) if state.dragged_block == Some(id) => {
            state.block_drag_y += dy as f32;
            while state.block_drag_y.abs() >= BLOCK_DRAG_STEP {
                let Some(index) = state.index_of(id) else {
                    break;
                };
                let target = if state.block_drag_y.is_sign_positive() {
                    (index + 1).min(state.blocks.len() - 1)
                } else {
                    index.saturating_sub(1)
                };
                if target == index {
                    state.block_drag_y = 0.0;
                    break;
                }
                state.blocks.swap(index, target);
                state.block_drag_y -= BLOCK_DRAG_STEP * state.block_drag_y.signum();
            }
        }
        BlockEditorEvent::BlockDragged(_, _, _) => {}
        BlockEditorEvent::BlockDragEnded => {
            state.dragged_block = None;
            state.block_drag_y = 0.0;
        }
        BlockEditorEvent::OpenCommentComposer(id) => {
            state.composer_for = Some(id);
            state.comment_draft.clear();
            state.comments_open = true;
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
                    offset_x: -16.0 * state.threads.len() as f32,
                    offset_y: 12.0 * state.threads.len() as f32,
                });
                state.next_thread_id += 1;
                state.comment_draft.clear();
                state.composer_for = None;
            }
        }
        BlockEditorEvent::Reply(id) => {
            state.replying_to = Some(id);
            state.reply_draft.clear();
        }
        BlockEditorEvent::ReplyDraftChanged(value) => state.reply_draft = value,
        BlockEditorEvent::SubmitReply(id) => {
            let reply = state.reply_draft.trim().to_owned();
            if !reply.is_empty()
                && let Some(thread) = state.thread_mut(id)
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
            if let Some(thread) = state.thread_mut(id) {
                thread.resolved = true;
            }
        }
        BlockEditorEvent::Reopen(id) => {
            if let Some(thread) = state.thread_mut(id) {
                thread.resolved = false;
            }
        }
        BlockEditorEvent::CommentDragged(id, dx, dy) => {
            if let Some(thread) = state.thread_mut(id) {
                thread.offset_x = (thread.offset_x + dx as f32).clamp(-900.0, 180.0);
                thread.offset_y = (thread.offset_y + dy as f32).clamp(-500.0, 900.0);
            }
        }
        BlockEditorEvent::ToggleComments => state.comments_open = !state.comments_open,
        BlockEditorEvent::ToggleFloating => {
            state.comments_open = true;
            state.floating_comments = !state.floating_comments;
        }
        BlockEditorEvent::ToggleResolved => state.show_resolved = !state.show_resolved,
        BlockEditorEvent::Scrolled(y) => state.scroll_y = y,
    }

    (state, focus)
}

impl BlockEditorState {
    fn index_of(&self, id: u64) -> Option<usize> {
        self.blocks.iter().position(|block| block.id == id)
    }

    fn block_mut(&mut self, id: u64) -> Option<&mut Block> {
        self.blocks.iter_mut().find(|block| block.id == id)
    }

    fn thread_mut(&mut self, id: u64) -> Option<&mut CommentThread> {
        self.threads.iter_mut().find(|thread| thread.id == id)
    }

    fn push_block(&mut self, kind: BlockKind, value: &str) -> u64 {
        let id = self.next_block_id;
        self.next_block_id += 1;
        self.blocks.push(Block {
            id,
            kind,
            text: value.into(),
            checked: false,
        });
        id
    }

    fn insert_after(&mut self, after: u64, kind: BlockKind, value: &str) -> u64 {
        let id = self.next_block_id;
        self.next_block_id += 1;
        let index = self
            .index_of(after)
            .map_or(self.blocks.len(), |index| index + 1);
        self.blocks.insert(
            index,
            Block {
                id,
                kind,
                text: value.into(),
                checked: false,
            },
        );
        id
    }

    fn block_y(&self, id: u64) -> f32 {
        52.0 + self
            .blocks
            .iter()
            .take_while(|block| block.id != id)
            .map(|block| block.kind.height())
            .sum::<f32>()
    }

    #[cfg(test)]
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    #[cfg(test)]
    pub fn block_text(&self, id: u64) -> Option<&str> {
        self.blocks
            .iter()
            .find(|block| block.id == id)
            .map(|block| block.text.as_str())
    }

    #[cfg(test)]
    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }
}

pub fn block_editor(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    responsive(move |size| block_editor_for_size(state, size.width, size.height)).into()
}

fn block_editor_for_size(
    state: &BlockEditorState,
    width: f32,
    height: f32,
) -> Element<'_, BlockEditorEvent> {
    let document = document(state);
    let docked = state.comments_open && !state.floating_comments;
    let base: Element<'_, _> = if docked {
        row![document, comments_panel(state)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        document
    };

    let mut layers = Stack::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(base);

    if state.comments_open && state.floating_comments {
        for (thread_index, thread) in state
            .threads
            .iter()
            .filter(|thread| !thread.resolved || state.show_resolved)
            .enumerate()
        {
            let x = (width - CARD_WIDTH - 14.0 + thread.offset_x)
                .clamp(8.0, (width - CARD_WIDTH).max(8.0));
            let anchor = state.block_y(thread.block_id) - state.scroll_y
                + thread.offset_y
                + thread_index as f32 * 8.0;
            let y = anchor.clamp(8.0, (height - 220.0).max(8.0));
            layers = layers.push(
                pin(thread_card(state, thread, true))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .x(x)
                    .y(y),
            );
        }
    }

    container(layers)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn document(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    let comments = state
        .threads
        .iter()
        .filter(|thread| !thread.resolved)
        .count();
    let words = state
        .blocks
        .iter()
        .map(|block| block.text.split_whitespace().count())
        .sum::<usize>();

    let toolbar = row![
        text("BLOCKS").size(10).color(MUTED),
        tool_button("Text", BlockEditorEvent::Add(BlockKind::Paragraph)),
        tool_button("H1", BlockEditorEvent::Add(BlockKind::HeadingOne)),
        tool_button("H2", BlockEditorEvent::Add(BlockKind::HeadingTwo)),
        tool_button("To-do", BlockEditorEvent::Add(BlockKind::Todo)),
        tool_button("List", BlockEditorEvent::Add(BlockKind::Bullet)),
        tool_button("Quote", BlockEditorEvent::Add(BlockKind::Quote)),
        tool_button("Divider", BlockEditorEvent::Add(BlockKind::Divider)),
        iced::widget::space().width(Length::Fill),
        tool_button(
            if state.comments_open {
                "Hide comments"
            } else {
                "Comments"
            },
            BlockEditorEvent::ToggleComments,
        ),
        tool_button(
            if state.floating_comments {
                "Float: on"
            } else {
                "Float: off"
            },
            BlockEditorEvent::ToggleFloating,
        ),
        tool_button(
            if state.show_resolved {
                "Hide resolved"
            } else {
                "Resolved"
            },
            BlockEditorEvent::ToggleResolved,
        ),
        text(format!("{comments} threads · {words} words"))
            .size(11)
            .color(MUTED),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);

    let blocks = keyed_column(
        state
            .blocks
            .iter()
            .map(|block| (block.id, block_view(state, block))),
    )
    .width(Length::Fill);

    let content = column![
        toolbar,
        blocks,
        text("Press Enter or use the toolbar to add blocks")
            .size(11)
            .color(MUTED)
    ]
    .spacing(4)
    .padding([8, 0])
    .max_width(920);

    scrollable(
        container(content)
            .width(Length::Fill)
            .center_x(Length::Fill),
    )
    .height(Length::Fill)
    .on_scroll(|viewport| BlockEditorEvent::Scrolled(viewport.absolute_offset().y))
    .into()
}

fn block_view<'a>(state: &'a BlockEditorState, block: &'a Block) -> Element<'a, BlockEditorEvent> {
    let id = block.id;
    let hovered = state.hovered_block == Some(id);
    let grip: Element<'a, _> = if hovered {
        resize_handle(
            container(
                text("⠿")
                    .size(16)
                    .color(if state.dragged_block == Some(id) {
                        PRIMARY
                    } else {
                        FAINT
                    }),
            )
            .center_x(24)
            .center_y(block.kind.height()),
        )
        .on_press(BlockEditorEvent::BlockDragStarted(id))
        .on_drag(move |dx, dy| BlockEditorEvent::BlockDragged(id, dx, dy))
        .on_release(BlockEditorEvent::BlockDragEnded)
        .interaction(mouse::Interaction::Grabbing)
        .into()
    } else {
        container(text(""))
            .center_x(24)
            .center_y(block.kind.height())
            .into()
    };

    let content: Element<'a, _> = if block.kind == BlockKind::Divider {
        container(
            container(text(""))
                .width(Length::Fill)
                .height(1)
                .style(|_| container::Style::default().background(BORDER)),
        )
        .width(Length::Fill)
        .center_y(block.kind.height())
        .into()
    } else {
        let prefix = match block.kind {
            BlockKind::Bullet => "•",
            BlockKind::Quote => "▎",
            _ => "",
        };
        let input = text_input(block.kind.placeholder(), &block.text)
            .id(block_widget_id(id))
            .on_input(move |value| BlockEditorEvent::Edit(id, value))
            .on_submit(BlockEditorEvent::AddAfter(id, block.kind.after_enter()))
            .padding([5, 6])
            .size(block.kind.text_size())
            .width(Length::Fill)
            .style(block_input_style);

        let mut content = row![].align_y(iced::Alignment::Center);
        if block.kind == BlockKind::Todo {
            content = content.push(
                button(text(if block.checked { "☑" } else { "☐" }).size(18))
                    .on_press(BlockEditorEvent::ToggleTodo(id))
                    .padding(3)
                    .style(button::text),
            );
        } else if !prefix.is_empty() {
            content = content.push(text(prefix).size(19).color(MUTED));
        }
        content.push(input).width(Length::Fill).into()
    };

    let actions: Element<'a, _> = if hovered {
        row![
            button(text("+").size(14))
                .on_press(BlockEditorEvent::AddAfter(id, BlockKind::Paragraph))
                .padding([3, 6])
                .style(button::text),
            button(text("Comment").size(10))
                .on_press(BlockEditorEvent::OpenCommentComposer(id))
                .padding([4, 6])
                .style(button::text),
            button(text("×").size(14))
                .on_press(BlockEditorEvent::Delete(id))
                .padding([3, 6])
                .style(button::text),
        ]
        .spacing(1)
        .align_y(iced::Alignment::Center)
        .into()
    } else {
        iced::widget::space().width(0).into()
    };

    let mut block_column = column![
        row![grip, content, actions]
            .align_y(iced::Alignment::Center)
            .width(Length::Fill)
            .height(block.kind.height())
    ]
    .width(Length::Fill);

    if block.text == "/" {
        block_column = block_column.push(kind_menu(id));
    }
    if state.composer_for == Some(id) {
        block_column = block_column.push(comment_composer(state));
    }

    mouse_area(
        container(block_column)
            .width(Length::Fill)
            .style(move |_| block_surface(state.dragged_block == Some(id))),
    )
    .on_enter(BlockEditorEvent::Hover(Some(id)))
    .on_exit(BlockEditorEvent::Hover(None))
    .into()
}

fn kind_menu(id: u64) -> Element<'static, BlockEditorEvent> {
    container(
        row![
            text("TURN INTO").size(10).color(MUTED),
            kind_button(id, "Text", BlockKind::Paragraph),
            kind_button(id, "H1", BlockKind::HeadingOne),
            kind_button(id, "H2", BlockKind::HeadingTwo),
            kind_button(id, "To-do", BlockKind::Todo),
            kind_button(id, "List", BlockKind::Bullet),
            kind_button(id, "Quote", BlockKind::Quote),
            kind_button(id, "Divider", BlockKind::Divider),
        ]
        .spacing(3)
        .align_y(iced::Alignment::Center),
    )
    .padding([3, 28])
    .into()
}

fn kind_button(
    id: u64,
    label: &'static str,
    kind: BlockKind,
) -> Element<'static, BlockEditorEvent> {
    tool_button(label, BlockEditorEvent::SetKind(id, kind)).into()
}

fn comment_composer(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    container(
        row![
            text_input("Write a comment…", &state.comment_draft)
                .on_input(BlockEditorEvent::CommentDraftChanged)
                .on_submit(BlockEditorEvent::SubmitComment)
                .padding(8)
                .width(Length::Fill),
            button("Comment")
                .on_press_maybe(
                    (!state.comment_draft.trim().is_empty())
                        .then_some(BlockEditorEvent::SubmitComment)
                )
                .style(button::primary),
            button("Cancel")
                .on_press(BlockEditorEvent::CloseCommentComposer)
                .style(button::text),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .padding([6, 28])
    .style(|_| soft_surface())
    .into()
}

fn comments_panel(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    let mut threads = Column::new().spacing(10);
    for thread in state
        .threads
        .iter()
        .filter(|thread| !thread.resolved || state.show_resolved)
    {
        threads = threads.push(thread_card(state, thread, false));
    }
    if state
        .threads
        .iter()
        .all(|thread| thread.resolved && !state.show_resolved)
    {
        threads = threads.push(text("No open comments").size(13).color(MUTED));
    }

    container(
        column![
            row![
                text("Comments").size(16),
                iced::widget::space().width(Length::Fill),
                tool_button(
                    if state.show_resolved {
                        "Hide resolved"
                    } else {
                        "Resolved"
                    },
                    BlockEditorEvent::ToggleResolved,
                ),
                tool_button("×", BlockEditorEvent::ToggleComments),
            ]
            .align_y(iced::Alignment::Center),
            scrollable(threads).height(Length::Fill),
        ]
        .spacing(12),
    )
    .padding(14)
    .width(312)
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
    floating: bool,
) -> Element<'a, BlockEditorEvent> {
    let id = thread.id;
    let header_content = container(
        row![
            text(if floating { "⠿" } else { "●" })
                .size(13)
                .color(PRIMARY),
            text(format!("Thread {}", thread.id)).size(12).color(MUTED),
            iced::widget::space().width(Length::Fill),
            text(format!(
                "{} replies",
                thread.messages.len().saturating_sub(1)
            ))
            .size(10)
            .color(FAINT),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([5, 2])
    .width(Length::Fill);
    let header: Element<'a, _> = if floating {
        resize_handle(header_content)
            .on_drag(move |dx, dy| BlockEditorEvent::CommentDragged(id, dx, dy))
            .interaction(mouse::Interaction::Grabbing)
            .into()
    } else {
        header_content.into()
    };

    let mut messages = Column::new().spacing(8);
    for message in &thread.messages {
        messages = messages.push(
            column![
                text(format!("{} · {}", message.author, message.time))
                    .size(10)
                    .color(MUTED),
                text(&message.body).size(13),
            ]
            .spacing(2),
        );
    }

    let mut card = column![header, messages].spacing(8);
    if state.replying_to == Some(id) {
        card = card.push(
            row![
                text_input("Reply…", &state.reply_draft)
                    .on_input(BlockEditorEvent::ReplyDraftChanged)
                    .on_submit(BlockEditorEvent::SubmitReply(id))
                    .padding(7)
                    .width(Length::Fill),
                button("Send")
                    .on_press_maybe(
                        (!state.reply_draft.trim().is_empty())
                            .then_some(BlockEditorEvent::SubmitReply(id))
                    )
                    .style(button::primary),
            ]
            .spacing(5),
        );
    }
    card = card.push(
        row![
            button("Reply")
                .on_press(BlockEditorEvent::Reply(id))
                .padding([4, 7])
                .style(button::text),
            if thread.resolved {
                button("Reopen")
                    .on_press(BlockEditorEvent::Reopen(id))
                    .padding([4, 7])
                    .style(button::text)
            } else {
                button("Resolve")
                    .on_press(BlockEditorEvent::Resolve(id))
                    .padding([4, 7])
                    .style(button::text)
            },
        ]
        .spacing(4),
    );

    container(card)
        .padding(12)
        .width(CARD_WIDTH)
        .style(move |_| comment_surface(thread.resolved, floating))
        .into()
}

fn tool_button(
    label: &'static str,
    event: BlockEditorEvent,
) -> iced::widget::Button<'static, BlockEditorEvent> {
    button(text(label).size(11))
        .on_press(event)
        .padding([4, 7])
        .style(button::text)
}

fn block_widget_id(id: u64) -> iced::widget::Id {
    format!("notion-block-{id}").into()
}

fn block_input_style(_: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: if focused { PRIMARY } else { Color::TRANSPARENT },
            width: if focused { 1.0 } else { 0.0 },
            radius: 4.0.into(),
        },
        icon: MUTED,
        placeholder: FAINT,
        value: FG,
        selection: Color::from_rgb8(194, 218, 247),
    }
}

fn block_surface(dragged: bool) -> container::Style {
    container::Style::default()
        .background(if dragged {
            BLUE_SOFT
        } else {
            Color::TRANSPARENT
        })
        .border(Border {
            color: if dragged { PRIMARY } else { Color::TRANSPARENT },
            width: if dragged { 1.0 } else { 0.0 },
            radius: 5.0.into(),
        })
}

fn soft_surface() -> container::Style {
    container::Style::default()
        .background(SIDEBAR)
        .border(Border {
            color: BORDER,
            width: 1.0,
            radius: 7.0.into(),
        })
}

fn comment_surface(resolved: bool, floating: bool) -> container::Style {
    container::Style {
        background: Some(Background::Color(if resolved {
            SIDEBAR
        } else {
            Color::WHITE
        })),
        border: Border {
            color: if resolved {
                BORDER
            } else {
                Color::from_rgb8(196, 218, 245)
            },
            width: 1.0,
            radius: 9.0.into(),
        },
        shadow: if floating {
            Shadow {
                color: Color::from_rgba8(0, 0, 0, 0.16),
                offset: Vector::new(0.0, 6.0),
                blur_radius: 18.0,
            }
        } else {
            Shadow::default()
        },
        ..container::Style::default()
    }
}

const FG: Color = Color::from_rgb8(55, 53, 47);
const MUTED: Color = Color::from_rgb8(120, 119, 116);
const FAINT: Color = Color::from_rgb8(155, 154, 151);
const BORDER: Color = Color::from_rgb8(233, 233, 231);
const SIDEBAR: Color = Color::from_rgb8(247, 247, 245);
const PRIMARY: Color = Color::from_rgb8(47, 128, 237);
const BLUE_SOFT: Color = Color::from_rgb8(234, 243, 251);

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(state: BlockEditorState, event: BlockEditorEvent) -> BlockEditorState {
        reduce(state, event).0
    }

    #[test]
    fn blocks_are_dynamic_and_reorder_by_drag() {
        let state = block_editor_state("untitled".into());
        let state = apply(state, BlockEditorEvent::Add(BlockKind::HeadingOne));
        let state = apply(state, BlockEditorEvent::Edit(2, "A real heading".into()));
        assert_eq!(state.block_count(), 2);
        assert_eq!(state.block_text(2), Some("A real heading"));

        let state = apply(state, BlockEditorEvent::BlockDragStarted(2));
        let state = apply(state, BlockEditorEvent::BlockDragged(2, 0.0, -40.0));
        assert_eq!(state.blocks[0].id, 2);

        let state = apply(state, BlockEditorEvent::Add(BlockKind::Paragraph));
        let state = apply(state, BlockEditorEvent::Add(BlockKind::Paragraph));
        let state = apply(state, BlockEditorEvent::BlockDragStarted(4));
        let state = apply(state, BlockEditorEvent::BlockDragged(4, 0.0, -120.0));
        assert_eq!(state.blocks[0].id, 4);
    }

    #[test]
    fn comments_support_threads_replies_and_resolution() {
        let mut state = block_editor_state("untitled".into());
        state = apply(state, BlockEditorEvent::OpenCommentComposer(1));
        state = apply(state, BlockEditorEvent::CommentDraftChanged("First".into()));
        state = apply(state, BlockEditorEvent::SubmitComment);
        assert_eq!(state.thread_count(), 1);

        let id = state.threads[0].id;
        state = apply(state, BlockEditorEvent::Reply(id));
        state = apply(state, BlockEditorEvent::ReplyDraftChanged("Reply".into()));
        state = apply(state, BlockEditorEvent::SubmitReply(id));
        assert_eq!(state.threads[0].messages.len(), 2);

        state = apply(state, BlockEditorEvent::Resolve(id));
        assert!(state.threads[0].resolved);
    }

    #[test]
    fn rendered_controls_emit_editor_and_comment_actions() {
        let state = block_editor_state("untitled".into());
        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1000.0, 620.0),
            block_editor(&state),
        );
        assert!(screen.find("⠿").is_err(), "block grip starts hidden");
        assert!(
            screen.find("Comment").is_err(),
            "block actions start hidden"
        );
        screen
            .click("Type '/' for commands")
            .expect("empty text block");
        let events = screen.into_messages().collect::<Vec<_>>();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, BlockEditorEvent::Hover(Some(1)))),
            "hovering the block reveals its actions"
        );

        let state = events.into_iter().fold(state, apply);
        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1000.0, 620.0),
            block_editor(&state),
        );
        screen.click("Text").expect("add text block button");
        assert!(
            screen
                .into_messages()
                .any(|message| matches!(message, BlockEditorEvent::Add(BlockKind::Paragraph)))
        );

        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1000.0, 620.0),
            block_editor(&state),
        );
        screen.click("Comment").expect("block comment button");
        assert!(
            screen
                .into_messages()
                .any(|message| matches!(message, BlockEditorEvent::OpenCommentComposer(1)))
        );
    }

    #[test]
    fn insertion_requests_focus_only_after_the_new_block_exists() {
        let state = block_editor_state("untitled".into());
        let state = block_editor_apply(state, BlockEditorEvent::AddAfter(1, BlockKind::Paragraph));
        assert_eq!(block_editor_pending_focus(state.clone()), 2);
        assert_eq!(
            block_editor_pending_focus(block_editor_clear_focus(state)),
            0
        );
    }
}
