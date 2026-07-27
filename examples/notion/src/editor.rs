use iced::widget::{
    Column, Stack, button, column, container, keyed_column, mouse_area, pin, responsive, row,
    scrollable, text, text_input,
};
use iced::{Background, Border, Color, Element, Length, Shadow, Task, Theme, Vector, mouse};
use ui_lang_runtime::{Role, StableId, accessible, resize_handle};

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
    page_hovered: bool,
    menu_for: Option<u64>,
    composer_for: Option<u64>,
    comment_draft: String,
    replying_to: Option<u64>,
    reply_draft: String,
    comments_open: bool,
    show_resolved: bool,
    scroll_y: f32,
}

#[derive(Debug, Clone)]
pub enum BlockEditorEvent {
    Edit(u64, String),
    Hover(Option<u64>),
    AddAfter(u64, BlockKind),
    Delete(u64),
    SetKind(u64, BlockKind),
    ToggleTodo(u64),
    MoveUp(u64),
    MoveDown(u64),
    BlockDragStarted(u64),
    BlockDragged(u64, f64, f64),
    BlockDragEnded,
    HoverPage(bool),
    ToggleBlockMenu(u64),
    OpenCommentComposer(u64),
    CloseCommentComposer,
    CommentDraftChanged(String),
    SubmitComment,
    ReplyDraftChanged(u64, String),
    SubmitReply(u64),
    JumpToBlock(u64),
    Resolve(u64),
    Reopen(u64),
    ToggleComments,
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
            (BlockKind::HeadingOne, "Build a calmer place to work."),
            (
                BlockKind::Paragraph,
                "Keep decisions, plans, and their context in one clear place.",
            ),
            (BlockKind::HeadingTwo, "Principles"),
            (
                BlockKind::Bullet,
                "Show the next useful action, not every possible action",
            ),
            (
                BlockKind::Bullet,
                "Keep decisions beside the work that informed them",
            ),
            (
                BlockKind::Bullet,
                "Make collaboration visible without interrupting writing",
            ),
            (BlockKind::HeadingTwo, "This quarter"),
            (
                BlockKind::Todo,
                "Validate the editor with five product teams",
            ),
            (BlockKind::Todo, "Connect customer research to the roadmap"),
            (BlockKind::Quote, "Clarity beats more surface area."),
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
                body: "Can we link the customer research notes here?".into(),
                time: "18m",
            }],
            resolved: false,
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
        page_hovered: false,
        menu_for: None,
        composer_for: None,
        comment_draft: String::new(),
        replying_to: None,
        reply_draft: String::new(),
        comments_open: false,
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

pub fn block_editor_toggle_comments(mut state: BlockEditorState) -> BlockEditorState {
    state.comments_open = !state.comments_open;
    state
}

pub fn block_editor_comments_open(state: BlockEditorState) -> bool {
    state.comments_open
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
                state.menu_for = None;
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
            state.menu_for = None;
            focus = Some(id);
        }
        BlockEditorEvent::ToggleTodo(id) => {
            if let Some(block) = state.block_mut(id) {
                block.checked = !block.checked;
            }
        }
        BlockEditorEvent::MoveUp(id) => {
            if let Some(index) = state.index_of(id)
                && index > 0
            {
                state.blocks.swap(index, index - 1);
                focus = Some(id);
            }
        }
        BlockEditorEvent::MoveDown(id) => {
            if let Some(index) = state.index_of(id)
                && index + 1 < state.blocks.len()
            {
                state.blocks.swap(index, index + 1);
                focus = Some(id);
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
        BlockEditorEvent::HoverPage(hovered) => state.page_hovered = hovered,
        BlockEditorEvent::ToggleBlockMenu(id) => {
            state.menu_for = (state.menu_for != Some(id)).then_some(id);
            state.composer_for = None;
        }
        BlockEditorEvent::OpenCommentComposer(id) => {
            state.composer_for = Some(id);
            state.comment_draft.clear();
            state.menu_for = None;
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
        BlockEditorEvent::JumpToBlock(id) => focus = Some(id),
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
        BlockEditorEvent::ToggleComments => state.comments_open = !state.comments_open,
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
        if id == 0 {
            return 8.0;
        }
        38.0 + self
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

pub fn block_editor(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    responsive(move |size| block_editor_for_size(state, size.width, size.height)).into()
}

fn block_editor_for_size(
    state: &BlockEditorState,
    width: f32,
    height: f32,
) -> Element<'_, BlockEditorEvent> {
    let compact_comments = width < 720.0;
    let open_inline_comments = state
        .threads
        .iter()
        .filter(|thread| thread.block_id != 0 && !thread.resolved)
        .count();
    let has_inline_comments = !state.comments_open && !compact_comments && open_inline_comments > 0;
    let document = document(state, has_inline_comments);
    let base: Element<'_, _> = if state.comments_open && !compact_comments {
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

    if !state.comments_open && !compact_comments {
        for (thread_index, thread) in state
            .threads
            .iter()
            .filter(|thread| thread.block_id != 0 && !thread.resolved)
            .enumerate()
        {
            let x = (width - CARD_WIDTH - 14.0).clamp(8.0, (width - CARD_WIDTH).max(8.0));
            let anchor =
                state.block_y(thread.block_id) - state.scroll_y + thread_index as f32 * 8.0;
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

    if !state.comments_open && compact_comments && open_inline_comments > 0 {
        let first = state
            .threads
            .iter()
            .find(|thread| thread.block_id != 0 && !thread.resolved)
            .expect("open inline comment count was positive");
        let y =
            (state.block_y(first.block_id) - state.scroll_y).clamp(8.0, (height - 40.0).max(8.0));
        let event = BlockEditorEvent::ToggleComments;
        layers = layers.push(
            pin(semantic_button(
                button(text(format!("☵ {open_inline_comments}")).size(11))
                    .on_press(event.clone())
                    .padding([5, 7])
                    .style(button::text),
                "notion-compact-comments",
                format!("Open {open_inline_comments} comments"),
                Some(event),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .x((width - 48.0).max(8.0))
            .y(y),
        );
    }

    if state.comments_open && compact_comments {
        layers = layers.push(
            pin(comments_panel(state))
                .width(Length::Fill)
                .height(Length::Fill)
                .x((width - 340.0).max(0.0))
                .y(0.0),
        );
    }

    container(layers)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn document(
    state: &BlockEditorState,
    reserve_comment_margin: bool,
) -> Element<'_, BlockEditorEvent> {
    let blocks = keyed_column(
        state
            .blocks
            .iter()
            .map(|block| (block.id, block_view(state, block))),
    )
    .width(Length::Fill);

    let page_comment: Element<'_, _> = if let Some(thread) = state
        .threads
        .iter()
        .find(|thread| thread.block_id == 0 && !thread.resolved)
    {
        thread_card(state, thread, false)
    } else if state.composer_for == Some(0) {
        comment_composer(state)
    } else if state.page_hovered {
        let event = BlockEditorEvent::OpenCommentComposer(0);
        semantic_button(
            button(text("Add comment").size(12).color(MUTED))
                .on_press(event.clone())
                .padding([4, 6])
                .style(button::text),
            "notion-page-comment",
            "Add page comment",
            Some(event),
        )
    } else {
        iced::widget::space().height(26).into()
    };

    let page_comment = mouse_area(container(page_comment).width(Length::Fill).height(34))
        .on_enter(BlockEditorEvent::HoverPage(true))
        .on_exit(BlockEditorEvent::HoverPage(false));

    let content = column![page_comment, blocks].spacing(2).max_width(920);

    let content: Element<'_, _> = if reserve_comment_margin {
        row![
            container(content).width(Length::Fill),
            iced::widget::space().width(CARD_WIDTH + 20.0),
        ]
        .width(Length::Fill)
        .into()
    } else {
        container(content)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .into()
    };

    scrollable(content)
        .height(Length::Fill)
        .on_scroll(|viewport| BlockEditorEvent::Scrolled(viewport.absolute_offset().y))
        .into()
}

fn block_view<'a>(state: &'a BlockEditorState, block: &'a Block) -> Element<'a, BlockEditorEvent> {
    let id = block.id;
    let hovered = state.hovered_block == Some(id);
    let grip = resize_handle(
        container(text(if hovered { "⠿" } else { "" }).size(16).color(
            if state.dragged_block == Some(id) {
                PRIMARY
            } else {
                FAINT
            },
        ))
        .center_x(24)
        .center_y(block.kind.height()),
    )
    .on_press(BlockEditorEvent::BlockDragStarted(id))
    .on_drag(move |dx, dy| BlockEditorEvent::BlockDragged(id, dx, dy))
    .on_release(BlockEditorEvent::BlockDragEnded)
    .interaction(mouse::Interaction::Grabbing);
    let grip: Element<'a, _> = accessible(
        grip,
        StableId::new(format!("notion-block-{id}-actions")),
        Role::Button,
    )
    .label("Block actions")
    .on_activate(BlockEditorEvent::ToggleBlockMenu(id))
    .into();

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
        let focus_id = block_widget_id(id);
        let input = text_input(block.kind.placeholder(), &block.text)
            .id(focus_id.clone())
            .on_input(move |value| BlockEditorEvent::Edit(id, value))
            .on_submit(BlockEditorEvent::AddAfter(id, block.kind.after_enter()))
            .padding([5, 6])
            .size(block.kind.text_size())
            .width(Length::Fill)
            .style(block_input_style);
        let input = semantic_text_input(
            input,
            format!("notion-block-{id}-input"),
            "Block text",
            block.text.clone(),
            focus_id,
        );

        let mut content = row![].align_y(iced::Alignment::Center);
        if block.kind == BlockKind::Todo {
            let event = BlockEditorEvent::ToggleTodo(id);
            content = content.push(semantic_button(
                button(text(if block.checked { "☑" } else { "☐" }).size(18))
                    .on_press(event.clone())
                    .padding(3)
                    .style(button::text),
                format!("notion-block-{id}-todo"),
                if block.checked {
                    "Mark to-do incomplete"
                } else {
                    "Mark to-do complete"
                },
                Some(event),
            ));
        } else if !prefix.is_empty() {
            content = content.push(text(prefix).size(19).color(MUTED));
        }
        content.push(input).width(Length::Fill).into()
    };

    let actions: Element<'a, _> = if hovered {
        row![
            tool_button_with_label(
                format!("notion-block-{id}-add"),
                "+",
                "Add block after",
                BlockEditorEvent::AddAfter(id, BlockKind::Paragraph),
            ),
            tool_button_with_label(
                format!("notion-block-{id}-comment"),
                "☵",
                "Add block comment",
                BlockEditorEvent::OpenCommentComposer(id),
            ),
            tool_button_with_label(
                format!("notion-block-{id}-menu"),
                "•••",
                "Open block actions",
                BlockEditorEvent::ToggleBlockMenu(id),
            ),
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
    if state.menu_for == Some(id) {
        block_column = block_column.push(block_menu(id));
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
        column![
            text("BASIC BLOCKS").size(10).color(MUTED),
            row![
                kind_button(id, "Text", BlockKind::Paragraph),
                kind_button(id, "Heading 1", BlockKind::HeadingOne),
                kind_button(id, "Heading 2", BlockKind::HeadingTwo),
                kind_button(id, "To-do", BlockKind::Todo),
            ]
            .spacing(3),
            row![
                kind_button(id, "Bulleted list", BlockKind::Bullet),
                kind_button(id, "Quote", BlockKind::Quote),
                kind_button(id, "Divider", BlockKind::Divider),
            ]
            .spacing(3),
        ]
        .spacing(4),
    )
    .padding([8, 28])
    .style(|_| comment_surface(false, false))
    .into()
}

fn block_menu(id: u64) -> Element<'static, BlockEditorEvent> {
    container(
        column![
            text("BLOCK ACTIONS").size(10).color(MUTED),
            row![
                kind_button(id, "Text", BlockKind::Paragraph),
                kind_button(id, "Heading 1", BlockKind::HeadingOne),
                kind_button(id, "Heading 2", BlockKind::HeadingTwo),
                kind_button(id, "To-do", BlockKind::Todo),
                kind_button(id, "List", BlockKind::Bullet),
                kind_button(id, "Quote", BlockKind::Quote),
            ]
            .spacing(3),
            row![
                tool_button(
                    format!("notion-block-{id}-action-comment"),
                    "Comment",
                    BlockEditorEvent::OpenCommentComposer(id),
                ),
                tool_button(
                    format!("notion-block-{id}-action-move-up"),
                    "Move up",
                    BlockEditorEvent::MoveUp(id),
                ),
                tool_button(
                    format!("notion-block-{id}-action-move-down"),
                    "Move down",
                    BlockEditorEvent::MoveDown(id),
                ),
                tool_button(
                    format!("notion-block-{id}-action-delete"),
                    "Delete",
                    BlockEditorEvent::Delete(id),
                ),
            ]
            .spacing(3),
        ]
        .spacing(4),
    )
    .padding([8, 28])
    .style(|_| comment_surface(false, false))
    .into()
}

fn kind_button(
    id: u64,
    label: &'static str,
    kind: BlockKind,
) -> Element<'static, BlockEditorEvent> {
    tool_button(
        format!("notion-block-{id}-kind-{label}"),
        label,
        BlockEditorEvent::SetKind(id, kind),
    )
}

fn comment_composer(state: &BlockEditorState) -> Element<'_, BlockEditorEvent> {
    let target = state.composer_for.unwrap_or(0);
    let input_key = format!("notion-comment-{target}-input");
    let input_focus: iced::widget::Id = input_key.clone().into();
    let input = text_input("Write a comment…", &state.comment_draft)
        .id(input_focus.clone())
        .on_input(BlockEditorEvent::CommentDraftChanged)
        .on_submit(BlockEditorEvent::SubmitComment)
        .padding(8)
        .width(Length::Fill);
    let submit =
        (!state.comment_draft.trim().is_empty()).then_some(BlockEditorEvent::SubmitComment);
    let submit_control = button("Comment")
        .on_press_maybe(submit.clone())
        .style(button::primary);
    let cancel = BlockEditorEvent::CloseCommentComposer;

    container(
        row![
            semantic_text_input(
                input,
                input_key,
                "Comment text",
                state.comment_draft.clone(),
                input_focus,
            ),
            semantic_button(
                submit_control,
                format!("notion-comment-{target}-submit"),
                "Submit comment",
                submit,
            ),
            semantic_button(
                button("Cancel")
                    .on_press(cancel.clone())
                    .style(button::text),
                format!("notion-comment-{target}-cancel"),
                "Cancel comment",
                Some(cancel),
            ),
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
        .rev()
        .filter(|thread| thread.resolved == state.show_resolved)
    {
        threads = threads.push(thread_card(state, thread, false));
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
                text("Comments").size(16),
                iced::widget::space().width(Length::Fill),
                tool_button(
                    "notion-comments-filter",
                    if state.show_resolved {
                        "Resolved ▾"
                    } else {
                        "Open ▾"
                    },
                    BlockEditorEvent::ToggleResolved,
                ),
                tool_button_with_label(
                    "notion-comments-close",
                    "×",
                    "Close comments",
                    BlockEditorEvent::ToggleComments,
                ),
            ]
            .align_y(iced::Alignment::Center),
            scrollable(threads).height(Length::Fill),
        ]
        .spacing(12),
    )
    .padding(14)
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
    inline: bool,
) -> Element<'a, BlockEditorEvent> {
    let id = thread.id;
    let context = if thread.block_id == 0 {
        "Page discussion"
    } else {
        state
            .blocks
            .iter()
            .find(|block| block.id == thread.block_id)
            .map_or("Deleted block", |block| block.text.as_str())
    };

    let context: Element<'a, _> = if thread.block_id == 0 || inline {
        text(context)
            .size(11)
            .color(MUTED)
            .width(Length::Fill)
            .into()
    } else {
        let event = BlockEditorEvent::JumpToBlock(thread.block_id);
        semantic_button(
            button(text(format!("↳ {context}")).size(11).color(MUTED))
                .on_press(event.clone())
                .padding(0)
                .width(Length::Fill)
                .style(button::text),
            format!("notion-thread-{id}-context"),
            "Jump to commented block",
            Some(event),
        )
    };

    let header = row![
        context,
        if thread.resolved {
            tool_button(
                format!("notion-thread-{id}-reopen"),
                "Reopen",
                BlockEditorEvent::Reopen(id),
            )
        } else {
            tool_button_with_label(
                format!("notion-thread-{id}-resolve"),
                if inline { "✓" } else { "Resolve" },
                "Resolve comment",
                BlockEditorEvent::Resolve(id),
            )
        },
    ]
    .align_y(iced::Alignment::Center);

    let mut messages = Column::new().spacing(8);
    for message in &thread.messages {
        messages = messages.push(
            row![
                container(text(message.author.chars().next().unwrap_or('?').to_string()).size(11))
                    .center_x(24)
                    .center_y(24)
                    .style(|_| soft_surface()),
                column![
                    text(format!("{} · {}", message.author, message.time))
                        .size(11)
                        .color(MUTED),
                    text(&message.body).size(13),
                ]
                .spacing(2),
            ]
            .spacing(8),
        );
    }

    let active_reply = state.replying_to == Some(id);
    let reply = if active_reply {
        state.reply_draft.as_str()
    } else {
        ""
    };
    let reply_key = format!("notion-thread-{id}-reply");
    let reply_focus: iced::widget::Id = reply_key.clone().into();
    let reply_input = text_input("Reply…", reply)
        .id(reply_focus.clone())
        .on_input(move |value| BlockEditorEvent::ReplyDraftChanged(id, value))
        .on_submit(BlockEditorEvent::SubmitReply(id))
        .padding(7)
        .width(Length::Fill);
    let submit_reply = (active_reply && !state.reply_draft.trim().is_empty())
        .then_some(BlockEditorEvent::SubmitReply(id));
    let card = column![
        header,
        messages,
        row![
            semantic_text_input(
                reply_input,
                reply_key,
                "Reply to comment",
                reply,
                reply_focus,
            ),
            semantic_button(
                button("↑").on_press_maybe(submit_reply.clone()).style(
                    move |theme: &Theme, status| {
                        let mut style = button::text(theme, status);
                        style.text_color = if active_reply && !state.reply_draft.trim().is_empty() {
                            PRIMARY
                        } else {
                            FAINT
                        };
                        style
                    }
                ),
                format!("notion-thread-{id}-reply-submit"),
                "Send reply",
                submit_reply,
            ),
        ]
        .spacing(5),
    ]
    .spacing(8);

    container(card)
        .padding(12)
        .width(if inline {
            Length::Fixed(CARD_WIDTH)
        } else {
            Length::Fill
        })
        .style(move |_| comment_surface(thread.resolved, inline))
        .into()
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

fn semantic_text_input<'a>(
    input: iced::widget::TextInput<'a, BlockEditorEvent>,
    key: impl Into<String>,
    label: impl Into<String>,
    value: impl Into<String>,
    focus_id: iced::widget::Id,
) -> Element<'a, BlockEditorEvent> {
    let key = key.into();
    accessible(input, StableId::new(&key), Role::TextInput)
        .logical_id(key)
        .focus_id(focus_id)
        .label(label)
        .value(value)
        .into()
}

fn tool_button(
    key: impl Into<String>,
    label: impl Into<String>,
    event: BlockEditorEvent,
) -> Element<'static, BlockEditorEvent> {
    let label = label.into();
    tool_button_with_label(key, label.clone(), label, event)
}

fn tool_button_with_label(
    key: impl Into<String>,
    visible: impl Into<String>,
    label: impl Into<String>,
    event: BlockEditorEvent,
) -> Element<'static, BlockEditorEvent> {
    semantic_button(
        button(text(visible.into()).size(11))
            .on_press(event.clone())
            .padding([4, 7])
            .style(button::text),
        key,
        label,
        Some(event),
    )
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
            color: BORDER,
            width: 1.0,
            radius: 9.0.into(),
        },
        shadow: if floating {
            Shadow {
                color: Color::from_rgba8(0, 0, 0, 0.08),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 10.0,
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

    fn accessibility_snapshot(
        state: &BlockEditorState,
        size: iced::Size,
    ) -> ui_lang_runtime::Snapshot<BlockEditorEvent> {
        use iced::advanced::renderer::Headless;
        use iced_test::futures::futures::StreamExt;

        let mut renderer = iced_test::futures::futures::executor::block_on(
            <iced::Renderer as Headless>::new(iced::Font::DEFAULT, iced::Pixels(16.0), None),
        )
        .expect("headless renderer");
        let mut ui = iced_test::runtime::UserInterface::build(
            block_editor(state),
            size,
            iced_test::runtime::user_interface::Cache::default(),
            &mut renderer,
        );
        let task = ui_lang_runtime::snapshot::<BlockEditorEvent>("Notion editor");
        let mut stream = iced_test::runtime::task::into_stream(task).expect("snapshot task");
        let action = iced_test::futures::futures::executor::block_on(stream.next())
            .expect("widget operation");
        let iced_test::runtime::Action::Widget(mut operation) = action else {
            panic!("snapshot task must begin with a widget operation");
        };
        ui.operate(&renderer, operation.as_mut());
        let _ = operation.finish();
        let output = iced_test::futures::futures::executor::block_on(stream.next())
            .expect("snapshot output");
        let iced_test::runtime::Action::Output(snapshot) = output else {
            panic!("snapshot operation must produce a tree");
        };
        snapshot
    }

    #[test]
    fn blocks_are_dynamic_and_reorder_by_drag() {
        let state = block_editor_state("untitled".into());
        let state = apply(state, BlockEditorEvent::AddAfter(1, BlockKind::HeadingOne));
        let state = apply(state, BlockEditorEvent::Edit(2, "A real heading".into()));
        assert_eq!(state.block_count(), 2);
        assert_eq!(state.block_text(2), Some("A real heading"));

        let state = apply(state, BlockEditorEvent::MoveUp(2));
        assert_eq!(state.blocks[0].id, 2);
        let state = apply(state, BlockEditorEvent::MoveDown(2));
        assert_eq!(state.blocks[1].id, 2);

        let state = apply(state, BlockEditorEvent::BlockDragStarted(2));
        let state = apply(state, BlockEditorEvent::BlockDragged(2, 0.0, -40.0));
        assert_eq!(state.blocks[0].id, 2);

        let state = apply(state, BlockEditorEvent::AddAfter(1, BlockKind::Paragraph));
        let state = apply(state, BlockEditorEvent::AddAfter(3, BlockKind::Paragraph));
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
        state = apply(
            state,
            BlockEditorEvent::ReplyDraftChanged(id, "Reply".into()),
        );
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
        screen.click("+").expect("add text block button");
        assert!(
            screen.into_messages().any(|message| matches!(
                message,
                BlockEditorEvent::AddAfter(1, BlockKind::Paragraph)
            ))
        );

        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1000.0, 620.0),
            block_editor(&state),
        );
        screen.click("☵").expect("block comment button");
        assert!(
            screen
                .into_messages()
                .any(|message| matches!(message, BlockEditorEvent::OpenCommentComposer(1)))
        );
    }

    #[test]
    fn editor_controls_expose_semantic_names() {
        let state = block_editor_state("home".into());
        let snapshot = accessibility_snapshot(&state, iced::Size::new(640.0, 620.0));
        let mut nodes = snapshot.update.nodes.iter().map(|(_, node)| node);

        assert!(
            nodes.clone().any(|node| {
                node.role() == Role::TextInput && node.label() == Some("Block text")
            })
        );
        assert!(
            nodes.clone().any(|node| {
                node.role() == Role::Button && node.label() == Some("Block actions")
            })
        );
        assert!(nodes.any(|node| {
            node.role() == Role::Button && node.label() == Some("Open 1 comments")
        }));
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

    #[test]
    fn compact_editor_uses_a_minimal_comment_indicator() {
        let state = block_editor_state("home".into());
        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(640.0, 620.0),
            block_editor(&state),
        );
        assert!(
            screen
                .find("Can we link the customer research notes here?")
                .is_err(),
            "compact mode keeps the thread out of the writing surface"
        );
        if let Ok(path) = std::env::var("NOTION_COMPACT_SNAPSHOT") {
            let snapshot = screen.snapshot(&Theme::Light).expect("render snapshot");
            assert!(snapshot.matches_image(path).expect("write snapshot"));
        }
        screen.click("☵ 1").expect("minimal comment indicator");
        let state = screen.into_messages().fold(state, apply);

        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(640.0, 620.0),
            block_editor(&state),
        );
        screen.find("Comments").expect("compact comments pane");
        screen
            .find("Can we link the customer research notes here?")
            .expect("thread in compact comments pane");
    }
}
