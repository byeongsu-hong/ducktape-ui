//! Runtime support for generated Ice applications.

mod boot;
pub use boot::boot_dispatch;
mod dashed_border;
#[cfg(feature = "data-grid")]
mod data_grid;
#[doc(hidden)]
pub mod dev;
mod dynamic_themer;
mod flex;
mod hover_reveal;
mod log_timeline;
mod memo_lazy;
mod press_area;
mod qr;
mod resize_handle;
mod responsive;
pub mod rev;
mod rev_memo;
#[cfg(feature = "full-runtime")]
pub mod rich_text_editor;
mod scroll_anchor;
mod secret;
mod selectable_text;
pub mod selection;
mod stack_relief;
pub mod template;
#[doc(hidden)]
#[cfg(feature = "test-runtime")]
pub mod testing;
#[cfg(not(feature = "test-runtime"))]
#[path = "testing_minimal.rs"]
pub mod testing;
pub mod tray;
mod tree_view;
pub mod view_tree;
mod virtual_children;
mod virtual_list;
mod virtual_scroll;
mod virtualization;
mod zstack;

pub use dashed_border::*;
#[cfg(feature = "data-grid")]
pub use data_grid::*;
pub use dynamic_themer::*;
pub use flex::*;
pub use hover_reveal::*;
pub use log_timeline::*;
pub use memo_lazy::*;
pub use press_area::*;
pub use qr::*;
pub use resize_handle::*;
pub use responsive::*;
pub use rev_memo::*;
#[cfg(feature = "full-runtime")]
pub use rich_text_editor::{ContentVersion, EditorChange, RichTextEditor};
pub use scroll_anchor::*;
pub use secret::{Secret, SecretStore};
pub use selectable_text::*;
pub use stack_relief::*;
pub use tree_view::*;
pub use virtual_children::*;
pub use virtual_list::*;
pub use virtual_scroll::*;
pub use zstack::*;

#[cfg(feature = "data-grid")]
pub use accesskit::Live as AccessibilityLive;
pub use accesskit::SortDirection as AccessibilitySortDirection;
pub use accesskit::{Action, ActionRequest, Node, NodeId, Role, Toggled, TreeUpdate};

use accesskit::{Rect, Tree, TreeId};
use iced::advanced::widget::operation::{
    self, Focusable, Operation, Outcome, Scrollable, TextInput, focusable,
};
use iced::advanced::widget::{self, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::keyboard::{self, key};
use iced::{Element, Event, Length, Padding, Rectangle, Size, Subscription, Task, Vector};
use std::any::Any;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

const ROOT_ID: NodeId = NodeId(0);

/// Stores state for component scopes that only live while mounted.
///
/// Pruning happens one pass late, at the START of the next render, and that is
/// deliberate: `view` returning is not the end of building the tree. A
/// `responsive` (and any other deferred builder) constructs its subtree during
/// layout, so components under one call `mount` after `finish_render` has
/// already run. Pruning there saw an empty active set, dropped their state, and
/// the next pass built it again from scratch — which for ordinary state is
/// invisible, and for an animation means restarting the motion every frame.
/// Holding the root until the next `begin_render` lets the active set collect
/// the whole pass, deferred builders included.
#[derive(Debug)]
pub struct MountedComponentState<T> {
    values: RefCell<HashMap<String, T>>,
    active: RefCell<HashSet<String>>,
    /// The root whose finished render is still waiting to be pruned.
    pending: RefCell<Option<String>>,
    next_generation: Cell<u64>,
    /// Scopes whose `boot` already fired; pruned with their values, so an
    /// instance that leaves and comes back boots again.
    booted: RefCell<HashSet<String>>,
}

impl<T> Default for MountedComponentState<T> {
    fn default() -> Self {
        Self {
            values: RefCell::new(HashMap::new()),
            active: RefCell::new(HashSet::new()),
            pending: RefCell::new(None),
            next_generation: Cell::new(0),
            booted: RefCell::new(HashSet::new()),
        }
    }
}

impl<T> MountedComponentState<T> {
    /// Prunes the previous render's scopes, then starts tracking a new one.
    pub fn begin_render(&self) {
        if let Some(root) = self.pending.borrow_mut().take() {
            self.prune(&root);
        }
        self.active.borrow_mut().clear();
    }

    /// Marks a component scope as present in the current render.
    pub fn mount(&self, scope: String) {
        self.active.borrow_mut().insert(scope);
    }

    /// Marks a component scope as present, answering whether this is the
    /// instance's FIRST sighting — the caller queues the boot message it
    /// builds from the render site's prop values. The mark is pruned with
    /// the instance, so a scope that leaves the tree and comes back boots
    /// again.
    pub fn mount_boot(&self, scope: String) -> bool {
        let first = self.booted.borrow_mut().insert(scope.clone());
        self.active.borrow_mut().insert(scope);
        first
    }

    /// Records that `root` finished rendering. Scopes under it that never
    /// mounted are dropped at the next [`Self::begin_render`].
    pub fn finish_render(&self, root: &str) {
        self.pending.borrow_mut().replace(root.to_owned());
    }

    fn prune(&self, root: &str) {
        let active = self.active.borrow();
        let survives = |scope: &str| {
            let suffix = scope.strip_prefix(root);
            !suffix.is_some_and(|suffix| suffix.is_empty() || suffix.starts_with('/'))
                || active.contains(scope)
        };
        self.values.borrow_mut().retain(|scope, _| survives(scope));
        self.booted.borrow_mut().retain(|scope| survives(scope));
    }

    /// Every live instance scope: the ones sighted by the current render
    /// pass plus the ones holding materialized state. A freshly mounted
    /// instance has no `values` entry until its first delivered event, so
    /// a harness that just rendered must see it HERE.
    pub fn scopes(&self) -> Vec<String> {
        let values = self.values.borrow();
        let active = self.active.borrow();
        let mut scopes: Vec<String> = values.keys().cloned().collect();
        for scope in active.iter() {
            if !values.contains_key(scope) {
                scopes.push(scope.clone());
            }
        }
        scopes
    }

    /// Borrows all mounted scope values.
    pub fn values(&self) -> Ref<'_, HashMap<String, T>> {
        self.values.borrow()
    }

    /// Mutably borrows all mounted scope values.
    pub fn values_mut(&self) -> RefMut<'_, HashMap<String, T>> {
        self.values.borrow_mut()
    }

    /// Returns a render-lifetime-stable generation for async completion filters.
    pub fn next_generation(&self) -> u64 {
        let next = self.next_generation.get().wrapping_add(1);
        self.next_generation.set(next);
        next
    }
}

/// A deterministic identity for one semantic node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StableId(NodeId);

impl StableId {
    /// Hashes a compiler-owned key with a stable FNV-1a hash.
    pub fn new(key: impl AsRef<str>) -> Self {
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in key.as_ref().as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Self(NodeId(if hash == 0 { 1 } else { hash }))
    }

    pub(crate) const fn from_node_id(node_id: NodeId) -> Self {
        Self(node_id)
    }

    /// Returns the AccessKit node identity.
    pub const fn node_id(self) -> NodeId {
        self.0
    }

    /// Returns the corresponding Iced widget identity used for focus actions.
    pub fn widget_id(self) -> widget::Id {
        format!("__ice_accessibility/{}", self.0.0).into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusBehavior {
    None,
    Wrapper,
    Descendant,
}

#[derive(Clone, PartialEq)]
struct SemanticSnapshot {
    // This metadata stays independent of `Message` so test inspection can
    // retain it through `Element::map`, whose wrapper changes the message type
    // without changing the accessible widget's stored state.
    id: StableId,
    logical_id: Option<String>,
    source: Option<testing::Location>,
    role: Role,
    label: Option<String>,
    description: Option<String>,
    value: Option<String>,
    checked: Option<bool>,
    selected: Option<bool>,
    expanded: Option<bool>,
    level: Option<usize>,
    row_count: Option<usize>,
    column_count: Option<usize>,
    row_index: Option<usize>,
    column_index: Option<usize>,
    sort_direction: Option<accesskit::SortDirection>,
    position_in_set: Option<usize>,
    size_of_set: Option<usize>,
    active_descendant: Option<StableId>,
    /// A live region: assistive technology announces this node's value when
    /// it changes, without the user moving to it.
    live: Option<accesskit::Live>,
    /// A slider's or progress bar's number, exported beside its text value so
    /// a screen reader can read the position within the range and step it.
    /// Boxed: every accessible node carries this state, and only range
    /// controls fill it.
    numeric: Option<Box<NumericRange>>,
    disabled: bool,
    focus: FocusBehavior,
    focused: bool,
    supports_activate: bool,
    supports_increment: bool,
    supports_decrement: bool,
}

/// The numeric contract of a range control: where it is, where it can go, and
/// how far one step moves it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericRange {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: Option<f64>,
}

#[derive(Clone)]
struct Semantics<Message> {
    snapshot: SemanticSnapshot,
    /// The id `operate` hands to its `custom` reader, when the caller named
    /// one. Left unset otherwise: the id derived from `snapshot.id` is a
    /// formatted `String` that only `operate` ever reads, so deriving it there
    /// costs one allocation per pass rather than one per node per build plus
    /// another for every `diff` that clones the node.
    focus_id: Option<widget::Id>,
    activate: Option<Message>,
    /// The messages one accessibility step up or down produces — the change
    /// route called with the next value, already clamped to the range. Boxed
    /// for the same reason as `numeric`: two `Message`s on every node would
    /// grow every widget tree for the few sliders in it.
    steps: Option<Box<StepMessages<Message>>>,
}

/// One accessibility step in each direction; `None` at that end of the range.
#[derive(Clone)]
struct StepMessages<Message> {
    increment: Option<Message>,
    decrement: Option<Message>,
}

impl<Message> Default for StepMessages<Message> {
    fn default() -> Self {
        Self {
            increment: None,
            decrement: None,
        }
    }
}

impl<Message> Semantics<Message> {
    fn increment(&self) -> Option<&Message> {
        self.steps
            .as_ref()
            .and_then(|steps| steps.increment.as_ref())
    }

    fn decrement(&self) -> Option<&Message> {
        self.steps
            .as_ref()
            .and_then(|steps| steps.decrement.as_ref())
    }

    fn set_step(&mut self, up: bool, message: Option<Message>) {
        let steps = self.steps.get_or_insert_with(Box::default);
        if up {
            steps.increment = message;
        } else {
            steps.decrement = message;
        }
        if steps.increment.is_none() && steps.decrement.is_none() {
            self.steps = None;
        }
    }
}

impl<Message> std::ops::Deref for Semantics<Message> {
    type Target = SemanticSnapshot;

    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
}

impl<Message> std::ops::DerefMut for Semantics<Message> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.snapshot
    }
}

impl<Message> Semantics<Message> {
    fn new(id: StableId, role: Role) -> Self {
        let focus = match role {
            Role::Button | Role::DefaultButton | Role::CheckBox | Role::Switch => {
                FocusBehavior::Wrapper
            }
            Role::TextInput
            | Role::MultilineTextInput
            | Role::SearchInput
            | Role::PasswordInput
            | Role::Slider
            | Role::ComboBox => FocusBehavior::Descendant,
            _ => FocusBehavior::None,
        };

        Self {
            snapshot: SemanticSnapshot {
                id,
                logical_id: None,
                source: None,
                role,
                label: None,
                description: None,
                value: None,
                checked: None,
                selected: None,
                expanded: None,
                level: None,
                row_count: None,
                column_count: None,
                row_index: None,
                column_index: None,
                sort_direction: None,
                position_in_set: None,
                size_of_set: None,
                active_descendant: None,
                live: None,
                numeric: None,
                disabled: false,
                focus,
                focused: false,
                supports_activate: false,
                supports_increment: false,
                supports_decrement: false,
            },
            focus_id: None,
            activate: None,
            steps: None,
        }
    }
}

struct SemanticState<Message> {
    semantics: Semantics<Message>,
    focus_visible: bool,
}

impl<Message> Focusable for SemanticState<Message> {
    fn is_focused(&self) -> bool {
        self.semantics.focused
    }

    fn focus(&mut self) {
        self.semantics.focused = true;
        self.focus_visible = true;
    }

    fn unfocus(&mut self) {
        self.semantics.focused = false;
        self.focus_visible = false;
    }
}

struct SemanticEnd;

/// "The next semantic node is item `position` of `size`."
///
/// A column that mounts only part of its children cannot reach into their
/// semantics — it is handed built elements — but it can say this on the way
/// past, and the first node the child publishes takes it. That is what lets a
/// virtualized list read as a list: a screen reader is told the whole set even
/// though the tree holds one screenful of it.
///
/// An explicit `position_in_set` on the child wins, since the child knows more
/// about itself than the column does.
struct SetPosition {
    position: usize,
    size: usize,
}

struct WithoutFocus<'a> {
    inner: &'a mut dyn Operation,
}

impl Operation for WithoutFocus<'_> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        self.inner.traverse(&mut |inner| {
            let mut filtered = WithoutFocus { inner };
            operate(&mut filtered);
        });
    }

    fn container(&mut self, id: Option<&widget::Id>, bounds: Rectangle) {
        self.inner.container(id, bounds);
    }

    fn scrollable(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        self.inner
            .scrollable(id, bounds, content_bounds, translation, state);
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        state.unfocus();
    }

    fn text_input(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        state: &mut dyn TextInput,
    ) {
        self.inner.text_input(id, bounds, state);
    }

    fn text(&mut self, id: Option<&widget::Id>, bounds: Rectangle, text: &str) {
        self.inner.text(id, bounds, text);
    }

    fn custom(&mut self, id: Option<&widget::Id>, bounds: Rectangle, state: &mut dyn Any) {
        self.inner.custom(id, bounds, state);
    }

    fn finish(&self) -> Outcome<()> {
        self.inner.finish()
    }
}

/// The ink channel through which a generated button hands its
/// status-resolved text color to `color=inherit` svg content.
///
/// iced's inherited-ink channel (`renderer::Style.text_color`) reaches text
/// widgets but never an svg's style closure, so the generated button block
/// binds one of these cells instead: the button's style closure writes its
/// FINAL `text_color` (disabled pass included), and iced's button draw
/// resolves that closure before drawing content, so an svg style closure
/// reading the cell during the same draw always sees this frame's status ink.
pub type ButtonInk = std::rc::Rc<std::cell::Cell<iced::Color>>;

/// Creates the ink cell a generated button shares with its `color=inherit`
/// svg content. The initial value is never drawn: the button's style closure
/// overwrites it before any reader draws.
pub fn button_ink() -> ButtonInk {
    std::rc::Rc::new(std::cell::Cell::new(iced::Color::TRANSPARENT))
}

/// Wraps an Iced widget with semantics owned by Ice.
pub struct Accessible<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    semantics: Semantics<Message>,
    focus_ring: Option<FocusRing>,
}

/// Recipe-owned looks for the wrapper's keyboard focus ring.
///
/// The ring's visibility is not configurable: it always keys on the wrapper's
/// focus-visible state, so a pointer press never wears it and keyboard
/// traversal always does. Only its paint is the caller's.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FocusRing {
    color: iced::Color,
    radius: f32,
}

/// Creates an accessible wrapper around an Iced widget.
pub fn accessible<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    id: StableId,
    role: Role,
) -> Accessible<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    Accessible {
        content: content.into(),
        semantics: Semantics::new(id, role),
        focus_ring: None,
    }
}

impl<'a, Message, Theme, Renderer> Accessible<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    /// Retains the logical Ice selector used to build this semantic node.
    #[doc(hidden)]
    pub fn logical_id(mut self, id: impl Into<String>) -> Self {
        self.semantics.logical_id = Some(id.into());
        self.semantics.source = testing::current_render_source();
        self
    }

    /// [`Self::logical_id`] where the caller decides whether the key is worth
    /// keeping. A key the caller owns is moved, exactly as it would be.
    ///
    /// Only test inspection reads a logical id back, and the generated view
    /// already gates its two sibling facilities — the render-source push and
    /// the id registration — on `cfg(test)`. It passes `None` here for the
    /// same reason: outside a test the copy is built, stored and never read.
    /// Nothing about the widget changes either way, so the `widget::Id` a
    /// node carries has the same spelling in both builds.
    #[doc(hidden)]
    pub fn logical_id_maybe(self, id: Option<impl Into<String>>) -> Self {
        match id {
            Some(id) => self.logical_id(id),
            None => self,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.semantics.label = Some(label.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.semantics.description = Some(description.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.semantics.value = Some(value.into());
        self
    }

    pub fn value_maybe(mut self, value: Option<String>) -> Self {
        self.semantics.value = value;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.semantics.checked = Some(checked);
        self
    }

    /// Marks whether this semantic item is selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.semantics.selected = Some(selected);
        self
    }

    /// Marks a hierarchical item as expanded or collapsed.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.semantics.expanded = Some(expanded);
        self
    }

    /// Sets the one-based level of an item in a hierarchical collection.
    pub fn level(mut self, level: usize) -> Self {
        self.semantics.level = Some(level);
        self
    }

    /// Sets the total logical row count of a table or grid.
    pub fn row_count(mut self, count: usize) -> Self {
        self.semantics.row_count = Some(count);
        self
    }

    /// Sets the total logical column count of a table or grid.
    pub fn column_count(mut self, count: usize) -> Self {
        self.semantics.column_count = Some(count);
        self
    }

    /// Sets the one-based logical row index of a row or cell.
    pub fn row_index(mut self, index: usize) -> Self {
        self.semantics.row_index = Some(index);
        self
    }

    /// Sets the one-based logical column index of a header or cell.
    pub fn column_index(mut self, index: usize) -> Self {
        self.semantics.column_index = Some(index);
        self
    }

    /// Sets the current sort direction of a sortable column header.
    pub fn sort_direction(mut self, direction: accesskit::SortDirection) -> Self {
        self.semantics.sort_direction = Some(direction);
        self
    }

    /// Sets this item's one-based position in its logical collection.
    pub fn position_in_set(mut self, position: usize) -> Self {
        self.semantics.position_in_set = Some(position);
        self
    }

    /// Sets the total number of items in this semantic collection.
    pub fn size_of_set(mut self, size: usize) -> Self {
        self.semantics.size_of_set = Some(size);
        self
    }

    /// Identifies the currently active semantic descendant of this collection.
    pub fn active_descendant(mut self, id: StableId) -> Self {
        self.semantics.active_descendant = Some(id);
        self
    }

    /// Identifies the active descendant when the item is currently mounted.
    pub fn active_descendant_maybe(mut self, id: Option<StableId>) -> Self {
        self.semantics.active_descendant = id;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.semantics.disabled = disabled;
        self
    }

    /// Makes this node a live region: a screen reader announces its value
    /// when it changes — politely, after the current utterance, or
    /// assertively, interrupting it.
    pub fn live(mut self, live: accesskit::Live) -> Self {
        self.semantics.live = Some(live);
        self
    }

    pub fn focus_id(mut self, id: impl Into<widget::Id>) -> Self {
        self.semantics.focus_id = Some(id.into());
        self
    }

    /// Maps focus from a native focusable descendant onto this semantic node.
    pub fn focus_descendant(mut self) -> Self {
        self.semantics.focus = FocusBehavior::Descendant;
        self
    }

    pub fn on_activate(mut self, message: Message) -> Self {
        self.semantics.supports_activate = true;
        self.semantics.activate = Some(message);
        self
    }

    pub fn on_activate_maybe(mut self, message: Option<Message>) -> Self {
        self.semantics.supports_activate = message.is_some();
        self.semantics.activate = message;
        self
    }

    /// Exports a range control's number beside its text value: the current
    /// value, the range, and the size of one step when it has one.
    pub fn numeric(mut self, value: f64, min: f64, max: f64, step: Option<f64>) -> Self {
        self.semantics.numeric = Some(Box::new(NumericRange {
            value,
            min,
            max,
            step,
        }));
        self
    }

    /// The message one accessibility step up produces; `None` at the top of
    /// the range, which leaves the action unexported.
    pub fn on_increment_maybe(mut self, message: Option<Message>) -> Self {
        self.semantics.supports_increment = message.is_some();
        self.semantics.set_step(true, message);
        self
    }

    /// The message one accessibility step down produces; `None` at the bottom.
    pub fn on_decrement_maybe(mut self, message: Option<Message>) -> Self {
        self.semantics.supports_decrement = message.is_some();
        self.semantics.set_step(false, message);
        self
    }

    /// Styles the keyboard focus ring this wrapper draws when focus is
    /// visible. The default ring uses the ambient text color with a
    /// three-pixel radius; a styled ring keeps the two-pixel stroke and takes
    /// the given color and corner radius instead.
    pub fn focus_ring(mut self, color: iced::Color, radius: f32) -> Self {
        self.focus_ring = Some(FocusRing { color, radius });
        self
    }
}

/// Where a text input's caret sits, in grapheme indices into its value — the
/// unit iced's cursor counts in and the unit AccessKit's `character_lengths`
/// describe — plus the widget to steer when assistive technology moves it.
struct TextCaret {
    anchor: usize,
    focus: usize,
    target: widget::Id,
}

/// The paragraph type every renderer the runtime ships with shares, so a
/// wrapped `text_input`'s state can be recognised without a paragraph bound
/// on every wrapper: `Tree::tag` says whether the child is one.
type TextInputState = iced::widget::text_input::State<iced::advanced::graphics::text::Paragraph>;

fn text_caret(child: &widget::Tree, value: Option<&str>, target: &widget::Id) -> Option<TextCaret> {
    if child.tag != tree::Tag::of::<TextInputState>() {
        return None;
    }
    let value = iced::widget::text_input::Value::new(value?);
    let cursor = child.state.downcast_ref::<TextInputState>().cursor();
    let (anchor, focus) = match cursor.state(&value) {
        iced::widget::text_input::cursor::State::Index(index) => (index, index),
        iced::widget::text_input::cursor::State::Selection { start, end } => (start, end),
    };
    Some(TextCaret {
        anchor,
        focus,
        target: target.clone(),
    })
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Accessible<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SemanticState<Message>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(SemanticState {
            semantics: self.semantics.clone(),
            focus_visible: false,
        })
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        let state = tree.state.downcast_mut::<SemanticState<Message>>();
        let focused = state.semantics.focused;
        // The snapshot owns the node's strings, and the tree keeps a copy. A
        // pass that hands over the same snapshot compares it against that
        // copy instead of cloning it and dropping the old one — a few
        // `String`s per node per frame, on a screen where most nodes hold.
        if state.semantics.snapshot != self.semantics.snapshot {
            state.semantics.snapshot = self.semantics.snapshot.clone();
        }
        if state.semantics.focus_id != self.semantics.focus_id {
            state.semantics.focus_id = self.semantics.focus_id.clone();
        }
        state.semantics.activate = self.semantics.activate.clone();
        // Almost every node has no steps: `None == None` skips the clone.
        if state.semantics.steps.is_some() || self.semantics.steps.is_some() {
            state.semantics.steps = self.semantics.steps.clone();
        }
        state.semantics.focused = focused;
        if state.semantics.disabled {
            state.semantics.focused = false;
            state.focus_visible = false;
        }
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
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<SemanticState<Message>>();
        let focus_id = state
            .semantics
            .focus_id
            .clone()
            .unwrap_or_else(|| state.semantics.id.widget_id());
        if state.semantics.disabled {
            state.semantics.focused = false;
            state.focus_visible = false;
        }
        operation.custom(None, layout.bounds(), &mut state.semantics.snapshot);
        operation.custom(Some(&focus_id), layout.bounds(), state);
        if let Some(mut caret) = text_caret(
            &tree.children[0],
            state.semantics.value.as_deref(),
            &focus_id,
        ) {
            operation.custom(None, layout.bounds(), &mut caret);
        }

        if !state.semantics.disabled && state.semantics.focus == FocusBehavior::Wrapper {
            operation.focusable(Some(&focus_id), layout.bounds(), state);
        }

        if state.semantics.focus == FocusBehavior::Wrapper
            || (state.semantics.disabled && state.semantics.focus == FocusBehavior::Descendant)
        {
            operation.traverse(&mut |operation| {
                let mut operation = WithoutFocus { inner: operation };
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    &mut operation,
                );
            });
        } else {
            operation.traverse(&mut |operation| {
                self.content.as_widget_mut().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    operation,
                );
            });
        }

        operation.custom(None, layout.bounds(), &mut SemanticEnd);
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<SemanticState<Message>>();
        let wrapper_focus = state.semantics.focus == FocusBehavior::Wrapper;

        if wrapper_focus && !state.semantics.disabled {
            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(iced::touch::Event::FingerPressed { .. }) => {
                    state.semantics.focused = cursor.is_over(layout.bounds());
                    state.focus_visible = false;
                }
                _ => {}
            }
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

        if shell.is_event_captured() || state.semantics.disabled || !state.semantics.focused {
            return;
        }

        let Event::Keyboard(keyboard::Event::KeyPressed { key, repeat, .. }) = event else {
            return;
        };

        // The web's `:focus-visible` heuristic: keyboard interaction with a
        // pointer-focused control makes its focus visible again.
        if wrapper_focus && !state.focus_visible {
            state.focus_visible = true;
            shell.request_redraw();
        }

        if *repeat {
            return;
        }

        let activates = match state.semantics.role {
            Role::Button | Role::DefaultButton => matches!(
                key,
                keyboard::Key::Named(key::Named::Enter | key::Named::Space)
            ),
            Role::CheckBox | Role::Switch => {
                matches!(key, keyboard::Key::Named(key::Named::Space))
            }
            _ => false,
        };

        if activates && let Some(message) = state.semantics.activate.clone() {
            shell.publish(message);
            shell.capture_event();
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
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
        let state = tree.state.downcast_ref::<SemanticState<Message>>();
        if state.focus_visible && !state.semantics.disabled {
            let ring = self.focus_ring.unwrap_or(FocusRing {
                color: style.text_color,
                radius: 3.0,
            });
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: iced::Border {
                        color: ring.color,
                        width: 2.0,
                        radius: ring.radius.into(),
                    },
                    ..renderer::Quad::default()
                },
                iced::Color::TRANSPARENT,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut widget::Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Accessible<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Renderer: iced::advanced::Renderer + 'a,
    Theme: 'a,
{
    fn from(accessible: Accessible<'a, Message, Theme, Renderer>) -> Self {
        Self::new(accessible)
    }
}

/// Root wrapper that turns Tab and Shift+Tab into Ice focus operations.
pub struct Navigation<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    next: Message,
    previous: Message,
    window: Option<iced::window::Id>,
}

pub fn navigation<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    next: Message,
    previous: Message,
) -> Navigation<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    Navigation {
        content: content.into(),
        next,
        previous,
        window: None,
    }
}

impl<'a, Message, Theme, Renderer> Navigation<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    /// Names the window this root draws in, so a focus operation scoped to
    /// a window knows where its subtree begins. A daemon runs every widget
    /// operation through every window's tree in turn; without the name,
    /// Tab would count the controls of all of them as one ring.
    pub fn in_window(mut self, window: iced::window::Id) -> Self {
        self.window = Some(window);
        self
    }
}

/// What a [`Navigation`] root tells a scoped focus operation before its
/// content: the window the subtree that follows belongs to.
struct WindowScope(iced::window::Id);

/// Focuses the next enabled semantic/native focus target of one window in
/// view-tree order; `None` traverses the whole tree, as an app has one.
pub fn focus_next_in<Message>(window: Option<iced::window::Id>) -> Task<Message>
where
    Message: Send + 'static,
{
    match window {
        Some(window) => {
            iced::advanced::widget::operate(ScopedTraversal::counting(window, Direction::Next))
        }
        None => focus_next(),
    }
}

/// Focuses the previous enabled semantic/native focus target of one window
/// in view-tree order; `None` traverses the whole tree, as an app has one.
pub fn focus_previous_in<Message>(window: Option<iced::window::Id>) -> Task<Message>
where
    Message: Send + 'static,
{
    match window {
        Some(window) => {
            iced::advanced::widget::operate(ScopedTraversal::counting(window, Direction::Previous))
        }
        None => focus_previous(),
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Next,
    Previous,
}

/// iced's focus traversal, counted and applied only inside the subtree a
/// [`WindowScope`] marker opened for the target window. The marker of any
/// other window closes it again, so the windows a daemon runs the operation
/// through in turn never share one ring.
struct ScopedTraversal<T> {
    window: iced::window::Id,
    direction: Direction,
    inside: bool,
    count: focusable::Count,
    /// `None` while counting; the index to focus once the count is known.
    target: Option<usize>,
    seen: usize,
    _outcome: std::marker::PhantomData<T>,
}

impl<T> ScopedTraversal<T> {
    fn counting(window: iced::window::Id, direction: Direction) -> Self {
        Self {
            window,
            direction,
            inside: false,
            count: focusable::Count::default(),
            target: None,
            seen: 0,
            _outcome: std::marker::PhantomData,
        }
    }
}

impl<T> Operation<T> for ScopedTraversal<T>
where
    T: Send + 'static,
{
    fn custom(&mut self, _id: Option<&widget::Id>, _bounds: Rectangle, state: &mut dyn Any) {
        if let Some(WindowScope(window)) = state.downcast_ref::<WindowScope>() {
            self.inside = *window == self.window;
        }
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if !self.inside {
            return;
        }
        match self.target {
            None => {
                if state.is_focused() {
                    self.count.focused = Some(self.count.total);
                }
                self.count.total += 1;
            }
            Some(target) => {
                if self.seen == target {
                    state.focus();
                } else {
                    state.unfocus();
                }
                self.seen += 1;
            }
        }
    }

    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn finish(&self) -> Outcome<T> {
        if self.target.is_some() || self.count.total == 0 {
            return Outcome::None;
        }
        let last = self.count.total - 1;
        let target = match (self.direction, self.count.focused) {
            (Direction::Next, None) => 0,
            (Direction::Next, Some(focused)) if focused == last => 0,
            (Direction::Next, Some(focused)) => focused + 1,
            (Direction::Previous, None | Some(0)) => last,
            (Direction::Previous, Some(focused)) => focused - 1,
        };
        Outcome::Chain(Box::new(ScopedTraversal {
            window: self.window,
            direction: self.direction,
            inside: false,
            count: focusable::Count::default(),
            target: Some(target),
            seen: 0,
            _outcome: std::marker::PhantomData::<T>,
        }))
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Navigation<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
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
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(window) = self.window {
            operation.custom(None, layout.bounds(), &mut WindowScope(window));
        }
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
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let tab = if let Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Tab),
            modifiers,
            repeat: false,
            ..
        }) = event
        {
            (!modifiers.control() && !modifiers.alt() && !modifiers.logo())
                .then(|| modifiers.shift())
        } else {
            None
        };

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

        if let Some(previous) = tab
            && !shell.is_event_captured()
        {
            shell.publish(if previous {
                self.previous.clone()
            } else {
                self.next.clone()
            });
            shell.capture_event();
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
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
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut widget::Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Navigation<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Renderer: iced::advanced::Renderer + 'a,
    Theme: 'a,
{
    fn from(navigation: Navigation<'a, Message, Theme, Renderer>) -> Self {
        Self::new(navigation)
    }
}

/// Content that keyboard focus cannot enter.
///
/// A modal layer captures the pointer with a backdrop, but nothing about
/// `Stack` confines the keyboard: [`iced::widget::Stack::operate`] visits every
/// layer unconditionally, so Tab — which Ice routes through the very same
/// `operate` call — walks straight into the inputs sitting invisibly behind the
/// dimmed backdrop, and the next keystroke lands somewhere the user cannot see.
///
/// Wrapping the covered layer in this keeps focus operations out of it: the
/// subtree is traversed with [`WithoutFocus`], so counting, moving and
/// restoring focus all behave as if it held no focusable widget at all, and any
/// focus it still held when the layer opened is dropped on the first operation.
/// Everything else an operation asks for — accessibility semantics, scroll
/// position, text — still answers, because covering a layer hides it from the
/// keyboard, not from the machinery that reports what is on screen.
///
/// Keyboard events stop here too. Denying focus is not enough on its own: a
/// widget that was already focused when the layer opened keeps its focus until
/// something operates on the tree, and until then every keystroke would still
/// be delivered to it. Nothing above is skipped — the root Tab handler and the
/// layer itself are both outside this wrapper — and every other kind of event
/// still passes, so animations and window changes behind the layer carry on.
pub struct FocusBarrier<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
}

/// Creates a [`FocusBarrier`] around content a modal layer covers.
pub fn focus_barrier<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> FocusBarrier<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    FocusBarrier {
        content: content.into(),
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for FocusBarrier<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    // Transparent to the widget tree, exactly as `iced::widget::opaque` is:
    // the barrier appears and disappears as the layer above it opens and
    // shuts, and a wrapper that owned a tree node of its own would rebuild
    // everything under it on each transition — dropping the scroll offsets,
    // selections and cursors of the very content it is protecting.
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(
            tree,
            layout,
            renderer,
            &mut WithoutFocus { inner: operation },
        );
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if matches!(event, Event::Keyboard(_)) {
            return;
        }
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut widget::Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<FocusBarrier<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Renderer: iced::advanced::Renderer + 'a,
    Theme: 'a,
{
    fn from(barrier: FocusBarrier<'a, Message, Theme, Renderer>) -> Self {
        Self::new(barrier)
    }
}

#[derive(Clone)]
struct ActionTarget<Message> {
    activate: Option<Message>,
    node: SemanticFocus,
    focusable: bool,
    /// The text input whose caret a `SetTextSelection` request moves.
    caret: Option<widget::Id>,
    increment: Option<Message>,
    decrement: Option<Message>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SemanticFocus {
    base: StableId,
    occurrence: u64,
}

/// A complete AccessKit tree and the action map for the same UI frame.
#[derive(Clone)]
pub struct Snapshot<Message> {
    pub update: TreeUpdate,
    actions: HashMap<NodeId, ActionTarget<Message>>,
}

impl<Message> fmt::Debug for Snapshot<Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snapshot")
            .field("update", &self.update)
            .field("action_count", &self.actions.len())
            .finish()
    }
}

/// The node no snapshot ever assigns: the target of a [`refresh_request`].
const REFRESH_NODE: NodeId = NodeId(u64::MAX);

/// The request an activation sends itself to get the tree re-snapshotted. It
/// targets no node, so dispatching it does nothing; it carries `Focus`
/// because that is the action after which the generated single-window arm
/// chains a snapshot (the per-window arm chains one after every action).
pub fn refresh_request() -> ActionRequest {
    ActionRequest {
        action: Action::Focus,
        target_tree: TreeId::ROOT,
        target_node: REFRESH_NODE,
        data: None,
    }
}

/// Whether a request is the activation refresh rather than a user action.
pub fn is_refresh_request(request: &ActionRequest) -> bool {
    request.target_node == REFRESH_NODE
}

#[cfg(feature = "test-runtime")]
impl<Message> Snapshot<Message> {
    /// A tree with no action targets, for a smoke that publishes a hand-built
    /// update through a native adapter and reads the request back off the
    /// bridge's channel rather than dispatching it.
    pub fn from_update(update: TreeUpdate) -> Self {
        Self {
            update,
            actions: HashMap::default(),
        }
    }
}

impl<Message: Clone + Send + 'static> Snapshot<Message> {
    pub fn dispatch(&self, request: ActionRequest) -> Task<Message> {
        if request.target_tree != TreeId::ROOT || is_refresh_request(&request) {
            return Task::none();
        }
        let Some(target) = self.actions.get(&request.target_node) else {
            return Task::none();
        };
        match request.action {
            Action::Click => target.activate.clone().map_or_else(Task::none, Task::done),
            Action::Focus if target.focusable => focus_semantic(target.node),
            Action::ScrollIntoView => scroll_into_view(target.node),
            // iced moves a caret, not an arbitrary selection: the focus end
            // is where the reader asked the insertion point to land.
            Action::SetTextSelection => match (&target.caret, &request.data) {
                (Some(input), Some(accesskit::ActionData::SetTextSelection(selection))) => {
                    iced::advanced::widget::operate(
                        iced::advanced::widget::operation::text_input::move_cursor_to::<Message>(
                            input.clone(),
                            selection.focus.character_index,
                        ),
                    )
                }
                _ => Task::none(),
            },
            Action::Increment => target.increment.clone().map_or_else(Task::none, Task::done),
            Action::Decrement => target.decrement.clone().map_or_else(Task::none, Task::done),
            _ => Task::none(),
        }
    }
}

fn duplicate_node_id(base: NodeId, occurrence: u64) -> NodeId {
    let mut value = base
        .0
        .wrapping_add(occurrence.wrapping_mul(0x9e3779b97f4a7c15));
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^= value >> 31;
    NodeId(if value == 0 { 1 } else { value })
}

fn disambiguate_semantic_id(
    base: StableId,
    occurrences: &mut HashMap<NodeId, u64>,
    used_ids: &mut HashSet<NodeId>,
) -> (NodeId, SemanticFocus) {
    let next_occurrence = occurrences.entry(base.node_id()).or_default();
    let mut occurrence = *next_occurrence;
    let mut id = if occurrence == 0 {
        base.node_id()
    } else {
        duplicate_node_id(base.node_id(), occurrence)
    };
    while used_ids.contains(&id) {
        occurrence += 1;
        id = duplicate_node_id(base.node_id(), occurrence);
    }
    *next_occurrence = occurrence + 1;
    used_ids.insert(id);
    (id, SemanticFocus { base, occurrence })
}

struct FocusOperation<Message> {
    target: SemanticFocus,
    occurrences: HashMap<NodeId, u64>,
    used_ids: HashSet<NodeId>,
    frames: Vec<Option<(SemanticFocus, FocusBehavior, bool)>>,
    marker: std::marker::PhantomData<Message>,
}

impl<Message: Send + 'static> Operation<()> for FocusOperation<Message> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
        operate(self);
    }

    fn custom(&mut self, _id: Option<&widget::Id>, _bounds: Rectangle, state: &mut dyn Any) {
        if state.downcast_mut::<SemanticEnd>().is_some() {
            self.frames.pop();
            return;
        }
        let Some(state) = state.downcast_mut::<SemanticState<Message>>() else {
            return;
        };
        if self.frames.iter().flatten().any(|(_, _, atomic)| *atomic) {
            self.frames.push(None);
            return;
        }
        let (_, current) = disambiguate_semantic_id(
            state.semantics.id,
            &mut self.occurrences,
            &mut self.used_ids,
        );
        self.frames.push(Some((
            current,
            state.semantics.focus,
            atomic_role(state.semantics.role),
        )));

        if state.semantics.focus == FocusBehavior::Wrapper {
            if !state.semantics.disabled && current == self.target {
                state.focus();
            } else {
                state.unfocus();
            }
        }
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if self
            .frames
            .iter()
            .rev()
            .flatten()
            .find(|(_, focus, _)| *focus != FocusBehavior::None)
            .is_some_and(|(current, _, _)| *current == self.target)
        {
            state.focus();
        } else {
            state.unfocus();
        }
    }

    fn finish(&self) -> Outcome<()> {
        Outcome::Some(())
    }
}

fn focus_semantic<Message: Send + 'static>(target: SemanticFocus) -> Task<Message> {
    iced::advanced::widget::operate(FocusOperation::<Message> {
        target,
        occurrences: HashMap::new(),
        used_ids: HashSet::from([ROOT_ID]),
        frames: Vec::new(),
        marker: std::marker::PhantomData,
    })
    .discard()
}

/// Scrolls the nearest identified scroll enclosing `target` until the node is
/// in view. Only the nearest one moves, and only as far as it must: a node
/// already in view moves nothing, a node above or left of the viewport lands on
/// its leading edge, one below or right lands on its trailing edge.
fn scroll_into_view<Message: Send + 'static>(target: SemanticFocus) -> Task<Message> {
    iced::advanced::widget::operate(ScrollIntoViewOperation::<Message>::new(target)).discard()
}

/// A scroll the walk is inside of, in its own untranslated coordinates.
struct ScrollFrame {
    id: widget::Id,
    viewport: Rectangle,
    content: Rectangle,
    translation: Vector,
}

struct ScrollIntoViewOperation<Message> {
    target: SemanticFocus,
    occurrences: HashMap<NodeId, u64>,
    used_ids: HashSet<NodeId>,
    /// One entry per open semantic frame, `true` for an atomic one, mirroring
    /// the snapshot so the same node gets the same occurrence here.
    frames: Vec<bool>,
    scrolls: Vec<ScrollFrame>,
    pending_scroll: Option<ScrollFrame>,
    found: Option<(
        widget::Id,
        operation::scrollable::AbsoluteOffset<Option<f32>>,
    )>,
    marker: std::marker::PhantomData<Message>,
}

impl<Message> ScrollIntoViewOperation<Message> {
    fn new(target: SemanticFocus) -> Self {
        Self {
            target,
            occurrences: HashMap::new(),
            used_ids: HashSet::from([ROOT_ID]),
            frames: Vec::new(),
            scrolls: Vec::new(),
            pending_scroll: None,
            found: None,
            marker: std::marker::PhantomData,
        }
    }

    /// Whether the walk found the target inside an identified scroll.
    pub(crate) fn found_scroll(&self) -> bool {
        self.found.is_some()
    }
}

/// The offset that brings `[start, end)` of the content into a viewport `len`
/// long currently showing from `shown`, or `None` when it is already in view.
fn reveal(start: f32, end: f32, shown: f32, len: f32) -> Option<f32> {
    if start < shown {
        Some(start)
    } else if end > shown + len {
        Some((end - len).max(0.0))
    } else {
        None
    }
}

impl<Message: Send + 'static> Operation<()> for ScrollIntoViewOperation<Message> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
        let scroll = self.pending_scroll.take();
        let entered = scroll.is_some();
        self.scrolls.extend(scroll);
        operate(self);
        if entered {
            self.scrolls.pop();
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.pending_scroll = id.map(|id| ScrollFrame {
            id: id.clone(),
            viewport: bounds,
            content: content_bounds,
            translation,
        });
    }

    fn custom(&mut self, _id: Option<&widget::Id>, bounds: Rectangle, state: &mut dyn Any) {
        if state.downcast_ref::<SemanticEnd>().is_some() {
            self.frames.pop();
            return;
        }
        let Some(semantics) = state.downcast_ref::<SemanticSnapshot>() else {
            return;
        };
        if self.frames.iter().any(|atomic| *atomic) {
            self.frames.push(false);
            return;
        }
        let (_, current) =
            disambiguate_semantic_id(semantics.id, &mut self.occurrences, &mut self.used_ids);
        self.frames.push(atomic_role(semantics.role));
        if current != self.target {
            return;
        }
        let Some(scroll) = self.scrolls.last() else {
            return;
        };
        let x = bounds.x - scroll.content.x;
        let y = bounds.y - scroll.content.y;
        self.found = Some((
            scroll.id.clone(),
            operation::scrollable::AbsoluteOffset {
                x: reveal(
                    x,
                    x + bounds.width,
                    scroll.translation.x,
                    scroll.viewport.width,
                ),
                y: reveal(
                    y,
                    y + bounds.height,
                    scroll.translation.y,
                    scroll.viewport.height,
                ),
            },
        ));
    }

    fn finish(&self) -> Outcome<()> {
        match &self.found {
            Some((id, offset)) => Outcome::Chain(Box::new(operation::scrollable::scroll_to::<()>(
                id.clone(),
                *offset,
            ))),
            None => Outcome::None,
        }
    }
}

struct SnapshotOperation<Message> {
    nodes: Vec<(NodeId, Node)>,
    root_children: Vec<NodeId>,
    frames: Vec<SemanticFrame>,
    actions: HashMap<NodeId, ActionTarget<Message>>,
    occurrences: HashMap<NodeId, u64>,
    used_ids: HashSet<NodeId>,
    focus: NodeId,
    root_label: String,
    translation: Vector,
    pending_translation: Option<Vector>,
    /// How many identified scrolls enclose the subtree being walked. A node
    /// inside one supports `ScrollIntoView`; the request finds the scroll
    /// again when it arrives, so nothing about it is stored per node.
    scroll_depth: usize,
    pending_scroll: bool,
    pending_set: Option<SetPosition>,
    /// The one window this snapshot describes, or `None` for the whole tree.
    /// A daemon runs every widget operation through every window's tree in
    /// turn, so an unscoped snapshot of a multi-window program is one tree
    /// holding all of them — and two windows carrying the same Ice id would
    /// collide in it.
    scope: Option<iced::window::Id>,
    /// Whether the subtree being walked belongs to `scope`. Always true when
    /// there is no scope.
    inside: bool,
}

struct SemanticFrame {
    node_index: Option<usize>,
    children: Vec<NodeId>,
    focus: Option<NodeId>,
    semantic_focus: Option<SemanticFocus>,
    atomic: bool,
}

fn atomic_role(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::DefaultButton
            | Role::CheckBox
            | Role::Switch
            | Role::TextInput
            | Role::MultilineTextInput
            | Role::SearchInput
            | Role::PasswordInput
            | Role::Slider
            | Role::ProgressIndicator
            | Role::Image
            | Role::Label
    )
}

impl<Message> Default for SnapshotOperation<Message> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            root_children: Vec::new(),
            frames: Vec::new(),
            actions: HashMap::new(),
            occurrences: HashMap::new(),
            used_ids: HashSet::from([ROOT_ID]),
            focus: ROOT_ID,
            root_label: "Ice application".into(),
            translation: Vector::ZERO,
            pending_translation: None,
            scroll_depth: 0,
            pending_scroll: false,
            pending_set: None,
            scope: None,
            inside: true,
        }
    }
}

impl<Message> SnapshotOperation<Message> {
    /// Gives the node on top of the stack — the text input whose `operate`
    /// just reported the caret — one `TextRun` child carrying its value
    /// grapheme by grapheme, and points the input's text selection into it.
    /// That pair is what a screen reader reads a caret position from and
    /// what `SetTextSelection` addresses when it moves one.
    fn text_run(&mut self, caret: &TextCaret) {
        let Some(frame) = self.frames.last_mut() else {
            return;
        };
        let Some(index) = frame.node_index else {
            return;
        };
        let input_id = self.nodes[index].0;
        let Some(value) = self.nodes[index].1.value().map(str::to_owned) else {
            return;
        };
        let mut run_id = duplicate_node_id(input_id, u64::MAX);
        while !self.used_ids.insert(run_id) {
            run_id = duplicate_node_id(run_id, u64::MAX);
        }
        let input = &mut self.nodes[index].1;
        let lengths: Box<[u8]> =
            unicode_segmentation::UnicodeSegmentation::graphemes(value.as_str(), true)
                .map(|grapheme| u8::try_from(grapheme.len()).unwrap_or(u8::MAX))
                .collect();
        let count = lengths.len();
        let position = |index: usize| accesskit::TextPosition {
            node: run_id,
            character_index: index.min(count),
        };
        input.set_text_selection(accesskit::TextSelection {
            anchor: position(caret.anchor),
            focus: position(caret.focus),
        });
        let mut run = Node::new(Role::TextRun);
        if let Some(bounds) = input.bounds() {
            run.set_bounds(bounds);
        }
        run.set_value(value);
        run.set_character_lengths(lengths);
        frame.children.push(run_id);
        self.nodes.push((run_id, run));
        if let Some(target) = self.actions.get_mut(&input_id) {
            target.caret = Some(caret.target.clone());
        }
    }

    fn named(root_label: impl Into<String>) -> Self {
        Self {
            root_label: root_label.into(),
            ..Self::default()
        }
    }

    /// The same snapshot, built only from the subtree a [`WindowScope`]
    /// marker opens for `window`. The marker of any other window closes it
    /// again, so every marker of one window — the semantic frame's start and
    /// its end alike — is admitted or skipped together and the frame stack
    /// stays balanced.
    fn scoped(root_label: impl Into<String>, window: iced::window::Id) -> Self {
        Self {
            root_label: root_label.into(),
            scope: Some(window),
            inside: false,
            ..Self::default()
        }
    }
}

impl<Message: Clone + Send + 'static> Operation<Snapshot<Message>> for SnapshotOperation<Message> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Snapshot<Message>>)) {
        let translation = self.pending_translation.take().unwrap_or(Vector::ZERO);
        let scroll = std::mem::take(&mut self.pending_scroll);
        self.translation += translation;
        self.scroll_depth += usize::from(scroll);
        operate(self);
        self.scroll_depth -= usize::from(scroll);
        self.translation -= translation;
    }

    fn scrollable(
        &mut self,
        id: Option<&widget::Id>,
        _bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if !self.inside {
            return;
        }
        self.pending_translation = Some(translation);
        self.pending_scroll = id.is_some();
    }

    fn custom(&mut self, _id: Option<&widget::Id>, bounds: Rectangle, state: &mut dyn Any) {
        if let Some(WindowScope(window)) = state.downcast_ref::<WindowScope>() {
            if let Some(scope) = self.scope {
                self.inside = *window == scope;
            }
            return;
        }
        if !self.inside {
            return;
        }
        if let Some(set) = state.downcast_mut::<SetPosition>() {
            self.pending_set = Some(SetPosition {
                position: set.position,
                size: set.size,
            });
            return;
        }
        if state.downcast_mut::<SemanticEnd>().is_some() {
            let Some(frame) = self.frames.pop() else {
                return;
            };
            if let Some(index) = frame.node_index {
                self.nodes[index].1.set_children(frame.children);
            }
            return;
        }
        if let Some(caret) = state.downcast_mut::<TextCaret>() {
            self.text_run(caret);
            return;
        }
        if let Some(state) = state.downcast_mut::<SemanticState<Message>>() {
            let Some(frame) = self.frames.last() else {
                return;
            };
            let (Some(index), Some(focus)) = (frame.node_index, frame.semantic_focus) else {
                return;
            };
            let id = self.nodes[index].0;
            let node = &mut self.nodes[index].1;
            let enabled = !state.semantics.disabled;
            let focusable = enabled && state.semantics.focus != FocusBehavior::None;
            let scrollable = self.scroll_depth > 0;
            if focusable {
                node.add_action(Action::Focus);
            }
            if enabled && state.semantics.activate.is_some() {
                node.add_action(Action::Click);
            }
            if enabled && state.semantics.increment().is_some() {
                node.add_action(Action::Increment);
            }
            if enabled && state.semantics.decrement().is_some() {
                node.add_action(Action::Decrement);
            }
            if scrollable {
                node.add_action(Action::ScrollIntoView);
            }
            if enabled || scrollable {
                self.actions.insert(
                    id,
                    ActionTarget {
                        activate: state.semantics.activate.clone().filter(|_| enabled),
                        node: focus,
                        focusable,
                        caret: None,
                        increment: state.semantics.increment().cloned().filter(|_| enabled),
                        decrement: state.semantics.decrement().cloned().filter(|_| enabled),
                    },
                );
            }
            return;
        }
        let Some(semantics) = state.downcast_mut::<SemanticSnapshot>() else {
            return;
        };
        let pending_set = self.pending_set.take();
        if self.frames.iter().any(|frame| frame.atomic) {
            self.frames.push(SemanticFrame {
                node_index: None,
                children: Vec::new(),
                focus: None,
                semantic_focus: None,
                atomic: false,
            });
            return;
        }
        let (id, focus) =
            disambiguate_semantic_id(semantics.id, &mut self.occurrences, &mut self.used_ids);
        let finite = |value: f32| {
            if value.is_nan() {
                0.0
            } else {
                f64::from(value.clamp(f32::MIN, f32::MAX))
            }
        };
        let x = finite(bounds.x) - finite(self.translation.x);
        let y = finite(bounds.y) - finite(self.translation.y);
        let mut node = Node::new(semantics.role);
        node.set_bounds(Rect {
            x0: x,
            y0: y,
            x1: x + finite(bounds.width),
            y1: y + finite(bounds.height),
        });
        if let Some(label) = &semantics.label {
            node.set_label(label.clone());
        }
        if let Some(description) = &semantics.description {
            node.set_description(description.clone());
        }
        if let Some(value) = &semantics.value {
            node.set_value(value.clone());
        }
        if let Some(checked) = semantics.checked {
            node.set_toggled(Toggled::from(checked));
        }
        if let Some(selected) = semantics.selected {
            node.set_selected(selected);
        }
        if let Some(expanded) = semantics.expanded {
            node.set_expanded(expanded);
        }
        if let Some(level) = semantics.level {
            node.set_level(level);
        }
        if let Some(row_count) = semantics.row_count {
            node.set_row_count(row_count);
        }
        if let Some(column_count) = semantics.column_count {
            node.set_column_count(column_count);
        }
        if let Some(row_index) = semantics.row_index {
            node.set_row_index(row_index);
        }
        if let Some(column_index) = semantics.column_index {
            node.set_column_index(column_index);
        }
        if let Some(sort_direction) = semantics.sort_direction {
            node.set_sort_direction(sort_direction);
        }
        if let Some(position) = semantics
            .position_in_set
            .or_else(|| pending_set.as_ref().map(|set| set.position))
        {
            node.set_position_in_set(position);
        }
        if let Some(size) = semantics
            .size_of_set
            .or_else(|| pending_set.as_ref().map(|set| set.size))
        {
            node.set_size_of_set(size);
        }
        if let Some(active_descendant) = semantics.active_descendant {
            node.set_active_descendant(active_descendant.node_id());
        }
        if let Some(live) = semantics.live {
            node.set_live(live);
        }
        if let Some(numeric) = &semantics.numeric {
            node.set_numeric_value(numeric.value);
            node.set_min_numeric_value(numeric.min);
            node.set_max_numeric_value(numeric.max);
            if let Some(step) = numeric.step {
                node.set_numeric_value_step(step);
            }
        }
        if semantics.disabled {
            node.set_disabled();
        }
        if semantics.focused {
            self.focus = id;
        }
        if let Some(parent) = self
            .frames
            .iter_mut()
            .rev()
            .find(|frame| frame.node_index.is_some())
        {
            parent.children.push(id);
        } else {
            self.root_children.push(id);
        }
        let node_index = self.nodes.len();
        self.nodes.push((id, node));
        self.frames.push(SemanticFrame {
            node_index: Some(node_index),
            children: Vec::new(),
            focus: (semantics.focus != FocusBehavior::None).then_some(id),
            semantic_focus: Some(focus),
            atomic: atomic_role(semantics.role),
        });
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if !self.inside {
            return;
        }
        if state.is_focused()
            && let Some(id) = self.frames.iter().rev().find_map(|frame| frame.focus)
        {
            self.focus = id;
        }
    }

    fn finish(&self) -> Outcome<Snapshot<Message>> {
        let mut root = Node::new(Role::Window);
        root.set_label(self.root_label.clone());
        root.set_children(self.root_children.clone());
        let mut nodes = Vec::with_capacity(self.nodes.len() + 1);
        nodes.push((ROOT_ID, root));
        nodes.extend(self.nodes.iter().cloned());
        Outcome::Some(Snapshot {
            update: TreeUpdate {
                nodes,
                tree: Some(Tree {
                    root: ROOT_ID,
                    toolkit_name: Some("Ice/Iced".into()),
                    toolkit_version: Some(concat!(env!("CARGO_PKG_VERSION"), "/0.14").into()),
                }),
                tree_id: TreeId::ROOT,
                focus: self.focus,
            },
            actions: self.actions.clone(),
        })
    }
}

/// Captures the live Iced widget tree as an AccessKit update.
pub fn snapshot<Message>(root_label: impl Into<String>) -> Task<Snapshot<Message>>
where
    Message: Clone + Send + 'static,
{
    iced::advanced::widget::operate(SnapshotOperation::named(root_label))
}

/// Captures the live Iced widget tree of ONE window as an AccessKit update.
///
/// A daemon owns several windows and one widget-operation pass walks all of
/// them, so every native adapter it drives needs the tree of its own window
/// and nothing else.
pub fn snapshot_in<Message>(
    root_label: impl Into<String>,
    window: iced::window::Id,
) -> Task<Snapshot<Message>>
where
    Message: Clone + Send + 'static,
{
    iced::advanced::widget::operate(SnapshotOperation::scoped(root_label, window))
}

#[derive(Clone)]
struct ActionSubscription {
    id: u64,
    receiver: Arc<Mutex<Option<iced::futures::channel::mpsc::Receiver<ActionRequest>>>>,
}

impl PartialEq for ActionSubscription {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ActionSubscription {}

impl Hash for ActionSubscription {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

fn action_stream(
    subscription: &ActionSubscription,
) -> iced::futures::channel::mpsc::Receiver<ActionRequest> {
    subscription
        .receiver
        .lock()
        .expect("accessibility action receiver lock")
        .take()
        .unwrap_or_else(|| {
            let (_sender, receiver) =
                iced::futures::channel::mpsc::channel(ACCESSIBILITY_ACTION_BUFFER);
            receiver
        })
}

static NEXT_BRIDGE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

// Keep this configured buffer aligned with iced_winit::Proxy::MAX_SIZE.
// `futures` reserves one additional slot for the sole sender.
const ACCESSIBILITY_ACTION_BUFFER: usize = 100;

/// Whether any platform assistive technology has activated the accessibility
/// tree in this process. Flipped by the adapters' activation/deactivation
/// callbacks below; read by the generated per-update snapshot gate.
static AT_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// True from the moment assistive technology asks for the tree (and, on
/// Linux, until it deactivates). Generated applications gate the per-update
/// accessibility snapshot on this: until an AT connects, walking the whole
/// widget tree after every message builds a `TreeUpdate` nobody consumes and
/// schedules an extra frame to deliver it. Test builds bypass the gate with
/// `cfg!(test)` — the Ice test harness drives the app through this tree.
pub fn accessibility_active() -> bool {
    AT_ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
}

/// The accessibility preferences a user sets in the operating system that a
/// program has to honour itself, because no assistive technology relays them:
/// motion to tone down, contrast to raise, and whether a screen reader is
/// running at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AccessibilitySettings {
    /// The user asked for less motion (macOS "Reduce motion").
    pub reduce_motion: bool,
    /// The user asked for more contrast (macOS "Increase contrast").
    pub increase_contrast: bool,
    /// A screen reader is running: VoiceOver on macOS, or any assistive
    /// technology that has activated this process's tree.
    pub screen_reader: bool,
}

/// Reads the settings as they are now. macOS answers from `NSWorkspace`; the
/// other platforms report no motion or contrast preference and learn about a
/// screen reader only once it activates the tree.
pub fn accessibility_settings() -> AccessibilitySettings {
    #[cfg(target_os = "macos")]
    {
        let workspace = objc2_app_kit::NSWorkspace::sharedWorkspace();
        AccessibilitySettings {
            reduce_motion: workspace.accessibilityDisplayShouldReduceMotion(),
            increase_contrast: workspace.accessibilityDisplayShouldIncreaseContrast(),
            screen_reader: workspace.isVoiceOverEnabled() || accessibility_active(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    AccessibilitySettings {
        screen_reader: accessibility_active(),
        ..AccessibilitySettings::default()
    }
}

/// The native Win32 handle captured before Iced shows its first window.
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
pub struct NativeWindow {
    id: iced::window::Id,
    hwnd: std::num::NonZeroIsize,
}

#[cfg(target_os = "windows")]
impl NativeWindow {
    pub fn id(self) -> iced::window::Id {
        self.id
    }
}

/// Captures the Win32 window handle on Iced's window-owning thread.
#[cfg(target_os = "windows")]
pub fn native_window(id: iced::window::Id) -> Task<NativeWindow> {
    iced::window::run(id, move |window| {
        let handle = window.window_handle().expect("Iced Windows window handle");
        let hwnd = match handle.as_raw() {
            iced::window::raw_window_handle::RawWindowHandle::Win32(handle) => handle.hwnd,
            _ => unreachable!("Iced uses a Win32 window on Windows"),
        };
        NativeWindow { id, hwnd }
    })
}

/// The native AppKit view captured once Iced owns its first window.
#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy)]
pub struct NativeWindow {
    id: iced::window::Id,
    /// The `NSView`, as an integer. `iced::window::run` delivers its value
    /// across a `Send` boundary and a raw pointer is not `Send`; the integer
    /// is turned back into a pointer only on the main thread, in
    /// `Bridge::attach_window`, which is the same event-loop turn that
    /// produced it.
    ns_view: usize,
}

#[cfg(target_os = "macos")]
impl NativeWindow {
    pub fn id(self) -> iced::window::Id {
        self.id
    }

    /// A handle for an `NSView` a test created itself, on the main thread,
    /// for the in-process NSAccessibility smoke; a shipped app only ever
    /// gets one from [`native_window`].
    #[cfg(feature = "test-runtime")]
    pub fn for_view(id: iced::window::Id, ns_view: usize) -> Self {
        Self { id, ns_view }
    }
}

/// Captures the AppKit view handle on Iced's window-owning thread.
#[cfg(target_os = "macos")]
pub fn native_window(id: iced::window::Id) -> Task<NativeWindow> {
    iced::window::run(id, move |window| {
        let handle = window.window_handle().expect("Iced AppKit window handle");
        let ns_view = match handle.as_raw() {
            iced::window::raw_window_handle::RawWindowHandle::AppKit(handle) => handle.ns_view,
            _ => unreachable!("Iced uses an AppKit window on macOS"),
        };
        NativeWindow {
            id,
            ns_view: ns_view.as_ptr() as usize,
        }
    })
}

/// The factor from iced's layout units to the physical pixels AccessKit
/// expects: the window's backing scale times the application's `scale`
/// setting. Published as the root node's transform, so the consumer scales
/// every descendant's bounds and hit-tests through the same matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Scale {
    window: f32,
    application: f32,
}

impl Default for Scale {
    fn default() -> Self {
        Self {
            window: 1.0,
            application: 1.0,
        }
    }
}

impl Scale {
    fn apply(self, update: &mut TreeUpdate) {
        let factor = f64::from(self.window) * f64::from(self.application);
        if factor == 1.0 || !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let root = update.tree.as_ref().map_or(ROOT_ID, |tree| tree.root);
        if let Some((_, node)) = update.nodes.iter_mut().find(|(id, _)| *id == root) {
            node.set_transform(accesskit::Affine::scale(factor));
        }
    }
}

/// Owns the native adapter and the action map for the latest frame.
pub struct Bridge<Message> {
    id: u64,
    snapshot: Option<Snapshot<Message>>,
    receiver: Arc<Mutex<Option<iced::futures::channel::mpsc::Receiver<ActionRequest>>>>,
    latest_tree: Arc<Mutex<Option<TreeUpdate>>>,
    /// Physical pixels per layout unit: the window's backing scale times the
    /// application's own `scale` setting. AccessKit takes physical pixels —
    /// every native adapter divides by the backing scale on its way to the
    /// platform — while iced lays out in logical units, so the published root
    /// carries this as its transform. The backing scale arrives as a
    /// `Rescaled` event: generated code asks `iced::window::scale_factor` once
    /// the native adapter is attached, and winit reports every later change.
    scale: Scale,
    #[cfg(target_os = "linux")]
    adapter: Option<accesskit_unix::Adapter>,
    #[cfg(target_os = "windows")]
    adapter: Option<accesskit_windows::SubclassingAdapter>,
    #[cfg(target_os = "macos")]
    adapter: Option<accesskit_macos::SubclassingAdapter>,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    sender: Option<iced::futures::channel::mpsc::Sender<ActionRequest>>,
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    window: Option<iced::window::Id>,
}

impl<Message> fmt::Debug for Bridge<Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bridge")
            .field("id", &self.id)
            .field("has_snapshot", &self.snapshot.is_some())
            .finish()
    }
}

/// What an activation does after handing out the cached tree: asks the program
/// for a fresh one. Sends [`refresh_request`] down the adapter's own action
/// channel, so the generated action arm re-snapshots exactly as it does after
/// any other assistive-technology request.
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
type Refresh = Box<dyn FnMut() + Send>;

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
struct Activation {
    latest_tree: Arc<Mutex<Option<TreeUpdate>>>,
    refresh: Refresh,
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
impl accesskit::ActivationHandler for Activation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        AT_ACTIVE.store(true, std::sync::atomic::Ordering::Relaxed);
        // The cached tree is as old as the last program message: snapshots are
        // taken only while assistive technology is active, and activation is
        // this very call. A reader that activates after the program has gone
        // quiet (a boot that finished before the first AX query) would keep
        // that stale tree until the next message, so ask for a fresh one now.
        (self.refresh)();
        self.latest_tree
            .lock()
            .expect("accessibility tree lock")
            .clone()
    }
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
struct Actions {
    sender: iced::futures::channel::mpsc::Sender<ActionRequest>,
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
impl accesskit::ActionHandler for Actions {
    fn do_action(&mut self, request: ActionRequest) {
        // A native callback cannot await without risking an event-loop cycle.
        // Preserve the bounded backlog and drop only overload or disconnects.
        let _ = self.sender.try_send(request);
    }
}

#[cfg(target_os = "linux")]
struct Deactivation;

#[cfg(target_os = "linux")]
impl accesskit::DeactivationHandler for Deactivation {
    fn deactivate_accessibility(&mut self) {
        AT_ACTIVE.store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

impl<Message> Bridge<Message> {
    pub fn new() -> Self {
        Self::with_native_adapter(true)
    }

    /// Creates a deterministic bridge without exporting a native platform tree.
    ///
    /// This is used for daemon/multi-window applications until Iced exposes a
    /// window-scoped widget-operation boundary.
    pub fn without_native_adapter() -> Self {
        Self::with_native_adapter(false)
    }

    fn with_native_adapter(native: bool) -> Self {
        let id = NEXT_BRIDGE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (sender, receiver) = iced::futures::channel::mpsc::channel(ACCESSIBILITY_ACTION_BUFFER);
        let receiver = Arc::new(Mutex::new(Some(receiver)));
        let latest_tree = Arc::new(Mutex::new(None));
        #[cfg(target_os = "linux")]
        let adapter = native.then(|| {
            let mut refresh = sender.clone();
            accesskit_unix::Adapter::new(
                Activation {
                    latest_tree: Arc::clone(&latest_tree),
                    refresh: Box::new(move || {
                        let _ = refresh.try_send(refresh_request());
                    }),
                },
                Actions { sender },
                Deactivation,
            )
        });
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let (adapter, sender) = (None, native.then_some(sender));
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            let _ = native;
            drop(sender);
        }

        Self {
            id,
            snapshot: None,
            receiver,
            latest_tree,
            scale: Scale::default(),
            #[cfg(target_os = "linux")]
            adapter,
            #[cfg(target_os = "windows")]
            adapter,
            #[cfg(target_os = "macos")]
            adapter,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            sender,
            #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
            window: None,
        }
    }

    pub fn subscription(&self) -> Subscription<ActionRequest> {
        Subscription::run_with(
            ActionSubscription {
                id: self.id,
                receiver: Arc::clone(&self.receiver),
            },
            action_stream,
        )
    }

    /// The channel the native adapter's requests arrive on, taken the way
    /// [`Bridge::subscription`] would take it, for a smoke with no iced
    /// runtime to run that subscription.
    #[cfg(feature = "test-runtime")]
    pub fn take_action_receiver(
        &self,
    ) -> Option<iced::futures::channel::mpsc::Receiver<ActionRequest>> {
        self.receiver
            .lock()
            .expect("accessibility action receiver lock")
            .take()
    }

    pub fn update(&mut self, snapshot: Snapshot<Message>) {
        // One clone out of the snapshot; the adapter clone happens inside
        // `update_if_active`'s closure, so it is paid only while assistive
        // technology is actually listening, and the activation cache takes
        // the value by move.
        let mut update = snapshot.update.clone();
        self.scale.apply(&mut update);
        #[cfg(target_os = "linux")]
        if let Some(adapter) = &mut self.adapter {
            adapter.update_if_active(|| update.clone());
        }
        // Windows raises inline: UI Automation's `QueuedEvents::raise` does
        // not service the run loop, so it cannot re-enter the event loop the
        // way NSAccessibility's does.
        #[cfg(target_os = "windows")]
        if let Some(adapter) = &mut self.adapter
            && let Some(events) = adapter.update_if_active(|| update.clone())
        {
            events.raise();
        }
        #[cfg(target_os = "macos")]
        if let Some(adapter) = &mut self.adapter
            && let Some(events) = adapter.update_if_active(|| update.clone())
        {
            raise_on_next_turn(events);
        }
        *self.latest_tree.lock().expect("accessibility tree lock") = Some(update);
        self.snapshot = Some(snapshot);
    }

    /// Returns whether the platform accessibility API owns the native window.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub fn is_attached(&self) -> bool {
        self.adapter.is_some()
    }

    /// Attaches UI Automation before the initial Win32 window is first shown.
    #[cfg(target_os = "windows")]
    pub fn attach_window(&mut self, window: NativeWindow) -> bool {
        let Some(sender) = self.sender.take() else {
            return false;
        };
        self.window = Some(window.id);
        let mut refresh = sender.clone();
        self.adapter = Some(accesskit_windows::SubclassingAdapter::new(
            accesskit_windows::HWND(window.hwnd.get() as *mut core::ffi::c_void),
            Activation {
                latest_tree: Arc::clone(&self.latest_tree),
                refresh: Box::new(move || {
                    let _ = refresh.try_send(refresh_request());
                }),
            },
            Actions { sender },
        ));
        true
    }

    /// Attaches NSAccessibility to the AppKit view backing the Iced window.
    ///
    /// Refuses off the main thread: the subclass rewrites the view's
    /// Objective-C class and every accessibility callback it installs is
    /// delivered on the main thread, so anywhere else this is a data race on
    /// AppKit. Generated boot also runs on worker threads — every Ice
    /// semantic test and frame probe constructs the app off-main — and there
    /// the refusal leaves the deterministic tree untouched, exactly as
    /// `without_native_adapter` does.
    #[cfg(target_os = "macos")]
    pub fn attach_window(&mut self, window: NativeWindow) -> bool {
        let on_main_thread = objc2::MainThreadMarker::new().is_some();
        if !on_main_thread {
            return false;
        }
        let Some(sender) = self.sender.take() else {
            return false;
        };
        self.window = Some(window.id);
        // SAFETY: `ns_view` is the `NSView` pointer AppKit gave Iced for this
        // window through `raw-window-handle`, read in `native_window` on the
        // window-owning thread. Iced owns the window for the whole run and
        // the adapter retains the view, so the pointer is a live,
        // unreleased `NSView` here.
        let mut refresh = sender.clone();
        #[expect(
            unsafe_code,
            reason = "accesskit_macos takes the NSView as a raw pointer; there is no safe constructor"
        )]
        let adapter = unsafe {
            accesskit_macos::SubclassingAdapter::new(
                window.ns_view as *mut core::ffi::c_void,
                Activation {
                    latest_tree: Arc::clone(&self.latest_tree),
                    refresh: Box::new(move || {
                        let _ = refresh.try_send(refresh_request());
                    }),
                },
                Actions { sender },
            )
        };
        self.adapter = Some(adapter);
        true
    }

    /// The application's own `scale` setting, multiplied into the window's
    /// backing scale for every tree published from here on.
    pub fn set_application_scale(&mut self, scale: f32) {
        self.scale.application = scale;
    }

    /// Applies focus and scale truth for the single native window owned by
    /// this bridge.
    pub fn window_event(&mut self, id: iced::window::Id, event: iced::window::Event) {
        if let iced::window::Event::Rescaled(scale) = event {
            #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
            if *self.window.get_or_insert(id) != id {
                return;
            }
            self.scale.window = scale;
            return;
        }
        #[cfg(target_os = "linux")]
        {
            let Some(adapter) = &mut self.adapter else {
                return;
            };
            let window = self.window.get_or_insert(id);
            if *window != id {
                return;
            }
            match event {
                iced::window::Event::Focused => adapter.update_window_focus_state(true),
                iced::window::Event::Unfocused | iced::window::Event::Closed => {
                    adapter.update_window_focus_state(false);
                }
                _ => {}
            }
        }
        #[cfg(target_os = "macos")]
        {
            let Some(adapter) = &mut self.adapter else {
                return;
            };
            let window = self.window.get_or_insert(id);
            if *window != id {
                return;
            }
            let focused = match event {
                iced::window::Event::Focused => true,
                iced::window::Event::Unfocused | iced::window::Event::Closed => false,
                _ => return,
            };
            if let Some(events) = adapter.update_view_focus_state(focused) {
                raise_on_next_turn(events);
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        let _ = (id, event);
        #[cfg(target_os = "windows")]
        let _ = (id, event);
    }
}

impl<Message: Clone + Send + 'static> Bridge<Message> {
    pub fn dispatch(&self, request: ActionRequest) -> Task<Message> {
        self.snapshot
            .as_ref()
            .map_or_else(Task::none, |snapshot| snapshot.dispatch(request))
    }
}

impl<Message> Default for Bridge<Message> {
    fn default() -> Self {
        Self::new()
    }
}

/// One native adapter per window, for a daemon.
///
/// [`Bridge`] owns the single window an `app` has. A daemon owns several and
/// opens them over its lifetime, so the native tree is not one adapter but a
/// map keyed by [`iced::window::Id`]: a window contributes its own tree, keeps
/// its own focus state, routes its own actions, and takes its adapter with it
/// when it closes.
///
/// macOS only, and the type does not exist elsewhere: `accesskit_macos`
/// subclasses one `NSView`, which is exactly per-window, while
/// `accesskit_unix`'s AT-SPI adapter is per-process and `accesskit_windows`
/// subclasses the Win32 handle a `Bridge` already owns. Neither can be keyed
/// by window, so on those targets a daemon exports nothing and generated code
/// never names this type — the behaviour it had before.
#[cfg(target_os = "macos")]
pub struct WindowBridges<Message> {
    id: u64,
    receiver: Arc<Mutex<Option<iced::futures::channel::mpsc::Receiver<WindowAction>>>>,
    sender: iced::futures::channel::mpsc::Sender<WindowAction>,
    windows: HashMap<iced::window::Id, WindowEntry<Message>>,
}

/// An action a native adapter raised, named with the window it came from —
/// two windows of one daemon can hold the same Ice id, so the node id alone
/// does not say which control was pressed.
#[cfg(target_os = "macos")]
pub type WindowAction = (iced::window::Id, ActionRequest);

#[cfg(target_os = "macos")]
struct WindowEntry<Message> {
    latest_tree: Arc<Mutex<Option<TreeUpdate>>>,
    snapshot: Option<Snapshot<Message>>,
    adapter: accesskit_macos::SubclassingAdapter,
    scale: Scale,
}

/// Hands a tree update's NSAccessibility notifications to the next turn of the
/// main queue instead of posting them here.
///
/// `QueuedEvents::raise` is synchronous, and its own documentation warns that
/// accessibility methods on the view may be called while it runs. In practice
/// it lets AppKit service the run loop, which drains the blocks winit queues
/// for its event handling — so raising from inside a `Bridge`/`WindowBridges`
/// update, which iced calls from inside its own event-loop turn, re-enters the
/// runner. A widget operation then runs against a half-built interface: an
/// empty `container` whose `operate` unwraps the first laid-out child took the
/// whole process down, through an Objective-C frame that cannot unwind.
///
/// Deferring costs one run-loop turn of latency and removes the re-entrancy:
/// the notifications go out with no iced frame on the stack, so anything
/// AppKit does in response starts a fresh turn instead of nesting inside one.
///
/// `QueuedEvents` is `!Send`, so it never leaves this thread: the queue is a
/// thread-local and the drain the main queue runs is a bare `fn`. Both only
/// ever run on the main thread, which is the only thread an adapter attaches
/// on.
#[cfg(target_os = "macos")]
fn raise_on_next_turn(events: accesskit_macos::QueuedEvents) {
    PENDING_RAISES.with(|pending| pending.borrow_mut().push(events));
    let already_scheduled = RAISE_SCHEDULED.with(|scheduled| scheduled.replace(true));
    if already_scheduled {
        return;
    }
    dispatch2::DispatchQueue::main().exec_async(drain_pending_raises);
}

#[cfg(target_os = "macos")]
fn drain_pending_raises() {
    RAISE_SCHEDULED.with(|scheduled| scheduled.set(false));
    let pending = PENDING_RAISES.with(|pending| std::mem::take(&mut *pending.borrow_mut()));
    for events in pending {
        events.raise();
    }
}

#[cfg(target_os = "macos")]
thread_local! {
    /// Notifications waiting for the next main-queue turn. Drained whole, so a
    /// burst of updates costs one scheduled block rather than one per update.
    static PENDING_RAISES: RefCell<Vec<accesskit_macos::QueuedEvents>> =
        const { RefCell::new(Vec::new()) };
    static RAISE_SCHEDULED: Cell<bool> = const { Cell::new(false) };
}

#[cfg(target_os = "macos")]
struct WindowActions {
    window: iced::window::Id,
    sender: iced::futures::channel::mpsc::Sender<WindowAction>,
}

#[cfg(target_os = "macos")]
impl accesskit::ActionHandler for WindowActions {
    fn do_action(&mut self, request: ActionRequest) {
        // Same bounded, non-blocking hand-off as the single-window bridge.
        let _ = self.sender.try_send((self.window, request));
    }
}

#[cfg(target_os = "macos")]
impl<Message> fmt::Debug for WindowBridges<Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowBridges")
            .field("id", &self.id)
            .field("windows", &self.windows.len())
            .finish()
    }
}

#[cfg(target_os = "macos")]
impl<Message> WindowBridges<Message> {
    pub fn new() -> Self {
        let id = NEXT_BRIDGE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (sender, receiver) = iced::futures::channel::mpsc::channel(ACCESSIBILITY_ACTION_BUFFER);
        Self {
            id,
            receiver: Arc::new(Mutex::new(Some(receiver))),
            sender,
            windows: HashMap::default(),
        }
    }

    pub fn subscription(&self) -> Subscription<WindowAction> {
        Subscription::run_with(
            WindowActionSubscription {
                id: self.id,
                receiver: Arc::clone(&self.receiver),
            },
            window_action_stream,
        )
    }

    /// The windows currently exporting a native tree, so generated code can
    /// refresh each one after a message without walking the closed ones.
    pub fn attached(&self) -> Vec<iced::window::Id> {
        self.windows.keys().copied().collect()
    }

    pub fn is_attached(&self, window: iced::window::Id) -> bool {
        self.windows.contains_key(&window)
    }

    /// Attaches NSAccessibility to one window's AppKit view.
    ///
    /// Refuses off the main thread and refuses a window already attached —
    /// `accesskit_macos` panics when a view is subclassed twice, and a daemon
    /// can be told a window opened more than once.
    pub fn attach(&mut self, window: NativeWindow) -> bool {
        let on_main_thread = objc2::MainThreadMarker::new().is_some();
        if !on_main_thread || self.windows.contains_key(&window.id) {
            return false;
        }
        let latest_tree = Arc::new(Mutex::new(None));
        // SAFETY: `ns_view` is the `NSView` pointer AppKit gave Iced for this
        // window through `raw-window-handle`, read in `native_window` on the
        // window-owning thread; Iced owns the window until it closes, and
        // `close` drops this adapter when it does.
        let mut refresh = self.sender.clone();
        let refresh_window = window.id;
        #[expect(
            unsafe_code,
            reason = "accesskit_macos takes the NSView as a raw pointer; there is no safe constructor"
        )]
        let adapter = unsafe {
            accesskit_macos::SubclassingAdapter::new(
                window.ns_view as *mut core::ffi::c_void,
                Activation {
                    latest_tree: Arc::clone(&latest_tree),
                    refresh: Box::new(move || {
                        let _ = refresh.try_send((refresh_window, refresh_request()));
                    }),
                },
                WindowActions {
                    window: window.id,
                    sender: self.sender.clone(),
                },
            )
        };
        self.windows.insert(
            window.id,
            WindowEntry {
                latest_tree,
                snapshot: None,
                adapter,
                scale: Scale::default(),
            },
        );
        true
    }

    /// Drops the window's adapter, which restores the view's original class.
    pub fn close(&mut self, window: iced::window::Id) {
        self.windows.remove(&window);
    }

    /// Publishes one window's tree through that window's adapter.
    pub fn update(&mut self, window: iced::window::Id, snapshot: Snapshot<Message>) {
        let Some(entry) = self.windows.get_mut(&window) else {
            return;
        };
        let mut update = snapshot.update.clone();
        entry.scale.apply(&mut update);
        if let Some(events) = entry.adapter.update_if_active(|| update.clone()) {
            raise_on_next_turn(events);
        }
        *entry.latest_tree.lock().expect("accessibility tree lock") = Some(update);
        entry.snapshot = Some(snapshot);
    }

    /// One window's share of the daemon's `scale` setting.
    pub fn set_application_scale(&mut self, window: iced::window::Id, scale: f32) {
        if let Some(entry) = self.windows.get_mut(&window) {
            entry.scale.application = scale;
        }
    }

    /// Applies focus and scale truth for one window.
    pub fn window_event(&mut self, window: iced::window::Id, event: iced::window::Event) {
        if matches!(event, iced::window::Event::Closed) {
            self.close(window);
            return;
        }
        if let iced::window::Event::Rescaled(scale) = event {
            if let Some(entry) = self.windows.get_mut(&window) {
                entry.scale.window = scale;
            }
            return;
        }
        let focused = match event {
            iced::window::Event::Focused => true,
            iced::window::Event::Unfocused => false,
            _ => return,
        };
        let Some(entry) = self.windows.get_mut(&window) else {
            return;
        };
        if let Some(events) = entry.adapter.update_view_focus_state(focused) {
            raise_on_next_turn(events);
        }
    }
}

#[cfg(target_os = "macos")]
impl<Message: Clone + Send + 'static> WindowBridges<Message> {
    /// Routes an action against the snapshot of the window it came from.
    pub fn dispatch(&self, window: iced::window::Id, request: ActionRequest) -> Task<Message> {
        self.windows
            .get(&window)
            .and_then(|entry| entry.snapshot.as_ref())
            .map_or_else(Task::none, |snapshot| snapshot.dispatch(request))
    }
}

#[cfg(target_os = "macos")]
impl<Message> Default for WindowBridges<Message> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone)]
struct WindowActionSubscription {
    id: u64,
    receiver: Arc<Mutex<Option<iced::futures::channel::mpsc::Receiver<WindowAction>>>>,
}

#[cfg(target_os = "macos")]
impl PartialEq for WindowActionSubscription {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[cfg(target_os = "macos")]
impl Eq for WindowActionSubscription {}

#[cfg(target_os = "macos")]
impl Hash for WindowActionSubscription {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(target_os = "macos")]
fn window_action_stream(
    subscription: &WindowActionSubscription,
) -> iced::futures::channel::mpsc::Receiver<WindowAction> {
    subscription
        .receiver
        .lock()
        .expect("accessibility action receiver lock")
        .take()
        .unwrap_or_else(|| {
            let (_sender, receiver) =
                iced::futures::channel::mpsc::channel(ACCESSIBILITY_ACTION_BUFFER);
            receiver
        })
}

/// Focuses the next enabled semantic/native focus target in view-tree order.
pub fn focus_next<Message>() -> Task<Message> {
    iced::widget::operation::focus_next()
}

/// Focuses the previous enabled semantic/native focus target in view-tree order.
pub fn focus_previous<Message>() -> Task<Message> {
    iced::widget::operation::focus_previous()
}

/// Adds gradient stops after discarding malformed stops from an extern value.
pub fn add_gradient_stops(
    linear: iced::gradient::Linear,
    stops: impl IntoIterator<Item = iced::gradient::ColorStop>,
) -> iced::gradient::Linear {
    iced::gradient::Linear::new(linear.angle)
        .add_stops(linear.stops.into_iter().flatten())
        .add_stops(stops)
}

/// Converts viewer scale bounds to a finite, positive, ordered `f32` range.
pub fn viewer_scale_bounds(min: f64, max: f64) -> (f32, f32) {
    let positive = |value: f64| {
        let value = value as f32;
        if value.is_nan() {
            f32::EPSILON
        } else {
            value.clamp(f32::EPSILON, f32::MAX)
        }
    };
    let min = positive(min);
    let max = positive(max);
    (min.min(max), min.max(max))
}

/// Converts progress inputs to a finite, ordered range and bounded value.
/// The value one accessibility step from `value` lands on, or `None` when the
/// control is already at that end of its range — which leaves the action
/// unexported, so a screen reader hears the edge instead of a no-op.
///
/// Generated sliders feed the result to their change route: Increment and
/// Decrement then run the same handler a drag does, with the same typed value.
pub fn step_value<T>(value: T, min: T, max: T, step: T, up: bool) -> Option<T>
where
    T: Copy + Into<f64> + num_traits::FromPrimitive,
{
    let (min, max): (f64, f64) = (min.into(), max.into());
    let current: f64 = value.into();
    let step: f64 = step.into();
    let next = if up { current + step } else { current - step }.clamp(min, max);
    (next != current && next.is_finite())
        .then(|| T::from_f64(next))
        .flatten()
}

pub fn progress_range(min: f64, max: f64, value: f64) -> (std::ops::RangeInclusive<f32>, f32) {
    let finite = |value: f64| {
        let value = value as f32;
        if value.is_nan() {
            0.0
        } else {
            value.clamp(-f32::MAX, f32::MAX)
        }
    };
    let min = finite(min);
    let max = finite(max);
    let (min, max) = (min.min(max), min.max(max));
    let value = finite(value).clamp(min, max);
    (min..=max, value)
}

/// Returns animation time remaining without letting overshooting easing produce a negative duration.
pub fn animation_remaining_millis(
    animation: &iced::Animation<bool>,
    at: iced::time::Instant,
) -> f64 {
    animation
        .clone()
        .easing(iced::animation::Easing::Linear)
        .remaining(at)
        .as_secs_f64()
        * 1_000.0
}

/// Bounds spacing so Iced can multiply it by every gap without overflowing.
pub fn bounded_spacing(spacing: f64, entries: usize) -> f32 {
    let spacing = bounded_nonnegative_f32(spacing);
    let gaps = entries.saturating_sub(1) as f32;
    if gaps <= 1.0 {
        spacing
    } else {
        spacing.min((f32::MAX / gaps).next_down())
    }
}

/// Converts padding without letting opposing sides overflow Iced's `f32` totals.
pub fn bounded_padding(top: f64, right: f64, bottom: f64, left: f64) -> Padding {
    let top = bounded_nonnegative_f32(top);
    let left = bounded_nonnegative_f32(left);
    Padding {
        top,
        right: bounded_nonnegative_f32(right).min(f32::MAX - left),
        bottom: bounded_nonnegative_f32(bottom).min(f32::MAX - top),
        left,
    }
}

/// Splits text into the units a tracked `text` renders one widget per.
///
/// Grapheme clusters, never `char`s: tracking already gives up shaping and
/// kerning, but splitting inside a cluster would separate a combining mark or
/// an emoji sequence from its base and render mojibake rather than wide text.
pub fn graphemes(value: &str) -> impl Iterator<Item = &str> {
    unicode_segmentation::UnicodeSegmentation::graphemes(value, true)
}

/// Bounds one table padding/separator metric across an entire row or column.
pub fn bounded_table_metric(value: f64, entries: usize) -> f32 {
    let terms = entries.max(1) as f32 * 3.0;
    bounded_nonnegative_f32(value).min((f32::MAX / terms).next_down())
}

fn bounded_nonnegative_f32(value: f64) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, f64::from(f32::MAX)) as f32
    }
}

/// Bounds a fill factor so Iced can sum its peers in a `u16`.
pub fn bounded_fill_length(length: impl Into<Length>, entries: usize) -> Length {
    let length = length.into();
    let max_factor = u16::try_from(entries.max(1)).map_or(0, |entries| u16::MAX / entries);
    match length {
        Length::Fill | Length::FillPortion(_) if max_factor == 0 => Length::Shrink,
        Length::FillPortion(factor) => Length::FillPortion(factor.min(max_factor)),
        length => length,
    }
}

/// Bounds one axis of an element only when its native fill-factor sum would overflow.
pub fn bounded_fill_element<'a, Message, Theme, Renderer>(
    content: Element<'a, Message, Theme, Renderer>,
    entries: usize,
    horizontal: bool,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    if entries <= 1 {
        return content;
    }
    Element::new(BoundedFill {
        content,
        entries,
        horizontal,
    })
}

struct BoundedFill<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    entries: usize,
    horizontal: bool,
}

impl<Message, Theme, Renderer> BoundedFill<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn size(&self, mut size: Size<Length>) -> Size<Length> {
        let length = if self.horizontal {
            &mut size.width
        } else {
            &mut size.height
        };
        *length = bounded_fill_length(*length, self.entries);
        size
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for BoundedFill<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.size(self.content.as_widget().size())
    }

    fn size_hint(&self) -> Size<Length> {
        self.size(self.content.as_widget().size_hint())
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut widget::Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

/// Crops a screenshot after checking the invariants assumed by Iced's native crop.
pub fn crop_screenshot(
    screenshot: &iced::window::Screenshot,
    region: Rectangle<u32>,
) -> Result<iced::window::Screenshot, iced::window::screenshot::CropError> {
    use iced::window::screenshot::CropError;

    if region.width == 0 || region.height == 0 {
        return Err(CropError::Zero);
    }
    let in_bounds = region
        .x
        .checked_add(region.width)
        .is_some_and(|right| right <= screenshot.size.width)
        && region
            .y
            .checked_add(region.height)
            .is_some_and(|bottom| bottom <= screenshot.size.height);
    let expected = u128::from(screenshot.size.width) * u128::from(screenshot.size.height) * 4;
    if !in_bounds || expected != screenshot.rgba.len() as u128 {
        return Err(CropError::OutOfBounds);
    }
    screenshot.crop(region)
}

#[cfg(test)]
#[global_allocator]
static TEST_GLOBAL: &stats_alloc::StatsAlloc<std::alloc::System> =
    &stats_alloc::INSTRUMENTED_SYSTEM;

#[cfg(test)]
#[allow(clippy::let_unit_value)]
mod tests {
    use super::*;
    use iced::advanced::widget::Tree as WidgetTree;
    use iced::advanced::widget::operation;
    use iced::advanced::{Layout, Widget, layout};
    use iced::{Font, Pixels, Point, Theme};
    use iced_test::futures::futures::StreamExt;
    use iced_test::runtime::UserInterface;
    use iced_test::runtime::user_interface;

    type TestRenderer = iced_test::renderer::Renderer;
    type TestUi<'a> = UserInterface<'a, Message, Theme, TestRenderer>;
    type TestElement<'a> = Element<'a, Message, Theme, TestRenderer>;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        First,
        Last,
        Next,
        Previous,
    }

    #[test]
    fn mount_boot_answers_first_sighting_and_reboots_after_prune() {
        let state: MountedComponentState<i32> = MountedComponentState::default();

        state.begin_render();
        assert!(state.mount_boot("App/Id(1)/pane".to_owned()));
        assert!(
            !state.mount_boot("App/Id(1)/pane".to_owned()),
            "one sighting per materialized instance"
        );
        state.finish_render("App/Id(1)");

        // Present again next pass: already booted.
        state.begin_render();
        assert!(!state.mount_boot("App/Id(1)/pane".to_owned()));
        state.finish_render("App/Id(1)");

        // Absent for a pass: the prune drops the booted mark with the
        // instance, so coming back boots again.
        state.begin_render();
        state.finish_render("App/Id(1)");
        state.begin_render();
        assert!(state.mount_boot("App/Id(1)/pane".to_owned()));
        state.finish_render("App/Id(1)");
    }

    #[test]
    fn mounted_component_state_prunes_scopes_and_drops_abort_handles() {
        let state = MountedComponentState::default();
        assert_eq!(state.next_generation(), 1);
        let (_, handle) = iced::Task::<()>::none().abortable();
        let observer = handle.clone();
        state
            .values_mut()
            .insert("app/search".into(), Some(handle.abort_on_drop()));
        state.values_mut().insert("app/keep".into(), None);
        state.values_mut().insert("other/search".into(), None);

        state.begin_render();
        state.mount("app/keep".into());
        state.finish_render("app");
        // Pruning lands at the start of the next pass, so a subtree still
        // being built cannot be mistaken for one that left.
        state.begin_render();

        assert!(observer.is_aborted());
        assert_eq!(state.values().len(), 2);
        assert!(state.values().contains_key("app/keep"));
        assert!(state.values().contains_key("other/search"));
        state.finish_render("app");
        state.begin_render();
        assert_eq!(state.values().len(), 1);
        assert_eq!(state.next_generation(), 2);
    }

    /// `view` returning is not the end of building the tree: a `responsive`
    /// builds its subtree during layout, so a component under one mounts after
    /// its root has finished rendering. Pruning there would drop state the
    /// pass was still about to claim — and rebuilding it every pass restarts
    /// any animation it holds, which is a highlight that never goes out.
    #[test]
    fn a_scope_mounted_after_its_root_finished_survives_the_next_pass() {
        let state = MountedComponentState::<u32>::default();

        state.begin_render();
        state.finish_render("app");
        // The deferred builder runs now, after the root reported it was done.
        state.values_mut().insert("app/deferred".into(), 7);
        state.mount("app/deferred".into());

        state.begin_render();
        state.finish_render("app");
        assert_eq!(
            state.values().get("app/deferred"),
            Some(&7),
            "a deferred mount is not a scope that left the tree"
        );

        // A scope that really does stop rendering still goes, one pass later.
        state.begin_render();
        state.finish_render("app");
        state.begin_render();
        assert!(state.values().is_empty());
    }

    #[test]
    fn safely_adds_stops_to_malformed_gradients() {
        let mut malformed = iced::gradient::Linear::new(iced::Radians(0.0));
        malformed.stops[0] = Some(iced::gradient::ColorStop {
            offset: f32::NAN,
            color: iced::Color::BLACK,
        });
        let safe = add_gradient_stops(
            malformed,
            [iced::gradient::ColorStop {
                offset: 0.5,
                color: iced::Color::WHITE,
            }],
        );

        assert_eq!(safe.stops[0].map(|stop| stop.offset), Some(0.5));
        assert!(safe.stops[1..].iter().all(Option::is_none));
    }

    #[test]
    fn normalizes_viewer_scale_bounds() {
        assert_eq!(viewer_scale_bounds(4.0, 0.5), (0.5, 4.0));
        assert_eq!(
            viewer_scale_bounds(f64::NAN, f64::INFINITY),
            (f32::EPSILON, f32::MAX)
        );
    }

    #[test]
    fn normalizes_progress_ranges() {
        assert_eq!(progress_range(10.0, -10.0, 20.0), (-10.0..=10.0, 10.0));
        assert_eq!(progress_range(f64::NAN, 1.0, f64::NAN), (0.0..=1.0, 0.0));
    }

    #[test]
    fn reads_remaining_time_with_overshooting_easing() {
        let started = iced::time::Instant::now();
        let animation = iced::Animation::new(false)
            .duration(std::time::Duration::from_secs(1))
            .easing(iced::animation::Easing::EaseOutBack)
            .go(true, started);
        let halfway = started
            .checked_add(std::time::Duration::from_millis(500))
            .expect("halfway instant");

        assert_eq!(animation_remaining_millis(&animation, halfway), 500.0);
    }

    #[test]
    fn bounds_native_spacing() {
        assert_eq!(bounded_spacing(f64::NAN, 3), 0.0);
        assert_eq!(bounded_spacing(-1.0, 3), 0.0);
        assert_eq!(bounded_spacing(8.0, 3), 8.0);
        for entries in [0, 1, 2, 3, usize::MAX] {
            let spacing = bounded_spacing(f64::MAX, entries);
            assert!((spacing * entries.saturating_sub(1) as f32).is_finite());
        }
    }

    #[test]
    fn bounds_native_padding() {
        let padding = bounded_padding(f64::MAX, f64::MAX, f64::MAX, f64::MAX);
        assert!(padding.x().is_finite());
        assert!(padding.y().is_finite());
        assert_eq!(bounded_padding(f64::NAN, -1.0, 2.0, 3.0).top, 0.0);
    }

    #[test]
    fn bounds_native_table_metrics() {
        for entries in [0, 1, 2, usize::MAX] {
            let metric = bounded_table_metric(f64::MAX, entries);
            let spacing = metric * 2.0 + metric;
            let total = spacing * entries.saturating_sub(1) as f32 + metric * 2.0;
            assert!(total.is_finite());
        }
    }

    #[test]
    fn bounds_native_fill_factors() {
        assert_eq!(
            bounded_fill_length(Length::FillPortion(u16::MAX), 2),
            Length::FillPortion(u16::MAX / 2)
        );
        assert_eq!(
            bounded_fill_length(Length::Fill, usize::from(u16::MAX) + 1),
            Length::Shrink
        );

        let column_item: TestElement<'_> = iced::widget::space()
            .height(Length::FillPortion(u16::MAX))
            .into();
        assert_eq!(
            bounded_fill_element(column_item, 2, false)
                .as_widget()
                .size()
                .height,
            Length::FillPortion(u16::MAX / 2)
        );
        let row_item: TestElement<'_> = iced::widget::space()
            .width(Length::FillPortion(u16::MAX))
            .into();
        assert_eq!(
            bounded_fill_element(row_item, 2, true)
                .as_widget()
                .size()
                .width,
            Length::FillPortion(u16::MAX / 2)
        );
    }

    #[test]
    fn safely_rejects_invalid_screenshot_crops() {
        use iced::window::screenshot::CropError;

        let one = Rectangle {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        let short = iced::window::Screenshot::new(vec![0; 4], Size::new(2, 2), 1.0);
        assert!(matches!(
            crop_screenshot(&short, one),
            Err(CropError::OutOfBounds)
        ));

        let valid = iced::window::Screenshot::new(vec![0; 4], Size::new(1, 1), 1.0);
        assert!(matches!(
            crop_screenshot(
                &valid,
                Rectangle {
                    x: u32::MAX,
                    y: 0,
                    width: 2,
                    height: 1,
                }
            ),
            Err(CropError::OutOfBounds)
        ));
        assert!(crop_screenshot(&valid, one).is_ok());
    }

    #[test]
    fn a_node_inside_an_identified_scroll_scrolls_into_view_on_request() {
        let far = StableId::new("far");
        let button: TestElement<'static> =
            iced::widget::button("Far").on_press(Message::First).into();
        let content: TestElement<'static> = iced::widget::column![
            iced::widget::Space::new().height(500),
            accessible(button, far, Role::Button).label("Far"),
        ]
        .into();
        let root: TestElement<'static> = iced::widget::scrollable(content)
            .id(widget::Id::new("list"))
            .height(100)
            .into();
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            root,
            Size::new(400.0, 100.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let node = |snapshot: &Snapshot<Message>| {
            snapshot
                .update
                .nodes
                .iter()
                .find(|(id, _)| *id == far.node_id())
                .map(|(_, node)| node.clone())
                .expect("far node")
        };

        let before = snapshot(&mut ui, &renderer);
        assert!(node(&before).supports_action(Action::ScrollIntoView));
        assert!(node(&before).bounds().expect("bounds").y0 >= 100.0);

        let task = before.dispatch(ActionRequest {
            action: Action::ScrollIntoView,
            target_tree: TreeId::ROOT,
            target_node: far.node_id(),
            data: None,
        });
        let mut stream = iced_test::runtime::task::into_stream(task).expect("scroll task");
        let iced_test::runtime::Action::Widget(mut operation) =
            iced_test::futures::futures::executor::block_on(stream.next()).expect("scroll output")
        else {
            panic!("a scroll request is a widget operation");
        };
        loop {
            ui.operate(&renderer, operation.as_mut());
            match operation.finish() {
                Outcome::Chain(next) => operation = next,
                Outcome::None | Outcome::Some(()) => break,
            }
        }

        let after = snapshot(&mut ui, &renderer);
        let bounds = node(&after).bounds().expect("bounds");
        assert!(
            bounds.y0 >= 0.0 && bounds.y1 <= 100.0,
            "the far button is still offscreen: {bounds:?}"
        );
    }

    fn renderer() -> TestRenderer {
        iced_test::futures::futures::executor::block_on(<TestRenderer as renderer::Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            None,
        ))
        .expect("headless renderer")
    }

    fn button(
        label: &'static str,
        id: StableId,
        message: Message,
        role: Role,
        disabled: bool,
    ) -> TestElement<'static> {
        let native: TestElement<'static> = iced::widget::button(iced::widget::text(label))
            .on_press_maybe((!disabled).then_some(message.clone()))
            .into();
        accessible(native, id, role)
            .label(label)
            .description(format!("{label} description"))
            .checked(role == Role::CheckBox)
            .disabled(disabled)
            .on_activate_maybe((!disabled).then_some(message))
            .into()
    }

    fn interface() -> (TestUi<'static>, TestRenderer) {
        let repeated = StableId::new("repeated-control");
        let children = vec![
            button("First", repeated, Message::First, Role::Button, false),
            button(
                "Disabled",
                StableId::new("disabled-control"),
                Message::First,
                Role::Button,
                true,
            ),
            button("Last", repeated, Message::Last, Role::CheckBox, false),
        ];
        let content: TestElement<'static> = iced::widget::Column::with_children(children).into();
        let root: TestElement<'static> =
            navigation(content, Message::Next, Message::Previous).into();
        let mut renderer = renderer();
        let ui = UserInterface::build(
            root,
            Size::new(400.0, 240.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        (ui, renderer)
    }

    fn snapshot(ui: &mut TestUi<'_>, renderer: &TestRenderer) -> Snapshot<Message> {
        let mut operation = SnapshotOperation::<Message>::named("Test application");
        ui.operate(renderer, &mut operation::black_box(&mut operation));
        match operation.finish() {
            Outcome::Some(snapshot) => snapshot,
            _ => panic!("snapshot operation did not finish"),
        }
    }

    #[test]
    fn a_live_text_exports_its_politeness() {
        let native: TestElement<'static> = iced::widget::text("Saved").into();
        let root: TestElement<'static> = accessible(native, StableId::new("status"), Role::Label)
            .value("Saved")
            .live(accesskit::Live::Polite)
            .into();
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            root,
            Size::new(400.0, 240.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let snapshot = snapshot(&mut ui, &renderer);
        let id = StableId::new("status").node_id();
        let (_, node) = snapshot
            .update
            .nodes
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .expect("status node");
        assert_eq!(node.live(), Some(accesskit::Live::Polite));
        assert_eq!(node.value(), Some("Saved"));
    }

    #[test]
    fn a_numeric_control_exports_its_range_and_steps_through_actions() {
        let native: TestElement<'static> =
            iced::widget::slider(0.0..=10.0, 4.0, |_| Message::First).into();
        let root: TestElement<'static> = accessible(native, StableId::new("volume"), Role::Slider)
            .label("Volume")
            .numeric(4.0, 0.0, 10.0, Some(1.0))
            .on_increment_maybe(Some(Message::Next))
            .on_decrement_maybe(Some(Message::Previous))
            .into();
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            root,
            Size::new(400.0, 240.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let snapshot = snapshot(&mut ui, &renderer);
        let id = StableId::new("volume").node_id();
        let (_, node) = snapshot
            .update
            .nodes
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .expect("slider node");
        assert_eq!(node.numeric_value(), Some(4.0));
        assert_eq!(node.min_numeric_value(), Some(0.0));
        assert_eq!(node.max_numeric_value(), Some(10.0));
        assert_eq!(node.numeric_value_step(), Some(1.0));
        assert!(node.supports_action(Action::Increment));
        assert!(node.supports_action(Action::Decrement));

        let request = |action| ActionRequest {
            action,
            target_tree: TreeId::ROOT,
            target_node: id,
            data: None,
        };
        let output = |task| {
            let mut stream = iced_test::runtime::task::into_stream(task).expect("step task");
            iced_test::futures::futures::executor::block_on(stream.next()).expect("step output")
        };
        assert!(matches!(
            output(snapshot.dispatch(request(Action::Increment))),
            iced_test::runtime::Action::Output(Message::Next)
        ));
        assert!(matches!(
            output(snapshot.dispatch(request(Action::Decrement))),
            iced_test::runtime::Action::Output(Message::Previous)
        ));
    }

    #[test]
    #[ignore = "accessibility snapshot allocation contract run explicitly in CI"]
    fn performance_contract_snapshot_finalization_allocations() {
        const SAMPLES: usize = 256;
        const ALLOCATIONS_PER_SNAPSHOT: usize = 16;

        let (mut ui, renderer) = interface();
        let mut operation = SnapshotOperation::<Message>::named("Test application");
        ui.operate(&renderer, &mut operation::black_box(&mut operation));
        assert_eq!(operation.nodes.len(), 3);

        let finish = || std::hint::black_box(operation.finish());
        std::mem::drop(finish());
        let region = stats_alloc::Region::new(TEST_GLOBAL);
        for _ in 0..SAMPLES {
            std::mem::drop(finish());
        }
        let stats = region.change();

        eprintln!(
            "256 accessibility snapshot finalizations: allocations={} bytes={}",
            stats.allocations, stats.bytes_allocated
        );
        assert_eq!(stats.allocations, SAMPLES * ALLOCATIONS_PER_SNAPSHOT);
    }

    fn semantic_nodes(snapshot: &Snapshot<Message>) -> Vec<(NodeId, &Node)> {
        snapshot
            .update
            .nodes
            .iter()
            .filter(|(id, _)| *id != ROOT_ID)
            .map(|(id, node)| (*id, node))
            .collect()
    }

    fn focus_next(ui: &mut TestUi<'_>, renderer: &TestRenderer) {
        let mut operation: Box<dyn Operation> = Box::new(operation::focusable::focus_next::<()>());
        loop {
            ui.operate(renderer, operation.as_mut());
            match operation.finish() {
                Outcome::Chain(next) => operation = next,
                Outcome::None | Outcome::Some(()) => break,
            }
        }
    }

    #[test]
    fn builds_real_accesskit_nodes_and_disambiguates_repeated_ids() {
        let (mut ui, renderer) = interface();
        let snapshot = snapshot(&mut ui, &renderer);
        let nodes = semantic_nodes(&snapshot);

        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].1.role(), Role::Button);
        assert_eq!(nodes[0].1.label(), Some("First"));
        assert_eq!(nodes[0].1.description(), Some("First description"));
        assert!(nodes[0].1.supports_action(Action::Click));
        assert!(nodes[0].1.supports_action(Action::Focus));
        assert!(nodes[1].1.is_disabled());
        assert!(!nodes[1].1.supports_action(Action::Click));
        assert_eq!(nodes[2].1.role(), Role::CheckBox);
        assert_eq!(nodes[2].1.toggled(), Some(Toggled::True));

        assert_ne!(nodes[0].0, nodes[2].0, "repeated source IDs stay unique");
        assert_eq!(snapshot.update.focus, ROOT_ID);
        assert_eq!(snapshot.actions[&nodes[0].0].activate, Some(Message::First));
        assert_eq!(snapshot.actions[&nodes[2].0].activate, Some(Message::Last));
        assert!(!snapshot.actions.contains_key(&nodes[1].0));

        let click = snapshot.dispatch(ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: nodes[0].0,
            data: None,
        });
        let mut stream = iced_test::runtime::task::into_stream(click).expect("click task");
        let action =
            iced_test::futures::futures::executor::block_on(stream.next()).expect("click output");
        assert!(matches!(
            action,
            iced_test::runtime::Action::Output(Message::First)
        ));

        let root = snapshot
            .update
            .nodes
            .iter()
            .find(|(id, _)| *id == ROOT_ID)
            .map(|(_, node)| node)
            .expect("root node");
        assert_eq!(root.label(), Some("Test application"));
        assert_eq!(root.children(), &[nodes[0].0, nodes[1].0, nodes[2].0]);
    }

    #[test]
    fn mapped_elements_retain_accesskit_semantics() {
        let inner: Element<'static, Option<()>, Theme, TestRenderer> = accessible(
            iced::widget::text("Chart"),
            StableId::new("mapped-chart"),
            Role::Image,
        )
        .label("Market chart")
        .into();
        let root: TestElement<'static> = inner.map(|_| Message::First);
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            root,
            Size::new(320.0, 200.0),
            user_interface::Cache::default(),
            &mut renderer,
        );

        let snapshot = snapshot(&mut ui, &renderer);
        let nodes = semantic_nodes(&snapshot);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].1.role(), Role::Image);
        assert_eq!(nodes[0].1.label(), Some("Market chart"));
    }

    #[test]
    fn logical_keys_keep_node_ids_when_source_order_changes() {
        fn ids(order: [(&'static str, &'static str); 2]) -> HashMap<String, NodeId> {
            let children: Vec<TestElement<'static>> = order
                .into_iter()
                .map(|(key, label)| {
                    button(
                        label,
                        StableId::new(key),
                        Message::First,
                        Role::Button,
                        false,
                    )
                })
                .collect();
            let root: TestElement<'static> = iced::widget::Column::with_children(children).into();
            let mut renderer = renderer();
            let mut ui = UserInterface::build(
                root,
                Size::new(400.0, 160.0),
                user_interface::Cache::default(),
                &mut renderer,
            );
            semantic_nodes(&snapshot(&mut ui, &renderer))
                .into_iter()
                .map(|(id, node)| (node.label().expect("label").to_owned(), id))
                .collect()
        }

        let before = ids([("item-a", "A"), ("item-b", "B")]);
        let after = ids([("item-b", "B"), ("item-a", "A")]);
        assert_eq!(before, after);
    }

    #[test]
    fn builds_hierarchy_and_suppresses_atomic_control_descendants() {
        let group_id = StableId::new("group");
        let readable_id = StableId::new("readable");
        let button_id = StableId::new("atomic-button");
        let nested_id = StableId::new("nested-button-label");

        let readable: TestElement<'static> =
            accessible(iced::widget::text("Readable"), readable_id, Role::Label)
                .value("Readable")
                .into();
        let nested: TestElement<'static> =
            accessible(iced::widget::text("Nested"), nested_id, Role::Label)
                .value("Nested")
                .into();
        let native_button: TestElement<'static> =
            iced::widget::button(nested).on_press(Message::First).into();
        let atomic: TestElement<'static> = accessible(native_button, button_id, Role::Button)
            .label("Atomic")
            .on_activate(Message::First)
            .into();
        let children = vec![readable, atomic];
        let column: TestElement<'static> = iced::widget::Column::with_children(children).into();
        let root: TestElement<'static> =
            accessible(column, group_id, Role::GenericContainer).into();
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            root,
            Size::new(400.0, 240.0),
            user_interface::Cache::default(),
            &mut renderer,
        );

        let snapshot = snapshot(&mut ui, &renderer);
        let node = |id| {
            snapshot
                .update
                .nodes
                .iter()
                .find(|(candidate, _)| *candidate == id)
                .map(|(_, node)| node)
                .expect("semantic node")
        };
        let root = node(ROOT_ID);
        let group = node(group_id.node_id());
        let readable = node(readable_id.node_id());
        let button = node(button_id.node_id());

        assert_eq!(root.children(), &[group_id.node_id()]);
        assert_eq!(
            group.children(),
            &[readable_id.node_id(), button_id.node_id()]
        );
        assert_eq!(readable.role(), Role::Label);
        assert_eq!(readable.value(), Some("Readable"));
        assert!(button.children().is_empty());
        assert!(
            snapshot
                .update
                .nodes
                .iter()
                .all(|(id, _)| *id != nested_id.node_id())
        );
    }

    #[test]
    fn password_nodes_never_expose_the_plaintext_value() {
        const SECRET: &str = "correct horse battery staple";
        let id = StableId::new("password");
        let native: TestElement<'static> = iced::widget::text_input("Password", SECRET).into();
        let root: TestElement<'static> = accessible(native, id, Role::PasswordInput)
            .label("Password")
            .value_maybe(None)
            .into();
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            root,
            Size::new(400.0, 80.0),
            user_interface::Cache::default(),
            &mut renderer,
        );

        let snapshot = snapshot(&mut ui, &renderer);
        let node = semantic_nodes(&snapshot)[0].1;
        assert_eq!(node.role(), Role::PasswordInput);
        assert_eq!(node.value(), None);
        assert!(!format!("{node:?}").contains(SECRET));
    }

    #[test]
    fn tab_order_skips_disabled_and_tree_focus_follows_operations() {
        let (mut ui, renderer) = interface();
        let initial = snapshot(&mut ui, &renderer);
        let nodes = semantic_nodes(&initial);
        let first = nodes[0].0;
        let last = nodes[2].0;

        focus_next(&mut ui, &renderer);
        assert_eq!(snapshot(&mut ui, &renderer).update.focus, first);
        focus_next(&mut ui, &renderer);
        assert_eq!(snapshot(&mut ui, &renderer).update.focus, last);

        let focus = initial.dispatch(ActionRequest {
            action: Action::Focus,
            target_tree: TreeId::ROOT,
            target_node: first,
            data: None,
        });
        let mut stream = iced_test::runtime::task::into_stream(focus).expect("focus task");
        let action = iced_test::futures::futures::executor::block_on(stream.next())
            .expect("focus operation");
        let iced_test::runtime::Action::Widget(mut operation) = action else {
            panic!("focus dispatch must produce a widget operation");
        };
        ui.operate(&renderer, operation.as_mut());
        assert_eq!(snapshot(&mut ui, &renderer).update.focus, first);

        let mut disabled_focus = FocusOperation::<Message> {
            target: SemanticFocus {
                base: StableId::new("disabled-control"),
                occurrence: 0,
            },
            occurrences: HashMap::new(),
            used_ids: HashSet::from([ROOT_ID]),
            frames: Vec::new(),
            marker: std::marker::PhantomData,
        };
        ui.operate(&renderer, &mut operation::black_box(&mut disabled_focus));
        assert_eq!(snapshot(&mut ui, &renderer).update.focus, ROOT_ID);
    }

    #[test]
    fn focus_actions_follow_disambiguated_node_ids() {
        let repeated = StableId(NodeId(42));
        let colliding = StableId(duplicate_node_id(repeated.node_id(), 1));
        let nested: TestElement<'static> =
            accessible(iced::widget::text("Nested"), repeated, Role::Label)
                .label("Nested")
                .into();
        let native_atomic: TestElement<'static> =
            iced::widget::button(nested).on_press(Message::First).into();
        let atomic: TestElement<'static> =
            accessible(native_atomic, StableId(NodeId(1_000)), Role::Button)
                .label("Atomic")
                .on_activate(Message::First)
                .into();
        let children = vec![
            button("First", repeated, Message::First, Role::Button, false),
            button("Collision", colliding, Message::First, Role::Button, false),
            atomic,
            button("Last", repeated, Message::Last, Role::Button, false),
        ];
        let root: TestElement<'static> = iced::widget::Column::with_children(children).into();
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            root,
            Size::new(400.0, 240.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let initial = snapshot(&mut ui, &renderer);
        let target = semantic_nodes(&initial)
            .into_iter()
            .find(|(_, node)| node.label() == Some("Last"))
            .map(|(id, _)| id)
            .expect("last node");

        let focus = initial.dispatch(ActionRequest {
            action: Action::Focus,
            target_tree: TreeId::ROOT,
            target_node: target,
            data: None,
        });
        let mut stream = iced_test::runtime::task::into_stream(focus).expect("focus task");
        let action = iced_test::futures::futures::executor::block_on(stream.next())
            .expect("focus operation");
        let iced_test::runtime::Action::Widget(mut operation) = action else {
            panic!("focus dispatch must produce a widget operation");
        };
        ui.operate(&renderer, operation.as_mut());

        assert_eq!(snapshot(&mut ui, &renderer).update.focus, target);
    }

    #[test]
    fn tab_and_keyboard_activation_emit_exactly_one_message() {
        let (mut ui, mut renderer) = interface();
        let mut messages = Vec::new();
        let events = iced_test::simulator::tap_key(key::Named::Tab, None).collect::<Vec<_>>();
        let _ = ui.update(
            &events,
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(messages, [Message::Next]);

        messages.clear();
        focus_next(&mut ui, &renderer);
        let events = iced_test::simulator::tap_key(key::Named::Enter, None).collect::<Vec<_>>();
        let _ = ui.update(
            &events,
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(messages, [Message::First]);

        messages.clear();
        focus_next(&mut ui, &renderer);
        let events = iced_test::simulator::tap_key(key::Named::Space, None).collect::<Vec<_>>();
        let _ = ui.update(
            &events,
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(messages, [Message::Last]);
    }

    #[test]
    fn pointer_focus_has_one_owner() {
        let (mut ui, mut renderer) = interface();
        let initial = snapshot(&mut ui, &renderer);
        let nodes = semantic_nodes(&initial);
        let first = nodes[0].0;
        let last = nodes[2].0;
        let centers = [nodes[0].1, nodes[2].1].map(|node| {
            let bounds = node.bounds().expect("semantic bounds");
            Point::new(
                ((bounds.x0 + bounds.x1) / 2.0) as f32,
                ((bounds.y0 + bounds.y1) / 2.0) as f32,
            )
        });
        drop(nodes);

        for (point, expected) in centers.into_iter().zip([first, last]) {
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
            assert_eq!(snapshot(&mut ui, &renderer).update.focus, expected);
        }
    }

    #[test]
    fn scroll_translation_reaches_semantics_and_touch_focus() {
        let target = StableId::new("scrolled-control");
        let scroll_id: widget::Id = "scrolled-semantics".into();
        let spacer: TestElement<'static> = iced::widget::Space::new().height(100.0).into();
        let control: TestElement<'static> = accessible(
            iced::widget::Space::new().width(10.0).height(20.0),
            target,
            Role::Button,
        )
        .into();
        let content: TestElement<'static> =
            iced::widget::Column::with_children(vec![spacer, control]).into();
        let root: TestElement<'static> = iced::widget::scrollable(content)
            .id(scroll_id.clone())
            .width(20.0)
            .height(50.0)
            .into();
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            root,
            Size::new(20.0, 50.0),
            user_interface::Cache::default(),
            &mut renderer,
        );

        let before = semantic_nodes(&snapshot(&mut ui, &renderer))[0]
            .1
            .bounds()
            .expect("semantic bounds");
        assert_eq!(before.y0, 100.0);

        let mut scroll = operation::scrollable::scroll_to::<()>(
            scroll_id,
            operation::scrollable::AbsoluteOffset {
                x: None,
                y: Some(100.0),
            },
        );
        ui.operate(&renderer, &mut operation::black_box(&mut scroll));

        let after = semantic_nodes(&snapshot(&mut ui, &renderer))[0]
            .1
            .bounds()
            .expect("semantic bounds");
        assert_eq!(after.y0, 30.0);

        let point = Point::new(5.0, 40.0);
        let mut messages = Vec::new();
        let _ = ui.update(
            &[Event::Touch(iced::touch::Event::FingerPressed {
                id: iced::touch::Finger(0),
                position: point,
            })],
            mouse::Cursor::Available(point),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(snapshot(&mut ui, &renderer).update.focus, target.node_id());
    }

    #[test]
    fn keeps_exported_accessibility_bounds_finite() {
        let mut operation = SnapshotOperation::<Message>::named("Test application");
        operation.translation = Vector::new(-f32::MAX, -f32::MAX);
        let mut state: SemanticState<Message> = SemanticState {
            semantics: Semantics::new(StableId::new("extreme-bounds"), Role::Button),
            focus_visible: false,
        };
        let bounds = Rectangle::new(
            Point::new(f32::MAX, f32::MAX),
            Size::new(f32::MAX, f32::MAX),
        );
        operation.custom(None, bounds, &mut state.semantics.snapshot);
        operation.custom(None, bounds, &mut state);
        operation.custom(None, Rectangle::default(), &mut SemanticEnd);
        let Outcome::Some(snapshot) = operation.finish() else {
            panic!("snapshot operation did not finish");
        };
        let bounds = semantic_nodes(&snapshot)[0]
            .1
            .bounds()
            .expect("semantic bounds");

        assert!(
            [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
                .into_iter()
                .all(f64::is_finite)
        );
    }

    #[derive(Default)]
    struct OperationCounts {
        focusable: usize,
        text_input: usize,
    }

    impl Operation for OperationCounts {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }

        fn focusable(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            _state: &mut dyn Focusable,
        ) {
            self.focusable += 1;
        }

        fn text_input(
            &mut self,
            _id: Option<&widget::Id>,
            _bounds: Rectangle,
            _state: &mut dyn TextInput,
        ) {
            self.text_input += 1;
        }
    }

    #[test]
    fn disabled_inputs_preserve_text_operations_but_filter_focus() {
        let id = StableId::new("disabled-input");
        let native: TestElement<'static> = iced::widget::text_input("", "value")
            .id(id.widget_id())
            .into();
        let root: TestElement<'static> = accessible(native, id, Role::TextInput)
            .disabled(true)
            .focus_id(id.widget_id())
            .into();
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            root,
            Size::new(400.0, 80.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let mut counts = OperationCounts::default();

        ui.operate(&renderer, &mut operation::black_box(&mut counts));

        assert_eq!(counts.text_input, 1);
        assert_eq!(counts.focusable, 0);
        assert_eq!(snapshot(&mut ui, &renderer).update.focus, ROOT_ID);
    }

    #[test]
    fn a_text_input_exports_its_caret_as_a_text_run_and_moves_it_on_request() {
        let id = StableId::new("caret-input");
        let value = "h\u{e9}llo w\u{f6}rld";
        let native: TestElement<'static> = iced::widget::text_input("", value)
            .id(id.widget_id())
            .into();
        let root: TestElement<'static> = accessible(native, id, Role::TextInput)
            .label("Greeting")
            .value(value)
            .focus_id(id.widget_id())
            .into();
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            root,
            Size::new(400.0, 80.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let mut place = operation::text_input::move_cursor_to::<()>(id.widget_id(), 3);
        ui.operate(&renderer, &mut operation::black_box(&mut place));

        let before = snapshot(&mut ui, &renderer);
        let input = |snapshot: &Snapshot<Message>| {
            snapshot
                .update
                .nodes
                .iter()
                .find(|(candidate, _)| *candidate == id.node_id())
                .map(|(_, node)| node.clone())
                .expect("input node")
        };
        let node = input(&before);
        let [run_id] = node.children() else {
            panic!(
                "the input owns exactly one text run, got {:?}",
                node.children()
            );
        };
        let (_, run) = before
            .update
            .nodes
            .iter()
            .find(|(candidate, _)| candidate == run_id)
            .expect("text run node");
        assert_eq!(run.role(), Role::TextRun);
        assert_eq!(run.value(), Some(value));
        assert_eq!(
            run.character_lengths(),
            &[1, 2, 1, 1, 1, 1, 1, 2, 1, 1, 1],
            "one entry per grapheme, in UTF-8 bytes"
        );
        let selection = node.text_selection().expect("a caret");
        assert_eq!(selection.focus.node, *run_id);
        assert_eq!(selection.focus.character_index, 3);
        assert_eq!(selection.anchor.character_index, 3);

        let request = before.dispatch(ActionRequest {
            action: Action::SetTextSelection,
            target_tree: TreeId::ROOT,
            target_node: id.node_id(),
            data: Some(accesskit::ActionData::SetTextSelection(
                accesskit::TextSelection {
                    anchor: accesskit::TextPosition {
                        node: *run_id,
                        character_index: 7,
                    },
                    focus: accesskit::TextPosition {
                        node: *run_id,
                        character_index: 7,
                    },
                },
            )),
        });
        let mut stream = iced_test::runtime::task::into_stream(request).expect("caret task");
        let action = iced_test::futures::futures::executor::block_on(stream.next())
            .expect("caret operation");
        let iced_test::runtime::Action::Widget(mut operation) = action else {
            panic!("a caret request must produce a widget operation");
        };
        ui.operate(&renderer, operation.as_mut());
        let after = input(&snapshot(&mut ui, &renderer));
        assert_eq!(
            after
                .text_selection()
                .expect("a caret")
                .focus
                .character_index,
            7
        );
    }

    struct CapturesTab;

    impl Widget<Message, Theme, TestRenderer> for CapturesTab {
        fn size(&self) -> Size<Length> {
            Size::new(Length::Fixed(80.0), Length::Fixed(30.0))
        }

        fn layout(
            &mut self,
            _tree: &mut WidgetTree,
            _renderer: &TestRenderer,
            _limits: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(Size::new(80.0, 30.0))
        }

        fn draw(
            &self,
            _tree: &WidgetTree,
            _renderer: &mut TestRenderer,
            _theme: &Theme,
            _style: &renderer::Style,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }

        fn update(
            &mut self,
            _tree: &mut WidgetTree,
            event: &Event,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _renderer: &TestRenderer,
            _clipboard: &mut dyn Clipboard,
            shell: &mut Shell<'_, Message>,
            _viewport: &Rectangle,
        ) {
            if matches!(
                event,
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(key::Named::Tab),
                    ..
                })
            ) {
                shell.publish(Message::First);
                shell.capture_event();
            }
        }
    }

    #[test]
    fn navigation_defers_to_children_and_ignores_modified_tab() {
        let child: TestElement<'static> = Element::new(CapturesTab);
        let root: TestElement<'static> = navigation(child, Message::Next, Message::Previous).into();
        let mut renderer = renderer();
        let mut ui = UserInterface::build(
            root,
            Size::new(400.0, 80.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let mut messages = Vec::new();
        let event = iced_test::simulator::press_key(key::Named::Tab, None);
        let _ = ui.update(
            &[event],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert_eq!(messages, [Message::First]);

        let passive: TestElement<'static> = iced::widget::Space::new().into();
        let root: TestElement<'static> =
            navigation(passive, Message::Next, Message::Previous).into();
        let cache = ui.into_cache();
        let mut ui = UserInterface::build(root, Size::new(400.0, 80.0), cache, &mut renderer);
        messages.clear();
        let Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modified_key,
            physical_key,
            location,
            repeat,
            text,
            ..
        }) = iced_test::simulator::press_key(key::Named::Tab, None)
        else {
            unreachable!()
        };
        let event = Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modified_key,
            physical_key,
            location,
            modifiers: keyboard::Modifiers::CTRL,
            repeat,
            text,
        });
        let _ = ui.update(
            &[event],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert!(messages.is_empty());
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
            panic!("test leaf never allocates images");
        }
    }

    struct Leaf;

    impl Widget<Message, (), RecordingRenderer> for Leaf {
        fn size(&self) -> Size<Length> {
            Size::new(Length::Fixed(80.0), Length::Fixed(30.0))
        }

        fn layout(
            &mut self,
            _tree: &mut WidgetTree,
            _renderer: &RecordingRenderer,
            _limits: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(Size::new(80.0, 30.0))
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
    fn keyboard_focused_wrapper_draws_a_visible_outline() {
        let id = StableId::new("focus-ring");
        let leaf: Element<'_, Message, (), RecordingRenderer> = Element::new(Leaf);
        let mut element: Element<'_, Message, (), RecordingRenderer> =
            accessible(leaf, id, Role::Button).label("Focusable").into();
        let mut tree = WidgetTree::new(&element);
        let mut renderer = RecordingRenderer::default();
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(100.0, 100.0)),
        );
        let mut focus = operation::focusable::focus::<()>(id.widget_id());
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
            &Rectangle::with_size(Size::new(100.0, 100.0)),
        );

        assert_eq!(renderer.quads.len(), 1);
        assert_eq!(renderer.quads[0].border.width, 2.0);
        assert_eq!(renderer.quads[0].border.color, iced::Color::WHITE);
    }

    /// Runs a focus operation the way a daemon does: through every window's
    /// tree in turn, then the chained operation the same way.
    fn traverse_windows(
        mut operation: Box<dyn Operation<()>>,
        windows: &mut [(Element<'_, Message, (), RecordingRenderer>, WidgetTree)],
        renderer: &RecordingRenderer,
    ) {
        let viewport = Rectangle::with_size(Size::new(100.0, 100.0));
        loop {
            for (element, tree) in windows.iter_mut() {
                let node = element.as_widget_mut().layout(
                    tree,
                    renderer,
                    &layout::Limits::new(Size::ZERO, viewport.size()),
                );
                element.as_widget_mut().operate(
                    tree,
                    Layout::new(&node),
                    renderer,
                    operation.as_mut(),
                );
            }
            match operation.finish() {
                Outcome::Chain(next) => operation = next,
                Outcome::None | Outcome::Some(()) => return,
            }
        }
    }

    fn window_button_focus(tree: &WidgetTree) -> (bool, bool) {
        let state = tree.children[0]
            .state
            .downcast_ref::<SemanticState<Message>>();
        (state.semantics.focused, state.focus_visible)
    }

    #[test]
    fn scoped_traversal_stays_inside_its_window() {
        let first = iced::window::Id::unique();
        let second = iced::window::Id::unique();
        let renderer = RecordingRenderer::default();
        let mut windows: Vec<(Element<'_, Message, (), RecordingRenderer>, WidgetTree)> =
            [first, second]
                .into_iter()
                .map(|window| {
                    let leaf: Element<'_, Message, (), RecordingRenderer> = Element::new(Leaf);
                    let button: Element<'_, Message, (), RecordingRenderer> =
                        accessible(leaf, StableId::new("go"), Role::Button)
                            .label("Go")
                            .into();
                    let root: Element<'_, Message, (), RecordingRenderer> = Element::new(
                        navigation(button, Message::Next, Message::Previous).in_window(window),
                    );
                    let tree = WidgetTree::new(&root);
                    (root, tree)
                })
                .collect();

        // Tab in the second window focuses its control and leaves the first
        // window's alone, although the first window's tree ran first.
        traverse_windows(
            Box::new(ScopedTraversal::<()>::counting(second, Direction::Next)),
            &mut windows,
            &renderer,
        );
        assert_eq!(window_button_focus(&windows[0].1), (false, false));
        assert_eq!(window_button_focus(&windows[1].1), (true, true));

        // Each window keeps a focus of its own: Tab in the first window does
        // not take the second's away.
        traverse_windows(
            Box::new(ScopedTraversal::<()>::counting(first, Direction::Next)),
            &mut windows,
            &renderer,
        );
        assert_eq!(window_button_focus(&windows[0].1), (true, true));
        assert_eq!(window_button_focus(&windows[1].1), (true, true));

        // Shift+Tab wraps within the window: one control stays focused.
        traverse_windows(
            Box::new(ScopedTraversal::<()>::counting(first, Direction::Previous)),
            &mut windows,
            &renderer,
        );
        assert_eq!(window_button_focus(&windows[0].1), (true, true));
    }

    #[test]
    fn scoped_snapshot_describes_one_window_and_the_unscoped_one_holds_all() {
        let first = iced::window::Id::unique();
        let second = iced::window::Id::unique();
        let renderer = RecordingRenderer::default();
        // The same Ice id in both windows, which is what a daemon's shared
        // component graph produces and what an unscoped snapshot cannot tell
        // apart. The labels differ so a captured node names its window.
        let build = |window: iced::window::Id,
                     label: &'static str|
         -> (Element<'_, Message, (), RecordingRenderer>, WidgetTree) {
            let leaf: Element<'_, Message, (), RecordingRenderer> = Element::new(Leaf);
            let button: Element<'_, Message, (), RecordingRenderer> =
                accessible(leaf, StableId::new("go"), Role::Button)
                    .label(label)
                    .on_activate(Message::Next)
                    .into();
            let root: Element<'_, Message, (), RecordingRenderer> = Element::new(
                navigation(button, Message::Next, Message::Previous).in_window(window),
            );
            let tree = WidgetTree::new(&root);
            (root, tree)
        };

        let labels = |operation: SnapshotOperation<Message>| -> Vec<String> {
            let mut windows = [build(first, "First"), build(second, "Second")];
            let viewport = Rectangle::with_size(Size::new(100.0, 100.0));
            let mut operation: Box<dyn Operation<Snapshot<Message>>> = Box::new(operation);
            loop {
                for (element, tree) in windows.iter_mut() {
                    let node = element.as_widget_mut().layout(
                        tree,
                        &renderer,
                        &layout::Limits::new(Size::ZERO, viewport.size()),
                    );
                    element.as_widget_mut().operate(
                        tree,
                        Layout::new(&node),
                        &renderer,
                        &mut widget::operation::black_box(operation.as_mut()),
                    );
                }
                match operation.finish() {
                    Outcome::Chain(next) => operation = next,
                    Outcome::Some(snapshot) => {
                        return snapshot
                            .update
                            .nodes
                            .iter()
                            .filter_map(|(_, node)| node.label().map(str::to_owned))
                            .collect();
                    }
                    Outcome::None => return Vec::new(),
                }
            }
        };

        // Scoped to one window, only that window's control joins the root —
        // even though the operation walked both windows in turn.
        assert_eq!(
            labels(SnapshotOperation::scoped("Daemon", first)),
            ["Daemon", "First"]
        );
        assert_eq!(
            labels(SnapshotOperation::scoped("Daemon", second)),
            ["Daemon", "Second"]
        );

        // Unscoped, one tree holds both windows: the shape a per-window
        // native adapter must not be handed.
        assert_eq!(
            labels(SnapshotOperation::named("Daemon")),
            ["Daemon", "First", "Second"]
        );
    }

    #[test]
    fn pointer_focused_wrapper_does_not_draw_an_outline() {
        let id = StableId::new("pointer-focus");
        let leaf: Element<'_, Message, (), RecordingRenderer> = Element::new(Leaf);
        let mut element: Element<'_, Message, (), RecordingRenderer> =
            accessible(leaf, id, Role::Button).label("Focusable").into();
        let mut tree = WidgetTree::new(&element);
        let mut renderer = RecordingRenderer::default();
        let viewport = Rectangle::with_size(Size::new(100.0, 100.0));
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let mut clipboard = iced::advanced::clipboard::Null;
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);

        element.as_widget_mut().update(
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(40.0, 15.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        drop(shell);

        let state = tree.state.downcast_ref::<SemanticState<Message>>();
        assert!(state.semantics.focused);
        assert!(!state.focus_visible);

        element.as_widget().draw(
            &tree,
            &mut renderer,
            &(),
            &renderer::Style {
                text_color: iced::Color::WHITE,
            },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &viewport,
        );

        assert!(renderer.quads.is_empty());
    }

    #[test]
    fn styled_focus_ring_follows_the_focus_origin() {
        let id = StableId::new("styled-focus-ring");
        let ring_color = iced::Color::from_rgb(0.2, 0.4, 1.0);
        let build = || -> Element<'_, Message, (), RecordingRenderer> {
            let leaf: Element<'_, Message, (), RecordingRenderer> = Element::new(Leaf);
            accessible(leaf, id, Role::Button)
                .label("Styled")
                .focus_ring(ring_color, 8.0)
                .into()
        };
        let viewport = Rectangle::with_size(Size::new(100.0, 100.0));
        let style = renderer::Style {
            text_color: iced::Color::WHITE,
        };

        // Pointer-acquired focus paints no ring at all.
        let mut element = build();
        let mut tree = WidgetTree::new(&element);
        let mut renderer = RecordingRenderer::default();
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let mut clipboard = iced::advanced::clipboard::Null;
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(40.0, 15.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        drop(shell);
        assert!(
            tree.state
                .downcast_ref::<SemanticState<Message>>()
                .semantics
                .focused
        );
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &(),
            &style,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &viewport,
        );
        assert!(renderer.quads.is_empty());

        // Keyboard-acquired focus paints the recipe's ring, not the default.
        let mut element = build();
        let mut tree = WidgetTree::new(&element);
        let mut renderer = RecordingRenderer::default();
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let mut focus = operation::focusable::focus::<()>(id.widget_id());
        element
            .as_widget_mut()
            .operate(&mut tree, Layout::new(&node), &renderer, &mut focus);
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &(),
            &style,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &viewport,
        );
        assert_eq!(renderer.quads.len(), 1);
        assert_eq!(renderer.quads[0].border.color, ring_color);
        assert_eq!(renderer.quads[0].border.width, 2.0);
        assert_eq!(renderer.quads[0].border.radius, 8.0.into());
    }

    #[test]
    fn key_press_after_pointer_focus_restores_the_outline() {
        let id = StableId::new("pointer-then-key");
        let leaf: Element<'_, Message, (), RecordingRenderer> = Element::new(Leaf);
        let mut element: Element<'_, Message, (), RecordingRenderer> =
            accessible(leaf, id, Role::Button).label("Focusable").into();
        let mut tree = WidgetTree::new(&element);
        let mut renderer = RecordingRenderer::default();
        let viewport = Rectangle::with_size(Size::new(100.0, 100.0));
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let mut clipboard = iced::advanced::clipboard::Null;
        let mut messages = Vec::new();
        for event in [
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            iced_test::simulator::press_key(key::Named::Enter, None),
        ] {
            let mut shell = Shell::new(&mut messages);
            element.as_widget_mut().update(
                &mut tree,
                &event,
                Layout::new(&node),
                mouse::Cursor::Available(Point::new(40.0, 15.0)),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }

        let state = tree.state.downcast_ref::<SemanticState<Message>>();
        assert!(state.semantics.focused);
        assert!(state.focus_visible);

        element.as_widget().draw(
            &tree,
            &mut renderer,
            &(),
            &renderer::Style {
                text_color: iced::Color::WHITE,
            },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &viewport,
        );

        assert_eq!(renderer.quads.len(), 1);
    }

    #[test]
    fn exported_bounds_follow_the_window_scale() {
        let (mut ui, renderer) = interface();
        let snapshot = snapshot(&mut ui, &renderer);
        let mut bridge = Bridge::<Message>::without_native_adapter();
        let window = iced::window::Id::unique();
        bridge.window_event(window, iced::window::Event::Rescaled(2.0));
        bridge.update(snapshot);

        let tree = bridge
            .latest_tree
            .lock()
            .expect("accessibility tree lock")
            .clone()
            .expect("published tree");
        let (_, root) = tree
            .nodes
            .iter()
            .find(|(id, _)| *id == ROOT_ID)
            .expect("root node");
        assert_eq!(root.transform(), Some(&accesskit::Affine::scale(2.0)));
    }

    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    #[test]
    fn native_adapter_action_handler_routes_requests_to_iced() {
        let (sender, mut receiver) =
            iced::futures::channel::mpsc::channel(ACCESSIBILITY_ACTION_BUFFER);
        let mut handler = Actions { sender };
        let request = ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: StableId::new("native-action").node_id(),
            data: None,
        };

        accesskit::ActionHandler::do_action(&mut handler, request.clone());

        let routed = iced_test::futures::futures::executor::block_on(receiver.next());
        assert_eq!(routed, Some(request));
    }

    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    #[test]
    fn native_adapter_action_handler_bounds_pending_requests() {
        let (sender, mut receiver) =
            iced::futures::channel::mpsc::channel(ACCESSIBILITY_ACTION_BUFFER);
        let mut handler = Actions { sender };

        for node in 1..=ACCESSIBILITY_ACTION_BUFFER + 2 {
            accesskit::ActionHandler::do_action(
                &mut handler,
                ActionRequest {
                    action: Action::Click,
                    target_tree: TreeId::ROOT,
                    target_node: NodeId(node as u64),
                    data: None,
                },
            );
        }

        let routed = (0..=ACCESSIBILITY_ACTION_BUFFER)
            .map(|_| receiver.try_recv().expect("buffered accessibility action"))
            .map(|request| request.target_node)
            .collect::<Vec<_>>();
        assert_eq!(
            routed,
            (1..=ACCESSIBILITY_ACTION_BUFFER + 1)
                .map(|node| NodeId(node as u64))
                .collect::<Vec<_>>(),
            "accepted accessibility actions must keep FIFO order"
        );
        assert!(
            receiver.try_recv().is_err(),
            "the native callback must not retain more than the configured buffer plus its sender slot"
        );

        drop(receiver);
        accesskit::ActionHandler::do_action(
            &mut handler,
            ActionRequest {
                action: Action::Click,
                target_tree: TreeId::ROOT,
                target_node: NodeId(u64::MAX),
                data: None,
            },
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_bridge_defers_adapter_until_a_window_handle_arrives() {
        let bridge = Bridge::<Message>::new();
        assert!(bridge.adapter.is_none());
        assert!(bridge.sender.is_some());
        assert!(!bridge.is_attached());

        let disabled = Bridge::<Message>::without_native_adapter();
        assert!(disabled.adapter.is_none());
        assert!(disabled.sender.is_none());
        assert!(!disabled.is_attached());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_bridge_defers_adapter_and_refuses_attachment_off_the_main_thread() {
        let mut bridge = Bridge::<Message>::new();
        assert!(bridge.adapter.is_none());
        assert!(bridge.sender.is_some());
        assert!(!bridge.is_attached());

        // The harness runs every test on a spawned thread, so this call is
        // the off-main case by construction: it must refuse, keep the sender
        // for a later main-thread attempt, and leave no adapter behind.
        let window = NativeWindow {
            id: iced::window::Id::unique(),
            ns_view: 1,
        };
        assert!(!bridge.attach_window(window));
        assert!(bridge.adapter.is_none());
        assert!(bridge.sender.is_some());
        assert!(!bridge.is_attached());

        let disabled = Bridge::<Message>::without_native_adapter();
        assert!(disabled.adapter.is_none());
        assert!(disabled.sender.is_none());
        assert!(!disabled.is_attached());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn window_bridges_start_empty_and_ignore_windows_that_never_attached() {
        let mut bridges = WindowBridges::<Message>::new();
        let window = iced::window::Id::unique();
        assert!(bridges.attached().is_empty());
        assert!(!bridges.is_attached(window));

        // An action naming a window with no adapter routes nowhere rather
        // than falling through to another window's controls.
        let request = ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: StableId::new("go").node_id(),
            data: None,
        };
        let _: Task<Message> = bridges.dispatch(window, request);

        // Closing and focusing an unknown window are both no-ops, and
        // neither invents an entry.
        bridges.close(window);
        bridges.window_event(window, iced::window::Event::Focused);
        bridges.window_event(window, iced::window::Event::Closed);
        assert!(bridges.attached().is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_window_action_handler_survives_a_flood_with_no_poll_between() {
        // What an assistive technology can do to a daemon: perform action
        // after action with the event loop never getting a turn in between.
        // The handler runs on the main thread inside an AppKit callback that
        // cannot unwind, so it must never panic and never block — past the
        // buffer it drops, and the window each request came from survives on
        // the ones that fit.
        let (sender, mut receiver) =
            iced::futures::channel::mpsc::channel(ACCESSIBILITY_ACTION_BUFFER);
        let first = iced::window::Id::unique();
        let second = iced::window::Id::unique();
        let mut handler = WindowActions {
            window: first,
            sender: sender.clone(),
        };
        let mut other = WindowActions {
            window: second,
            sender,
        };

        let flood = ACCESSIBILITY_ACTION_BUFFER * 4;
        for node in 1..=flood {
            let request = ActionRequest {
                action: Action::Click,
                target_tree: TreeId::ROOT,
                target_node: NodeId(node as u64),
                data: None,
            };
            // Alternating windows, so a dropped request cannot be mistaken
            // for one that was merely routed to the other window.
            accesskit::ActionHandler::do_action(&mut handler, request.clone());
            accesskit::ActionHandler::do_action(&mut other, request);
        }

        // Everything that fit is in order and still names its own window;
        // the rest was dropped rather than panicking or blocking.
        // `futures` reserves one slot per sender on top of the buffer, and a
        // daemon holds one sender per window, so two windows can hold two
        // more than the configured backlog. The point of the assertion is the
        // bound itself: the flood is dropped, not queued without limit.
        let routed = std::iter::from_fn(|| receiver.try_recv().ok()).collect::<Vec<_>>();
        assert_eq!(routed.len(), ACCESSIBILITY_ACTION_BUFFER + 2);
        assert!(routed.len() < flood);
        assert_eq!(routed[0].0, first);
        assert_eq!(routed[1].0, second);
        assert!(
            routed
                .iter()
                .all(|(window, _)| *window == first || *window == second)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_window_bridges_refuse_attachment_off_the_main_thread() {
        let mut bridges = WindowBridges::<Message>::new();
        let window = iced::window::Id::unique();
        // The harness runs every test on a spawned thread, so this is the
        // off-main case by construction: it must refuse and leave no adapter.
        assert!(!bridges.attach(NativeWindow {
            id: window,
            ns_view: 1
        }));
        assert!(!bridges.is_attached(window));
        assert!(bridges.attached().is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_window_focus_events_are_inert_without_an_attached_adapter() {
        let (mut ui, renderer) = interface();
        let snapshot = snapshot(&mut ui, &renderer);
        let mut bridge = Bridge::<Message>::new();
        bridge.update(snapshot);

        let first = iced::window::Id::unique();
        bridge.window_event(first, iced::window::Event::Focused);
        assert_eq!(bridge.window, None);
    }

    /// Activation hands out the cached tree and asks for a fresh one; the ask
    /// targets no node, so the snapshot it reaches has nothing to run. The
    /// Linux test above covers the same on the platform whose adapter also
    /// deactivates, and only one test per platform may flip the active flag.
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn activation_asks_for_a_fresh_tree() {
        let (mut ui, renderer) = interface();
        let snapshot = snapshot(&mut ui, &renderer);
        let latest_tree = Arc::new(Mutex::new(Some(snapshot.update.clone())));
        let (mut refresh, mut refreshed) = iced::futures::channel::mpsc::channel(1);
        let mut activation = Activation {
            latest_tree,
            refresh: Box::new(move || {
                let _ = refresh.try_send(refresh_request());
            }),
        };

        let initial = accesskit::ActivationHandler::request_initial_tree(&mut activation)
            .expect("latest tree");
        assert_eq!(initial.nodes, snapshot.update.nodes);
        assert!(accessibility_active());

        let request = refreshed.try_recv().expect("refresh sent");
        assert!(is_refresh_request(&request));
        assert!(!snapshot.actions.contains_key(&request.target_node));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_bridge_activation_uses_latest_tree_and_one_window() {
        let (mut ui, renderer) = interface();
        let snapshot = snapshot(&mut ui, &renderer);
        let mut bridge = Bridge::<Message>::new();
        bridge.update(snapshot.clone());
        let (mut refresh, mut refreshed) = iced::futures::channel::mpsc::channel(1);
        let mut activation = Activation {
            latest_tree: Arc::clone(&bridge.latest_tree),
            refresh: Box::new(move || {
                let _ = refresh.try_send(refresh_request());
            }),
        };

        let initial = accesskit::ActivationHandler::request_initial_tree(&mut activation)
            .expect("latest tree");
        assert_eq!(initial.nodes, snapshot.update.nodes);
        assert_eq!(initial.focus, snapshot.update.focus);
        // The tree it handed out may be stale, so it also asked for a new one.
        let request = refreshed.try_recv().expect("refresh sent");
        assert!(is_refresh_request(&request));

        // Activation is also what opens the generated per-update snapshot
        // gate, and deactivation is what closes it. Asserted here — in the
        // one test that calls the activation handler — so no parallel test
        // races the process-wide flag.
        assert!(accessibility_active());
        assert!(accessibility_settings().screen_reader);
        accesskit::DeactivationHandler::deactivate_accessibility(&mut Deactivation);
        assert!(!accessibility_active());
        assert!(!accessibility_settings().screen_reader);

        let first = iced::window::Id::unique();
        let second = iced::window::Id::unique();
        bridge.window_event(first, iced::window::Event::Focused);
        bridge.window_event(second, iced::window::Event::Unfocused);
        assert_eq!(bridge.window, Some(first));

        let disabled = Bridge::<Message>::without_native_adapter();
        assert!(disabled.adapter.is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires an isolated Linux AT-SPI bus; run scripts/a11y-smoke.sh"]
    fn linux_native_atspi_exports_tree_and_routes_action() {
        use std::process::Command;
        use std::thread;
        use std::time::Duration;

        fn gdbus(args: &[&str]) -> Result<String, String> {
            let output = Command::new("gdbus")
                .args(args)
                .output()
                .map_err(|error| format!("failed to run gdbus: {error}"))?;
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).into_owned())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            }
        }

        fn quoted_values(output: &str) -> Vec<&str> {
            output.split('\'').skip(1).step_by(2).collect()
        }

        fn set_enabled(enabled: bool) -> Result<(), String> {
            gdbus(&[
                "call",
                "--session",
                "--dest",
                "org.a11y.Bus",
                "--object-path",
                "/org/a11y/bus",
                "--method",
                "org.freedesktop.DBus.Properties.Set",
                "org.a11y.Status",
                "IsEnabled",
                if enabled { "<true>" } else { "<false>" },
            ])
            .map(|_| ())
        }

        struct StatusGuard(bool);
        impl Drop for StatusGuard {
            fn drop(&mut self) {
                let _ = set_enabled(self.0);
            }
        }

        let address = std::env::var("AT_SPI_BUS_ADDRESS")
            .expect("run this gate through scripts/a11y-smoke.sh");

        let status = gdbus(&[
            "call",
            "--session",
            "--dest",
            "org.a11y.Bus",
            "--object-path",
            "/org/a11y/bus",
            "--method",
            "org.freedesktop.DBus.Properties.Get",
            "org.a11y.Status",
            "IsEnabled",
        ])
        .expect("query org.a11y.Status.IsEnabled");
        let initially_enabled = status.contains("true");
        let _guard = StatusGuard(initially_enabled);

        let label = format!("ui-lang-native-smoke-{}", std::process::id());
        let id = StableId::new(&label).node_id();
        let mut root = Node::new(Role::Window);
        root.set_label(label.clone());
        root.set_children(vec![id]);
        let mut button = Node::new(Role::Button);
        button.set_label(label.clone());
        button.add_action(Action::Click);
        let snapshot = Snapshot {
            update: TreeUpdate {
                nodes: vec![(ROOT_ID, root), (id, button)],
                tree: Some(Tree {
                    root: ROOT_ID,
                    toolkit_name: Some("Ice native smoke".into()),
                    toolkit_version: Some(env!("CARGO_PKG_VERSION").into()),
                }),
                tree_id: TreeId::ROOT,
                focus: ROOT_ID,
            },
            actions: HashMap::from([(
                id,
                ActionTarget {
                    activate: Some(Message::First),
                    node: SemanticFocus {
                        base: StableId::new("button"),
                        occurrence: 0,
                    },
                    focusable: false,
                    caret: None,
                    increment: None,
                    decrement: None,
                },
            )]),
        };
        let mut bridge = Bridge::new();
        bridge.update(snapshot);
        bridge.window_event(iced::window::Id::unique(), iced::window::Event::Focused);
        let mut receiver = bridge
            .receiver
            .lock()
            .expect("native action receiver")
            .take()
            .expect("native action receiver owner");

        thread::sleep(Duration::from_millis(250));
        if initially_enabled {
            set_enabled(false).expect("temporarily disable accessibility");
            thread::sleep(Duration::from_millis(100));
        }
        set_enabled(true).expect("enable accessibility for native smoke");
        let mut exported = None;
        let mut diagnostic = String::new();
        for _ in 0..50 {
            let Ok(applications) = gdbus(&[
                "call",
                "--address",
                &address,
                "--dest",
                "org.a11y.atspi.Registry",
                "--object-path",
                "/org/a11y/atspi/accessible/root",
                "--method",
                "org.a11y.atspi.Accessible.GetChildren",
            ]) else {
                thread::sleep(Duration::from_millis(100));
                continue;
            };
            diagnostic = format!("applications={applications}");
            for bus in quoted_values(&applications)
                .into_iter()
                .filter(|value| value.starts_with(':'))
            {
                let Ok(roots) = gdbus(&[
                    "call",
                    "--address",
                    &address,
                    "--dest",
                    bus,
                    "--object-path",
                    "/org/a11y/atspi/accessible/root",
                    "--method",
                    "org.a11y.atspi.Accessible.GetChildren",
                ]) else {
                    continue;
                };
                diagnostic.push_str(&format!(" bus={bus} roots={roots}"));
                for root_path in quoted_values(&roots)
                    .into_iter()
                    .filter(|value| value.starts_with('/'))
                {
                    let Ok(name) = gdbus(&[
                        "call",
                        "--address",
                        &address,
                        "--dest",
                        bus,
                        "--object-path",
                        root_path,
                        "--method",
                        "org.freedesktop.DBus.Properties.Get",
                        "org.a11y.atspi.Accessible",
                        "Name",
                    ]) else {
                        continue;
                    };
                    diagnostic.push_str(&format!(" path={root_path} name={name}"));
                    if !name.contains(&label) {
                        continue;
                    }
                    let Ok(children) = gdbus(&[
                        "call",
                        "--address",
                        &address,
                        "--dest",
                        bus,
                        "--object-path",
                        root_path,
                        "--method",
                        "org.a11y.atspi.Accessible.GetChildren",
                    ]) else {
                        continue;
                    };
                    let Some(path) = quoted_values(&children)
                        .into_iter()
                        .find(|value| value.starts_with('/'))
                    else {
                        continue;
                    };
                    exported = Some((bus.to_owned(), path.to_owned()));
                    break;
                }
                if exported.is_some() {
                    break;
                }
            }
            if exported.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }

        let (bus, path) = exported.unwrap_or_else(|| {
            panic!("AccessKit tree was not exported through AT-SPI; {diagnostic}")
        });
        gdbus(&[
            "call",
            "--address",
            &address,
            "--dest",
            &bus,
            "--object-path",
            &path,
            "--method",
            "org.a11y.atspi.Action.DoAction",
            "0",
        ])
        .expect("invoke exported AT-SPI action");

        // The AT's first query activated the adapter, and activation puts a
        // refresh on the channel ahead of anything the AT does next; the
        // click routed through AT-SPI is the first request after it.
        let mut routed = Vec::new();
        for _ in 0..20 {
            while let Ok(request) = receiver.try_recv() {
                routed.push(request);
            }
            let click_arrived = routed.iter().any(|request| !is_refresh_request(request));
            if click_arrived {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }
        let refresh_arrived = routed.iter().any(is_refresh_request);
        assert!(refresh_arrived, "activation did not ask for a fresh tree");
        let request = routed
            .into_iter()
            .find(|request| !is_refresh_request(request))
            .expect("native AT-SPI action was not routed to Iced");
        assert_eq!(request.action, Action::Click);
        assert_eq!(request.target_node, id);
    }
}
