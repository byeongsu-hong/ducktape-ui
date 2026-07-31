//! Fixed-height, keyed list virtualization.
//!
//! The caller owns the item data and [`VirtualListState`]. Only the current
//! visible range plus overscan is converted into Iced elements. This keeps
//! layout, diff, and draw work proportional to mounted rows instead of the
//! logical collection size.

use crate::{StableId, accessible};
use iced::advanced::text;
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::keyboard;
use iced::widget::{container, keyed_column, scrollable, space};
use iced::{Element, Event, Length, Rectangle, Size, Task, Vector};
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::rc::Rc;

/// Validated fixed-row geometry for a virtual list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualListConfig {
    row_height: f32,
    viewport_height: f32,
    overscan: usize,
}

impl VirtualListConfig {
    /// Creates fixed-row geometry.
    ///
    /// Row height must be finite and strictly positive. Viewport height must
    /// be finite and non-negative.
    pub fn new(row_height: f32, viewport_height: f32) -> Result<Self, VirtualListConfigError> {
        if !row_height.is_finite() || row_height <= 0.0 {
            return Err(VirtualListConfigError::RowHeight);
        }
        if !viewport_height.is_finite() || viewport_height < 0.0 {
            return Err(VirtualListConfigError::ViewportHeight);
        }
        Ok(Self {
            row_height,
            viewport_height,
            overscan: 2,
        })
    }

    /// Sets the number of extra rows mounted on each side of the viewport.
    #[must_use]
    pub const fn overscan(mut self, overscan: usize) -> Self {
        self.overscan = overscan;
        self
    }

    pub const fn row_height(self) -> f32 {
        self.row_height
    }

    pub const fn viewport_height(self) -> f32 {
        self.viewport_height
    }

    pub const fn overscan_rows(self) -> usize {
        self.overscan
    }

    fn rows_per_page(self) -> usize {
        (self.viewport_height / self.row_height).floor().max(1.0) as usize
    }

    fn total_height(self, item_count: usize) -> f32 {
        ((item_count as f64) * f64::from(self.row_height)).min(f64::from(f32::MAX)) as f32
    }

    fn max_offset(self, item_count: usize) -> f32 {
        (self.total_height(item_count) - self.viewport_height).max(0.0)
    }

    fn visible_range(self, item_count: usize, offset: f32) -> Range<usize> {
        if item_count == 0 || self.viewport_height == 0.0 {
            return 0..0;
        }
        let offset = offset.clamp(0.0, self.max_offset(item_count));
        let first = (offset / self.row_height).floor() as usize;
        let end = ((offset + self.viewport_height) / self.row_height).ceil() as usize;
        first..end.min(item_count)
    }

    fn mounted_range(self, item_count: usize, offset: f32) -> Range<usize> {
        let visible = self.visible_range(item_count, offset);
        if visible.is_empty() {
            return visible;
        }
        visible.start.saturating_sub(self.overscan)
            ..visible.end.saturating_add(self.overscan).min(item_count)
    }
}

/// Invalid [`VirtualListConfig`] geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualListConfigError {
    RowHeight,
    ViewportHeight,
}

impl fmt::Display for VirtualListConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RowHeight => "virtual-list row height must be finite and positive",
            Self::ViewportHeight => "virtual-list viewport height must be finite and non-negative",
        })
    }
}

impl std::error::Error for VirtualListConfigError {}

/// Keyboard movement supported by the v1 list contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualListNavigation {
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
}

/// A strongly typed interaction emitted by [`virtual_list`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VirtualListEvent<Key> {
    Scrolled { offset_y: f32 },
    Select { index: usize, key: Key },
    Navigate(VirtualListNavigation),
}

/// Result of applying a virtual-list interaction to caller-owned state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualListOutcome<Key> {
    pub selected: Option<Key>,
    pub selection_changed: bool,
    pub visible_range_changed: bool,
    pub scroll_changed: bool,
}

/// Deterministic headless accounting for one virtual-list render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualListInspection {
    pub logical_items: usize,
    pub visible_range: Range<usize>,
    pub mounted_range: Range<usize>,
    pub mounted_rows: usize,
    /// Exact keyed-column child slots: mounted rows plus top and bottom spacers.
    pub child_slots: usize,
}

/// Retained selection and viewport state for one virtual list.
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualListState<Key> {
    id: String,
    selected: Option<Key>,
    selected_index: Option<usize>,
    scroll_offset: f32,
}

/// A data-identity error found during explicit reconciliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualListReconcileError<Key> {
    DuplicateKey(Key),
}

impl<Key> VirtualListState<Key>
where
    Key: Copy + Eq + Hash,
{
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            selected: None,
            selected_index: None,
            scroll_offset: 0.0,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn selected(&self) -> Option<Key> {
        self.selected
    }

    pub const fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub const fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    /// Returns the logical rows intersecting the viewport for the current offset.
    pub fn visible_range(&self, item_count: usize, config: VirtualListConfig) -> Range<usize> {
        config.visible_range(item_count, self.effective_offset(item_count, config))
    }

    /// Returns the exact range mounted by [`virtual_list`], including overscan.
    pub fn mounted_range(&self, item_count: usize, config: VirtualListConfig) -> Range<usize> {
        config.mounted_range(item_count, self.effective_offset(item_count, config))
    }

    pub fn inspect(&self, item_count: usize, config: VirtualListConfig) -> VirtualListInspection {
        let visible_range = self.visible_range(item_count, config);
        let mounted_range = self.mounted_range(item_count, config);
        let mounted_rows = mounted_range.len();
        VirtualListInspection {
            logical_items: item_count,
            visible_range,
            mounted_range,
            mounted_rows,
            child_slots: mounted_rows.saturating_add(2),
        }
    }

    /// Reconciles retained identity after items are inserted, reordered, or removed.
    ///
    /// A retained key follows its item to the new index. Removing the selected
    /// key clears selection; it never transfers selection to an unrelated row.
    pub fn reconcile<T>(
        &mut self,
        items: &[T],
        key: impl Fn(&T) -> Key,
        config: VirtualListConfig,
    ) -> Result<(), VirtualListReconcileError<Key>> {
        let mut keys = HashSet::with_capacity(items.len());
        for item in items {
            let item_key = key(item);
            if !keys.insert(item_key) {
                return Err(VirtualListReconcileError::DuplicateKey(item_key));
            }
        }
        self.selected_index = self
            .selected
            .and_then(|selected| items.iter().position(|item| key(item) == selected));
        if self.selected_index.is_none() {
            self.selected = None;
        }
        self.set_offset(self.scroll_offset, items.len(), config);
        Ok(())
    }

    /// Applies a mouse, scroll, or keyboard interaction.
    pub fn apply<T>(
        &mut self,
        event: VirtualListEvent<Key>,
        items: &[T],
        key: impl Fn(&T) -> Key,
        config: VirtualListConfig,
    ) -> VirtualListOutcome<Key> {
        let previous_selected = self.selected;
        let previous_range = self.visible_range(items.len(), config);
        let previous_offset = self.scroll_offset;

        match event {
            VirtualListEvent::Scrolled { offset_y } => {
                self.set_offset(offset_y, items.len(), config);
            }
            VirtualListEvent::Select {
                index,
                key: selected,
            } => {
                let resolved = items
                    .get(index)
                    .filter(|item| key(item) == selected)
                    .map(|_| index)
                    .or_else(|| items.iter().position(|item| key(item) == selected));
                if let Some(index) = resolved {
                    self.selected = Some(selected);
                    self.selected_index = Some(index);
                    self.reveal(index, items.len(), config);
                }
            }
            VirtualListEvent::Navigate(navigation) => {
                if let Some(index) = navigation_index(
                    self.selected_index,
                    items.len(),
                    navigation,
                    config.rows_per_page(),
                ) {
                    self.selected = items.get(index).map(&key);
                    self.selected_index = self.selected.map(|_| index);
                    self.reveal(index, items.len(), config);
                }
            }
        }

        VirtualListOutcome {
            selected: self.selected,
            selection_changed: self.selected != previous_selected,
            visible_range_changed: self.visible_range(items.len(), config) != previous_range,
            scroll_changed: self.scroll_offset != previous_offset,
        }
    }

    /// Scrolls an item into view without changing selection.
    pub fn scroll_to_item<Message>(
        &mut self,
        index: usize,
        item_count: usize,
        config: VirtualListConfig,
    ) -> Task<Message> {
        if index < item_count {
            self.scroll_offset = (index as f64 * f64::from(config.row_height))
                .min(f64::from(config.max_offset(item_count)))
                as f32;
        }
        iced::widget::operation::scroll_to(
            self.scroll_id(),
            iced::widget::operation::AbsoluteOffset {
                x: None,
                y: Some(self.scroll_offset),
            },
        )
    }

    /// Scrolls a stable key into view, returning `None` when it is absent.
    pub fn scroll_to_key<T, Message>(
        &mut self,
        selected: Key,
        items: &[T],
        key: impl Fn(&T) -> Key,
        config: VirtualListConfig,
    ) -> Option<Task<Message>> {
        let index = items.iter().position(|item| key(item) == selected)?;
        Some(self.scroll_to_item(index, items.len(), config))
    }

    /// Synchronizes the native scrollable with this state's current offset.
    pub fn sync_scroll<Message>(
        &self,
        item_count: usize,
        config: VirtualListConfig,
    ) -> Task<Message> {
        iced::widget::operation::scroll_to(
            self.scroll_id(),
            iced::widget::operation::AbsoluteOffset {
                x: None,
                y: Some(self.effective_offset(item_count, config)),
            },
        )
    }

    fn reveal(&mut self, index: usize, item_count: usize, config: VirtualListConfig) {
        let top = index as f64 * f64::from(config.row_height);
        let bottom = top + f64::from(config.row_height);
        let offset = f64::from(self.scroll_offset);
        let viewport_bottom = offset + f64::from(config.viewport_height);
        if top < offset {
            self.set_offset(top as f32, item_count, config);
        } else if bottom > viewport_bottom {
            self.set_offset(
                (bottom - f64::from(config.viewport_height)) as f32,
                item_count,
                config,
            );
        }
    }

    fn set_offset(&mut self, offset: f32, item_count: usize, config: VirtualListConfig) {
        self.scroll_offset = if offset.is_finite() {
            offset.clamp(0.0, config.max_offset(item_count))
        } else {
            0.0
        };
    }

    fn effective_offset(&self, item_count: usize, config: VirtualListConfig) -> f32 {
        self.scroll_offset.clamp(0.0, config.max_offset(item_count))
    }

    fn widget_id(&self) -> iced::advanced::widget::Id {
        format!("{}/focus", self.id).into()
    }

    fn scroll_id(&self) -> iced::advanced::widget::Id {
        format!("{}/scroll", self.id).into()
    }
}

impl<Key> Default for VirtualListState<Key>
where
    Key: Copy + Eq + Hash,
{
    fn default() -> Self {
        Self::new("virtual-list")
    }
}

fn navigation_index(
    selected: Option<usize>,
    item_count: usize,
    navigation: VirtualListNavigation,
    page: usize,
) -> Option<usize> {
    if item_count == 0 {
        return None;
    }
    let last = item_count - 1;
    Some(match navigation {
        VirtualListNavigation::Home => 0,
        VirtualListNavigation::End => last,
        VirtualListNavigation::Down => {
            selected.map_or(0, |index| index.saturating_add(1).min(last))
        }
        VirtualListNavigation::Up => selected.map_or(last, |index| index.saturating_sub(1)),
        VirtualListNavigation::PageDown => {
            selected.map_or(0, |index| index.saturating_add(page).min(last))
        }
        VirtualListNavigation::PageUp => selected.map_or(last, |index| index.saturating_sub(page)),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MountedKey<Key> {
    Top,
    Item(Key),
    Bottom,
}

/// Builds a fixed-height keyed virtual list.
///
/// `view` is called exactly once for each row in [`VirtualListState::mounted_range`].
/// It is never called for offscreen items outside overscan.
/// `collection_label` supplies the accessible name for the list.
/// `label` supplies the AccessKit name for each mounted item.
#[allow(clippy::too_many_arguments)]
pub fn virtual_list<'a, T, Key, Message, Theme, Renderer>(
    state: &VirtualListState<Key>,
    items: &'a [T],
    config: VirtualListConfig,
    collection_label: impl Into<String>,
    key: impl Fn(&T) -> Key,
    label: impl Fn(&T) -> String,
    view: impl Fn(usize, &'a T, bool) -> Element<'a, Message, Theme, Renderer>,
    on_event: impl Fn(VirtualListEvent<Key>) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Key: Copy + Eq + Hash + 'static,
    Message: Clone + 'static,
    Theme: container::Catalog + scrollable::Catalog + 'a,
    Renderer: text::Renderer + iced::advanced::Renderer + 'a,
{
    let scroll_offset = state.effective_offset(items.len(), config);
    let range = config.mounted_range(items.len(), scroll_offset);
    let top = (range.start as f64 * f64::from(config.row_height)).min(f64::from(f32::MAX)) as f32;
    let bottom = (config.total_height(items.len())
        - (range.end as f64 * f64::from(config.row_height)) as f32)
        .max(0.0);
    let mut children = Vec::with_capacity(range.len().saturating_add(2));
    children.push((
        MountedKey::Top,
        space().height(top).width(Length::Fill).into(),
    ));
    let mut mounted = Vec::with_capacity(range.len());
    for index in range.clone() {
        let item = &items[index];
        let item_key = key(item);
        let selected = state.selected == Some(item_key);
        let row = container(view(index, item, selected))
            .width(Length::Fill)
            .height(config.row_height);
        let semantic_key = stable_key_hash(&item_key);
        let logical_id = format!("{}/item/{semantic_key:016x}", state.id);
        let row: Element<'a, Message, Theme, Renderer> = accessible(
            row,
            stable_item_id(&state.id, &item_key),
            crate::Role::ListItem,
        )
        .logical_id(logical_id)
        .label(label(item))
        .position_in_set(index.saturating_add(1))
        .size_of_set(items.len())
        .selected(selected)
        .into();
        children.push((MountedKey::Item(item_key), row));
        mounted.push((index, item_key));
    }
    children.push((
        MountedKey::Bottom,
        space().height(bottom).width(Length::Fill).into(),
    ));

    let on_event: Rc<dyn Fn(VirtualListEvent<Key>) -> Message + 'a> = Rc::new(on_event);
    let on_scroll = Rc::clone(&on_event);
    let content = scrollable(keyed_column(children).width(Length::Fill))
        .id(state.scroll_id())
        .width(Length::Fill)
        .height(config.viewport_height)
        .on_scroll(move |viewport| {
            on_scroll(VirtualListEvent::Scrolled {
                offset_y: viewport.absolute_offset().y,
            })
        });
    let list = VirtualList {
        content: content.into(),
        id: state.widget_id(),
        mounted,
        config,
        scroll_offset,
        on_event,
    };
    accessible(
        Element::new(list),
        StableId::new(&state.id),
        crate::Role::List,
    )
    .logical_id(state.id.clone())
    .label(collection_label)
    .focus_descendant()
    .size_of_set(items.len())
    .into()
}

fn stable_item_id<Key: Hash>(scope: &str, key: &Key) -> StableId {
    StableId::new(format!("{scope}/item/{:016x}", stable_key_hash(key)))
}

fn stable_key_hash<Key: Hash>(key: &Key) -> u64 {
    let mut hasher = StableKeyHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

struct StableKeyHasher(u64);

impl StableKeyHasher {
    const fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
}

impl Hasher for StableKeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.write(&value.to_le_bytes());
    }

    fn write_u16(&mut self, value: u16) {
        self.write(&value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.write(&value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.write(&value.to_le_bytes());
    }

    fn write_u128(&mut self, value: u128) {
        self.write(&value.to_le_bytes());
    }

    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }

    fn write_i8(&mut self, value: i8) {
        self.write(&value.to_le_bytes());
    }

    fn write_i16(&mut self, value: i16) {
        self.write(&value.to_le_bytes());
    }

    fn write_i32(&mut self, value: i32) {
        self.write(&value.to_le_bytes());
    }

    fn write_i64(&mut self, value: i64) {
        self.write(&value.to_le_bytes());
    }

    fn write_i128(&mut self, value: i128) {
        self.write(&value.to_le_bytes());
    }

    fn write_isize(&mut self, value: isize) {
        self.write_i64(value as i64);
    }
}

struct VirtualList<'a, Key, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    id: iced::advanced::widget::Id,
    mounted: Vec<(usize, Key)>,
    config: VirtualListConfig,
    scroll_offset: f32,
    on_event: Rc<dyn Fn(VirtualListEvent<Key>) -> Message + 'a>,
}

#[derive(Default)]
struct State {
    focused: bool,
    focus_visible: bool,
}

impl Focusable for State {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
        self.focus_visible = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
        self.focus_visible = false;
    }
}

impl<Key, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for VirtualList<'_, Key, Message, Theme, Renderer>
where
    Key: Copy + 'static,
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.focusable(
            Some(&self.id),
            layout.bounds(),
            tree.state.downcast_mut::<State>(),
        );
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let pointer_position = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.focused = false;
                state.focus_visible = false;
                cursor.position()
            }
            Event::Touch(iced::touch::Event::FingerPressed { position, .. }) => {
                state.focused = layout.bounds().contains(*position);
                state.focus_visible = false;
                Some(*position)
            }
            _ => None,
        };
        if pointer_position.is_some_and(|position| !layout.bounds().contains(position)) {
            state.focused = false;
            state.focus_visible = false;
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) && let Some(position) =
            pointer_position.filter(|position| layout.bounds().contains(*position))
        {
            state.focused = true;
            state.focus_visible = false;
            let local_y = position.y - layout.bounds().y;
            let index = ((self.scroll_offset + local_y) / self.config.row_height).floor() as usize;
            if let Some((index, key)) = self.mounted.iter().find(|(mounted, _)| *mounted == index) {
                shell.publish((self.on_event)(VirtualListEvent::Select {
                    index: *index,
                    key: *key,
                }));
            }
            shell.capture_event();
            return;
        }

        if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
            && state.focused
        {
            let navigation = match key {
                keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                    Some(VirtualListNavigation::Up)
                }
                keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                    Some(VirtualListNavigation::Down)
                }
                keyboard::Key::Named(keyboard::key::Named::Home) => {
                    Some(VirtualListNavigation::Home)
                }
                keyboard::Key::Named(keyboard::key::Named::End) => Some(VirtualListNavigation::End),
                keyboard::Key::Named(keyboard::key::Named::PageUp) => {
                    Some(VirtualListNavigation::PageUp)
                }
                keyboard::Key::Named(keyboard::key::Named::PageDown) => {
                    Some(VirtualListNavigation::PageDown)
                }
                _ => None,
            };
            if let Some(navigation) = navigation {
                shell.publish((self.on_event)(VirtualListEvent::Navigate(navigation)));
                shell.capture_event();
            }
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        crate::dev::record_draw_probe("virtual-list");
        if tree.state.downcast_ref::<State>().focus_visible {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: iced::Border {
                        color: style.text_color,
                        width: 2.0,
                        radius: 3.0.into(),
                    },
                    ..renderer::Quad::default()
                },
                iced::Color::TRANSPARENT,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            self.content.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            )
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ROOT_ID, SnapshotOperation};
    use accesskit::{NodeId, TreeId};
    use iced::advanced::widget::Tree as WidgetTree;
    use iced::advanced::widget::operation::{self, Outcome};
    use iced::advanced::{Layout, renderer};
    use iced::{Font, Pixels, Point, Theme};
    use iced_test::futures::futures::StreamExt as _;
    use iced_test::runtime::UserInterface;
    use iced_test::runtime::user_interface;
    use std::cell::Cell;
    use std::fmt;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        List(VirtualListEvent<u64>),
        First(VirtualListEvent<u64>),
        Second(VirtualListEvent<u64>),
        Child,
        Input(String),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct DisplayCollision(u64);

    impl fmt::Display for DisplayCollision {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("same-display")
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct CollisionMessage;

    fn config() -> VirtualListConfig {
        VirtualListConfig::new(20.0, 100.0).unwrap().overscan(2)
    }

    fn renderer() -> iced_test::renderer::Renderer {
        iced_test::futures::futures::executor::block_on(
            <iced_test::renderer::Renderer as renderer::Headless>::new(
                Font::DEFAULT,
                Pixels(16.0),
                None,
            ),
        )
        .expect("headless renderer")
    }

    fn key_pressed(named: keyboard::key::Named) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(named),
            modified_key: keyboard::Key::Named(named),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        })
    }

    #[test]
    fn validates_fixed_geometry_and_bounds_empty_ranges() {
        assert_eq!(
            VirtualListConfig::new(0.0, 100.0),
            Err(VirtualListConfigError::RowHeight)
        );
        assert_eq!(
            VirtualListConfig::new(20.0, f32::NAN),
            Err(VirtualListConfigError::ViewportHeight)
        );
        let mut state = VirtualListState::<u64>::new("empty");
        state.reconcile::<u64>(&[], |key| *key, config()).unwrap();
        assert_eq!(state.visible_range(0, config()), 0..0);
        assert_eq!(state.mounted_range(0, config()), 0..0);
        let outcome = state.apply(
            VirtualListEvent::Navigate(VirtualListNavigation::End),
            &[],
            |key| *key,
            config(),
        );
        assert_eq!(outcome.selected, None);
        assert_eq!(
            state.reconcile(&[7_u64, 7], |key| *key, config()),
            Err(VirtualListReconcileError::DuplicateKey(7))
        );
    }

    #[test]
    fn direct_render_and_queries_share_ranges_across_count_and_config_changes() {
        let items: Vec<u64> = (0..100).collect();
        let state = VirtualListState::new("direct-render");
        let first = config();
        let builds = Cell::new(0_usize);
        let element: Element<'_, Message> = virtual_list(
            &state,
            &items,
            first,
            "Direct render results",
            |key| *key,
            |key| format!("Item {key}"),
            |index, _, _| {
                builds.set(builds.get() + 1);
                iced::widget::text(index).into()
            },
            Message::List,
        );
        assert_eq!(state.visible_range(items.len(), first), 0..5);
        assert_eq!(state.mounted_range(items.len(), first), 0..7);
        assert_eq!(builds.get(), 7);
        drop(element);

        let changed = VirtualListConfig::new(10.0, 35.0).unwrap().overscan(1);
        assert_eq!(state.visible_range(items.len(), changed), 0..4);
        assert_eq!(state.mounted_range(items.len(), changed), 0..5);
        assert_eq!(state.mounted_range(1, changed), 0..1);

        let mut scrolled = VirtualListState::new("shrinking-render");
        scrolled.apply(
            VirtualListEvent::Scrolled { offset_y: 1_900.0 },
            &items,
            |key| *key,
            first,
        );
        assert_eq!(scrolled.visible_range(1, first), 0..1);
        assert_eq!(scrolled.mounted_range(1, first), 0..1);
        let one = &items[..1];
        let one_build = Cell::new(0_usize);
        let element: Element<'_, Message> = virtual_list(
            &scrolled,
            one,
            first,
            "Shrunk results",
            |key| *key,
            |key| format!("Item {key}"),
            |_, _, _| {
                one_build.set(one_build.get() + 1);
                iced::widget::text("one").into()
            },
            Message::List,
        );
        assert_eq!(one_build.get(), 1);
        drop(element);
    }

    #[test]
    fn selection_follows_stable_key_and_clears_when_deleted() {
        let mut state = VirtualListState::new("reconcile");
        let items = [10_u64, 20, 30];
        state.apply(
            VirtualListEvent::Select { index: 1, key: 20 },
            &items,
            |key| *key,
            config(),
        );
        let reordered = [30, 10, 20];
        state.reconcile(&reordered, |key| *key, config()).unwrap();
        assert_eq!(state.selected(), Some(20));
        assert_eq!(state.selected_index(), Some(2));
        state.reconcile(&[30, 10], |key| *key, config()).unwrap();
        assert_eq!(state.selected(), None);
        assert_eq!(state.selected_index(), None);
    }

    #[test]
    fn semantic_identity_depends_on_scope_and_item_key_not_position() {
        let before_reorder = stable_item_id("list", &20_u64);
        let after_reorder = stable_item_id("list", &20_u64);
        assert_eq!(before_reorder, after_reorder);
        assert_ne!(before_reorder, stable_item_id("list", &30_u64));
        assert_ne!(before_reorder, stable_item_id("other-list", &20_u64));
    }

    #[test]
    fn equal_display_strings_do_not_alias_semantic_identity_across_reorder() {
        fn identities(items: &[DisplayCollision]) -> std::collections::HashMap<String, NodeId> {
            let mut state = VirtualListState::new("collision-list");
            state.reconcile(items, |item| *item, config()).unwrap();
            let element: Element<'_, CollisionMessage, Theme, iced_test::renderer::Renderer> =
                virtual_list(
                    &state,
                    items,
                    config(),
                    "Collision results",
                    |item| *item,
                    |item| format!("Item {}", item.0),
                    |_, item, _| iced::widget::text(item.0).into(),
                    |_event: VirtualListEvent<DisplayCollision>| CollisionMessage,
                );
            let mut renderer = renderer();
            let mut ui = UserInterface::build(
                element,
                Size::new(240.0, 100.0),
                user_interface::Cache::default(),
                &mut renderer,
            );
            let mut operation = SnapshotOperation::<CollisionMessage>::named("Collision test");
            ui.operate(&renderer, &mut operation::black_box(&mut operation));
            let Outcome::Some(snapshot) = operation.finish() else {
                panic!("snapshot operation did not finish");
            };
            snapshot
                .update
                .nodes
                .into_iter()
                .filter(|(_, node)| node.role() == crate::Role::ListItem)
                .map(|(id, node)| (node.label().expect("item label").to_owned(), id))
                .collect()
        }

        let before = identities(&[
            DisplayCollision(1),
            DisplayCollision(2),
            DisplayCollision(3),
        ]);
        let after = identities(&[
            DisplayCollision(3),
            DisplayCollision(1),
            DisplayCollision(2),
        ]);
        assert_ne!(before["Item 1"], before["Item 2"]);
        assert_eq!(before["Item 1"], after["Item 1"]);
        assert_eq!(before["Item 2"], after["Item 2"]);
        assert_eq!(before["Item 3"], after["Item 3"]);
    }

    #[test]
    fn keyboard_navigation_and_programmatic_scroll_are_bounded() {
        let items: Vec<u64> = (0..100).collect();
        let mut state = VirtualListState::new("keyboard");
        let navigate = |state: &mut VirtualListState<u64>, navigation| {
            state.apply(
                VirtualListEvent::Navigate(navigation),
                &items,
                |key| *key,
                config(),
            );
        };
        navigate(&mut state, VirtualListNavigation::End);
        assert_eq!(state.selected(), Some(99));
        assert_eq!(state.scroll_offset(), 1_900.0);
        navigate(&mut state, VirtualListNavigation::PageUp);
        assert_eq!(state.selected(), Some(94));
        navigate(&mut state, VirtualListNavigation::Up);
        assert_eq!(state.selected(), Some(93));
        navigate(&mut state, VirtualListNavigation::Home);
        assert_eq!(state.selected(), Some(0));
        navigate(&mut state, VirtualListNavigation::Up);
        assert_eq!(state.selected(), Some(0));
        navigate(&mut state, VirtualListNavigation::PageDown);
        assert_eq!(state.selected(), Some(5));
        navigate(&mut state, VirtualListNavigation::Down);
        assert_eq!(state.selected(), Some(6));
        let _: Task<()> = state.scroll_to_item(42, items.len(), config());
        assert_eq!(state.scroll_offset(), 840.0);
        assert_eq!(state.visible_range(items.len(), config()), 42..47);
        assert_eq!(state.mounted_range(items.len(), config()), 40..49);
        let task: Option<Task<()>> = state.scroll_to_key(84, &items, |key| *key, config());
        assert!(task.is_some());
        assert_eq!(state.scroll_offset(), 1_680.0);
        let missing: Option<Task<()>> = state.scroll_to_key(1_000, &items, |key| *key, config());
        assert!(missing.is_none());
        let _: Task<()> = state.scroll_to_item(usize::MAX, items.len(), config());
        assert_eq!(state.scroll_offset(), 1_680.0);
    }

    #[test]
    fn builds_only_visible_and_overscan_rows_for_one_hundred_thousand_items() {
        let items: Vec<u64> = (0..100_000).collect();
        let mut state = VirtualListState::new("performance");
        state.apply(
            VirtualListEvent::Scrolled {
                offset_y: 1_000_000.0,
            },
            &items,
            |key| *key,
            config(),
        );
        let builds = Cell::new(0_usize);
        let element: Element<'_, Message> = virtual_list(
            &state,
            &items,
            config(),
            "Performance results",
            |key| *key,
            |key| format!("Item {key}"),
            |index, _, _| {
                builds.set(builds.get() + 1);
                iced::widget::text(index).into()
            },
            Message::List,
        );
        assert_eq!(
            builds.get(),
            state.mounted_range(items.len(), config()).len()
        );
        assert!(builds.get() <= config().rows_per_page() + config().overscan_rows() * 2 + 1);
        let inspection = state.inspect(items.len(), config());
        assert_eq!(inspection.logical_items, 100_000);
        assert_eq!(inspection.mounted_rows, builds.get());
        assert_eq!(inspection.child_slots, builds.get() + 2);
        drop(element);
    }

    #[test]
    #[ignore = "100k-item performance contract run explicitly in CI"]
    fn performance_contract_one_thousand_100k_view_builds_stay_mounted_only() {
        const BUDGET: std::time::Duration = std::time::Duration::from_secs(2);
        let items: Vec<u64> = (0..100_000).collect();
        let mut state = VirtualListState::new("performance-contract");
        let builds = Cell::new(0_usize);
        let started = std::time::Instant::now();
        for pass in 0..1_000 {
            state.apply(
                VirtualListEvent::Scrolled {
                    offset_y: (pass * 1_997) as f32,
                },
                &items,
                |key| *key,
                config(),
            );
            let element: Element<'_, Message> = virtual_list(
                &state,
                &items,
                config(),
                "Performance contract results",
                |key| *key,
                |key| format!("Item {key}"),
                |index, _, _| {
                    builds.set(builds.get() + 1);
                    iced::widget::text(index).into()
                },
                Message::List,
            );
            let inspection = state.inspect(items.len(), config());
            assert!(inspection.mounted_rows <= 10);
            assert!(inspection.child_slots <= 12);
            drop(element);
        }
        let elapsed = started.elapsed();
        assert!(builds.get() <= 10_000);
        assert!(
            elapsed <= BUDGET,
            "1,000 virtual-list views over 100k items took {elapsed:?}; budget is {BUDGET:?}"
        );
        eprintln!(
            "1,000 virtual-list views over 100k items in {elapsed:?}; {} total rows built",
            builds.get()
        );
    }

    #[test]
    fn accesskit_exports_only_mounted_rows_with_collection_metadata() {
        let items: Vec<u64> = (0..100).collect();
        let mut state = VirtualListState::new("semantic-list");
        state.reconcile(&items, |key| *key, config()).unwrap();
        state.apply(
            VirtualListEvent::Select { index: 2, key: 2 },
            &items,
            |key| *key,
            config(),
        );
        let element: Element<'_, Message, Theme, iced_test::renderer::Renderer> = virtual_list(
            &state,
            &items,
            config(),
            "Semantic results",
            |key| *key,
            |key| format!("Item {key}"),
            |index, _, _| iced::widget::text(index).into(),
            Message::List,
        );
        let mut renderer = iced_test::futures::futures::executor::block_on(
            <iced_test::renderer::Renderer as renderer::Headless>::new(
                Font::DEFAULT,
                Pixels(16.0),
                None,
            ),
        )
        .expect("headless renderer");
        let mut ui = UserInterface::build(
            element,
            Size::new(240.0, 100.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let mut operation = SnapshotOperation::<Message>::named("Virtual list test");
        ui.operate(&renderer, &mut operation::black_box(&mut operation));
        let Outcome::Some(snapshot) = operation.finish() else {
            panic!("snapshot operation did not finish");
        };
        let (list_id, list) = snapshot
            .update
            .nodes
            .iter()
            .find(|(id, node)| *id != ROOT_ID && node.role() == crate::Role::List)
            .map(|(id, node)| (*id, node))
            .expect("list semantic node");
        assert_eq!(list.size_of_set(), Some(100));
        assert_eq!(list.label(), Some("Semantic results"));
        assert!(list.supports_action(crate::Action::Focus));
        let rows = snapshot
            .update
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == crate::Role::ListItem)
            .map(|(_, node)| node)
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), state.mounted_range(items.len(), config()).len());
        assert_eq!(rows[0].position_in_set(), Some(1));
        assert_eq!(rows[0].size_of_set(), Some(100));
        assert_eq!(rows[0].is_selected(), Some(false));
        assert_eq!(rows[2].label(), Some("Item 2"));
        assert_eq!(rows[2].is_selected(), Some(true));

        let focus = snapshot.dispatch(crate::ActionRequest {
            action: crate::Action::Focus,
            target_tree: TreeId::ROOT,
            target_node: list_id,
            data: None,
        });
        let mut stream = iced_test::runtime::task::into_stream(focus).expect("focus task");
        let action = iced_test::futures::futures::executor::block_on(stream.next())
            .expect("focus operation");
        let iced_test::runtime::Action::Widget(mut focus) = action else {
            panic!("focus dispatch must produce a widget operation");
        };
        ui.operate(&renderer, focus.as_mut());
        let mut operation = SnapshotOperation::<Message>::named("Focused virtual list test");
        ui.operate(&renderer, &mut operation::black_box(&mut operation));
        let Outcome::Some(focused) = operation.finish() else {
            panic!("focused snapshot operation did not finish");
        };
        assert_eq!(focused.update.focus, list_id);
        let mut messages = Vec::new();
        let _ = ui.update(
            &[key_pressed(keyboard::key::Named::End)],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(
            messages,
            vec![Message::List(VirtualListEvent::Navigate(
                VirtualListNavigation::End
            ))]
        );
    }

    #[test]
    fn headless_mouse_and_keyboard_emit_typed_interactions() {
        let items: Vec<u64> = (0..100).collect();
        let mut state = VirtualListState::new("headless-list");
        state.reconcile(&items, |key| *key, config()).unwrap();
        let element: Element<'_, Message, Theme, iced_test::renderer::Renderer> = virtual_list(
            &state,
            &items,
            config(),
            "Headless results",
            |key| *key,
            |key| format!("Item {key}"),
            |index, _, selected| {
                iced::widget::text(format!("row {index} selected={selected}")).into()
            },
            Message::List,
        );
        let mut renderer = iced_test::futures::futures::executor::block_on(
            <iced_test::renderer::Renderer as renderer::Headless>::new(
                Font::DEFAULT,
                Pixels(16.0),
                None,
            ),
        )
        .expect("headless renderer");
        let mut ui = UserInterface::build(
            element,
            Size::new(240.0, 100.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        ui.draw(
            &mut renderer,
            &Theme::Light,
            &renderer::Style::default(),
            mouse::Cursor::Unavailable,
        );
        let point = Point::new(10.0, 45.0);
        let mut messages = Vec::new();
        let _ = ui.update(
            &[Event::Mouse(mouse::Event::ButtonPressed(
                mouse::Button::Left,
            ))],
            mouse::Cursor::Available(point),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(
            messages,
            vec![Message::List(VirtualListEvent::Select { index: 2, key: 2 })]
        );
        let mut operation = SnapshotOperation::<Message>::named("Pointer-focused list");
        ui.operate(&renderer, &mut operation::black_box(&mut operation));
        let Outcome::Some(pointer_focused) = operation.finish() else {
            panic!("pointer-focused snapshot operation did not finish");
        };
        let list_id = pointer_focused
            .update
            .nodes
            .iter()
            .find(|(_, node)| node.role() == crate::Role::List)
            .map(|(id, _)| *id)
            .expect("list semantic node");
        assert_eq!(pointer_focused.update.focus, list_id);
        messages.clear();
        let _ = ui.update(
            &[Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::PageDown),
                modified_key: keyboard::Key::Named(keyboard::key::Named::PageDown),
                physical_key: keyboard::key::Physical::Code(keyboard::key::Code::PageDown),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::default(),
                text: None,
                repeat: false,
            })],
            mouse::Cursor::Available(point),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(
            messages,
            vec![Message::List(VirtualListEvent::Navigate(
                VirtualListNavigation::PageDown
            ))]
        );
    }

    #[test]
    fn interactive_row_content_captures_click_before_row_selection() {
        let items: Vec<u64> = (0..100).collect();
        let state = VirtualListState::new("interactive-row-list");
        let element: Element<'_, Message, Theme, iced_test::renderer::Renderer> = virtual_list(
            &state,
            &items,
            config(),
            "Interactive results",
            |key| *key,
            |key| format!("Item {key}"),
            |_, _, _| {
                iced::widget::button("Child action")
                    .on_press(Message::Child)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            },
            Message::List,
        );
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            element,
            Size::new(240.0, 100.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let mut messages = Vec::new();
        let point = Point::new(20.0, 10.0);
        let _ = ui.update(
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(messages, vec![Message::Child]);
        messages.clear();
        let _ = ui.update(
            &[key_pressed(keyboard::key::Named::ArrowDown)],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert!(messages.is_empty());
    }

    #[test]
    fn native_scrollbar_press_and_drag_never_select_a_row() {
        let items: Vec<u64> = (0..100).collect();
        let state = VirtualListState::new("scrollbar-list");
        let element: Element<'_, Message, Theme, iced_test::renderer::Renderer> = virtual_list(
            &state,
            &items,
            config(),
            "Scrollbar results",
            |key| *key,
            |key| format!("Item {key}"),
            |index, _, _| iced::widget::text(index).into(),
            Message::List,
        );
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            element,
            Size::new(240.0, 100.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        ui.draw(
            &mut renderer,
            &Theme::Light,
            &renderer::Style::default(),
            mouse::Cursor::Unavailable,
        );
        let mut messages = Vec::new();
        let press = Point::new(239.0, 40.0);
        let _ = ui.update(
            &[Event::Mouse(mouse::Event::ButtonPressed(
                mouse::Button::Left,
            ))],
            mouse::Cursor::Available(press),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        let drag = Point::new(239.0, 85.0);
        let _ = ui.update(
            &[Event::Mouse(mouse::Event::CursorMoved { position: drag })],
            mouse::Cursor::Available(drag),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert!(messages.iter().any(|message| matches!(
            message,
            Message::List(VirtualListEvent::Scrolled { offset_y }) if *offset_y > 0.0
        )));
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, Message::List(VirtualListEvent::Select { .. })))
        );
    }

    #[test]
    fn pointer_focus_moves_between_lists_and_clears_for_text_input() {
        let items: Vec<u64> = (0..100).collect();
        let first = VirtualListState::new("first-list");
        let second = VirtualListState::new("second-list");
        let first_list: Element<'_, Message, Theme, iced_test::renderer::Renderer> = virtual_list(
            &first,
            &items,
            config(),
            "First results",
            |key| *key,
            |key| format!("First item {key}"),
            |index, _, _| iced::widget::text(index).into(),
            Message::First,
        );
        let second_list: Element<'_, Message, Theme, iced_test::renderer::Renderer> = virtual_list(
            &second,
            &items,
            config(),
            "Second results",
            |key| *key,
            |key| format!("Second item {key}"),
            |index, _, _| iced::widget::text(index).into(),
            Message::Second,
        );
        let input: Element<'_, Message, Theme, iced_test::renderer::Renderer> =
            iced::widget::container(
                iced::widget::text_input("Filter", "").on_input(Message::Input),
            )
            .height(36.0)
            .into();
        let element: Element<'_, Message, Theme, iced_test::renderer::Renderer> =
            iced::widget::column![first_list, second_list, input].into();
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            element,
            Size::new(240.0, 236.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let mut messages = Vec::new();
        for point in [Point::new(10.0, 40.0), Point::new(10.0, 140.0)] {
            let _ = ui.update(
                &[Event::Mouse(mouse::Event::ButtonPressed(
                    mouse::Button::Left,
                ))],
                mouse::Cursor::Available(point),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            messages.clear();
        }
        let _ = ui.update(
            &[key_pressed(keyboard::key::Named::ArrowDown)],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(
            messages,
            vec![Message::Second(VirtualListEvent::Navigate(
                VirtualListNavigation::Down
            ))]
        );

        messages.clear();
        let input_point = Point::new(20.0, 218.0);
        let _ = ui.update(
            &[Event::Mouse(mouse::Event::ButtonPressed(
                mouse::Button::Left,
            ))],
            mouse::Cursor::Available(input_point),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        messages.clear();
        let _ = ui.update(
            &[key_pressed(keyboard::key::Named::ArrowDown)],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert!(
            messages
                .iter()
                .all(|message| matches!(message, Message::Input(_)))
        );

        messages.clear();
        let first_point = Point::new(10.0, 40.0);
        let finger = iced::touch::Finger(7);
        let _ = ui.update(
            &[
                Event::Touch(iced::touch::Event::FingerPressed {
                    id: finger,
                    position: first_point,
                }),
                Event::Touch(iced::touch::Event::FingerLifted {
                    id: finger,
                    position: first_point,
                }),
            ],
            mouse::Cursor::Available(first_point),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        messages.clear();
        let _ = ui.update(
            &[key_pressed(keyboard::key::Named::Home)],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(
            messages,
            vec![Message::First(VirtualListEvent::Navigate(
                VirtualListNavigation::Home
            ))]
        );

        messages.clear();
        let _ = ui.update(
            &[Event::Touch(iced::touch::Event::FingerPressed {
                id: iced::touch::Finger(8),
                position: input_point,
            })],
            mouse::Cursor::Available(input_point),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        messages.clear();
        let _ = ui.update(
            &[key_pressed(keyboard::key::Named::End)],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert!(
            messages
                .iter()
                .all(|message| matches!(message, Message::Input(_)))
        );
    }

    #[derive(Default)]
    struct RecordingRenderer {
        quads: Vec<renderer::Quad>,
    }

    impl renderer::Renderer for RecordingRenderer {
        fn start_layer(&mut self, _bounds: Rectangle) {}
        fn end_layer(&mut self) {}
        fn start_transformation(&mut self, _transformation: iced::Transformation) {}
        fn end_transformation(&mut self) {}
        fn fill_quad(&mut self, quad: renderer::Quad, _background: impl Into<iced::Background>) {
            self.quads.push(quad);
        }
        fn reset(&mut self, _new_bounds: Rectangle) {}
        fn allocate_image(
            &mut self,
            _handle: &iced::advanced::image::Handle,
            _callback: impl FnOnce(
                Result<iced::advanced::image::Allocation, iced::advanced::image::Error>,
            ) + Send
            + 'static,
        ) {
            panic!("focus leaf never allocates images");
        }
    }

    struct FocusLeaf;

    impl Widget<Message, (), RecordingRenderer> for FocusLeaf {
        fn size(&self) -> Size<Length> {
            Size::new(Length::Fixed(100.0), Length::Fixed(40.0))
        }

        fn layout(
            &mut self,
            _tree: &mut WidgetTree,
            _renderer: &RecordingRenderer,
            _limits: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(Size::new(100.0, 40.0))
        }

        fn draw(
            &self,
            _tree: &WidgetTree,
            _renderer: &mut RecordingRenderer,
            _theme: &(),
            _style: &renderer::Style,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }
    }

    #[test]
    fn keyboard_or_accessibility_focus_draws_a_visible_list_outline() {
        let id: iced::advanced::widget::Id = "visible-list-focus".into();
        let list = VirtualList {
            content: Element::new(FocusLeaf),
            id: id.clone(),
            mounted: Vec::<(usize, u64)>::new(),
            config: config(),
            scroll_offset: 0.0,
            on_event: Rc::new(Message::List),
        };
        let mut element: Element<'_, Message, (), RecordingRenderer> = Element::new(list);
        let mut tree = WidgetTree::new(&element);
        let mut renderer = RecordingRenderer::default();
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(100.0, 40.0)),
        );
        let mut focus = operation::focusable::focus::<()>(id);
        element
            .as_widget_mut()
            .operate(&mut tree, Layout::new(&node), &renderer, &mut focus);
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &(),
            &renderer::Style {
                text_color: iced::Color::WHITE,
            },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(Size::new(100.0, 40.0)),
        );
        assert_eq!(renderer.quads.len(), 1);
        assert_eq!(renderer.quads[0].border.width, 2.0);
        assert_eq!(renderer.quads[0].border.color, iced::Color::WHITE);
    }
}
