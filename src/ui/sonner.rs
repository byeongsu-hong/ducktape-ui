use super::focus_control::{self, focus_control};
use super::theme::{Theme, mix};
use super::toast::{DEFAULT_DURATION, TOAST_WIDTH, ToastData, ToastDuration, ToastVariant, toast};
use iced::advanced::widget::Operation as _;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, renderer, widget};
use iced::alignment::{Horizontal, Vertical};
use iced::font::Weight;
use iced::widget::{Container, Id, MouseArea, container, mouse_area, text};
use iced::{Background, Border, Color, Element, Event, Font, Length, Rectangle, Shadow, Size};
use std::collections::VecDeque;
use std::time::Duration;

const DEFAULT_VISIBLE: usize = 3;
const DEFAULT_OFFSET: f32 = 24.0;
const DEFAULT_SWIPE_THRESHOLD: f32 = 80.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ToastId(u64);

impl ToastId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToastPlacement {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    #[default]
    BottomRight,
}

impl ToastPlacement {
    pub const fn horizontal(self) -> Horizontal {
        match self {
            Self::TopLeft | Self::BottomLeft => Horizontal::Left,
            Self::TopCenter | Self::BottomCenter => Horizontal::Center,
            Self::TopRight | Self::BottomRight => Horizontal::Right,
        }
    }

    pub const fn vertical(self) -> Vertical {
        match self {
            Self::TopLeft | Self::TopCenter | Self::TopRight => Vertical::Top,
            Self::BottomLeft | Self::BottomCenter | Self::BottomRight => Vertical::Bottom,
        }
    }

    pub const fn is_top(self) -> bool {
        matches!(self, Self::TopLeft | Self::TopCenter | Self::TopRight)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SwipeDirection {
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastInteraction {
    Hovered(bool),
    Focused(bool),
    PointerMoved(f32),
    SwipeStarted,
    SwipeEnded,
    SwipeCancelled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SonnerEvent {
    Dismiss(ToastId),
    Action(ToastId),
    FocusChanged(Option<ToastId>),
    Interaction(ToastId, ToastInteraction),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SonnerOutcome {
    #[default]
    None,
    Action(ToastId),
    Dismissed(ToastId),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PauseState {
    hover: bool,
    focus: bool,
}

impl PauseState {
    const fn any(self) -> bool {
        self.hover || self.focus
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct DragState {
    pointer_x: Option<f32>,
    origin_x: Option<f32>,
    offset: f32,
    dragging: bool,
}

impl DragState {
    fn reset(&mut self) {
        self.origin_x = None;
        self.offset = 0.0;
        self.dragging = false;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToastEntry {
    id: ToastId,
    data: ToastData,
    remaining: Option<Duration>,
    deadline: Option<Duration>,
    visible: bool,
    pause: PauseState,
    drag: DragState,
}

impl ToastEntry {
    pub const fn id(&self) -> ToastId {
        self.id
    }

    pub const fn data(&self) -> &ToastData {
        &self.data
    }

    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    pub const fn is_paused(&self) -> bool {
        self.pause.any() || self.drag.dragging
    }

    pub const fn deadline(&self) -> Option<Duration> {
        self.deadline
    }

    pub const fn remaining(&self) -> Option<Duration> {
        self.remaining
    }

    pub const fn swipe_offset(&self) -> f32 {
        self.drag.offset
    }

    fn freeze(&mut self, now: Duration) {
        if self.visible
            && let Some(deadline) = self.deadline
        {
            self.remaining = Some(deadline.saturating_sub(now));
            self.deadline = None;
        }
    }

    fn resume(&mut self, now: Duration) {
        if self.visible {
            self.deadline = self
                .remaining
                .map(|remaining| now.saturating_add(remaining));
        }
    }

    fn activate(&mut self, now: Duration) {
        self.visible = true;
        if !self.is_paused() {
            self.resume(now);
        }
    }

    fn deactivate(&mut self, now: Duration) {
        self.freeze(now);
        self.visible = false;
        self.drag.reset();
    }
}

/// Caller-owned Sonner queue. Every time value is elapsed monotonic time from
/// the same origin; no timer or runtime is hidden inside the component.
#[derive(Debug, Clone)]
pub struct SonnerState {
    entries: VecDeque<ToastEntry>,
    next_id: u64,
    max_visible: usize,
    placement: ToastPlacement,
    default_duration: Duration,
    offset: f32,
    expanded: bool,
    reduced_motion: bool,
    swipe_direction: SwipeDirection,
    swipe_threshold: f32,
}

impl Default for SonnerState {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            next_id: 1,
            max_visible: DEFAULT_VISIBLE,
            placement: ToastPlacement::BottomRight,
            default_duration: DEFAULT_DURATION,
            offset: DEFAULT_OFFSET,
            expanded: false,
            reduced_motion: false,
            swipe_direction: SwipeDirection::Right,
            swipe_threshold: DEFAULT_SWIPE_THRESHOLD,
        }
    }
}

impl SonnerState {
    pub fn new(max_visible: usize, placement: ToastPlacement) -> Self {
        Self {
            max_visible: max_visible.max(1),
            placement,
            ..Self::default()
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub const fn max_visible(&self) -> usize {
        self.max_visible
    }

    pub const fn placement(&self) -> ToastPlacement {
        self.placement
    }

    pub const fn default_duration(&self) -> Duration {
        self.default_duration
    }

    pub const fn offset(&self) -> f32 {
        self.offset
    }

    pub const fn reduced_motion(&self) -> bool {
        self.reduced_motion
    }

    /// Reduced motion forces the readable, non-overlapping expanded layout.
    /// All state changes remain immediate in either mode; no animation clock is
    /// built into this source-owned component.
    pub const fn is_expanded(&self) -> bool {
        self.expanded || self.reduced_motion
    }

    pub const fn swipe_direction(&self) -> SwipeDirection {
        self.swipe_direction
    }

    pub const fn swipe_threshold(&self) -> f32 {
        self.swipe_threshold
    }

    pub fn visible(&self) -> impl DoubleEndedIterator<Item = &ToastEntry> {
        self.entries.iter().take(self.max_visible)
    }

    pub fn queued(&self) -> impl Iterator<Item = &ToastEntry> {
        self.entries.iter().skip(self.max_visible)
    }

    pub fn get(&self, id: ToastId) -> Option<&ToastEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn set_max_visible(&mut self, max_visible: usize, now: Duration) {
        self.max_visible = max_visible.max(1);
        self.reconcile(now);
    }

    pub const fn set_placement(&mut self, placement: ToastPlacement) {
        self.placement = placement;
    }

    pub const fn set_default_duration(&mut self, duration: Duration) {
        self.default_duration = duration;
    }

    pub fn set_offset(&mut self, offset: f32) {
        self.offset = offset.max(0.0);
    }

    pub const fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    pub const fn set_reduced_motion(&mut self, reduced_motion: bool) {
        self.reduced_motion = reduced_motion;
    }

    pub const fn set_swipe_direction(&mut self, direction: SwipeDirection) {
        self.swipe_direction = direction;
    }

    pub fn set_swipe_threshold(&mut self, threshold: f32) {
        self.swipe_threshold = threshold.max(1.0);
    }

    pub fn push(&mut self, data: ToastData, now: Duration) -> ToastId {
        let id = self.allocate_id();
        let remaining = self.resolve_duration(&data);
        self.entries.push_back(ToastEntry {
            id,
            data,
            remaining,
            deadline: None,
            visible: false,
            pause: PauseState::default(),
            drag: DragState::default(),
        });
        self.reconcile(now);
        id
    }

    pub fn show(&mut self, title: impl Into<String>, now: Duration) -> ToastId {
        self.push(ToastData::new(title), now)
    }

    pub fn success(&mut self, title: impl Into<String>, now: Duration) -> ToastId {
        self.push(ToastData::new(title).variant(ToastVariant::Success), now)
    }

    pub fn info(&mut self, title: impl Into<String>, now: Duration) -> ToastId {
        self.push(ToastData::new(title).variant(ToastVariant::Info), now)
    }

    pub fn warning(&mut self, title: impl Into<String>, now: Duration) -> ToastId {
        self.push(ToastData::new(title).variant(ToastVariant::Warning), now)
    }

    pub fn error(&mut self, title: impl Into<String>, now: Duration) -> ToastId {
        self.push(
            ToastData::new(title).variant(ToastVariant::Destructive),
            now,
        )
    }

    pub fn loading(&mut self, title: impl Into<String>, now: Duration) -> ToastId {
        self.push(
            ToastData::new(title)
                .variant(ToastVariant::Loading)
                .persistent(),
            now,
        )
    }

    /// Reuses a stable ID when a loading or promise-like operation changes.
    pub fn replace(&mut self, id: ToastId, data: ToastData, now: Duration) -> bool {
        let duration = self.resolve_duration(&data);
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) else {
            return false;
        };

        entry.data = data;
        entry.remaining = duration;
        entry.deadline = None;
        entry.drag.reset();
        if entry.visible && !entry.is_paused() {
            entry.resume(now);
        }
        true
    }

    pub fn dismiss(&mut self, id: ToastId, now: Duration) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
            return false;
        };
        self.entries.remove(index);
        self.reconcile(now);
        true
    }

    pub fn clear(&mut self) -> Vec<ToastId> {
        self.entries.drain(..).map(|entry| entry.id).collect()
    }

    /// Removes expired visible toasts and starts the next queued deadlines.
    pub fn tick(&mut self, now: Duration) -> Vec<ToastId> {
        let mut expired = Vec::new();
        self.entries.retain(|entry| {
            let is_expired = entry.visible
                && !entry.is_paused()
                && entry.deadline.is_some_and(|deadline| deadline <= now);
            if is_expired {
                expired.push(entry.id);
            }
            !is_expired
        });

        if !expired.is_empty() {
            self.reconcile(now);
        }
        expired
    }

    pub fn update(&mut self, event: SonnerEvent, now: Duration) -> SonnerOutcome {
        match event {
            SonnerEvent::Dismiss(id) => {
                if self.dismiss(id, now) {
                    SonnerOutcome::Dismissed(id)
                } else {
                    SonnerOutcome::default()
                }
            }
            SonnerEvent::Action(id) => {
                if self.get(id).is_some() {
                    SonnerOutcome::Action(id)
                } else {
                    SonnerOutcome::default()
                }
            }
            SonnerEvent::FocusChanged(focused) => {
                for entry in &mut self.entries {
                    let was_paused = entry.is_paused();
                    entry.pause.focus = focused == Some(entry.id);
                    let is_paused = entry.is_paused();

                    if !was_paused && is_paused {
                        entry.freeze(now);
                    } else if was_paused && !is_paused {
                        entry.resume(now);
                    }
                }

                SonnerOutcome::None
            }
            SonnerEvent::Interaction(id, interaction) => self.interact(id, interaction, now),
        }
    }

    /// Immediately queries which rendered toast owns focus. Focus changes are
    /// otherwise reported on the next widget event; chain this after an iced
    /// focus operation when the timer must synchronize in the same task batch.
    pub fn focus_task(&self) -> iced::Task<SonnerEvent> {
        iced::Task::batch(self.focus_targets().into_iter().map(|(toast, control)| {
            iced::widget::operation::is_focused(control).map(move |focused| (toast, focused))
        }))
        .collect()
        .map(|focused| {
            SonnerEvent::FocusChanged(
                focused
                    .into_iter()
                    .find_map(|(toast, focused)| focused.then_some(toast)),
            )
        })
    }

    fn focus_targets(&self) -> Vec<(ToastId, Id)> {
        let mut targets = Vec::new();

        for entry in self.visible() {
            if entry.data.action_label().is_some() {
                targets.push((entry.id, control_id(entry.id, "action")));
            }
            targets.push((entry.id, control_id(entry.id, "dismiss")));
        }

        targets
    }

    fn focused_toast(&self) -> Option<ToastId> {
        self.entries
            .iter()
            .find(|entry| entry.pause.focus)
            .map(|entry| entry.id)
    }

    fn interact(
        &mut self,
        id: ToastId,
        interaction: ToastInteraction,
        now: Duration,
    ) -> SonnerOutcome {
        let direction = self.swipe_direction;
        let threshold = self.swipe_threshold;
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) else {
            return SonnerOutcome::None;
        };

        let was_paused = entry.is_paused();
        let mut dismiss = false;
        match interaction {
            ToastInteraction::Hovered(hovered) => {
                entry.pause.hover = hovered;
                if !hovered {
                    entry.drag.reset();
                }
            }
            ToastInteraction::Focused(focused) => entry.pause.focus = focused,
            ToastInteraction::PointerMoved(x) => {
                entry.drag.pointer_x = Some(x);
                if entry.drag.dragging {
                    let origin = *entry.drag.origin_x.get_or_insert(x);
                    entry.drag.offset = x - origin;
                }
            }
            ToastInteraction::SwipeStarted => {
                entry.drag.dragging = true;
                entry.drag.origin_x = entry.drag.pointer_x;
                entry.drag.offset = 0.0;
            }
            ToastInteraction::SwipeEnded => {
                let distance = match direction {
                    SwipeDirection::Left => -entry.drag.offset,
                    SwipeDirection::Right => entry.drag.offset,
                };
                dismiss = entry.drag.dragging && distance >= threshold;
                entry.drag.reset();
            }
            ToastInteraction::SwipeCancelled => entry.drag.reset(),
        }

        let is_paused = entry.is_paused();
        if !was_paused && is_paused {
            entry.freeze(now);
        } else if was_paused && !is_paused {
            entry.resume(now);
        }

        if dismiss {
            self.dismiss(id, now);
            SonnerOutcome::Dismissed(id)
        } else {
            SonnerOutcome::None
        }
    }

    fn resolve_duration(&self, data: &ToastData) -> Option<Duration> {
        match data.toast_duration() {
            ToastDuration::Default => Some(self.default_duration),
            ToastDuration::Auto(duration) => Some(duration),
            ToastDuration::Persistent => None,
        }
    }

    fn reconcile(&mut self, now: Duration) {
        for (index, entry) in self.entries.iter_mut().enumerate() {
            let should_be_visible = index < self.max_visible;
            match (entry.visible, should_be_visible) {
                (false, true) => entry.activate(now),
                (true, false) => entry.deactivate(now),
                _ => {}
            }
        }
    }

    fn allocate_id(&mut self) -> ToastId {
        loop {
            let id = ToastId(self.next_id);
            self.next_id = self.next_id.checked_add(1).unwrap_or(1);
            if !self.entries.iter().any(|entry| entry.id == id) {
                return id;
            }
        }
    }
}

/// Renders the visible Sonner stack over a fill-sized viewport.
///
/// Hover and descendant focus pause deadlines automatically. Use
/// [`SonnerState::focus_task`] only to synchronize immediately after an iced
/// focus operation instead of waiting for the next widget event. The stock
/// mouse area exposes horizontal mouse dragging; it does not claim full
/// touch-swipe support.
pub fn sonner<'a, Message, F>(
    state: &'a SonnerState,
    on_event: F,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: Clone + 'a,
    F: Fn(SonnerEvent) -> Message + Clone + 'a,
{
    let mut stack = iced::widget::Column::new()
        .spacing(if state.is_expanded() { 8 } else { 4 })
        .width(TOAST_WIDTH);
    let mut visible = state.visible().collect::<Vec<_>>();
    if state.placement.is_top() {
        visible.reverse();
    }

    for entry in visible {
        stack = stack.push(render_entry(entry, on_event.clone(), theme));
    }

    let focus_reporter = FocusReporter {
        content: stack.into(),
        targets: state.focus_targets(),
        previous: state.focused_toast(),
        on_change: Box::new(move |focused| on_event(SonnerEvent::FocusChanged(focused))),
    };

    container(Element::new(focus_reporter))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(state.offset)
        .align_x(state.placement.horizontal())
        .align_y(state.placement.vertical())
}

fn render_entry<'a, Message, F>(
    entry: &'a ToastEntry,
    on_event: F,
    theme: &Theme,
) -> MouseArea<'a, Message>
where
    Message: Clone + 'a,
    F: Fn(SonnerEvent) -> Message + Clone + 'a,
{
    let id = entry.id;
    let title = text(entry.data.title())
        .size(theme.typography.base)
        .line_height(1.3)
        .font(Font {
            weight: Weight::Medium,
            ..Font::DEFAULT
        });
    let mut surface = toast(title, theme).variant(entry.data.toast_variant());

    if let Some(description) = entry.data.description_text() {
        surface = surface.description(
            text(description)
                .size(theme.typography.sm)
                .line_height(1.4)
                .color(secondary_text_color(theme, entry.data.toast_variant())),
        );
    }
    if let Some(label) = entry.data.action_label() {
        surface = surface.action(control(
            id,
            "action",
            label,
            on_event.clone()(SonnerEvent::Action(id)),
            true,
            theme,
        ));
    }
    surface = surface.dismiss(control(
        id,
        "dismiss",
        "Dismiss",
        on_event.clone()(SonnerEvent::Dismiss(id)),
        false,
        theme,
    ));

    let hover_on = on_event.clone();
    let hover_off = on_event.clone();
    let move_event = on_event.clone();
    let swipe_start = on_event.clone();
    let swipe_end = on_event;
    mouse_area(surface)
        .on_enter(hover_on(SonnerEvent::Interaction(
            id,
            ToastInteraction::Hovered(true),
        )))
        .on_exit(hover_off(SonnerEvent::Interaction(
            id,
            ToastInteraction::Hovered(false),
        )))
        .on_move(move |point| {
            move_event(SonnerEvent::Interaction(
                id,
                ToastInteraction::PointerMoved(point.x),
            ))
        })
        .on_press(swipe_start(SonnerEvent::Interaction(
            id,
            ToastInteraction::SwipeStarted,
        )))
        .on_release(swipe_end(SonnerEvent::Interaction(
            id,
            ToastInteraction::SwipeEnded,
        )))
}

fn control<'a, Message>(
    toast_id: ToastId,
    kind: &str,
    label: &'a str,
    message: Message,
    outlined: bool,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let content = container(text(label).size(theme.typography.xs).line_height(1.0))
        .height(28)
        .padding([6, 10])
        .align_y(Vertical::Center);
    let style_theme = *theme;

    focus_control(control_id(toast_id, kind), content, message, theme)
        .style(move |_iced_theme, status| control_style(&style_theme, outlined, status))
        .into()
}

fn control_id(toast_id: ToastId, kind: &str) -> Id {
    Id::from(format!("ducktape-sonner-{kind}-{}", toast_id.get()))
}

struct FocusReporter<'a, Message> {
    content: Element<'a, Message>,
    targets: Vec<(ToastId, Id)>,
    previous: Option<ToastId>,
    on_change: Box<dyn Fn(Option<ToastId>) -> Message + 'a>,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for FocusReporter<'_, Message> {
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
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
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
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

        let mut query = widget::operation::focusable::find_focused();
        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            &mut widget::operation::black_box::<Id, ()>(&mut query),
        );
        let focused = match query.finish() {
            widget::operation::Outcome::Some(id) => self
                .targets
                .iter()
                .find_map(|(toast, target)| (target == &id).then_some(*toast)),
            _ => None,
        };
        if focused != self.previous {
            shell.publish((self.on_change)(focused));
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
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
}

fn control_style(
    theme: &Theme,
    outlined: bool,
    status: focus_control::Status,
) -> focus_control::Style {
    let hovered = matches!(
        status,
        focus_control::Status::Hovered | focus_control::Status::Pressed
    );
    let pressed = status == focus_control::Status::Pressed;
    let background = hovered.then_some(Background::Color(if pressed {
        mix(theme.palette.accent, theme.palette.foreground, 0.08)
    } else {
        theme.palette.accent
    }));

    focus_control::Style {
        background,
        text_color: Some(if hovered {
            theme.palette.accent_foreground
        } else {
            theme.palette.foreground
        }),
        border: Border {
            color: if outlined {
                theme.palette.input
            } else {
                Color::TRANSPARENT
            },
            width: if outlined { 1.0 } else { 0.0 },
            radius: theme.radius.sm.into(),
        },
        shadow: Shadow::default(),
        focus_ring: Border {
            color: theme.palette.ring,
            width: 2.0,
            radius: (theme.radius.sm + 2.0).into(),
        },
        focus_offset: 1.0,
    }
}

fn secondary_text_color(theme: &Theme, variant: ToastVariant) -> Color {
    let background = match super::toast::style(theme, variant).background {
        Some(Background::Color(color)) => color,
        _ => theme.palette.background,
    };
    mix(theme.palette.foreground, background, 0.24)
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    fn seconds(value: u64) -> Duration {
        Duration::from_secs(value)
    }

    #[test]
    fn queued_toasts_get_stable_ids_and_start_only_when_visible() {
        let mut state = SonnerState::new(1, ToastPlacement::BottomRight);
        state.set_default_duration(seconds(5));
        let first = state.show("First", seconds(10));
        let second = state.show("Second", seconds(10));

        assert_ne!(first, second);
        assert_eq!(
            state.visible().map(ToastEntry::id).collect::<Vec<_>>(),
            [first]
        );
        assert_eq!(
            state.queued().map(ToastEntry::id).collect::<Vec<_>>(),
            [second]
        );
        assert_eq!(state.get(first).unwrap().deadline(), Some(seconds(15)));
        assert_eq!(state.get(second).unwrap().deadline(), None);

        assert_eq!(state.tick(seconds(15)), [first]);
        assert_eq!(state.get(second).unwrap().deadline(), Some(seconds(20)));
        assert_eq!(state.tick(seconds(19)), []);
    }

    #[test]
    fn hover_and_focus_pause_independently_without_losing_remaining_time() {
        let mut state = SonnerState::new(3, ToastPlacement::TopRight);
        let id = state.push(ToastData::new("Saved").duration(seconds(10)), seconds(2));

        state.update(
            SonnerEvent::Interaction(id, ToastInteraction::Hovered(true)),
            seconds(5),
        );
        assert_eq!(state.get(id).unwrap().remaining(), Some(seconds(7)));
        assert_eq!(state.get(id).unwrap().deadline(), None);

        state.update(
            SonnerEvent::Interaction(id, ToastInteraction::Focused(true)),
            seconds(6),
        );
        state.update(
            SonnerEvent::Interaction(id, ToastInteraction::Hovered(false)),
            seconds(20),
        );
        assert!(state.get(id).unwrap().is_paused());
        assert_eq!(state.get(id).unwrap().deadline(), None);

        state.update(
            SonnerEvent::Interaction(id, ToastInteraction::Focused(false)),
            seconds(21),
        );
        assert_eq!(state.get(id).unwrap().deadline(), Some(seconds(28)));
        assert_eq!(state.tick(seconds(27)), []);
        assert_eq!(state.tick(seconds(28)), [id]);
    }

    #[test]
    fn focus_snapshot_moves_pause_between_toasts_and_clears_it() {
        let mut state = SonnerState::default();
        let first = state.push(
            ToastData::new("First").duration(seconds(10)),
            Duration::ZERO,
        );
        let second = state.push(
            ToastData::new("Second").duration(seconds(10)),
            Duration::ZERO,
        );

        state.update(SonnerEvent::FocusChanged(Some(first)), seconds(2));
        assert_eq!(state.get(first).unwrap().remaining(), Some(seconds(8)));
        assert_eq!(state.get(first).unwrap().deadline(), None);

        state.update(SonnerEvent::FocusChanged(Some(second)), seconds(4));
        assert_eq!(state.get(first).unwrap().deadline(), Some(seconds(12)));
        assert_eq!(state.get(second).unwrap().remaining(), Some(seconds(6)));
        assert_eq!(state.get(second).unwrap().deadline(), None);

        state.update(SonnerEvent::FocusChanged(None), seconds(5));
        assert_eq!(state.get(second).unwrap().deadline(), Some(seconds(11)));
    }

    #[test]
    fn rendered_control_focus_is_reported_on_the_next_widget_event() {
        use iced::advanced::renderer::Headless as _;
        use iced::advanced::{Layout, clipboard, layout, widget};
        use iced::{Pixels, Point, Rectangle, Size};

        let mut state = SonnerState::default();
        let id = state.push(ToastData::new("Saved").action("Undo"), Duration::ZERO);
        let targets = state.focus_targets();
        assert_eq!(targets.len(), 2);
        assert_eq!(state.focus_task().units(), 2);

        let mut view: Element<'_, SonnerEvent> = sonner(&state, |event| event, &LIGHT).into();
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(640.0, 480.0));
        let mut tree = widget::Tree::new(view.as_widget());
        let node = view.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let layout = Layout::new(&node);
        let target = control_id(id, "action");
        let mut focus = widget::operation::focusable::focus::<()>(target);
        view.as_widget_mut()
            .operate(&mut tree, layout, &renderer, &mut focus);
        let mut clipboard = clipboard::Null;
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        view.as_widget_mut().update(
            &mut tree,
            &Event::Window(iced::window::Event::RedrawRequested(
                iced::time::Instant::now(),
            )),
            layout,
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );

        assert_eq!(messages, [SonnerEvent::FocusChanged(Some(id))]);
    }

    #[test]
    fn demoting_a_paused_toast_preserves_its_remaining_duration() {
        let mut state = SonnerState::new(2, ToastPlacement::BottomRight);
        let first = state.push(
            ToastData::new("First").duration(seconds(20)),
            Duration::ZERO,
        );
        let second = state.push(
            ToastData::new("Second").duration(seconds(10)),
            Duration::ZERO,
        );
        state.update(
            SonnerEvent::Interaction(second, ToastInteraction::Hovered(true)),
            seconds(3),
        );

        state.set_max_visible(1, seconds(4));
        assert_eq!(state.get(second).unwrap().remaining(), Some(seconds(7)));
        assert!(!state.get(second).unwrap().is_visible());

        state.dismiss(first, seconds(20));
        assert!(state.get(second).unwrap().is_visible());
        assert_eq!(state.get(second).unwrap().deadline(), None);
        state.update(
            SonnerEvent::Interaction(second, ToastInteraction::Hovered(false)),
            seconds(21),
        );
        assert_eq!(state.get(second).unwrap().deadline(), Some(seconds(28)));
    }

    #[test]
    fn swipe_reducer_honors_direction_threshold_and_cancels_cleanly() {
        let mut state = SonnerState::default();
        state.set_swipe_threshold(50.0);
        let keep = state.show("Keep", Duration::ZERO);
        state.update(
            SonnerEvent::Interaction(keep, ToastInteraction::PointerMoved(100.0)),
            Duration::ZERO,
        );
        state.update(
            SonnerEvent::Interaction(keep, ToastInteraction::SwipeStarted),
            Duration::ZERO,
        );
        state.update(
            SonnerEvent::Interaction(keep, ToastInteraction::PointerMoved(140.0)),
            Duration::ZERO,
        );
        assert_eq!(state.get(keep).unwrap().swipe_offset(), 40.0);
        assert_eq!(
            state.update(
                SonnerEvent::Interaction(keep, ToastInteraction::SwipeEnded),
                Duration::ZERO,
            ),
            SonnerOutcome::None
        );
        assert_eq!(state.get(keep).unwrap().swipe_offset(), 0.0);

        let dismiss = state.show("Dismiss", Duration::ZERO);
        state.update(
            SonnerEvent::Interaction(dismiss, ToastInteraction::PointerMoved(10.0)),
            Duration::ZERO,
        );
        state.update(
            SonnerEvent::Interaction(dismiss, ToastInteraction::SwipeStarted),
            Duration::ZERO,
        );
        state.update(
            SonnerEvent::Interaction(dismiss, ToastInteraction::PointerMoved(80.0)),
            Duration::ZERO,
        );
        assert_eq!(
            state.update(
                SonnerEvent::Interaction(dismiss, ToastInteraction::SwipeEnded),
                Duration::ZERO,
            ),
            SonnerOutcome::Dismissed(dismiss)
        );
        assert!(state.get(dismiss).is_none());
    }

    #[test]
    fn replacement_keeps_id_and_reduced_motion_forces_static_expansion() {
        let mut state = SonnerState::default();
        let id = state.loading("Uploading", seconds(1));
        assert_eq!(state.get(id).unwrap().deadline(), None);

        assert!(state.replace(
            id,
            ToastData::new("Uploaded").variant(ToastVariant::Success),
            seconds(8),
        ));
        assert_eq!(state.get(id).unwrap().id(), id);
        assert_eq!(state.get(id).unwrap().deadline(), Some(seconds(13)));

        assert!(!state.is_expanded());
        state.set_reduced_motion(true);
        assert!(state.is_expanded());
        assert_eq!(state.get(id).unwrap().deadline(), Some(seconds(13)));
    }

    #[test]
    fn top_and_bottom_placements_use_expected_render_order() {
        let mut top = SonnerState::new(3, ToastPlacement::TopLeft);
        let first = top.show("First", Duration::ZERO);
        let second = top.show("Second", Duration::ZERO);
        let mut rendered = top.visible().map(ToastEntry::id).collect::<Vec<_>>();
        if top.placement().is_top() {
            rendered.reverse();
        }
        assert_eq!(rendered, [second, first]);

        top.set_placement(ToastPlacement::BottomLeft);
        let rendered = top.visible().map(ToastEntry::id).collect::<Vec<_>>();
        assert_eq!(rendered, [first, second]);
        assert_eq!(top.placement().horizontal(), Horizontal::Left);
        assert_eq!(top.placement().vertical(), Vertical::Bottom);
    }

    #[test]
    fn renderer_excludes_queue_and_description_color_keeps_contrast() {
        let mut state = SonnerState::new(1, ToastPlacement::BottomCenter);
        state.push(
            ToastData::new("Saved")
                .description("The file is on disk.")
                .action("Undo")
                .variant(ToastVariant::Success),
            Duration::ZERO,
        );
        state.error("Queued", Duration::ZERO);
        let view: Element<'_, SonnerEvent> = sonner(&state, |event| event, &LIGHT).into();
        let root = view.as_widget().children();

        assert_eq!(root[0].children.len(), 1);

        for theme in [LIGHT, DARK] {
            for variant in [
                ToastVariant::Default,
                ToastVariant::Success,
                ToastVariant::Info,
                ToastVariant::Warning,
                ToastVariant::Destructive,
                ToastVariant::Loading,
            ] {
                let background = match super::super::toast::style(&theme, variant).background {
                    Some(Background::Color(color)) => color,
                    _ => panic!("toast needs a surface"),
                };
                assert!(
                    contrast(secondary_text_color(&theme, variant), background) >= 4.5,
                    "{} {variant:?}",
                    theme.name
                );
            }
        }
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let (lighter, darker) = if luminance(a) > luminance(b) {
            (luminance(a), luminance(b))
        } else {
            (luminance(b), luminance(a))
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn luminance(color: Color) -> f32 {
        fn channel(value: f32) -> f32 {
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }

        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    }
}
