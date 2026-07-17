use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use super::button::{ButtonSize, ButtonVariant, button};
use super::focus_control::FocusControl;
use super::theme::Theme;
use iced::advanced::widget;
use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::{self, key::Named};
use iced::widget::scrollable::{self, AutoScroll, Rail, Scrollable, Scroller};
use iced::widget::{
    Column, Id, Space, Stack, container, operation as widget_operation,
    scrollable as iced_scrollable,
};
use iced::{Background, Border, Element, Length, Rectangle, Shadow, Task, Vector};

pub const DEFAULT_SCROLL_EDGE_THRESHOLD: f32 = 8.0;
pub const DEFAULT_SCROLL_PREVIOUS_ITEM_PEEK: f32 = 64.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MessageScrollerInitialPosition {
    Start,
    #[default]
    End,
    LastAnchor,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MessageScrollerAlignment {
    #[default]
    Start,
    Center,
    End,
    Nearest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageScrollerCommand {
    Start,
    End,
    LineUp,
    LineDown,
    PageUp,
    PageDown,
    Item {
        id: String,
        alignment: MessageScrollerAlignment,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageScrollerItemMeta {
    id: String,
    scroll_anchor: bool,
}

impl MessageScrollerItemMeta {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            scroll_anchor: false,
        }
    }

    #[must_use]
    pub const fn scroll_anchor(mut self, scroll_anchor: bool) -> Self {
        self.scroll_anchor = scroll_anchor;
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn is_scroll_anchor(&self) -> bool {
        self.scroll_anchor
    }
}

pub struct MessageScrollerItem<'a, Message> {
    meta: MessageScrollerItemMeta,
    content: Element<'a, Message>,
}

pub fn message_scroller_item<'a, Message>(
    id: impl Into<String>,
    content: impl Into<Element<'a, Message>>,
) -> MessageScrollerItem<'a, Message> {
    MessageScrollerItem {
        meta: MessageScrollerItemMeta::new(id),
        content: content.into(),
    }
}

impl<'a, Message> MessageScrollerItem<'a, Message> {
    #[must_use]
    pub fn scroll_anchor(mut self, scroll_anchor: bool) -> Self {
        self.meta.scroll_anchor = scroll_anchor;
        self
    }

    pub fn metadata(&self) -> &MessageScrollerItemMeta {
        &self.meta
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MessageScrollerViewport {
    pub offset_from_end: f32,
    pub viewport_height: f32,
    pub content_height: f32,
}

impl From<scrollable::Viewport> for MessageScrollerViewport {
    fn from(viewport: scrollable::Viewport) -> Self {
        Self {
            offset_from_end: viewport.absolute_offset().y,
            viewport_height: viewport.bounds().height,
            content_height: viewport.content_bounds().height,
        }
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct MessageScrollerMeasurement {
    revision: u64,
    viewport: Rectangle,
    content: Rectangle,
    translation: Vector,
    rows: Vec<MeasuredRow>,
}

#[derive(Debug, Clone, PartialEq)]
struct MeasuredRow {
    meta: MessageScrollerItemMeta,
    bounds: Rectangle,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageScrollerEvent {
    ItemsChanged(Vec<MessageScrollerItemMeta>),
    ViewportChanged {
        viewport: MessageScrollerViewport,
        items: Arc<[MessageScrollerItemMeta]>,
    },
    UserScrollIntent,
    Command(MessageScrollerCommand),
    #[doc(hidden)]
    Measured(MessageScrollerMeasurement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScrollMode {
    Following,
    Anchored(String),
    Free,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ProgrammaticScroll {
    PreserveStart,
    PreserveEnd,
    PreserveOffset(f32),
    RearmOffset(f32),
}

impl ProgrammaticScroll {
    fn is_settled(self, viewport: MessageScrollerViewport) -> bool {
        match self {
            Self::PreserveStart => {
                let max_offset = (viewport.content_height - viewport.viewport_height).max(0.0);
                !different(viewport.offset_from_end, max_offset)
            }
            Self::PreserveEnd => !different(viewport.offset_from_end, 0.0),
            Self::PreserveOffset(expected) | Self::RearmOffset(expected) => {
                let max_offset = (viewport.content_height - viewport.viewport_height).max(0.0);
                !different(viewport.offset_from_end, expected.clamp(0.0, max_offset))
            }
        }
    }

    const fn rearms_following(self) -> bool {
        matches!(self, Self::RearmOffset(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingTarget {
    id: String,
    alignment: MessageScrollerAlignment,
    peek: bool,
    anchor_after_scroll: bool,
}

/// Caller-owned transcript scroll behavior.
///
/// Route emitted [`MessageScrollerEvent`] values through [`Self::update`]. Seed
/// the state with [`MessageScrollerEvent::ItemsChanged`] before the first view,
/// then send it whenever row content or metadata changes. This explicit path is
/// required because iced does not emit `Scrollable::on_scroll` for every layout
/// change, especially while the transcript still fits in the viewport.
#[derive(Debug, Clone)]
pub struct MessageScrollerState {
    key: String,
    auto_scroll: bool,
    initial_position: MessageScrollerInitialPosition,
    edge_threshold: f32,
    scroll_margin: f32,
    previous_item_peek: f32,
    layout_revision: u64,
    initialized: bool,
    default_position_applied: bool,
    mode: ScrollMode,
    items: Vec<MessageScrollerItemMeta>,
    handled_anchor_ids: HashSet<String>,
    measurement: Option<MessageScrollerMeasurement>,
    viewport: Option<MessageScrollerViewport>,
    visible_message_ids: Vec<String>,
    current_anchor_id: Option<String>,
    can_scroll_start: bool,
    can_scroll_end: bool,
    unread_count: usize,
    pending_unread_count: usize,
    spacer_height: f32,
    pending_command: Option<MessageScrollerCommand>,
    pending_target: Option<PendingTarget>,
    preserve_visible_row: bool,
    skip_preservation_once: bool,
    programmatic_scroll: Option<ProgrammaticScroll>,
}

impl MessageScrollerState {
    pub fn new(key: impl ToString) -> Self {
        Self {
            key: key.to_string(),
            auto_scroll: false,
            initial_position: MessageScrollerInitialPosition::End,
            edge_threshold: DEFAULT_SCROLL_EDGE_THRESHOLD,
            scroll_margin: 0.0,
            previous_item_peek: DEFAULT_SCROLL_PREVIOUS_ITEM_PEEK,
            layout_revision: 0,
            initialized: false,
            default_position_applied: false,
            mode: ScrollMode::Free,
            items: Vec::new(),
            handled_anchor_ids: HashSet::new(),
            measurement: None,
            viewport: None,
            visible_message_ids: Vec::new(),
            current_anchor_id: None,
            can_scroll_start: false,
            can_scroll_end: false,
            unread_count: 0,
            pending_unread_count: 0,
            spacer_height: 0.0,
            pending_command: None,
            pending_target: None,
            preserve_visible_row: false,
            skip_preservation_once: false,
            programmatic_scroll: None,
        }
    }

    #[must_use]
    pub fn auto_scroll(mut self, auto_scroll: bool) -> Self {
        self.set_auto_scroll(auto_scroll);
        self
    }

    pub fn set_auto_scroll(&mut self, auto_scroll: bool) {
        self.auto_scroll = auto_scroll;
        if auto_scroll
            && !self.can_scroll_end
            && self.pending_target.is_none()
            && !matches!(self.mode, ScrollMode::Anchored(_))
        {
            self.mode = ScrollMode::Following;
            self.pending_unread_count = 0;
            self.unread_count = 0;
        } else if !auto_scroll && matches!(self.mode, ScrollMode::Following) {
            self.mode = ScrollMode::Free;
        }
    }

    #[must_use]
    pub const fn initial_position(
        mut self,
        initial_position: MessageScrollerInitialPosition,
    ) -> Self {
        self.initial_position = initial_position;
        self
    }

    #[must_use]
    pub fn scroll_edge_threshold(mut self, threshold: f32) -> Self {
        self.edge_threshold = finite_nonnegative(threshold, DEFAULT_SCROLL_EDGE_THRESHOLD);
        self
    }

    #[must_use]
    pub fn scroll_margin(mut self, margin: f32) -> Self {
        self.scroll_margin = finite_nonnegative(margin, 0.0);
        self
    }

    #[must_use]
    pub fn scroll_previous_item_peek(mut self, peek: f32) -> Self {
        self.previous_item_peek = finite_nonnegative(peek, DEFAULT_SCROLL_PREVIOUS_ITEM_PEEK);
        self
    }

    pub const fn can_scroll_start(&self) -> bool {
        self.can_scroll_start
    }

    pub const fn can_scroll_end(&self) -> bool {
        self.can_scroll_end
    }

    pub const fn unread_count(&self) -> usize {
        self.unread_count
    }

    pub fn visible_message_ids(&self) -> &[String] {
        &self.visible_message_ids
    }

    pub fn current_anchor_id(&self) -> Option<&str> {
        self.current_anchor_id.as_deref()
    }

    pub fn items(&self) -> &[MessageScrollerItemMeta] {
        &self.items
    }

    pub const fn is_following(&self) -> bool {
        matches!(self.mode, ScrollMode::Following)
    }

    /// Applies an emitted event and returns the smallest native iced operation
    /// needed to settle the viewport.
    pub fn update(&mut self, event: MessageScrollerEvent) -> Task<MessageScrollerEvent> {
        match event {
            MessageScrollerEvent::ItemsChanged(items) => {
                self.layout_revision = self.layout_revision.wrapping_add(1);
                self.sync_items(items);
                self.measure_task()
            }
            MessageScrollerEvent::ViewportChanged { viewport, items } => {
                if self.items.as_slice() != items.as_ref() {
                    return Task::none();
                }
                if self.viewport.is_some_and(|previous| {
                    different(previous.viewport_height, viewport.viewport_height)
                        || different(previous.content_height, viewport.content_height)
                }) {
                    self.layout_revision = self.layout_revision.wrapping_add(1);
                }
                let spacer_height = self.spacer_height;
                let had_pending_target = self.pending_target.is_some();
                self.apply_viewport(viewport);
                let viewport_changed_layout = different(spacer_height, self.spacer_height)
                    || had_pending_target != self.pending_target.is_some();
                if !viewport_changed_layout && self.update_cached_scroll(viewport) {
                    Task::none()
                } else {
                    self.measure_task()
                }
            }
            MessageScrollerEvent::UserScrollIntent => self.apply_user_scroll_intent(),
            MessageScrollerEvent::Command(command) => self.apply_command(command),
            MessageScrollerEvent::Measured(measurement) => {
                if measurement.revision == self.layout_revision {
                    self.apply_measurement(measurement)
                } else {
                    Task::none()
                }
            }
        }
    }

    fn sync_items(&mut self, items: Vec<MessageScrollerItemMeta>) {
        let current_ids = items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<HashSet<_>>();
        self.handled_anchor_ids
            .retain(|id| current_ids.contains(id.as_str()));
        drop(current_ids);
        let previous_anchor_ids = self
            .items
            .iter()
            .filter(|item| item.scroll_anchor)
            .map(|item| item.id.as_str())
            .collect::<HashSet<_>>();
        let retained = retained_range(&self.items, &items);
        let did_prepend = retained.is_some_and(|(start, _)| start > 0);
        let appended = retained.map(|(_, end)| &items[end..]).unwrap_or_default();
        let appended_anchor_count = appended.iter().filter(|item| item.scroll_anchor).count();
        let explicit_target_pending = self
            .pending_target
            .as_ref()
            .is_some_and(|target| !target.anchor_after_scroll);
        let changed_existing_anchor = retained
            .map(|(start, end)| &items[start..end])
            .or_else(|| (items.len() == self.items.len()).then_some(items.as_slice()))
            .and_then(|retained| {
                retained.iter().find(|item| {
                    item.scroll_anchor
                        && !self.handled_anchor_ids.contains(item.id.as_str())
                        && !previous_anchor_ids.contains(item.id.as_str())
                })
            })
            .cloned();
        let new_anchor = if self.items.is_empty() || did_prepend || explicit_target_pending {
            None
        } else if changed_existing_anchor.is_some() {
            changed_existing_anchor
        } else if !appended.is_empty() {
            (!(self.auto_scroll
                && matches!(self.mode, ScrollMode::Following)
                && appended_anchor_count > 1))
                .then(|| appended.iter().find(|item| item.scroll_anchor))
                .flatten()
                .cloned()
        } else {
            None
        };
        drop(previous_anchor_ids);

        if self.initialized && !self.items.is_empty() && !appended.is_empty() {
            if new_anchor.is_some()
                || (!matches!(self.mode, ScrollMode::Following) && self.can_scroll_end)
            {
                self.unread_count = self.unread_count.saturating_add(appended.len());
            } else if new_anchor.is_none() && matches!(self.mode, ScrollMode::Free) {
                self.pending_unread_count =
                    self.pending_unread_count.saturating_add(appended.len());
            }
        }

        self.preserve_visible_row = self.initialized
            && new_anchor.is_none()
            && !explicit_target_pending
            && matches!(self.mode, ScrollMode::Free);
        self.items = items;

        if let Some(anchor) = new_anchor {
            self.handled_anchor_ids.insert(anchor.id.clone());
            self.pending_target = Some(PendingTarget {
                id: anchor.id,
                alignment: MessageScrollerAlignment::Start,
                peek: true,
                anchor_after_scroll: true,
            });
            self.preserve_visible_row = false;
        }
    }

    fn apply_viewport(&mut self, viewport: MessageScrollerViewport) {
        let previous = self.viewport.replace(viewport);
        let max_offset = (viewport.content_height - viewport.viewport_height).max(0.0);
        self.can_scroll_end = viewport.offset_from_end > self.edge_threshold;
        self.can_scroll_start = max_offset - viewport.offset_from_end > self.edge_threshold;

        if !self.can_scroll_end
            && self.pending_target.is_none()
            && !matches!(self.mode, ScrollMode::Anchored(_))
        {
            self.unread_count = 0;
        }

        if self.settle_programmatic_scroll(viewport) {
            return;
        }

        let Some(previous) = previous else {
            return;
        };
        let layout_changed = different(previous.content_height, viewport.content_height)
            || different(previous.viewport_height, viewport.viewport_height);
        let offset_changed = different(previous.offset_from_end, viewport.offset_from_end);

        if offset_changed && !layout_changed {
            self.skip_preservation_once = true;
            self.programmatic_scroll = None;
            self.preserve_visible_row = false;
            self.pending_target = None;
            if self.spacer_height > 0.0 {
                self.set_spacer_height(0.0);
            }
            self.mode = if self.auto_scroll && !self.can_scroll_end {
                ScrollMode::Following
            } else {
                ScrollMode::Free
            };
        } else if layout_changed
            && self.programmatic_scroll.is_none()
            && !self.skip_preservation_once
            && matches!(self.mode, ScrollMode::Free)
        {
            self.preserve_visible_row = true;
        }
    }

    fn settle_programmatic_scroll(&mut self, viewport: MessageScrollerViewport) -> bool {
        let Some(programmatic) = self
            .programmatic_scroll
            .filter(|programmatic| programmatic.is_settled(viewport))
        else {
            return false;
        };

        self.programmatic_scroll = None;
        if programmatic.rearms_following()
            && self.auto_scroll
            && !self.can_scroll_end
            && matches!(self.mode, ScrollMode::Free)
        {
            self.mode = ScrollMode::Following;
        }
        true
    }

    fn apply_user_scroll_intent(&mut self) -> Task<MessageScrollerEvent> {
        let removed_spacer = self.spacer_height > 0.0;
        self.programmatic_scroll = None;
        self.pending_command = None;
        self.pending_target = None;
        self.preserve_visible_row = false;
        self.skip_preservation_once = false;
        self.set_spacer_height(0.0);
        self.mode = ScrollMode::Free;

        if removed_spacer {
            self.measure_task()
        } else {
            Task::none()
        }
    }

    fn apply_command(&mut self, command: MessageScrollerCommand) -> Task<MessageScrollerEvent> {
        if let MessageScrollerCommand::Item { id, .. } = &command
            && !self.items.is_empty()
            && !self.items.iter().any(|item| item.id == *id)
        {
            return Task::none();
        }

        self.default_position_applied = true;

        if !self.initialized {
            self.programmatic_scroll = None;
            match command {
                MessageScrollerCommand::Item { id, alignment } => {
                    self.pending_command = None;
                    self.mode = ScrollMode::Free;
                    self.preserve_visible_row = false;
                    self.skip_preservation_once = false;
                    self.pending_target = Some(PendingTarget {
                        id,
                        alignment,
                        peek: false,
                        anchor_after_scroll: false,
                    });
                }
                command => {
                    self.pending_target = None;
                    self.pending_command = Some(command);
                }
            }
            return self.measure_task();
        }

        self.pending_command = None;
        if !matches!(&command, MessageScrollerCommand::Item { .. }) {
            self.preserve_visible_row = false;
            self.skip_preservation_once = false;
        }

        match command {
            MessageScrollerCommand::Start => {
                let removed_spacer = self.spacer_height > 0.0;
                self.pending_target = None;
                self.set_spacer_height(0.0);
                self.mode = ScrollMode::Free;
                self.programmatic_scroll = self
                    .can_scroll_start
                    .then_some(ProgrammaticScroll::PreserveStart);
                self.remeasure_after_spacer_removal(
                    widget_operation::snap_to(self.viewport_id(), scrollable::RelativeOffset::END),
                    removed_spacer,
                )
            }
            MessageScrollerCommand::End => {
                let removed_spacer = self.spacer_height > 0.0;
                self.pending_target = None;
                self.set_spacer_height(0.0);
                self.mode = if self.auto_scroll {
                    ScrollMode::Following
                } else {
                    ScrollMode::Free
                };
                self.unread_count = 0;
                self.pending_unread_count = 0;
                self.programmatic_scroll = self
                    .can_scroll_end
                    .then_some(ProgrammaticScroll::PreserveEnd);
                self.remeasure_after_spacer_removal(
                    widget_operation::snap_to(
                        self.viewport_id(),
                        scrollable::RelativeOffset::START,
                    ),
                    removed_spacer,
                )
            }
            MessageScrollerCommand::LineUp
            | MessageScrollerCommand::LineDown
            | MessageScrollerCommand::PageUp
            | MessageScrollerCommand::PageDown => {
                let viewport_height = self
                    .measurement
                    .as_ref()
                    .map(|measurement| measurement.viewport.height)
                    .or_else(|| self.viewport.map(|viewport| viewport.viewport_height))
                    .unwrap_or(120.0);
                let page = (viewport_height - 60.0).max(60.0);
                let delta = match command {
                    MessageScrollerCommand::LineUp => 60.0,
                    MessageScrollerCommand::LineDown => -60.0,
                    MessageScrollerCommand::PageUp => page,
                    MessageScrollerCommand::PageDown => -page,
                    _ => unreachable!(),
                };
                let removed_spacer = self.spacer_height > 0.0;
                self.pending_target = None;
                self.set_spacer_height(0.0);
                self.mode = ScrollMode::Free;
                let can_scroll = match command {
                    MessageScrollerCommand::LineUp | MessageScrollerCommand::PageUp => {
                        self.can_scroll_start
                    }
                    MessageScrollerCommand::LineDown | MessageScrollerCommand::PageDown => {
                        self.can_scroll_end
                    }
                    _ => unreachable!(),
                };
                let moves_toward_end = matches!(
                    command,
                    MessageScrollerCommand::LineDown | MessageScrollerCommand::PageDown
                );
                let (offset, max_offset) = self.current_scroll_offsets();
                let expected = (offset + delta).clamp(0.0, max_offset);
                self.programmatic_scroll = can_scroll.then_some(if moves_toward_end {
                    ProgrammaticScroll::RearmOffset(expected)
                } else {
                    ProgrammaticScroll::PreserveOffset(expected)
                });
                if moves_toward_end && !can_scroll && self.auto_scroll && !self.can_scroll_end {
                    self.mode = ScrollMode::Following;
                }
                self.remeasure_after_spacer_removal(
                    scroll_by_task(self.viewport_id(), delta),
                    removed_spacer,
                )
            }
            MessageScrollerCommand::Item { id, alignment } => {
                self.mode = ScrollMode::Free;
                self.preserve_visible_row = false;
                self.skip_preservation_once = false;
                self.pending_target = Some(PendingTarget {
                    id,
                    alignment,
                    peek: false,
                    anchor_after_scroll: false,
                });
                self.programmatic_scroll = None;
                self.measure_task()
            }
        }
    }

    fn apply_measurement(
        &mut self,
        measurement: MessageScrollerMeasurement,
    ) -> Task<MessageScrollerEvent> {
        let old = self.measurement.take();
        self.update_derived_state(&measurement);
        let max_offset = (measurement.content.height - measurement.viewport.height).max(0.0);
        self.settle_programmatic_scroll(MessageScrollerViewport {
            offset_from_end: (max_offset - measurement.translation.y).clamp(0.0, max_offset),
            viewport_height: measurement.viewport.height,
            content_height: measurement.content.height,
        });
        self.publish_pending_unread();

        if !self.initialized && measurement.rows.is_empty() {
            self.measurement = Some(measurement);
            return Task::none();
        }

        if !self.initialized {
            self.initialized = true;
            if let Some(command) = self.pending_command.take() {
                self.measurement = Some(measurement);
                return self.apply_command(command);
            }

            if !self.default_position_applied {
                self.default_position_applied = true;
                self.pending_target = match self.initial_position {
                    MessageScrollerInitialPosition::Start => {
                        self.measurement = Some(measurement);
                        return self.apply_command(MessageScrollerCommand::Start);
                    }
                    MessageScrollerInitialPosition::End => {
                        self.measurement = Some(measurement);
                        return self.apply_command(MessageScrollerCommand::End);
                    }
                    MessageScrollerInitialPosition::LastAnchor => measurement
                        .rows
                        .iter()
                        .rev()
                        .find(|row| row.meta.scroll_anchor)
                        .filter(|row| {
                            measurement.content.y + measurement.content.height - row.bounds.y
                                > measurement.viewport.height
                        })
                        .map(|row| PendingTarget {
                            id: row.meta.id.clone(),
                            alignment: MessageScrollerAlignment::Start,
                            peek: true,
                            anchor_after_scroll: true,
                        }),
                };

                if self.pending_target.is_none() {
                    self.measurement = Some(measurement);
                    return self.apply_command(MessageScrollerCommand::End);
                }
            }
        }

        if let Some(target) = self.pending_target.clone() {
            let Some(row) = measurement.rows.iter().find(|row| row.meta.id == target.id) else {
                if !self.items.is_empty() && !self.items.iter().any(|item| item.id == target.id) {
                    self.pending_target = None;
                }
                self.measurement = Some(measurement);
                return Task::none();
            };

            self.preserve_visible_row = false;
            self.skip_preservation_once = false;
            if !target.anchor_after_scroll && self.spacer_height > 0.0 {
                self.set_spacer_height(0.0);
                self.measurement = None;
                return self.measure_task();
            }

            let margin = self.scroll_margin
                + if target.peek {
                    self.previous_item_peek
                } else {
                    0.0
                };
            let alignment = resolved_alignment(&measurement, row, target.alignment, margin);
            if target.alignment == MessageScrollerAlignment::Nearest
                && alignment != MessageScrollerAlignment::Nearest
                && let Some(target) = &mut self.pending_target
            {
                target.alignment = alignment;
            }
            let required =
                required_bottom_spacer(&measurement, row, self.spacer_height, alignment, margin);
            if different(required, self.spacer_height) {
                self.set_spacer_height(required);
                self.measurement = Some(measurement);
                return self.measure_task();
            }

            let delta = alignment_delta_from_bottom(&measurement, row, alignment, margin);
            self.pending_target = None;
            self.mode = if target.anchor_after_scroll {
                ScrollMode::Anchored(target.id)
            } else {
                ScrollMode::Free
            };
            self.programmatic_scroll = expected_offset_after_delta(&measurement, delta)
                .map(ProgrammaticScroll::PreserveOffset);
            self.measurement = Some(measurement);
            return scroll_by_task(self.viewport_id(), delta);
        }

        if let ScrollMode::Anchored(id) = self.mode.clone() {
            if let Some(row) = measurement.rows.iter().find(|row| row.meta.id == id) {
                let previous_spacer = self.spacer_height;
                let required = required_bottom_spacer(
                    &measurement,
                    row,
                    previous_spacer,
                    MessageScrollerAlignment::Start,
                    self.scroll_margin + self.previous_item_peek,
                );

                if self.auto_scroll && previous_spacer > 0.5 && required <= 0.5 {
                    self.measurement = Some(measurement);
                    return self.apply_command(MessageScrollerCommand::End);
                }

                if different(required, previous_spacer) {
                    self.set_spacer_height(required);
                    self.measurement = Some(measurement);
                    return self.measure_task();
                }

                let delta = alignment_delta_from_bottom(
                    &measurement,
                    row,
                    MessageScrollerAlignment::Start,
                    self.scroll_margin + self.previous_item_peek,
                );
                if let Some(expected) = expected_offset_after_delta(&measurement, delta) {
                    self.programmatic_scroll = Some(ProgrammaticScroll::PreserveOffset(expected));
                    self.measurement = Some(measurement);
                    return scroll_by_task(self.viewport_id(), delta);
                }
            } else {
                let removed_spacer = self.spacer_height > 0.0;
                self.mode = ScrollMode::Free;
                self.set_spacer_height(0.0);
                if removed_spacer {
                    self.measurement = None;
                    return self.measure_task();
                }
            }
        }

        if matches!(self.mode, ScrollMode::Free)
            && self.preserve_visible_row
            && !self.skip_preservation_once
            && let Some(delta) = old
                .as_ref()
                .and_then(|old| preservation_delta(old, &measurement))
            && let Some(expected) = expected_offset_after_delta(&measurement, delta)
        {
            self.preserve_visible_row = false;
            self.programmatic_scroll = Some(ProgrammaticScroll::PreserveOffset(expected));
            self.measurement = Some(measurement);
            return scroll_by_task(self.viewport_id(), delta);
        }

        self.preserve_visible_row = false;
        self.skip_preservation_once = false;
        if !self.can_scroll_end {
            self.pending_unread_count = 0;
        }
        self.measurement = Some(measurement);
        Task::none()
    }

    fn update_derived_state(&mut self, measurement: &MessageScrollerMeasurement) {
        let max_scroll = (measurement.content.height - measurement.viewport.height).max(0.0);
        self.can_scroll_start = measurement.translation.y > self.edge_threshold;
        self.can_scroll_end = max_scroll - measurement.translation.y > self.edge_threshold;
        if !self.can_scroll_end
            && self.pending_target.is_none()
            && !matches!(self.mode, ScrollMode::Anchored(_))
        {
            self.unread_count = 0;
        }

        let reading_line = measurement.viewport.y + self.scroll_margin + self.previous_item_peek;
        let visible_top = reading_line.min(measurement.viewport.y + measurement.viewport.height);
        let visible_viewport = Rectangle {
            y: visible_top,
            height: (measurement.viewport.y + measurement.viewport.height - visible_top).max(0.0),
            ..measurement.viewport
        };

        self.visible_message_ids = measurement
            .rows
            .iter()
            .filter(|row| {
                visible_viewport.height > 0.0
                    && intersects(
                        screen_bounds(row.bounds, measurement.translation),
                        visible_viewport,
                    )
            })
            .map(|row| row.meta.id.clone())
            .collect();

        self.current_anchor_id = measurement
            .rows
            .iter()
            .filter(|row| row.meta.scroll_anchor)
            // Row bounds and scroll translation can round on opposite sides of
            // the same physical pixel at the reading line.
            .take_while(|row| {
                screen_bounds(row.bounds, measurement.translation).y <= reading_line + 1.0
            })
            .last()
            .map(|row| row.meta.id.clone());
    }

    fn update_cached_scroll(&mut self, viewport: MessageScrollerViewport) -> bool {
        let Some(mut measurement) = self.measurement.take() else {
            return false;
        };
        if different(measurement.viewport.height, viewport.viewport_height)
            || different(measurement.content.height, viewport.content_height)
        {
            self.measurement = Some(measurement);
            return false;
        }

        measurement.translation.y = (viewport.content_height - viewport.viewport_height).max(0.0)
            - viewport.offset_from_end;
        self.update_derived_state(&measurement);
        self.publish_pending_unread();
        if !self.can_scroll_end {
            self.pending_unread_count = 0;
        }
        self.preserve_visible_row = false;
        self.skip_preservation_once = false;
        self.measurement = Some(measurement);
        true
    }

    fn publish_pending_unread(&mut self) {
        if self.can_scroll_end {
            self.unread_count = self
                .unread_count
                .saturating_add(std::mem::take(&mut self.pending_unread_count));
        }
    }

    fn current_scroll_offsets(&self) -> (f32, f32) {
        if let Some(viewport) = self.viewport {
            let max_offset = (viewport.content_height - viewport.viewport_height).max(0.0);
            return (viewport.offset_from_end.clamp(0.0, max_offset), max_offset);
        }

        self.measurement
            .as_ref()
            .map(|measurement| {
                let max_offset =
                    (measurement.content.height - measurement.viewport.height).max(0.0);
                (
                    (max_offset - measurement.translation.y).clamp(0.0, max_offset),
                    max_offset,
                )
            })
            .unwrap_or((0.0, 0.0))
    }

    fn set_spacer_height(&mut self, height: f32) {
        if self.spacer_height != height {
            self.spacer_height = height;
            self.layout_revision = self.layout_revision.wrapping_add(1);
        }
    }

    fn viewport_id(&self) -> Id {
        viewport_id(&self.key)
    }

    fn measure_task(&self) -> Task<MessageScrollerEvent> {
        widget::operate(MeasureMessageScroller::new(
            &self.key,
            &self.items,
            self.layout_revision,
        ))
    }

    fn remeasure_after_spacer_removal(
        &mut self,
        scroll: Task<MessageScrollerEvent>,
        removed_spacer: bool,
    ) -> Task<MessageScrollerEvent> {
        if removed_spacer {
            self.measurement = None;
            Task::batch([scroll, self.measure_task()])
        } else {
            scroll
        }
    }
}

pub struct MessageScrollerView<'a, Message> {
    content: Element<'a, Message>,
    width: Length,
    height: Length,
}

impl<'a, Message> MessageScrollerView<'a, Message> {
    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl<'a, Message: 'a> From<MessageScrollerView<'a, Message>> for Element<'a, Message> {
    fn from(view: MessageScrollerView<'a, Message>) -> Self {
        container(view.content)
            .width(view.width)
            .height(view.height)
            .into()
    }
}

/// A controlled, row-aware transcript viewport with an end control.
///
/// The native scrollable remains responsible for wheel, touch, and scrollbar
/// input. Stable item IDs let the state restore the reader's row, target a
/// message, and derive visibility without a custom renderer.
pub fn controlled_message_scroller<'a, Message>(
    state: &MessageScrollerState,
    items: impl IntoIterator<Item = MessageScrollerItem<'a, Message>>,
    on_event: impl Fn(MessageScrollerEvent) -> Message + 'a,
    theme: &Theme,
) -> MessageScrollerView<'a, Message>
where
    Message: Clone + 'a,
{
    let on_event = Rc::new(on_event);
    let mut metadata = Vec::new();
    let mut rows = Vec::new();

    for item in items {
        let id = item_widget_id(&state.key, &item.meta.id);
        metadata.push(item.meta);
        rows.push(container(item.content).id(id).width(Length::Fill).into());
    }
    rows.push(Space::new().height(state.spacer_height).into());

    let content = container(
        Column::with_children(rows)
            .spacing(theme.spacing.lg)
            .width(Length::Fill),
    )
    .padding([theme.spacing.lg, theme.spacing.md])
    .width(Length::Fill);
    let theme_copy = *theme;
    let scroll_handler = Rc::clone(&on_event);
    let items_for_scroll: Arc<[MessageScrollerItemMeta]> = metadata.into();
    let viewport: Element<'a, Message> = iced_scrollable(content)
        .id(state.viewport_id())
        .width(Length::Fill)
        .height(Length::Fill)
        .anchor_bottom()
        .on_scroll(move |viewport| {
            scroll_handler(MessageScrollerEvent::ViewportChanged {
                viewport: viewport.into(),
                items: items_for_scroll.clone(),
            })
        })
        .style(move |_iced_theme, status| style(&theme_copy, status))
        .into();
    let keyboard_handler = Rc::clone(&on_event);
    let viewport: Element<'a, Message> =
        FocusControl::passive(focus_id(&state.key), viewport, theme)
            .on_scroll_intent(on_event(MessageScrollerEvent::UserScrollIntent))
            .repeat_key_presses(true)
            .on_key_press(move |key, modifiers| {
                let command = keyboard_command(&key, modifiers)?;
                Some(keyboard_handler(MessageScrollerEvent::Command(command)))
            })
            .into();

    let mut layers = vec![viewport];
    if state.can_scroll_end || state.unread_count > 0 {
        let label = if state.unread_count > 0 {
            format!("Jump to latest ({})", state.unread_count)
        } else {
            "Jump to latest".to_owned()
        };
        let end_control: Element<'a, Message> = button(label, theme)
            .variant(ButtonVariant::Secondary)
            .size(ButtonSize::Small)
            .on_press(on_event(MessageScrollerEvent::Command(
                MessageScrollerCommand::End,
            )))
            .into();
        layers.push(
            container(end_control)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Bottom)
                .padding(theme.spacing.md)
                .into(),
        );
    }

    MessageScrollerView {
        content: Stack::with_children(layers).into(),
        width: Length::Fill,
        height: Length::Fill,
    }
}

struct MeasureMessageScroller {
    revision: u64,
    viewport: Id,
    items: HashMap<Id, MessageScrollerItemMeta>,
    measurement: Option<MessageScrollerMeasurement>,
    rows: Vec<MeasuredRow>,
    skip_children: bool,
}

impl MeasureMessageScroller {
    fn new(key: &str, items: &[MessageScrollerItemMeta], revision: u64) -> Self {
        Self {
            revision,
            viewport: viewport_id(key),
            items: items
                .iter()
                .cloned()
                .map(|item| (item_widget_id(key, &item.id), item))
                .collect(),
            measurement: None,
            rows: Vec::new(),
            skip_children: false,
        }
    }
}

impl widget::Operation<MessageScrollerEvent> for MeasureMessageScroller {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn widget::Operation<MessageScrollerEvent>),
    ) {
        if !std::mem::take(&mut self.skip_children) {
            operate(self);
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn widget::operation::Scrollable,
    ) {
        if id == Some(&self.viewport) {
            self.measurement = Some(MessageScrollerMeasurement {
                revision: self.revision,
                viewport: bounds,
                content: content_bounds,
                translation,
                rows: Vec::new(),
            });
        }
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if let Some(meta) = id.and_then(|id| self.items.get(id)) {
            self.rows.push(MeasuredRow {
                meta: meta.clone(),
                bounds,
            });
            self.skip_children = true;
        }
    }

    fn finish(&self) -> widget::operation::Outcome<MessageScrollerEvent> {
        let Some(mut measurement) = self.measurement.clone() else {
            return widget::operation::Outcome::None;
        };
        measurement.rows.clone_from(&self.rows);
        widget::operation::Outcome::Some(MessageScrollerEvent::Measured(measurement))
    }
}

fn viewport_id(key: &str) -> Id {
    Id::from(format!(
        "ducktape-message-scroller:{}:{key}:viewport",
        key.len()
    ))
}

fn focus_id(key: &str) -> Id {
    Id::from(format!(
        "ducktape-message-scroller:{}:{key}:focus",
        key.len()
    ))
}

fn item_widget_id(key: &str, item: &str) -> Id {
    Id::from(format!(
        "ducktape-message-scroller:{}:{key}:item:{}:{item}",
        key.len(),
        item.len()
    ))
}

fn scroll_by_task(id: Id, delta: f32) -> Task<MessageScrollerEvent> {
    if delta.is_finite() && delta.abs() > 0.5 {
        widget_operation::scroll_by(id, scrollable::AbsoluteOffset { x: 0.0, y: delta })
    } else {
        Task::none()
    }
}

fn finite_nonnegative(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        fallback
    }
}

fn keyboard_command(
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
) -> Option<MessageScrollerCommand> {
    match key {
        keyboard::Key::Named(Named::ArrowUp) => Some(MessageScrollerCommand::LineUp),
        keyboard::Key::Named(Named::ArrowDown) => Some(MessageScrollerCommand::LineDown),
        keyboard::Key::Named(Named::PageUp) => Some(MessageScrollerCommand::PageUp),
        keyboard::Key::Named(Named::PageDown) => Some(MessageScrollerCommand::PageDown),
        keyboard::Key::Named(Named::Home) => Some(MessageScrollerCommand::Start),
        keyboard::Key::Named(Named::End) => Some(MessageScrollerCommand::End),
        keyboard::Key::Named(Named::Space) if modifiers.shift() => {
            Some(MessageScrollerCommand::PageUp)
        }
        keyboard::Key::Named(Named::Space) => Some(MessageScrollerCommand::PageDown),
        _ => None,
    }
}

fn different(a: f32, b: f32) -> bool {
    (a - b).abs() > 0.5
}

fn retained_range(
    previous: &[MessageScrollerItemMeta],
    next: &[MessageScrollerItemMeta],
) -> Option<(usize, usize)> {
    if previous.is_empty() {
        return Some((0, 0));
    }
    next.iter()
        .position(|item| item.id == previous[0].id)
        .filter(|start| {
            next.get(*start..*start + previous.len())
                .is_some_and(|retained| retained.iter().zip(previous).all(|(a, b)| a.id == b.id))
        })
        .map(|start| (start, start + previous.len()))
}

fn screen_bounds(bounds: Rectangle, translation: Vector) -> Rectangle {
    Rectangle {
        x: bounds.x - translation.x,
        y: bounds.y - translation.y,
        ..bounds
    }
}

fn intersects(a: Rectangle, b: Rectangle) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

fn first_visible_row(measurement: &MessageScrollerMeasurement) -> Option<(&str, f32)> {
    measurement.rows.iter().find_map(|row| {
        let bounds = screen_bounds(row.bounds, measurement.translation);
        intersects(bounds, measurement.viewport).then_some((row.meta.id.as_str(), bounds.y))
    })
}

fn preservation_delta(
    previous: &MessageScrollerMeasurement,
    next: &MessageScrollerMeasurement,
) -> Option<f32> {
    let (id, previous_y) = first_visible_row(previous)?;
    let next_y = next
        .rows
        .iter()
        .find(|row| row.meta.id == id)
        .map(|row| screen_bounds(row.bounds, next.translation).y)?;
    let delta = previous_y - next_y;
    (delta.abs() > 0.5).then_some(delta)
}

fn expected_offset_after_delta(
    measurement: &MessageScrollerMeasurement,
    delta: f32,
) -> Option<f32> {
    let max_offset = (measurement.content.height - measurement.viewport.height).max(0.0);
    let offset = (max_offset - measurement.translation.y).clamp(0.0, max_offset);
    let expected = (offset + delta).clamp(0.0, max_offset);
    different(offset, expected).then_some(expected)
}

fn resolved_alignment(
    measurement: &MessageScrollerMeasurement,
    row: &MeasuredRow,
    alignment: MessageScrollerAlignment,
    margin: f32,
) -> MessageScrollerAlignment {
    if alignment != MessageScrollerAlignment::Nearest {
        return alignment;
    }

    let row = screen_bounds(row.bounds, measurement.translation);
    let viewport = measurement.viewport;
    if row.y >= viewport.y + margin && row.y + row.height <= viewport.y + viewport.height - margin {
        MessageScrollerAlignment::Nearest
    } else if row.y < viewport.y + margin {
        MessageScrollerAlignment::Start
    } else {
        MessageScrollerAlignment::End
    }
}

fn required_bottom_spacer(
    measurement: &MessageScrollerMeasurement,
    row: &MeasuredRow,
    current_spacer: f32,
    alignment: MessageScrollerAlignment,
    margin: f32,
) -> f32 {
    let content_bottom = measurement.content.y + measurement.content.height - current_spacer;
    let row_bottom = row.bounds.y + row.bounds.height;
    let content_after_row = (content_bottom - row_bottom).max(0.0);
    let remaining_viewport = (measurement.viewport.height - row.bounds.height).max(0.0);
    let required_after_row = match resolved_alignment(measurement, row, alignment, margin) {
        MessageScrollerAlignment::Start => (remaining_viewport - margin).max(0.0),
        MessageScrollerAlignment::Center => (remaining_viewport / 2.0 - margin).max(0.0),
        MessageScrollerAlignment::End => margin,
        MessageScrollerAlignment::Nearest => 0.0,
    };
    (required_after_row - content_after_row).max(0.0).ceil()
}

fn alignment_delta_from_bottom(
    measurement: &MessageScrollerMeasurement,
    row: &MeasuredRow,
    alignment: MessageScrollerAlignment,
    margin: f32,
) -> f32 {
    let alignment = resolved_alignment(measurement, row, alignment, margin);
    let row = screen_bounds(row.bounds, measurement.translation);
    let viewport = measurement.viewport;
    let desired = match alignment {
        MessageScrollerAlignment::Start => viewport.y + margin,
        MessageScrollerAlignment::Center => {
            viewport.y + (viewport.height - row.height) / 2.0 + margin
        }
        MessageScrollerAlignment::End => viewport.y + viewport.height - row.height - margin,
        MessageScrollerAlignment::Nearest => row.y,
    };

    // Bottom anchoring stores distance from the end, so increasing the native
    // offset moves content down instead of up.
    desired - row.y
}

/// A bottom-anchored native transcript viewport.
///
/// The returned `Scrollable` keeps its native `.on_scroll(...)` builder so the
/// caller owns follow-output and unread behavior instead of hidden widget state.
pub fn message_scroller<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    id: impl Into<Id>,
    theme: &Theme,
) -> Scrollable<'a, Message>
where
    Message: 'a,
{
    let theme = *theme;
    iced_scrollable(
        container(content)
            .padding([theme.spacing.lg, theme.spacing.md])
            .width(Length::Fill),
    )
    .id(id)
    .width(Length::Fill)
    .height(Length::Fill)
    .anchor_bottom()
    .style(move |_iced_theme, status| style(&theme, status))
}

pub fn style(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let active = matches!(
        status,
        scrollable::Status::Hovered {
            is_horizontal_scrollbar_hovered: true,
            ..
        } | scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        } | scrollable::Status::Dragged {
            is_horizontal_scrollbar_dragged: true,
            ..
        } | scrollable::Status::Dragged {
            is_vertical_scrollbar_dragged: true,
            ..
        }
    );
    let scroller = Scroller {
        background: Background::Color(if active {
            theme.palette.ring
        } else {
            theme.palette.muted_foreground
        }),
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
    };
    let rail = Rail {
        background: Some(Background::Color(theme.palette.muted)),
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        scroller,
    };

    scrollable::Style {
        container: iced::widget::container::Style {
            background: Some(Background::Color(theme.palette.background)),
            text_color: Some(theme.palette.foreground),
            ..Default::default()
        },
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: Some(Background::Color(theme.palette.muted)),
        auto_scroll: AutoScroll {
            background: Background::Color(theme.palette.background),
            border: Border {
                color: theme.palette.ring,
                width: 1.0,
                radius: 999.0.into(),
            },
            shadow: Shadow::default(),
            icon: theme.palette.foreground,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    fn rect(y: f32, height: f32) -> Rectangle {
        Rectangle {
            x: 0.0,
            y,
            width: 100.0,
            height,
        }
    }

    fn row(id: &str, y: f32, height: f32, anchor: bool) -> MeasuredRow {
        MeasuredRow {
            meta: MessageScrollerItemMeta::new(id).scroll_anchor(anchor),
            bounds: rect(y, height),
        }
    }

    fn measurement(
        translation: f32,
        content_height: f32,
        rows: Vec<MeasuredRow>,
    ) -> MessageScrollerMeasurement {
        measurement_with_viewport(200.0, translation, content_height, rows)
    }

    fn measurement_with_viewport(
        viewport_height: f32,
        translation: f32,
        content_height: f32,
        rows: Vec<MeasuredRow>,
    ) -> MessageScrollerMeasurement {
        MessageScrollerMeasurement {
            revision: 0,
            viewport: rect(0.0, viewport_height),
            content: rect(0.0, content_height),
            translation: Vector::new(0.0, translation),
            rows,
        }
    }

    #[test]
    fn stable_ids_distinguish_prepend_and_append() {
        let old = [
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b"),
        ];
        let changed = [
            MessageScrollerItemMeta::new("older"),
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b"),
            MessageScrollerItemMeta::new("new"),
        ];

        assert_eq!(retained_range(&old, &changed), Some((1, 3)));
        assert_eq!(retained_range(&old, &changed[..3]), Some((1, 3)));
        assert_eq!(retained_range(&old, &changed[3..]), None);
    }

    #[test]
    fn measurement_derives_edges_visibility_and_current_anchor() {
        let layout = measurement(
            200.0,
            600.0,
            vec![
                row("a", 150.0, 60.0, false),
                row("b", 220.0, 60.0, true),
                row("c", 390.0, 40.0, true),
                row("d", 450.0, 40.0, false),
            ],
        );
        let mut state = MessageScrollerState::new("visibility");
        state.update_derived_state(&layout);

        assert!(state.can_scroll_start());
        assert!(state.can_scroll_end());
        assert_eq!(state.visible_message_ids(), ["b", "c"]);
        assert_eq!(state.current_anchor_id(), Some("b"));
    }

    #[test]
    fn current_anchor_tolerates_pixel_rounding_at_the_reading_line() {
        let layout = measurement(
            235.25,
            600.0,
            vec![
                row("previous", 200.0, 40.0, true),
                row("turn", 300.0, 40.0, true),
            ],
        );
        let mut state = MessageScrollerState::new("rounded-anchor");

        state.update_derived_state(&layout);

        assert_eq!(state.current_anchor_id(), Some("turn"));
    }

    #[test]
    fn pure_scroll_reuses_cached_row_geometry() {
        let mut state = MessageScrollerState::new("cached-scroll");
        state.measurement = Some(measurement(
            0.0,
            600.0,
            vec![
                row("above", 300.0, 40.0, false),
                row("visible", 380.0, 40.0, false),
            ],
        ));

        assert!(!state.update_cached_scroll(MessageScrollerViewport {
            offset_from_end: 100.0,
            viewport_height: 200.0,
            content_height: 650.0,
        }));
        assert!(state.update_cached_scroll(MessageScrollerViewport {
            offset_from_end: 100.0,
            viewport_height: 200.0,
            content_height: 600.0,
        }));

        assert_eq!(state.measurement.as_ref().unwrap().translation.y, 300.0);
        assert_eq!(state.visible_message_ids(), ["visible"]);
        assert!(state.can_scroll_start());
        assert!(state.can_scroll_end());
    }

    #[test]
    fn user_scroll_that_removes_anchor_layout_still_remeasures() {
        let mut state = MessageScrollerState::new("spacer-scroll");
        state.initialized = true;
        state.items = vec![MessageScrollerItemMeta::new("row")];
        state.mode = ScrollMode::Anchored("row".into());
        state.spacer_height = 64.0;
        state.viewport = Some(MessageScrollerViewport {
            offset_from_end: 0.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });
        state.measurement = Some(measurement(
            400.0,
            600.0,
            vec![row("row", 500.0, 40.0, true)],
        ));
        let items = state.items.clone().into();

        let _ = state.update(MessageScrollerEvent::ViewportChanged {
            viewport: MessageScrollerViewport {
                offset_from_end: 100.0,
                viewport_height: 200.0,
                content_height: 600.0,
            },
            items,
        });

        assert_eq!(state.spacer_height, 0.0);
        assert_eq!(state.measurement.as_ref().unwrap().translation.y, 400.0);
    }

    #[test]
    fn cached_programmatic_scroll_publishes_pending_unread() {
        let mut state = MessageScrollerState::new("cached-unread");
        state.initialized = true;
        state.items = vec![MessageScrollerItemMeta::new("row")];
        state.mode = ScrollMode::Free;
        state.programmatic_scroll = Some(ProgrammaticScroll::PreserveOffset(100.0));
        state.pending_unread_count = 2;
        state.viewport = Some(MessageScrollerViewport {
            offset_from_end: 0.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });
        state.measurement = Some(measurement(
            400.0,
            600.0,
            vec![row("row", 500.0, 40.0, false)],
        ));
        let items = state.items.clone().into();

        let _ = state.update(MessageScrollerEvent::ViewportChanged {
            viewport: MessageScrollerViewport {
                offset_from_end: 100.0,
                viewport_height: 200.0,
                content_height: 600.0,
            },
            items,
        });

        assert_eq!(state.measurement.as_ref().unwrap().translation.y, 300.0);
        assert_eq!(state.unread_count(), 2);
        assert_eq!(state.pending_unread_count, 0);
    }

    #[test]
    fn layout_only_viewport_change_keeps_programmatic_scroll_pending() {
        let mut state = MessageScrollerState::new("layout-programmatic");
        state.items = vec![MessageScrollerItemMeta::new("row")];
        state.programmatic_scroll = Some(ProgrammaticScroll::PreserveOffset(0.0));
        state.viewport = Some(MessageScrollerViewport {
            offset_from_end: 100.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });
        state.measurement = Some(measurement(
            300.0,
            600.0,
            vec![row("row", 380.0, 40.0, false)],
        ));
        let items = state.items.clone().into();

        let _ = state.update(MessageScrollerEvent::ViewportChanged {
            viewport: MessageScrollerViewport {
                offset_from_end: 100.0,
                viewport_height: 200.0,
                content_height: 650.0,
            },
            items,
        });

        assert_eq!(
            state.programmatic_scroll,
            Some(ProgrammaticScroll::PreserveOffset(0.0))
        );
        assert_eq!(state.measurement.as_ref().unwrap().content.height, 600.0);
    }

    #[test]
    fn measurement_does_not_descend_into_matched_rows() {
        let mut operation =
            MeasureMessageScroller::new("shallow", &[MessageScrollerItemMeta::new("row")], 0);
        let row_id = item_widget_id("shallow", "row");

        widget::Operation::<MessageScrollerEvent>::container(
            &mut operation,
            Some(&row_id),
            rect(0.0, 40.0),
        );
        let mut traversed = false;
        widget::Operation::<MessageScrollerEvent>::traverse(&mut operation, &mut |_| {
            traversed = true
        });
        assert!(!traversed);
        assert_eq!(operation.rows.len(), 1);

        widget::Operation::<MessageScrollerEvent>::container(&mut operation, None, rect(0.0, 40.0));
        widget::Operation::<MessageScrollerEvent>::traverse(&mut operation, &mut |_| {
            traversed = true
        });
        assert!(traversed);
    }

    #[test]
    fn anchored_resize_realigns_after_remeasuring_its_spacer() {
        let mut state = MessageScrollerState::new("anchored-resize");
        state.initialized = true;
        state.mode = ScrollMode::Anchored("turn".into());
        state.spacer_height = 36.0;

        let _ = state.apply_measurement(measurement_with_viewport(
            180.0,
            120.0,
            300.0,
            vec![row("turn", 200.0, 40.0, true)],
        ));
        assert_eq!(state.spacer_height, 52.0);
        assert!(state.programmatic_scroll.is_none());

        let _ = state.apply_measurement(measurement_with_viewport(
            180.0,
            120.0,
            316.0,
            vec![row("turn", 200.0, 40.0, true)],
        ));
        assert!(state.programmatic_scroll.is_some());
        assert_eq!(state.mode, ScrollMode::Anchored("turn".into()));
    }

    #[test]
    fn auto_follow_snaps_to_end_when_an_anchor_spacer_disappears() {
        let mut state = MessageScrollerState::new("anchor-to-tail").auto_scroll(true);
        state.initialized = true;
        state.mode = ScrollMode::Anchored("turn".into());
        state.spacer_height = 36.0;
        state.unread_count = 2;

        let _ = state.apply_measurement(measurement_with_viewport(
            120.0,
            100.0,
            300.0,
            vec![row("turn", 200.0, 40.0, true)],
        ));

        assert_eq!(state.spacer_height, 0.0);
        assert!(state.is_following());
        assert_eq!(state.unread_count(), 0);
        assert!(state.programmatic_scroll.is_some());
    }

    #[test]
    fn zero_spacer_turn_stays_anchored() {
        let mut state = MessageScrollerState::new("zero-spacer-anchor").auto_scroll(true);
        state.initialized = true;
        state.mode = ScrollMode::Anchored("turn".into());

        let _ = state.apply_measurement(measurement(
            36.0,
            600.0,
            vec![row("turn", 100.0, 40.0, true)],
        ));

        assert_eq!(state.spacer_height, 0.0);
        assert_eq!(state.mode, ScrollMode::Anchored("turn".into()));
    }

    #[test]
    fn enabling_auto_scroll_does_not_release_an_anchor_hold() {
        let mut state = MessageScrollerState::new("enable-while-anchored");
        state.initialized = true;
        state.mode = ScrollMode::Anchored("turn".into());
        state.spacer_height = 36.0;

        state.set_auto_scroll(true);

        assert_eq!(state.mode, ScrollMode::Anchored("turn".into()));
        assert_eq!(state.spacer_height, 36.0);
    }

    #[test]
    fn preservation_tracks_a_stable_row_across_content_changes() {
        let before = measurement(200.0, 600.0, vec![row("visible", 220.0, 60.0, false)]);
        let appended = measurement(300.0, 700.0, vec![row("visible", 220.0, 60.0, false)]);
        let prepended = measurement(300.0, 700.0, vec![row("visible", 320.0, 60.0, false)]);

        assert_eq!(preservation_delta(&before, &appended), Some(100.0));
        assert_eq!(preservation_delta(&before, &prepended), None);
    }

    #[test]
    fn anchor_spacer_and_bottom_relative_alignment_share_one_geometry() {
        let layout = measurement(100.0, 300.0, vec![row("turn", 200.0, 40.0, true)]);
        let turn = &layout.rows[0];

        assert_eq!(
            required_bottom_spacer(&layout, turn, 0.0, MessageScrollerAlignment::Start, 64.0,),
            36.0
        );
        assert_eq!(
            alignment_delta_from_bottom(&layout, turn, MessageScrollerAlignment::Start, 64.0,),
            -36.0
        );
        assert_eq!(
            alignment_delta_from_bottom(&layout, turn, MessageScrollerAlignment::Nearest, 0.0,),
            0.0
        );
    }

    #[test]
    fn last_row_center_and_end_margin_allocate_reachable_tail_space() {
        let layout = measurement(100.0, 300.0, vec![row("last", 260.0, 40.0, false)]);
        let last = &layout.rows[0];

        assert_eq!(
            required_bottom_spacer(&layout, last, 0.0, MessageScrollerAlignment::Center, 10.0,),
            70.0
        );
        assert_eq!(
            alignment_delta_from_bottom(&layout, last, MessageScrollerAlignment::Center, 10.0,),
            -70.0
        );
        assert_eq!(
            required_bottom_spacer(&layout, last, 0.0, MessageScrollerAlignment::End, 10.0,),
            10.0
        );
        assert_eq!(
            alignment_delta_from_bottom(&layout, last, MessageScrollerAlignment::End, 10.0,),
            -10.0
        );
    }

    #[test]
    fn user_scroll_releases_and_reaching_end_resumes_following() {
        let mut state = MessageScrollerState::new("follow").auto_scroll(true);
        state.apply_viewport(MessageScrollerViewport {
            offset_from_end: 0.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });
        assert!(state.is_following());

        state.apply_viewport(MessageScrollerViewport {
            offset_from_end: 100.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });
        assert!(!state.is_following());
        assert!(state.can_scroll_end());

        state.apply_viewport(MessageScrollerViewport {
            offset_from_end: 0.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });
        assert!(state.is_following());
    }

    #[test]
    fn user_scroll_during_layout_change_releases_following_and_anchor_hold() {
        for anchored in [false, true] {
            let mut state =
                MessageScrollerState::new(format!("mixed-scroll-{anchored}")).auto_scroll(true);
            state.viewport = Some(MessageScrollerViewport {
                offset_from_end: 0.0,
                viewport_height: 200.0,
                content_height: 600.0,
            });
            if anchored {
                state.mode = ScrollMode::Anchored("turn".into());
                state.spacer_height = 64.0;
            }

            state.apply_viewport(MessageScrollerViewport {
                offset_from_end: 60.0,
                viewport_height: 200.0,
                content_height: 660.0,
            });
            assert_eq!(
                state.mode,
                if anchored {
                    ScrollMode::Anchored("turn".into())
                } else {
                    ScrollMode::Following
                }
            );

            let _ = state.apply_user_scroll_intent();
            assert_eq!(state.mode, ScrollMode::Free);
            assert_eq!(state.spacer_height, 0.0);
        }
    }

    #[test]
    fn keyboard_scroll_to_live_edge_resumes_following() {
        let mut state = MessageScrollerState::new("keyboard-follow").auto_scroll(true);
        state.initialized = true;
        state.mode = ScrollMode::Free;
        state.viewport = Some(MessageScrollerViewport {
            offset_from_end: 60.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });
        state.can_scroll_end = true;
        state.can_scroll_start = true;

        let _ = state.apply_command(MessageScrollerCommand::LineDown);
        assert!(state.programmatic_scroll.is_some());
        assert!(!state.is_following());

        state.apply_viewport(MessageScrollerViewport {
            offset_from_end: 0.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });

        assert!(state.programmatic_scroll.is_none());
        assert!(state.is_following());

        let _ = state.apply_command(MessageScrollerCommand::LineDown);
        assert!(state.programmatic_scroll.is_none());
        assert!(state.is_following());
    }

    #[test]
    fn first_viewport_acknowledges_programmatic_scroll_without_rearming() {
        let mut state = MessageScrollerState::new("initial-scroll").auto_scroll(true);
        state.mode = ScrollMode::Free;
        state.programmatic_scroll = Some(ProgrammaticScroll::PreserveEnd);

        state.apply_viewport(MessageScrollerViewport {
            offset_from_end: 0.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });

        assert!(state.programmatic_scroll.is_none());
        assert!(!state.is_following());
    }

    #[test]
    fn layout_changing_end_command_does_not_ignore_the_next_user_scroll() {
        let mut state = MessageScrollerState::new("command-layout").auto_scroll(true);
        state.initialized = true;
        state.mode = ScrollMode::Anchored("turn".into());
        state.spacer_height = 64.0;
        state.viewport = Some(MessageScrollerViewport {
            offset_from_end: 100.0,
            viewport_height: 200.0,
            content_height: 664.0,
        });
        state.can_scroll_end = true;

        let _ = state.apply_command(MessageScrollerCommand::End);
        state.apply_viewport(MessageScrollerViewport {
            offset_from_end: 0.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });
        assert!(state.programmatic_scroll.is_none());

        state.apply_viewport(MessageScrollerViewport {
            offset_from_end: 80.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });

        assert!(!state.is_following());
        assert!(state.can_scroll_end());
    }

    #[test]
    fn item_jump_to_last_stays_free_when_auto_scroll_is_enabled() {
        let mut state = MessageScrollerState::new("item-last").auto_scroll(true);
        state.initialized = true;
        state.items = vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("last"),
        ];
        state.viewport = Some(MessageScrollerViewport {
            offset_from_end: 100.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });

        let _ = state.apply_command(MessageScrollerCommand::Item {
            id: "last".into(),
            alignment: MessageScrollerAlignment::End,
        });
        let _ = state.apply_measurement(measurement(
            300.0,
            600.0,
            vec![row("a", 0.0, 40.0, false), row("last", 560.0, 40.0, false)],
        ));
        assert_eq!(
            state.programmatic_scroll,
            Some(ProgrammaticScroll::PreserveOffset(0.0))
        );

        state.apply_viewport(MessageScrollerViewport {
            offset_from_end: 0.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });
        assert_eq!(state.mode, ScrollMode::Free);

        state.sync_items(vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("last"),
            MessageScrollerItemMeta::new("new"),
        ]);
        assert_eq!(state.mode, ScrollMode::Free);
        assert!(!state.is_following());
        assert_eq!(state.pending_unread_count, 1);
    }

    #[test]
    fn clamped_item_alignment_does_not_leave_a_programmatic_marker() {
        let mut state = MessageScrollerState::new("clamped-item");
        state.initialized = true;
        state.items = vec![MessageScrollerItemMeta::new("first")];

        let _ = state.apply_command(MessageScrollerCommand::Item {
            id: "first".into(),
            alignment: MessageScrollerAlignment::End,
        });
        let _ = state.apply_measurement(measurement(
            0.0,
            600.0,
            vec![row("first", 0.0, 40.0, false)],
        ));

        assert!(state.programmatic_scroll.is_none());
        assert_eq!(state.mode, ScrollMode::Free);
    }

    #[test]
    fn pre_measure_item_and_start_commands_beat_default_end() {
        let mut item_state = MessageScrollerState::new("premount-item").auto_scroll(true);
        item_state.items = vec![MessageScrollerItemMeta::new("target")];
        let _ = item_state.apply_command(MessageScrollerCommand::Item {
            id: "target".into(),
            alignment: MessageScrollerAlignment::Start,
        });
        let _ = item_state.apply_measurement(measurement(
            0.0,
            600.0,
            vec![row("target", 0.0, 40.0, false)],
        ));
        assert_eq!(item_state.mode, ScrollMode::Free);
        assert!(item_state.pending_target.is_none());

        let mut start_state = MessageScrollerState::new("premount-start").auto_scroll(true);
        start_state.items = vec![MessageScrollerItemMeta::new("target")];
        let _ = start_state.apply_command(MessageScrollerCommand::Start);
        let _ = start_state.apply_measurement(measurement(
            400.0,
            600.0,
            vec![row("target", 0.0, 40.0, false)],
        ));
        assert_eq!(start_state.mode, ScrollMode::Free);
        assert_eq!(
            start_state.programmatic_scroll,
            Some(ProgrammaticScroll::PreserveStart)
        );
    }

    #[test]
    fn missing_item_is_a_noop_for_an_anchor_hold() {
        let mut state = MessageScrollerState::new("missing-item");
        state.initialized = true;
        state.items = vec![MessageScrollerItemMeta::new("turn").scroll_anchor(true)];
        state.mode = ScrollMode::Anchored("turn".into());
        state.spacer_height = 64.0;

        let _ = state.apply_command(MessageScrollerCommand::Item {
            id: "missing".into(),
            alignment: MessageScrollerAlignment::Start,
        });

        assert_eq!(state.mode, ScrollMode::Anchored("turn".into()));
        assert_eq!(state.spacer_height, 64.0);
        assert!(state.pending_target.is_none());
    }

    #[test]
    fn item_target_waits_for_an_async_empty_transcript() {
        let mut state = MessageScrollerState::new("async-item").auto_scroll(true);
        let _ = state.apply_command(MessageScrollerCommand::Item {
            id: "future".into(),
            alignment: MessageScrollerAlignment::Start,
        });
        let _ = state.apply_measurement(measurement_with_viewport(200.0, 0.0, 0.0, vec![]));
        assert_eq!(
            state
                .pending_target
                .as_ref()
                .map(|target| target.id.as_str()),
            Some("future")
        );

        state.sync_items(vec![MessageScrollerItemMeta::new("future")]);
        let _ = state.apply_measurement(measurement(
            0.0,
            300.0,
            vec![row("future", 100.0, 40.0, false)],
        ));

        assert!(state.pending_target.is_none());
        assert_eq!(state.mode, ScrollMode::Free);
        assert_eq!(state.unread_count(), 0);
    }

    #[test]
    fn default_start_waits_for_async_initial_rows() {
        let mut state = MessageScrollerState::new("async-default")
            .auto_scroll(true)
            .initial_position(MessageScrollerInitialPosition::Start);

        let _ = state.apply_measurement(measurement_with_viewport(200.0, 0.0, 0.0, vec![]));
        assert!(!state.initialized);
        assert!(!state.default_position_applied);

        state.sync_items(vec![MessageScrollerItemMeta::new("first")]);
        let _ = state.apply_measurement(measurement(
            400.0,
            600.0,
            vec![row("first", 0.0, 40.0, false)],
        ));

        assert!(state.initialized);
        assert!(state.default_position_applied);
        assert_eq!(state.mode, ScrollMode::Free);
        assert_eq!(
            state.programmatic_scroll,
            Some(ProgrammaticScroll::PreserveStart)
        );
    }

    #[test]
    fn fitting_last_anchor_falls_back_to_end_instead_of_an_older_anchor() {
        let mut state = MessageScrollerState::new("fitting-last-anchor")
            .auto_scroll(true)
            .initial_position(MessageScrollerInitialPosition::LastAnchor);
        state.items = vec![
            MessageScrollerItemMeta::new("old").scroll_anchor(true),
            MessageScrollerItemMeta::new("latest").scroll_anchor(true),
        ];

        let _ = state.apply_measurement(measurement(
            0.0,
            600.0,
            vec![
                row("old", 100.0, 40.0, true),
                row("latest", 450.0, 40.0, true),
            ],
        ));

        assert!(state.pending_target.is_none());
        assert!(state.is_following());
    }

    #[test]
    fn overflowing_last_anchor_is_the_initial_target() {
        let mut state = MessageScrollerState::new("overflowing-last-anchor")
            .initial_position(MessageScrollerInitialPosition::LastAnchor);
        state.items = vec![
            MessageScrollerItemMeta::new("old").scroll_anchor(true),
            MessageScrollerItemMeta::new("latest").scroll_anchor(true),
        ];

        let _ = state.apply_measurement(measurement(
            0.0,
            600.0,
            vec![
                row("old", 100.0, 40.0, true),
                row("latest", 350.0, 40.0, true),
            ],
        ));

        assert_eq!(state.mode, ScrollMode::Anchored("latest".into()));
    }

    #[test]
    fn first_appended_anchor_sets_target_and_unread_while_reader_is_away() {
        let mut state = MessageScrollerState::new("items");
        state.initialized = true;
        state.can_scroll_end = true;
        state.items = vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b"),
        ];

        state.sync_items(vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b"),
            MessageScrollerItemMeta::new("c").scroll_anchor(true),
            MessageScrollerItemMeta::new("d").scroll_anchor(true),
        ]);

        assert_eq!(state.unread_count(), 2);
        assert_eq!(
            state
                .pending_target
                .as_ref()
                .map(|target| target.id.as_str()),
            Some("c")
        );
        assert!(!state.preserve_visible_row);
    }

    #[test]
    fn following_ignores_a_multi_anchor_batch_and_stays_at_end() {
        let mut state = MessageScrollerState::new("multi-anchor-follow").auto_scroll(true);
        state.initialized = true;
        state.items = vec![MessageScrollerItemMeta::new("a")];

        state.sync_items(vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b").scroll_anchor(true),
            MessageScrollerItemMeta::new("c").scroll_anchor(true),
        ]);

        assert!(state.is_following());
        assert!(state.pending_target.is_none());
        assert_eq!(state.unread_count(), 0);
        assert!(!state.preserve_visible_row);
    }

    #[test]
    fn stale_viewport_cannot_cancel_an_appended_anchor_hold() {
        let mut state = MessageScrollerState::new("stale-viewport").auto_scroll(true);
        state.initialized = true;
        state.mode = ScrollMode::Following;
        state.items = vec![MessageScrollerItemMeta::new("a")];
        state.viewport = Some(MessageScrollerViewport {
            offset_from_end: 0.0,
            viewport_height: 200.0,
            content_height: 400.0,
        });
        let old_items: Arc<[MessageScrollerItemMeta]> = state.items.clone().into();
        let next_items = vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("turn").scroll_anchor(true),
            MessageScrollerItemMeta::new("stream"),
        ];

        let _ = state.update(MessageScrollerEvent::ItemsChanged(next_items.clone()));
        assert_eq!(state.unread_count(), 2);
        assert_eq!(
            state
                .pending_target
                .as_ref()
                .map(|target| target.id.as_str()),
            Some("turn")
        );

        let _ = state.update(MessageScrollerEvent::ViewportChanged {
            viewport: MessageScrollerViewport {
                offset_from_end: 0.0,
                viewport_height: 200.0,
                content_height: 400.0,
            },
            items: old_items,
        });
        assert_eq!(state.items, next_items);
        assert_eq!(
            state
                .pending_target
                .as_ref()
                .map(|target| target.id.as_str()),
            Some("turn")
        );

        let _ = state.update(MessageScrollerEvent::Measured(measurement(
            200.0,
            400.0,
            vec![row("a", 200.0, 40.0, false)],
        )));
        assert!(state.pending_target.is_some());

        let rows = || {
            vec![
                row("a", 200.0, 40.0, false),
                row("turn", 300.0, 40.0, true),
                row("stream", 350.0, 30.0, false),
            ]
        };
        let mut first = measurement(200.0, 400.0, rows());
        first.revision = state.layout_revision;
        let _ = state.update(MessageScrollerEvent::Measured(first.clone()));
        assert_eq!(state.spacer_height, 36.0);
        let _ = state.update(MessageScrollerEvent::Measured(first));
        assert_eq!(state.spacer_height, 36.0);
        let mut settled = measurement(236.0, 436.0, rows());
        settled.revision = state.layout_revision;
        let _ = state.update(MessageScrollerEvent::Measured(settled));

        assert_eq!(state.mode, ScrollMode::Anchored("turn".into()));
        assert_eq!(state.unread_count(), 2);

        let _ = state.update(MessageScrollerEvent::ViewportChanged {
            viewport: MessageScrollerViewport {
                offset_from_end: 0.0,
                viewport_height: 200.0,
                content_height: 436.0,
            },
            items: next_items.into(),
        });
        assert_eq!(state.mode, ScrollMode::Anchored("turn".into()));
        assert_eq!(state.unread_count(), 2);

        let _ = state.apply_command(MessageScrollerCommand::End);
        assert!(state.is_following());
        assert_eq!(state.unread_count(), 0);
    }

    #[test]
    fn promoting_an_existing_row_to_anchor_is_handled_once() {
        let mut state = MessageScrollerState::new("promoted-anchor");
        state.initialized = true;
        state.items = vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b"),
        ];

        state.sync_items(vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b").scroll_anchor(true),
        ]);
        assert_eq!(
            state
                .pending_target
                .as_ref()
                .map(|target| target.id.as_str()),
            Some("b")
        );

        state.pending_target = None;
        state.sync_items(vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b"),
        ]);
        state.sync_items(vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b").scroll_anchor(true),
        ]);

        assert!(state.pending_target.is_none());
    }

    #[test]
    fn same_length_replacement_targets_new_anchor_not_historical_anchor() {
        let mut state = MessageScrollerState::new("replacement-anchor");
        state.initialized = true;
        state.items = vec![
            MessageScrollerItemMeta::new("historical").scroll_anchor(true),
            MessageScrollerItemMeta::new("placeholder"),
        ];

        state.sync_items(vec![
            MessageScrollerItemMeta::new("historical").scroll_anchor(true),
            MessageScrollerItemMeta::new("new-turn").scroll_anchor(true),
        ]);

        assert_eq!(
            state
                .pending_target
                .as_ref()
                .map(|target| target.id.as_str()),
            Some("new-turn")
        );
    }

    #[test]
    fn append_publishes_unread_after_preserving_the_old_live_edge() {
        let mut state = MessageScrollerState::new("live-edge");
        state.initialized = true;
        state.items = vec![MessageScrollerItemMeta::new("a")];
        state.viewport = Some(MessageScrollerViewport {
            offset_from_end: 0.0,
            viewport_height: 200.0,
            content_height: 300.0,
        });
        state.measurement = Some(measurement(
            100.0,
            300.0,
            vec![row("a", 200.0, 40.0, false)],
        ));

        state.sync_items(vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b"),
        ]);
        assert_eq!(state.unread_count(), 0);
        assert_eq!(state.pending_unread_count, 1);

        let _ = state.apply_measurement(measurement(
            180.0,
            380.0,
            vec![row("a", 200.0, 40.0, false), row("b", 300.0, 80.0, false)],
        ));
        assert_eq!(state.unread_count(), 0);
        assert_eq!(state.pending_unread_count, 1);
        assert_eq!(
            state.programmatic_scroll,
            Some(ProgrammaticScroll::PreserveOffset(80.0))
        );

        let items = state.items.clone().into();
        let _ = state.update(MessageScrollerEvent::ViewportChanged {
            viewport: MessageScrollerViewport {
                offset_from_end: 80.0,
                viewport_height: 200.0,
                content_height: 380.0,
            },
            items,
        });

        assert_eq!(state.unread_count(), 1);
        assert_eq!(state.pending_unread_count, 0);
    }

    #[test]
    fn fitting_append_settles_without_unread() {
        let mut state = MessageScrollerState::new("fitting-append");
        state.initialized = true;
        state.items = vec![MessageScrollerItemMeta::new("a")];
        state.measurement = Some(measurement_with_viewport(
            200.0,
            0.0,
            120.0,
            vec![row("a", 0.0, 60.0, false)],
        ));

        state.sync_items(vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b"),
        ]);
        let _ = state.apply_measurement(measurement_with_viewport(
            200.0,
            0.0,
            180.0,
            vec![row("a", 0.0, 60.0, false), row("b", 80.0, 60.0, false)],
        ));

        assert_eq!(state.unread_count(), 0);
        assert_eq!(state.pending_unread_count, 0);
    }

    #[test]
    fn same_id_resize_preserves_a_free_readers_row() {
        let mut state = MessageScrollerState::new("same-id-resize");
        state.initialized = true;
        state.mode = ScrollMode::Free;
        state.items = vec![MessageScrollerItemMeta::new("visible")];
        state.measurement = Some(measurement(
            300.0,
            600.0,
            vec![row("visible", 320.0, 60.0, false)],
        ));

        state.sync_items(vec![MessageScrollerItemMeta::new("visible")]);
        assert!(state.preserve_visible_row);
        let _ = state.apply_measurement(measurement(
            400.0,
            700.0,
            vec![row("visible", 320.0, 60.0, false)],
        ));

        assert_eq!(
            state.programmatic_scroll,
            Some(ProgrammaticScroll::PreserveOffset(200.0))
        );
    }

    #[test]
    fn boundary_scroll_intent_does_not_poison_the_next_resize() {
        let mut state = MessageScrollerState::new("boundary-intent").auto_scroll(true);
        state.initialized = true;
        state.items = vec![MessageScrollerItemMeta::new("visible")];
        state.measurement = Some(measurement(
            400.0,
            600.0,
            vec![row("visible", 420.0, 60.0, false)],
        ));

        let _ = state.apply_user_scroll_intent();
        assert_eq!(state.mode, ScrollMode::Free);
        assert!(!state.skip_preservation_once);

        state.sync_items(vec![MessageScrollerItemMeta::new("visible")]);
        let _ = state.apply_measurement(measurement(
            500.0,
            700.0,
            vec![row("visible", 420.0, 60.0, false)],
        ));

        assert_eq!(
            state.programmatic_scroll,
            Some(ProgrammaticScroll::PreserveOffset(100.0))
        );
    }

    #[test]
    fn fitting_spacer_removal_invalidates_and_remeasures_geometry() {
        let mut state = MessageScrollerState::new("fitting-spacer").auto_scroll(true);
        state.initialized = true;
        state.items = vec![MessageScrollerItemMeta::new("row")];
        state.spacer_height = 100.0;
        state.can_scroll_start = true;
        state.can_scroll_end = true;
        state.measurement = Some(measurement(
            50.0,
            250.0,
            vec![row("row", 0.0, 150.0, false)],
        ));

        let _ = state.apply_command(MessageScrollerCommand::End);
        assert_eq!(state.spacer_height, 0.0);
        assert!(state.measurement.is_none());

        let _ = state.apply_measurement(measurement_with_viewport(
            200.0,
            0.0,
            150.0,
            vec![row("row", 0.0, 150.0, false)],
        ));
        assert!(!state.can_scroll_start());
        assert!(!state.can_scroll_end());
        assert!(state.programmatic_scroll.is_none());
    }

    #[test]
    fn explicit_item_target_wins_over_interleaved_anchor_append() {
        let mut state = MessageScrollerState::new("explicit-priority").auto_scroll(true);
        state.initialized = true;
        state.items = vec![
            MessageScrollerItemMeta::new("old"),
            MessageScrollerItemMeta::new("target"),
        ];

        let _ = state.apply_command(MessageScrollerCommand::Item {
            id: "target".into(),
            alignment: MessageScrollerAlignment::Start,
        });
        assert_eq!(state.mode, ScrollMode::Free);
        state.sync_items(vec![
            MessageScrollerItemMeta::new("old"),
            MessageScrollerItemMeta::new("target"),
            MessageScrollerItemMeta::new("new-turn").scroll_anchor(true),
            MessageScrollerItemMeta::new("reply"),
        ]);

        assert_eq!(
            state
                .pending_target
                .as_ref()
                .map(|target| target.id.as_str()),
            Some("target")
        );
        assert_eq!(state.pending_unread_count, 2);
    }

    #[test]
    fn prepend_preservation_wins_over_an_appended_anchor() {
        let mut state = MessageScrollerState::new("mixed-prepend-append");
        state.initialized = true;
        state.mode = ScrollMode::Free;
        state.can_scroll_end = true;
        state.items = vec![
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b"),
        ];

        state.sync_items(vec![
            MessageScrollerItemMeta::new("older"),
            MessageScrollerItemMeta::new("a"),
            MessageScrollerItemMeta::new("b"),
            MessageScrollerItemMeta::new("new-turn").scroll_anchor(true),
            MessageScrollerItemMeta::new("reply"),
        ]);

        assert!(state.pending_target.is_none());
        assert!(state.preserve_visible_row);
        assert_eq!(state.unread_count(), 2);
    }

    #[test]
    fn promoted_anchor_wins_when_a_non_anchor_is_also_appended() {
        let mut state = MessageScrollerState::new("promote-and-append");
        state.initialized = true;
        state.items = vec![
            MessageScrollerItemMeta::new("turn"),
            MessageScrollerItemMeta::new("reply"),
        ];

        state.sync_items(vec![
            MessageScrollerItemMeta::new("turn").scroll_anchor(true),
            MessageScrollerItemMeta::new("reply"),
            MessageScrollerItemMeta::new("tail"),
        ]);

        assert_eq!(
            state
                .pending_target
                .as_ref()
                .map(|target| target.id.as_str()),
            Some("turn")
        );
    }

    #[test]
    fn layout_clamp_does_not_release_anchor_without_user_intent() {
        let mut state = MessageScrollerState::new("anchor-clamp").auto_scroll(true);
        state.initialized = true;
        state.mode = ScrollMode::Anchored("turn".into());
        state.spacer_height = 64.0;
        state.viewport = Some(MessageScrollerViewport {
            offset_from_end: 100.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });

        state.apply_viewport(MessageScrollerViewport {
            offset_from_end: 40.0,
            viewport_height: 200.0,
            content_height: 540.0,
        });

        assert_eq!(state.mode, ScrollMode::Anchored("turn".into()));
        assert_eq!(state.spacer_height, 64.0);
    }

    #[test]
    fn net_zero_layout_scroll_acknowledges_expected_offset() {
        let mut state = MessageScrollerState::new("net-zero-ack");
        state.initialized = true;
        state.mode = ScrollMode::Free;
        state.programmatic_scroll = Some(ProgrammaticScroll::PreserveOffset(400.0));
        state.viewport = Some(MessageScrollerViewport {
            offset_from_end: 400.0,
            viewport_height: 200.0,
            content_height: 600.0,
        });

        state.apply_viewport(MessageScrollerViewport {
            offset_from_end: 400.0,
            viewport_height: 200.0,
            content_height: 700.0,
        });
        assert!(state.programmatic_scroll.is_none());

        state.apply_viewport(MessageScrollerViewport {
            offset_from_end: 300.0,
            viewport_height: 200.0,
            content_height: 700.0,
        });
        assert!(state.programmatic_scroll.is_none());
        assert_eq!(state.mode, ScrollMode::Free);
    }

    #[test]
    fn transcript_keyboard_commands_use_native_scroll_semantics() {
        assert_eq!(
            keyboard_command(
                &keyboard::Key::Named(Named::Space),
                keyboard::Modifiers::SHIFT,
            ),
            Some(MessageScrollerCommand::PageUp)
        );
        assert_eq!(
            keyboard_command(
                &keyboard::Key::Named(Named::Space),
                keyboard::Modifiers::default(),
            ),
            Some(MessageScrollerCommand::PageDown)
        );
        assert_eq!(
            keyboard_command(
                &keyboard::Key::Named(Named::Home),
                keyboard::Modifiers::default(),
            ),
            Some(MessageScrollerCommand::Start)
        );
    }

    #[test]
    fn interaction_highlights_the_native_scroll_handle() {
        for theme in [LIGHT, DARK] {
            let normal = style(
                &theme,
                scrollable::Status::Active {
                    is_horizontal_scrollbar_disabled: true,
                    is_vertical_scrollbar_disabled: false,
                },
            );
            let hovered = style(
                &theme,
                scrollable::Status::Hovered {
                    is_horizontal_scrollbar_hovered: false,
                    is_vertical_scrollbar_hovered: true,
                    is_horizontal_scrollbar_disabled: true,
                    is_vertical_scrollbar_disabled: false,
                },
            );

            assert_ne!(
                normal.vertical_rail.scroller.background,
                hovered.vertical_rail.scroller.background
            );
            assert_eq!(
                hovered.vertical_rail.scroller.background,
                Background::Color(theme.palette.ring)
            );
            assert_eq!(
                hovered.container.background,
                Some(Background::Color(theme.palette.background))
            );
            for thumb in [
                normal.vertical_rail.scroller.background,
                hovered.vertical_rail.scroller.background,
            ] {
                let Background::Color(thumb) = thumb else {
                    panic!("message scrollbar thumb must be a solid color");
                };
                assert!(thumb.relative_contrast(theme.palette.muted) >= 3.0);
            }
        }
    }
}
