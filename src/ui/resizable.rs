//! Controlled, accessible resizable panel groups.
//!
//! Sizes and minimums are normalized shares (`0.25` is 25%). The caller owns
//! the sizes and applies the vector published by [`resizable`].

use std::rc::Rc;

use super::focus_control::{self, FocusControl};
use super::theme::{Theme as UiTheme, alpha};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::keyboard::{self, key::Named};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Size, Task, Vector, touch,
};

const DIVIDER_THICKNESS: f32 = 1.0;
const FOCUSED_DIVIDER_THICKNESS: f32 = 2.0;
const DEFAULT_POINTER_HIT_SIZE: f32 = 12.0;
const DEFAULT_TOUCH_HIT_SIZE: f32 = 32.0;
const DEFAULT_KEYBOARD_STEP: f32 = 0.05;
const GRIP_SHORT: f32 = 12.0;
const GRIP_LONG: f32 = 16.0;
const GRIP_DOT: f32 = 1.5;
const GRIP_DOT_GAP: f32 = 3.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ResizableOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeCommand {
    Decrease,
    Increase,
    PreviousMinimum,
    NextMinimum,
}

/// Per-separator behavior and decoration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResizableHandle {
    pub disabled: bool,
    pub with_grip: bool,
}

impl ResizableHandle {
    pub const fn new() -> Self {
        Self {
            disabled: false,
            with_grip: false,
        }
    }

    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub const fn with_grip(mut self, with_grip: bool) -> Self {
        self.with_grip = with_grip;
        self
    }
}

/// Sanitized panel sizes and constraints shared by every input path.
#[derive(Debug, Clone, PartialEq)]
pub struct ResizableLayout {
    sizes: Vec<f32>,
    minimums: Vec<f32>,
}

impl ResizableLayout {
    pub fn new(panel_count: usize, sizes: &[f32], minimums: &[f32]) -> Self {
        let minimums = normalize_minimums(panel_count, minimums);
        let sizes = normalize_sizes(panel_count, sizes, &minimums);

        Self { sizes, minimums }
    }

    pub fn sizes(&self) -> &[f32] {
        &self.sizes
    }

    pub fn minimums(&self) -> &[f32] {
        &self.minimums
    }

    /// Moves a separator by a normalized delta while preserving its pair total.
    pub fn resize(&self, handle: usize, delta: f32) -> Vec<f32> {
        if handle >= self.sizes.len().saturating_sub(1) || !delta.is_finite() {
            return self.sizes.clone();
        }

        let next_panel = handle + 1;
        let lower = self.minimums[handle] - self.sizes[handle];
        let upper = self.sizes[next_panel] - self.minimums[next_panel];
        let delta = delta.clamp(lower, upper);
        let mut sizes = self.sizes.clone();
        sizes[handle] += delta;
        sizes[next_panel] -= delta;
        sizes
    }

    pub fn reduce(&self, handle: usize, command: ResizeCommand, step: f32) -> Vec<f32> {
        if handle >= self.sizes.len().saturating_sub(1) {
            return self.sizes.clone();
        }

        let step = if step.is_finite() && step > 0.0 {
            step.min(1.0)
        } else {
            DEFAULT_KEYBOARD_STEP
        };
        let delta = match command {
            ResizeCommand::Decrease => -step,
            ResizeCommand::Increase => step,
            ResizeCommand::PreviousMinimum => self.minimums[handle] - self.sizes[handle],
            ResizeCommand::NextMinimum => self.sizes[handle + 1] - self.minimums[handle + 1],
        };

        self.resize(handle, delta)
    }
}

fn normalize_minimums(panel_count: usize, minimums: &[f32]) -> Vec<f32> {
    let mut minimums = (0..panel_count)
        .map(|index| {
            minimums
                .get(index)
                .copied()
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(0.0)
        })
        .collect::<Vec<_>>();
    let total = minimums.iter().sum::<f32>();

    if total > 1.0 {
        minimums.iter_mut().for_each(|value| *value /= total);
    }

    minimums
}

fn normalize_sizes(panel_count: usize, sizes: &[f32], minimums: &[f32]) -> Vec<f32> {
    if panel_count == 0 {
        return Vec::new();
    }

    let mut desired = (0..panel_count)
        .map(|index| {
            sizes
                .get(index)
                .copied()
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(0.0)
        })
        .collect::<Vec<_>>();
    let desired_total = desired.iter().sum::<f32>();

    if desired_total > 0.0 {
        desired.iter_mut().for_each(|value| *value /= desired_total);
    } else {
        desired.fill(1.0 / panel_count as f32);
    }

    // Iteratively pin undersized panels, then preserve the relative weights of
    // the remaining panels. This is the smallest projection that keeps every
    // minimum and the exact group total.
    let mut result = vec![0.0; panel_count];
    let mut open = vec![true; panel_count];
    let mut remaining = 1.0;

    loop {
        let weight = desired
            .iter()
            .zip(&open)
            .filter_map(|(value, open)| open.then_some(*value))
            .sum::<f32>();
        let mut pinned = false;

        for index in 0..panel_count {
            if !open[index] {
                continue;
            }

            let candidate = if weight > 0.0 {
                remaining * desired[index] / weight
            } else {
                remaining / open.iter().filter(|open| **open).count() as f32
            };

            if candidate + f32::EPSILON < minimums[index] {
                result[index] = minimums[index];
                remaining = (remaining - minimums[index]).max(0.0);
                open[index] = false;
                pinned = true;
            }
        }

        if !pinned {
            let weight = desired
                .iter()
                .zip(&open)
                .filter_map(|(value, open)| open.then_some(*value))
                .sum::<f32>();
            let open_count = open.iter().filter(|open| **open).count();

            for index in 0..panel_count {
                if open[index] {
                    result[index] = if weight > 0.0 {
                        remaining * desired[index] / weight
                    } else {
                        remaining / open_count as f32
                    };
                }
            }
            break;
        }
    }

    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleStatus {
    Active,
    Hovered,
    Focused,
    Dragging,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandleStyle {
    pub divider: Color,
    pub grip_background: Color,
    pub grip_border: Color,
    pub grip_dot: Color,
}

pub fn handle_style(theme: &UiTheme, status: HandleStatus) -> HandleStyle {
    let emphasis = match status {
        HandleStatus::Active => theme.palette.input,
        HandleStatus::Hovered => theme.palette.muted_foreground,
        HandleStatus::Focused | HandleStatus::Dragging => theme.palette.ring,
        HandleStatus::Disabled => alpha(theme.palette.border, 0.45),
    };
    let disabled = status == HandleStatus::Disabled;

    HandleStyle {
        divider: emphasis,
        grip_background: if disabled {
            alpha(theme.palette.card, 0.6)
        } else {
            theme.palette.card
        },
        grip_border: emphasis,
        grip_dot: if disabled {
            alpha(theme.palette.muted_foreground, 0.45)
        } else {
            theme.palette.muted_foreground
        },
    }
}

pub fn keyboard_command(
    key: &keyboard::Key,
    orientation: ResizableOrientation,
) -> Option<ResizeCommand> {
    match (orientation, key) {
        (_, keyboard::Key::Named(Named::Home)) => Some(ResizeCommand::PreviousMinimum),
        (_, keyboard::Key::Named(Named::End)) => Some(ResizeCommand::NextMinimum),
        (ResizableOrientation::Horizontal, keyboard::Key::Named(Named::ArrowLeft))
        | (ResizableOrientation::Vertical, keyboard::Key::Named(Named::ArrowUp)) => {
            Some(ResizeCommand::Decrease)
        }
        (ResizableOrientation::Horizontal, keyboard::Key::Named(Named::ArrowRight))
        | (ResizableOrientation::Vertical, keyboard::Key::Named(Named::ArrowDown)) => {
            Some(ResizeCommand::Increase)
        }
        _ => None,
    }
}

pub fn resizable_handle_id(group_id: &str, handle: usize) -> widget::Id {
    widget::Id::from(format!("ducktape-resizable:{group_id}:{handle}"))
}

pub fn focus_resizable_handle<Message>(group_id: &str, handle: usize) -> Task<Message> {
    iced::widget::operation::focus(resizable_handle_id(group_id, handle))
}

/// A controlled flat panel group with one separator between each panel.
pub struct Resizable<'a, Message>
where
    Message: Clone + 'a,
{
    id: String,
    panels: Vec<Element<'a, Message>>,
    sizes: Vec<f32>,
    minimums: Vec<f32>,
    on_resize: Rc<dyn Fn(Vec<f32>) -> Message + 'a>,
    orientation: ResizableOrientation,
    handles: Vec<ResizableHandle>,
    disabled: bool,
    keyboard_step: f32,
    pointer_hit_size: f32,
    touch_hit_size: f32,
    width: Length,
    height: Length,
    theme: UiTheme,
}

pub fn resizable<'a, Message>(
    id: impl Into<String>,
    panels: impl IntoIterator<Item = Element<'a, Message>>,
    sizes: impl Into<Vec<f32>>,
    minimums: impl Into<Vec<f32>>,
    on_resize: impl Fn(Vec<f32>) -> Message + 'a,
    theme: &UiTheme,
) -> Resizable<'a, Message>
where
    Message: Clone + 'a,
{
    let panels = panels.into_iter().collect::<Vec<_>>();
    let handles = vec![ResizableHandle::new(); panels.len().saturating_sub(1)];

    Resizable {
        id: id.into(),
        panels,
        sizes: sizes.into(),
        minimums: minimums.into(),
        on_resize: Rc::new(on_resize),
        orientation: ResizableOrientation::Horizontal,
        handles,
        disabled: false,
        keyboard_step: DEFAULT_KEYBOARD_STEP,
        pointer_hit_size: DEFAULT_POINTER_HIT_SIZE,
        touch_hit_size: DEFAULT_TOUCH_HIT_SIZE,
        width: Length::Fill,
        height: Length::Fill,
        theme: *theme,
    }
}

impl<'a, Message> Resizable<'a, Message>
where
    Message: Clone + 'a,
{
    #[must_use]
    pub fn orientation(mut self, orientation: ResizableOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    #[must_use]
    pub fn handle(mut self, index: usize, handle: ResizableHandle) -> Self {
        if let Some(slot) = self.handles.get_mut(index) {
            *slot = handle;
        }
        self
    }

    #[must_use]
    pub fn with_handles(mut self, with_grip: bool) -> Self {
        self.handles
            .iter_mut()
            .for_each(|handle| handle.with_grip = with_grip);
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn keyboard_step(mut self, step: f32) -> Self {
        if step.is_finite() && step > 0.0 {
            self.keyboard_step = step.min(1.0);
        }
        self
    }

    #[must_use]
    pub fn pointer_hit_size(mut self, size: f32) -> Self {
        if size.is_finite() && size >= DIVIDER_THICKNESS {
            self.pointer_hit_size = size;
        }
        self
    }

    #[must_use]
    pub fn touch_hit_size(mut self, size: f32) -> Self {
        if size.is_finite() && size >= DIVIDER_THICKNESS {
            self.touch_hit_size = size;
        }
        self
    }

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

    fn into_widget(self) -> ResizableWidget<'a, Message> {
        let layout = ResizableLayout::new(self.panels.len(), &self.sizes, &self.minimums);
        let mut children =
            Vec::with_capacity(self.panels.len().saturating_mul(2).saturating_sub(1));
        let mut panels = self.panels.into_iter();

        if let Some(first) = panels.next() {
            children.push(first);
        }

        for (index, panel) in panels.enumerate() {
            let handle = self.handles[index];
            let disabled = self.disabled || handle.disabled;
            let current = layout.clone();
            let on_resize = Rc::clone(&self.on_resize);
            let orientation = self.orientation;
            let step = self.keyboard_step;
            let control: Element<'a, Message> = FocusControl::passive(
                resizable_handle_id(&self.id, index),
                iced::widget::Space::new(),
                &self.theme,
            )
            .disabled(disabled)
            .on_key_press(move |key, _modifiers| {
                let command = keyboard_command(&key, orientation)?;
                Some(on_resize(current.reduce(index, command, step)))
            })
            .style(transparent_focus_style)
            .into();

            children.push(control);
            children.push(panel);
        }

        ResizableWidget {
            layout,
            on_resize: self.on_resize,
            orientation: self.orientation,
            handles: self.handles,
            disabled: self.disabled,
            pointer_hit_size: self.pointer_hit_size,
            touch_hit_size: self.touch_hit_size,
            width: self.width,
            height: self.height,
            theme: self.theme,
            children,
        }
    }
}

impl<'a, Message> From<Resizable<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(resizable: Resizable<'a, Message>) -> Self {
        Element::new(resizable.into_widget())
    }
}

fn transparent_focus_style(
    _theme: &iced::Theme,
    _status: focus_control::Status,
) -> focus_control::Style {
    focus_control::Style {
        background: None,
        text_color: None,
        border: Border::default(),
        shadow: iced::Shadow::default(),
        focus_ring: Border::default(),
        focus_offset: 0.0,
    }
}

struct ResizableWidget<'a, Message> {
    layout: ResizableLayout,
    on_resize: Rc<dyn Fn(Vec<f32>) -> Message + 'a>,
    orientation: ResizableOrientation,
    handles: Vec<ResizableHandle>,
    disabled: bool,
    pointer_hit_size: f32,
    touch_hit_size: f32,
    width: Length,
    height: Length,
    theme: UiTheme,
    children: Vec<Element<'a, Message>>,
}

#[derive(Debug, Clone, Default)]
struct State {
    drag: Option<Drag>,
}

#[derive(Debug, Clone)]
struct Drag {
    handle: usize,
    source: DragSource,
    origin: f32,
    layout: ResizableLayout,
    last: Vec<f32>,
    orientation: ResizableOrientation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragSource {
    Mouse,
    Touch(touch::Finger),
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for ResizableWidget<'_, Message>
where
    Message: Clone,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.children.iter().map(widget::Tree::new).collect()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&self.children);
        let state = tree.state.downcast_mut::<State>();

        if state.drag.as_ref().is_some_and(|drag| {
            drag.handle >= self.handles.len()
                || self.handle_disabled(drag.handle)
                || drag.layout.sizes().len() != self.layout.sizes().len()
                || drag.layout.minimums() != self.layout.minimums()
                || drag.orientation != self.orientation
        }) {
            state.drag = None;
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(self.width, self.height, Size::ZERO);
        let bounds = Rectangle::new(Point::ORIGIN, size);
        let geometry = group_geometry(
            bounds,
            self.layout.sizes(),
            self.orientation,
            self.pointer_hit_size,
            self.touch_hit_size,
        );
        let mut nodes = Vec::with_capacity(self.children.len());

        for (index, (child, tree)) in self.children.iter_mut().zip(&mut tree.children).enumerate() {
            let child_bounds = if index % 2 == 0 {
                geometry.panels[index / 2]
            } else {
                geometry.handles[index / 2].pointer_hit
            };
            let exact = layout::Limits::new(child_bounds.size(), child_bounds.size());
            nodes.push(
                child
                    .as_widget_mut()
                    .layout(tree, renderer, &exact)
                    .move_to(child_bounds.position()),
            );
        }

        layout::Node::with_children(size, nodes)
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, tree), layout)| {
                    child
                        .as_widget_mut()
                        .operate(tree, layout, renderer, operation);
                });
        });
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
        if reset_on_window_unfocus(
            tree.state.downcast_mut::<State>(),
            &mut tree.children,
            event,
            self.handles.len(),
        ) {
            self.update_children(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
            shell.request_redraw();
            return;
        }

        if matches!(event, Event::Keyboard(_)) {
            self.update_children(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
            return;
        }

        let bounds = layout.bounds();
        let geometry = group_geometry(
            bounds,
            self.layout.sizes(),
            self.orientation,
            self.pointer_hit_size,
            self.touch_hit_size,
        );
        let dragging = tree.state.downcast_ref::<State>().drag.clone();

        let movement = match (event, dragging.as_ref()) {
            (
                Event::Mouse(mouse::Event::CursorMoved { position }),
                Some(Drag {
                    source: DragSource::Mouse,
                    ..
                }),
            ) => Some(*position),
            (
                Event::Touch(
                    touch::Event::FingerMoved { id, position }
                    | touch::Event::FingerLifted { id, position },
                ),
                Some(Drag {
                    source: DragSource::Touch(active),
                    ..
                }),
            ) if id == active => Some(*position),
            _ => None,
        };
        let finished = finish_drag(tree.state.downcast_mut::<State>(), event);

        if let (Some(point), Some(drag)) = (movement, dragging.as_ref()) {
            let delta = drag_delta(drag.origin, point, bounds, self.orientation);
            let sizes = drag.layout.resize(drag.handle, delta);
            let changed = sizes != drag.last;
            if !finished
                && changed
                && let Some(active) = tree.state.downcast_mut::<State>().drag.as_mut()
            {
                active.last = sizes.clone();
            }
            if !finished || changed {
                shell.publish((self.on_resize)(sizes));
            }
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        if finished {
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        if dragging.is_some() {
            return;
        }

        let press = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => cursor
                .land()
                .position()
                .map(|point| (DragSource::Mouse, point, false)),
            Event::Touch(touch::Event::FingerPressed { id, position }) => {
                Some((DragSource::Touch(*id), *position, true))
            }
            _ => None,
        };

        if let Some((source, point, coarse)) = press {
            let handle = hit_handle(&geometry, point, coarse, |index| {
                !self.handle_disabled(index)
            });
            focus_handle(&mut tree.children, handle, self.handles.len());

            if let Some(handle) = handle {
                tree.state.downcast_mut::<State>().drag = Some(Drag {
                    handle,
                    source,
                    origin: main_position(point, self.orientation),
                    layout: self.layout.clone(),
                    last: self.layout.sizes().to_vec(),
                    orientation: self.orientation,
                });
                shell.capture_event();
                shell.request_redraw();
                return;
            }
        }

        self.update_children(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();

        if state.drag.is_some() {
            return match self.orientation {
                ResizableOrientation::Horizontal => mouse::Interaction::ResizingHorizontally,
                ResizableOrientation::Vertical => mouse::Interaction::ResizingVertically,
            };
        }

        let geometry = group_geometry(
            layout.bounds(),
            self.layout.sizes(),
            self.orientation,
            self.pointer_hit_size,
            self.touch_hit_size,
        );
        let over_handle = cursor.position().and_then(|point| {
            hit_handle(&geometry, point, false, |index| {
                !self.handle_disabled(index)
            })
        });

        if over_handle.is_some() {
            return match self.orientation {
                ResizableOrientation::Horizontal => mouse::Interaction::ResizingHorizontally,
                ResizableOrientation::Vertical => mouse::Interaction::ResizingVertically,
            };
        }

        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        iced_theme: &iced::Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let layouts = layout.children().collect::<Vec<_>>();

        for index in (0..self.children.len()).step_by(2) {
            renderer.with_layer(layouts[index].bounds(), |renderer| {
                self.children[index].as_widget().draw(
                    &tree.children[index],
                    renderer,
                    iced_theme,
                    renderer_style,
                    layouts[index],
                    cursor,
                    viewport,
                );
            });
        }

        let geometry = group_geometry(
            layout.bounds(),
            self.layout.sizes(),
            self.orientation,
            self.pointer_hit_size,
            self.touch_hit_size,
        );
        let dragging = tree
            .state
            .downcast_ref::<State>()
            .drag
            .as_ref()
            .map(|drag| drag.handle);
        let cursor_position = cursor.position();

        for (index, handle) in geometry.handles.iter().enumerate() {
            let disabled = self.handle_disabled(index);
            let focused = handle_focused(&tree.children, index);
            let hovered = cursor_position.is_some_and(|point| handle.pointer_hit.contains(point));
            let status = if disabled {
                HandleStatus::Disabled
            } else if dragging == Some(index) {
                HandleStatus::Dragging
            } else if focused {
                HandleStatus::Focused
            } else if hovered {
                HandleStatus::Hovered
            } else {
                HandleStatus::Active
            };
            draw_handle(
                renderer,
                handle,
                self.orientation,
                self.handles[index].with_grip,
                handle_style(&self.theme, status),
                matches!(status, HandleStatus::Focused | HandleStatus::Dragging),
            );

            let child_index = handle_tree_index(index);
            self.children[child_index].as_widget().draw(
                &tree.children[child_index],
                renderer,
                iced_theme,
                renderer_style,
                layouts[child_index],
                cursor,
                viewport,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<Message> ResizableWidget<'_, Message> {
    fn handle_disabled(&self, index: usize) -> bool {
        self.disabled || self.handles.get(index).is_none_or(|handle| handle.disabled)
    }
}

impl<Message> ResizableWidget<'_, Message>
where
    Message: Clone,
{
    #[allow(clippy::too_many_arguments)]
    fn update_children(
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
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );

            if shell.is_event_captured() {
                break;
            }
        }
    }
}

fn handle_tree_index(handle: usize) -> usize {
    handle * 2 + 1
}

fn handle_focused(children: &[widget::Tree], handle: usize) -> bool {
    children.get(handle_tree_index(handle)).is_some_and(|tree| {
        tree.state
            .downcast_ref::<focus_control::State>()
            .is_focused()
    })
}

fn reset_on_window_unfocus(
    state: &mut State,
    children: &mut [widget::Tree],
    event: &Event,
    handle_count: usize,
) -> bool {
    if !matches!(event, Event::Window(iced::window::Event::Unfocused)) {
        return false;
    }

    state.drag = None;
    focus_handle(children, None, handle_count);
    true
}

fn finish_drag(state: &mut State, event: &Event) -> bool {
    let Some(source) = state.drag.as_ref().map(|drag| drag.source) else {
        return false;
    };
    let released = matches!(
        (event, source),
        (
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            DragSource::Mouse
        )
    ) || matches!(
        (event, source),
        (
            Event::Touch(touch::Event::FingerLifted { id, .. })
                | Event::Touch(touch::Event::FingerLost { id, .. }),
            DragSource::Touch(active)
        ) if *id == active
    );

    if released {
        state.drag = None;
    }
    released
}

fn focus_handle(children: &mut [widget::Tree], target: Option<usize>, handle_count: usize) {
    for handle in 0..handle_count {
        let Some(tree) = children.get_mut(handle_tree_index(handle)) else {
            continue;
        };
        let state = tree.state.downcast_mut::<focus_control::State>();

        if target == Some(handle) {
            state.focus();
        } else {
            state.unfocus();
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct GroupGeometry {
    panels: Vec<Rectangle>,
    handles: Vec<HandleGeometry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HandleGeometry {
    divider: Rectangle,
    pointer_hit: Rectangle,
    touch_hit: Rectangle,
    grip: Rectangle,
}

fn group_geometry(
    bounds: Rectangle,
    sizes: &[f32],
    orientation: ResizableOrientation,
    pointer_hit_size: f32,
    touch_hit_size: f32,
) -> GroupGeometry {
    let mut panels = Vec::with_capacity(sizes.len());
    let mut handles = Vec::with_capacity(sizes.len().saturating_sub(1));
    let mut fraction = 0.0;

    for (index, size) in sizes.iter().enumerate() {
        let start = fraction;
        fraction = if index + 1 == sizes.len() {
            1.0
        } else {
            (fraction + size).clamp(0.0, 1.0)
        };
        panels.push(axis_segment(bounds, start, fraction, orientation));

        if index + 1 < sizes.len() {
            let center =
                main_start(bounds, orientation) + fraction * main_size(bounds, orientation);
            let pointer_hit = centered_axis_rect(bounds, center, pointer_hit_size, orientation);
            let touch_hit = centered_axis_rect(bounds, center, touch_hit_size, orientation);
            let divider = centered_axis_rect(bounds, center, DIVIDER_THICKNESS, orientation);
            let grip_size = match orientation {
                ResizableOrientation::Horizontal => Size::new(GRIP_SHORT, GRIP_LONG),
                ResizableOrientation::Vertical => Size::new(GRIP_LONG, GRIP_SHORT),
            };
            let grip = Rectangle::new(
                Point::new(
                    center - grip_size.width / 2.0,
                    bounds.center_y() - grip_size.height / 2.0,
                ),
                grip_size,
            );
            let grip = match orientation {
                ResizableOrientation::Horizontal => grip,
                ResizableOrientation::Vertical => Rectangle::new(
                    Point::new(
                        bounds.center_x() - grip_size.width / 2.0,
                        center - grip_size.height / 2.0,
                    ),
                    grip_size,
                ),
            };

            handles.push(HandleGeometry {
                divider,
                pointer_hit,
                touch_hit,
                grip,
            });
        }
    }

    GroupGeometry { panels, handles }
}

fn axis_segment(
    bounds: Rectangle,
    start: f32,
    end: f32,
    orientation: ResizableOrientation,
) -> Rectangle {
    match orientation {
        ResizableOrientation::Horizontal => Rectangle {
            x: bounds.x + bounds.width * start,
            y: bounds.y,
            width: bounds.width * (end - start),
            height: bounds.height,
        },
        ResizableOrientation::Vertical => Rectangle {
            x: bounds.x,
            y: bounds.y + bounds.height * start,
            width: bounds.width,
            height: bounds.height * (end - start),
        },
    }
}

fn centered_axis_rect(
    bounds: Rectangle,
    center: f32,
    thickness: f32,
    orientation: ResizableOrientation,
) -> Rectangle {
    let start = main_start(bounds, orientation);
    let end = start + main_size(bounds, orientation);
    let leading = (center - thickness / 2.0).max(start);
    let trailing = (center + thickness / 2.0).min(end).max(leading);

    match orientation {
        ResizableOrientation::Horizontal => Rectangle {
            x: leading,
            y: bounds.y,
            width: trailing - leading,
            height: bounds.height,
        },
        ResizableOrientation::Vertical => Rectangle {
            x: bounds.x,
            y: leading,
            width: bounds.width,
            height: trailing - leading,
        },
    }
}

fn main_start(bounds: Rectangle, orientation: ResizableOrientation) -> f32 {
    match orientation {
        ResizableOrientation::Horizontal => bounds.x,
        ResizableOrientation::Vertical => bounds.y,
    }
}

fn main_size(bounds: Rectangle, orientation: ResizableOrientation) -> f32 {
    match orientation {
        ResizableOrientation::Horizontal => bounds.width,
        ResizableOrientation::Vertical => bounds.height,
    }
}

fn main_position(point: Point, orientation: ResizableOrientation) -> f32 {
    match orientation {
        ResizableOrientation::Horizontal => point.x,
        ResizableOrientation::Vertical => point.y,
    }
}

fn drag_delta(
    origin: f32,
    current: Point,
    bounds: Rectangle,
    orientation: ResizableOrientation,
) -> f32 {
    let span = main_size(bounds, orientation);

    if span > 0.0 {
        (main_position(current, orientation) - origin) / span
    } else {
        0.0
    }
}

fn hit_handle(
    geometry: &GroupGeometry,
    point: Point,
    coarse: bool,
    enabled: impl Fn(usize) -> bool,
) -> Option<usize> {
    geometry
        .handles
        .iter()
        .enumerate()
        .filter(|(index, handle)| {
            enabled(*index)
                && if coarse {
                    handle.touch_hit.contains(point)
                } else {
                    handle.pointer_hit.contains(point)
                }
        })
        .min_by(|(_, left), (_, right)| {
            let left = distance_to_divider(left, point);
            let right = distance_to_divider(right, point);
            left.total_cmp(&right)
        })
        .map(|(index, _)| index)
}

fn distance_to_divider(handle: &HandleGeometry, point: Point) -> f32 {
    if handle.divider.width > handle.divider.height {
        (point.y - handle.divider.center_y()).abs()
    } else {
        (point.x - handle.divider.center_x()).abs()
    }
}

fn draw_handle(
    renderer: &mut iced::Renderer,
    geometry: &HandleGeometry,
    orientation: ResizableOrientation,
    with_grip: bool,
    style: HandleStyle,
    emphasized: bool,
) {
    let mut divider = geometry.divider;

    if emphasized {
        match orientation {
            ResizableOrientation::Horizontal => {
                divider.x = divider.center_x() - FOCUSED_DIVIDER_THICKNESS / 2.0;
                divider.width = FOCUSED_DIVIDER_THICKNESS;
            }
            ResizableOrientation::Vertical => {
                divider.y = divider.center_y() - FOCUSED_DIVIDER_THICKNESS / 2.0;
                divider.height = FOCUSED_DIVIDER_THICKNESS;
            }
        }
    }

    renderer.fill_quad(
        renderer::Quad {
            bounds: divider,
            ..renderer::Quad::default()
        },
        Background::Color(style.divider),
    );

    if !with_grip {
        return;
    }

    renderer.fill_quad(
        renderer::Quad {
            bounds: geometry.grip,
            border: Border {
                color: style.grip_border,
                width: 1.0,
                radius: 3.0.into(),
            },
            ..renderer::Quad::default()
        },
        Background::Color(style.grip_background),
    );

    for dot in grip_dots(geometry.grip, orientation) {
        renderer.fill_quad(
            renderer::Quad {
                bounds: dot,
                border: Border {
                    radius: 999.0.into(),
                    ..Border::default()
                },
                ..renderer::Quad::default()
            },
            Background::Color(style.grip_dot),
        );
    }
}

fn grip_dots(grip: Rectangle, orientation: ResizableOrientation) -> [Rectangle; 6] {
    std::array::from_fn(|index| {
        let major = index / 2;
        let minor = index % 2;
        let offset_major = (major as f32 - 1.0) * GRIP_DOT_GAP;
        let offset_minor = (minor as f32 - 0.5) * GRIP_DOT_GAP;
        let center = match orientation {
            ResizableOrientation::Horizontal => Point::new(
                grip.center_x() + offset_minor,
                grip.center_y() + offset_major,
            ),
            ResizableOrientation::Vertical => Point::new(
                grip.center_x() + offset_major,
                grip.center_y() + offset_minor,
            ),
        };

        Rectangle::new(
            Point::new(center.x - GRIP_DOT / 2.0, center.y - GRIP_DOT / 2.0),
            Size::new(GRIP_DOT, GRIP_DOT),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    fn close(left: f32, right: f32) {
        assert!((left - right).abs() < 0.000_01, "{left} != {right}");
    }

    #[test]
    fn layout_normalizes_bad_inputs_and_feasible_minimums() {
        let layout = ResizableLayout::new(3, &[2.0, f32::NAN, 1.0], &[0.1, -1.0, 0.2]);
        close(layout.sizes().iter().sum(), 1.0);
        close(layout.sizes()[0], 2.0 / 3.0);
        close(layout.sizes()[1], 0.0);
        close(layout.sizes()[2], 1.0 / 3.0);
        assert_eq!(layout.minimums(), &[0.1, 0.0, 0.2]);

        let impossible = ResizableLayout::new(2, &[1.0, 1.0], &[0.8, 0.8]);
        assert_eq!(impossible.minimums(), &[0.5, 0.5]);
        assert_eq!(impossible.sizes(), &[0.5, 0.5]);
    }

    #[test]
    fn reducer_preserves_total_and_stops_at_both_minimums() {
        let layout = ResizableLayout::new(3, &[0.2, 0.5, 0.3], &[0.1, 0.2, 0.1]);
        let previous_min = layout.resize(0, -1.0);
        close(previous_min[0], 0.1);
        close(previous_min[1], 0.6);
        close(previous_min[2], 0.3);
        let next_min = layout.resize(1, 1.0);
        close(next_min[0], 0.2);
        close(next_min[1], 0.7);
        close(next_min[2], 0.1);
        assert_eq!(layout.resize(8, 0.2), layout.sizes());
        close(layout.resize(0, 0.05).iter().sum(), 1.0);

        let previous_min = layout.reduce(0, ResizeCommand::PreviousMinimum, 0.05);
        close(previous_min[0], 0.1);
        close(previous_min[1], 0.6);
        let next_min = layout.reduce(1, ResizeCommand::NextMinimum, 0.05);
        close(next_min[1], 0.7);
        close(next_min[2], 0.1);
    }

    #[test]
    fn keyboard_mapping_follows_the_visual_axis() {
        let left = keyboard::Key::Named(Named::ArrowLeft);
        let down = keyboard::Key::Named(Named::ArrowDown);
        let home = keyboard::Key::Named(Named::Home);

        assert_eq!(
            keyboard_command(&left, ResizableOrientation::Horizontal),
            Some(ResizeCommand::Decrease)
        );
        assert_eq!(
            keyboard_command(&left, ResizableOrientation::Vertical),
            None
        );
        assert_eq!(
            keyboard_command(&down, ResizableOrientation::Vertical),
            Some(ResizeCommand::Increase)
        );
        assert_eq!(
            keyboard_command(&home, ResizableOrientation::Horizontal),
            Some(ResizeCommand::PreviousMinimum)
        );
    }

    #[test]
    fn geometry_aligns_panels_dividers_hits_and_grips() {
        let bounds = Rectangle::new(Point::new(10.0, 20.0), Size::new(200.0, 100.0));
        let horizontal = group_geometry(
            bounds,
            &[0.25, 0.5, 0.25],
            ResizableOrientation::Horizontal,
            12.0,
            32.0,
        );
        assert_eq!(horizontal.panels[0].width, 50.0);
        assert_eq!(horizontal.panels[1].x, 60.0);
        assert_eq!(horizontal.panels[2].x + horizontal.panels[2].width, 210.0);
        assert_eq!(horizontal.handles[0].divider.center_x(), 60.0);
        assert_eq!(horizontal.handles[0].pointer_hit.center_x(), 60.0);
        assert_eq!(horizontal.handles[0].pointer_hit.width, 12.0);
        assert_eq!(horizontal.handles[0].touch_hit.width, 32.0);
        assert_eq!(horizontal.handles[0].grip.center(), Point::new(60.0, 70.0));
        assert_eq!(horizontal.handles[0].grip.size(), Size::new(12.0, 16.0));

        let vertical = group_geometry(
            bounds,
            &[0.5, 0.5],
            ResizableOrientation::Vertical,
            12.0,
            32.0,
        );
        assert_eq!(vertical.handles[0].divider.center_y(), 70.0);
        assert_eq!(vertical.handles[0].pointer_hit.height, 12.0);
        assert_eq!(vertical.handles[0].touch_hit.height, 32.0);
        assert_eq!(vertical.handles[0].grip.center(), Point::new(110.0, 70.0));
        assert_eq!(vertical.handles[0].grip.size(), Size::new(16.0, 12.0));
    }

    #[test]
    fn overlapping_hit_targets_choose_the_nearest_enabled_divider() {
        let geometry = group_geometry(
            Rectangle::new(Point::ORIGIN, Size::new(100.0, 40.0)),
            &[0.48, 0.04, 0.48],
            ResizableOrientation::Horizontal,
            12.0,
            32.0,
        );
        assert_eq!(
            hit_handle(&geometry, Point::new(49.0, 20.0), false, |_| true),
            Some(0)
        );
        assert_eq!(
            hit_handle(&geometry, Point::new(51.0, 20.0), false, |_| true),
            Some(1)
        );
        assert_eq!(
            hit_handle(&geometry, Point::new(49.0, 20.0), false, |index| index == 1),
            Some(1)
        );
        assert_eq!(
            hit_handle(&geometry, Point::new(40.0, 20.0), false, |_| true),
            None
        );
        assert_eq!(
            hit_handle(&geometry, Point::new(40.0, 20.0), true, |_| true),
            Some(0)
        );
    }

    #[test]
    fn pointer_and_touch_motion_map_to_normalized_axis_deltas() {
        let bounds = Rectangle::new(Point::new(10.0, 20.0), Size::new(200.0, 100.0));
        close(
            drag_delta(
                60.0,
                Point::new(110.0, 999.0),
                bounds,
                ResizableOrientation::Horizontal,
            ),
            0.25,
        );
        close(
            drag_delta(
                30.0,
                Point::new(999.0, 80.0),
                bounds,
                ResizableOrientation::Vertical,
            ),
            0.5,
        );
        close(
            drag_delta(
                0.0,
                Point::new(10.0, 10.0),
                Rectangle::new(Point::ORIGIN, Size::ZERO),
                ResizableOrientation::Horizontal,
            ),
            0.0,
        );
    }

    #[test]
    fn grip_dots_rotate_without_losing_center_alignment() {
        let grip = Rectangle::new(Point::new(4.0, 2.0), Size::new(12.0, 16.0));
        let horizontal = grip_dots(grip, ResizableOrientation::Horizontal);
        close(
            horizontal[0].center_x() + horizontal[1].center_x(),
            grip.center_x() * 2.0,
        );
        close(
            horizontal[0].center_y() + horizontal[4].center_y(),
            grip.center_y() * 2.0,
        );

        let vertical = grip_dots(grip, ResizableOrientation::Vertical);
        close(
            vertical[0].center_y() + vertical[1].center_y(),
            grip.center_y() * 2.0,
        );
        close(
            vertical[0].center_x() + vertical[4].center_x(),
            grip.center_x() * 2.0,
        );
    }

    #[test]
    fn idle_and_focused_dividers_clear_three_to_one_contrast_in_both_themes() {
        for theme in [LIGHT, DARK] {
            let active = handle_style(&theme, HandleStatus::Active);
            let focused = handle_style(&theme, HandleStatus::Focused);
            let disabled = handle_style(&theme, HandleStatus::Disabled);

            assert!(contrast(active.divider, theme.palette.background) >= 3.0);
            assert!(contrast(active.grip_border, active.grip_background) >= 3.0);
            assert!(contrast(focused.divider, theme.palette.background) >= 3.0);
            assert!(disabled.divider.a < active.divider.a);
        }
    }

    #[test]
    fn separators_ignore_activation_keys_but_resize_with_arrow_keys() {
        use iced::advanced::renderer::Headless as _;
        use iced::widget::{container, text};

        fn key_event(named: Named, pressed: bool) -> Event {
            let key = keyboard::Key::Named(named);
            let physical_key =
                keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Unidentified);

            Event::Keyboard(if pressed {
                keyboard::Event::KeyPressed {
                    key: key.clone(),
                    modified_key: key,
                    physical_key,
                    location: keyboard::Location::Standard,
                    modifiers: keyboard::Modifiers::default(),
                    text: None,
                    repeat: false,
                }
            } else {
                keyboard::Event::KeyReleased {
                    key: key.clone(),
                    modified_key: key,
                    physical_key,
                    location: keyboard::Location::Standard,
                    modifiers: keyboard::Modifiers::default(),
                }
            })
        }

        let panels = ["One", "Two"].map(|label| container(text(label)).into());
        let mut widget = resizable(
            "keyboard",
            panels,
            vec![0.5, 0.5],
            vec![0.1, 0.1],
            |sizes| sizes,
            &LIGHT,
        )
        .into_widget();
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let viewport = Rectangle::with_size(Size::new(200.0, 100.0));
        let mut tree = widget::Tree::new(&widget as &dyn Widget<_, _, _>);
        let node = widget.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let mut focus =
            widget::operation::focusable::focus::<()>(resizable_handle_id("keyboard", 0));
        widget.operate(&mut tree, Layout::new(&node), &renderer, &mut focus);

        let mut clipboard = iced::advanced::clipboard::Null;
        for named in [Named::Enter, Named::Space] {
            let mut messages = Vec::new();
            let mut statuses = [iced::event::Status::Ignored; 2];
            for (index, pressed) in [true, false].into_iter().enumerate() {
                let mut shell = Shell::new(&mut messages);
                widget.update(
                    &mut tree,
                    &key_event(named, pressed),
                    Layout::new(&node),
                    mouse::Cursor::Unavailable,
                    &renderer,
                    &mut clipboard,
                    &mut shell,
                    &viewport,
                );
                statuses[index] = shell.event_status();
            }
            assert!(messages.is_empty());
            assert_eq!(statuses, [iced::event::Status::Ignored; 2]);
        }

        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        widget.update(
            &mut tree,
            &key_event(Named::ArrowRight, true),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        assert_eq!(shell.event_status(), iced::event::Status::Captured);
        drop(shell);
        assert_eq!(messages.len(), 1);
        close(messages[0][0], 0.55);
        close(messages[0][1], 0.45);
        assert!(handle_focused(&tree.children, 0));
    }

    #[test]
    fn touch_release_applies_its_final_position_before_ending_drag() {
        use iced::advanced::renderer::Headless as _;
        use iced::widget::{container, text};

        fn update(
            widget: &mut ResizableWidget<'_, Vec<f32>>,
            tree: &mut widget::Tree,
            node: &layout::Node,
            renderer: &iced::Renderer,
            viewport: &Rectangle,
            event: Event,
            messages: &mut Vec<Vec<f32>>,
        ) -> iced::event::Status {
            let mut clipboard = iced::advanced::clipboard::Null;
            let mut shell = Shell::new(messages);
            widget.update(
                tree,
                &event,
                Layout::new(node),
                mouse::Cursor::Unavailable,
                renderer,
                &mut clipboard,
                &mut shell,
                viewport,
            );
            shell.event_status()
        }

        let panels = ["One", "Two"].map(|label| container(text(label)).into());
        let mut widget = resizable(
            "touch-release",
            panels,
            vec![0.5, 0.5],
            vec![0.1, 0.1],
            |sizes| sizes,
            &LIGHT,
        )
        .into_widget();
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let viewport = Rectangle::with_size(Size::new(200.0, 100.0));
        let mut tree = widget::Tree::new(&widget as &dyn Widget<_, _, _>);
        let node = widget.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let finger = touch::Finger(7);
        let mut messages = Vec::new();
        let press = |id, x| {
            Event::Touch(touch::Event::FingerPressed {
                id,
                position: Point::new(x, 50.0),
            })
        };
        let movement = |id, x| {
            Event::Touch(touch::Event::FingerMoved {
                id,
                position: Point::new(x, 50.0),
            })
        };
        let lift = |id, x| {
            Event::Touch(touch::Event::FingerLifted {
                id,
                position: Point::new(x, 50.0),
            })
        };
        macro_rules! send {
            ($event:expr) => {
                update(
                    &mut widget,
                    &mut tree,
                    &node,
                    &renderer,
                    &viewport,
                    $event,
                    &mut messages,
                )
            };
        }

        assert_eq!(send!(press(finger, 100.0)), iced::event::Status::Captured);

        assert_eq!(
            send!(lift(touch::Finger(8), 180.0)),
            iced::event::Status::Ignored
        );
        assert!(messages.is_empty());
        assert!(tree.state.downcast_ref::<State>().drag.is_some());

        assert_eq!(send!(lift(finger, 140.0)), iced::event::Status::Captured);
        assert_eq!(messages.len(), 1);
        close(messages[0][0], 0.7);
        close(messages[0][1], 0.3);
        assert!(tree.state.downcast_ref::<State>().drag.is_none());

        messages.clear();
        assert_eq!(send!(press(finger, 100.0)), iced::event::Status::Captured);
        assert_eq!(send!(lift(finger, 100.0)), iced::event::Status::Captured);
        assert!(messages.is_empty());
        assert!(tree.state.downcast_ref::<State>().drag.is_none());

        assert_eq!(send!(press(finger, 100.0)), iced::event::Status::Captured);
        assert_eq!(
            send!(movement(finger, 140.0)),
            iced::event::Status::Captured
        );
        assert_eq!(send!(lift(finger, 100.0)), iced::event::Status::Captured);
        assert_eq!(messages.len(), 2);
        close(messages[0][0], 0.7);
        close(messages[0][1], 0.3);
        close(messages[1][0], 0.5);
        close(messages[1][1], 0.5);
        assert!(tree.state.downcast_ref::<State>().drag.is_none());
    }

    #[test]
    fn drag_lifecycle_clears_on_blur_or_minimum_change() {
        use iced::widget::{container, text};

        let finger = touch::Finger(7);
        let drag = Drag {
            handle: 0,
            source: DragSource::Touch(finger),
            origin: 50.0,
            layout: ResizableLayout::new(2, &[0.5, 0.5], &[0.1, 0.1]),
            last: vec![0.5, 0.5],
            orientation: ResizableOrientation::Horizontal,
        };
        let mut state = State { drag: Some(drag) };

        assert!(!finish_drag(
            &mut state,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ));
        assert!(!finish_drag(
            &mut state,
            &Event::Touch(touch::Event::FingerLifted {
                id: touch::Finger(8),
                position: Point::ORIGIN,
            }),
        ));
        assert!(state.drag.is_some());

        let make_widget = |minimums| {
            let panels = ["One", "Two"].map(|label| container(text(label)).into());
            resizable(
                "blur",
                panels,
                vec![0.5, 0.5],
                minimums,
                |sizes| sizes,
                &LIGHT,
            )
            .into_widget()
        };
        let widget = make_widget(vec![0.1, 0.1]);
        let mut tree = widget::Tree::new(&widget as &dyn Widget<_, _, _>);
        *tree.state.downcast_mut::<State>() = state.clone();
        focus_handle(&mut tree.children, Some(0), 1);

        assert!(reset_on_window_unfocus(
            tree.state.downcast_mut::<State>(),
            &mut tree.children,
            &Event::Window(iced::window::Event::Unfocused),
            1,
        ));
        assert!(tree.state.downcast_ref::<State>().drag.is_none());
        assert!(!handle_focused(&tree.children, 0));

        *tree.state.downcast_mut::<State>() = state;
        make_widget(vec![0.4, 0.4]).diff(&mut tree);
        assert!(tree.state.downcast_ref::<State>().drag.is_none());
    }

    #[test]
    fn arbitrary_groups_build_one_focusable_separator_per_gap() {
        use iced::widget::{container, text};

        let panels = ["One", "Two", "Three"].map(|label| container(text(label)).into());
        let group: Element<'_, Vec<f32>> = resizable(
            "workspace",
            panels,
            vec![0.2, 0.5, 0.3],
            vec![0.1, 0.2, 0.1],
            |sizes| sizes,
            &LIGHT,
        )
        .with_handles(true)
        .into();

        assert_eq!(group.as_widget().children().len(), 5);
    }

    fn contrast(left: Color, right: Color) -> f32 {
        let left = luminance(left);
        let right = luminance(right);
        (left.max(right) + 0.05) / (left.min(right) + 0.05)
    }

    fn luminance(color: Color) -> f32 {
        let channel = |value: f32| {
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    }
}
