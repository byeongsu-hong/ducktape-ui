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
use std::hash::Hash;
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
        first.saturating_sub(self.overscan)..end.saturating_add(self.overscan).min(item_count)
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
    visible_range: Range<usize>,
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
            visible_range: 0..0,
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

    pub fn visible_range(&self) -> Range<usize> {
        self.visible_range.clone()
    }

    pub fn inspect(&self, item_count: usize, config: VirtualListConfig) -> VirtualListInspection {
        let mounted_range = config.visible_range(item_count, self.scroll_offset);
        let mounted_rows = mounted_range.len();
        VirtualListInspection {
            logical_items: item_count,
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
        let previous_range = self.visible_range.clone();
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
            visible_range_changed: self.visible_range != previous_range,
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
            self.visible_range = config.visible_range(item_count, self.scroll_offset);
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
    pub fn sync_scroll<Message>(&self) -> Task<Message> {
        iced::widget::operation::scroll_to(
            self.scroll_id(),
            iced::widget::operation::AbsoluteOffset {
                x: None,
                y: Some(self.scroll_offset),
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
        self.visible_range = config.visible_range(item_count, self.scroll_offset);
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
/// `view` is called exactly once for each row in [`VirtualListState::visible_range`].
/// It is never called for offscreen items outside overscan.
/// `label` supplies the AccessKit name for each mounted item.
pub fn virtual_list<'a, T, Key, Message, Theme, Renderer>(
    state: &VirtualListState<Key>,
    items: &'a [T],
    config: VirtualListConfig,
    key: impl Fn(&T) -> Key,
    label: impl Fn(&T) -> String,
    view: impl Fn(usize, &'a T, bool) -> Element<'a, Message, Theme, Renderer>,
    on_event: impl Fn(VirtualListEvent<Key>) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Key: Copy + Eq + Hash + fmt::Display + 'static,
    Message: Clone + 'static,
    Theme: container::Catalog + scrollable::Catalog + 'a,
    Renderer: text::Renderer + iced::advanced::Renderer + 'a,
{
    let range = config.visible_range(items.len(), state.scroll_offset);
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
        let logical_id = format!("{}/item/{item_key}", state.id);
        let row: Element<'a, Message, Theme, Renderer> = accessible(
            row,
            stable_item_id(&state.id, item_key),
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
        scroll_offset: state.scroll_offset,
        on_event,
    };
    accessible(
        Element::new(list),
        StableId::new(&state.id),
        crate::Role::List,
    )
    .logical_id(state.id.clone())
    .size_of_set(items.len())
    .into()
}

fn stable_item_id<Key: fmt::Display>(scope: &str, key: Key) -> StableId {
    StableId::new(format!("{scope}/item/{key}"))
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
}

impl Focusable for State {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
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
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(layout.bounds()) =>
            {
                state.focused = true;
                if let Some(position) = cursor.position_in(layout.bounds()) {
                    let index = ((self.scroll_offset + position.y) / self.config.row_height).floor()
                        as usize;
                    if let Some((index, key)) =
                        self.mounted.iter().find(|(mounted, _)| *mounted == index)
                    {
                        shell.publish((self.on_event)(VirtualListEvent::Select {
                            index: *index,
                            key: *key,
                        }));
                    }
                }
                shell.capture_event();
                return;
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) if state.focused => {
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
                    keyboard::Key::Named(keyboard::key::Named::End) => {
                        Some(VirtualListNavigation::End)
                    }
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
                    return;
                }
            }
            _ => {}
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
    use iced::advanced::widget::operation::{self, Outcome};
    use iced::{Font, Pixels, Point, Theme};
    use iced_test::runtime::UserInterface;
    use iced_test::runtime::user_interface;
    use std::cell::Cell;

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Message {
        List(VirtualListEvent<u64>),
    }

    fn config() -> VirtualListConfig {
        VirtualListConfig::new(20.0, 100.0).unwrap().overscan(2)
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
        assert_eq!(state.visible_range(), 0..0);
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
        let before_reorder = stable_item_id("list", 20_u64);
        let after_reorder = stable_item_id("list", 20_u64);
        assert_eq!(before_reorder, after_reorder);
        assert_ne!(before_reorder, stable_item_id("list", 30_u64));
        assert_ne!(before_reorder, stable_item_id("other-list", 20_u64));
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
        assert_eq!(state.visible_range(), 40..49);
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
            |key| *key,
            |key| format!("Item {key}"),
            |index, _, _| {
                builds.set(builds.get() + 1);
                iced::widget::text(index).into()
            },
            Message::List,
        );
        assert_eq!(builds.get(), state.visible_range().len());
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
        let list = snapshot
            .update
            .nodes
            .iter()
            .find(|(id, node)| *id != ROOT_ID && node.role() == crate::Role::List)
            .map(|(_, node)| node)
            .expect("list semantic node");
        assert_eq!(list.size_of_set(), Some(100));
        let rows = snapshot
            .update
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == crate::Role::ListItem)
            .map(|(_, node)| node)
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), state.visible_range().len());
        assert_eq!(rows[0].position_in_set(), Some(1));
        assert_eq!(rows[0].size_of_set(), Some(100));
        assert_eq!(rows[0].is_selected(), Some(false));
        assert_eq!(rows[2].label(), Some("Item 2"));
        assert_eq!(rows[2].is_selected(), Some(true));
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
}
