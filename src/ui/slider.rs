use std::ops::RangeInclusive;
use std::rc::Rc;

use super::focus_control::{self, FocusControl, Status};
use super::theme::{Theme as UiTheme, alpha, mix};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::keyboard::{self, key::Named};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Shadow, Size, Task,
    Vector, touch,
};

const THUMB_DIAMETER: f32 = 16.0;
const TRACK_THICKNESS: f32 = 4.0;
const DEFAULT_LENGTH: f32 = 160.0;
const DEFAULT_CROSS: f32 = 32.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SliderOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderCommand {
    Decrement,
    Increment,
    PageDecrement,
    PageIncrement,
    Minimum,
    Maximum,
}

/// Sanitized numeric rules shared by pointer and keyboard updates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderSpec {
    min: f32,
    max: f32,
    step: f32,
    page_step: f32,
}

impl SliderSpec {
    pub fn new(range: RangeInclusive<f32>, step: f32) -> Self {
        let (mut min, mut max) = range.into_inner();

        if !min.is_finite() || !max.is_finite() {
            min = 0.0;
            max = 100.0;
        } else if min > max {
            std::mem::swap(&mut min, &mut max);
        }

        let step = if step.is_finite() && step > 0.0 {
            step
        } else {
            1.0
        };

        Self {
            min,
            max,
            step,
            page_step: step * 10.0,
        }
    }

    #[must_use]
    pub fn page_step(mut self, page_step: f32) -> Self {
        if page_step.is_finite() && page_step > 0.0 {
            self.page_step = page_step;
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderStyle {
    pub track: Color,
    pub range: Color,
    pub thumb: Color,
    pub thumb_border: Color,
    pub focus_ring: Color,
}

pub fn style(theme: &UiTheme, disabled: bool, invalid: bool) -> SliderStyle {
    let range = if invalid {
        theme.palette.destructive
    } else {
        theme.palette.primary
    };

    if disabled {
        SliderStyle {
            track: alpha(
                mix(theme.palette.background, theme.palette.foreground, 0.14),
                0.6,
            ),
            range: alpha(range, 0.45),
            thumb: alpha(theme.palette.card, 0.75),
            thumb_border: alpha(range, 0.45),
            focus_ring: Color::TRANSPARENT,
        }
    } else {
        SliderStyle {
            track: mix(theme.palette.background, theme.palette.foreground, 0.14),
            range,
            thumb: theme.palette.card,
            thumb_border: range,
            focus_ring: if invalid {
                theme.palette.destructive
            } else {
                theme.palette.ring
            },
        }
    }
}

/// A controlled, multi-thumb slider. Values are sorted, stepped, and clamped.
/// Enter or Space advances the focused thumb by one step.
pub struct Slider<'a, Message>
where
    Message: Clone + 'a,
{
    id: String,
    values: Vec<f32>,
    spec: SliderSpec,
    on_change: Rc<dyn Fn(Vec<f32>) -> Message + 'a>,
    orientation: SliderOrientation,
    reversed: bool,
    disabled: bool,
    invalid: bool,
    width: Option<Length>,
    height: Option<Length>,
    theme: UiTheme,
}

pub fn slider<'a, Message>(
    id: impl Into<String>,
    values: impl Into<Vec<f32>>,
    range: RangeInclusive<f32>,
    step: f32,
    on_change: impl Fn(Vec<f32>) -> Message + 'a,
    theme: &UiTheme,
) -> Slider<'a, Message>
where
    Message: Clone + 'a,
{
    Slider {
        id: id.into(),
        values: values.into(),
        spec: SliderSpec::new(range, step),
        on_change: Rc::new(on_change),
        orientation: SliderOrientation::Horizontal,
        reversed: false,
        disabled: false,
        invalid: false,
        width: None,
        height: None,
        theme: *theme,
    }
}

impl<'a, Message> Slider<'a, Message>
where
    Message: Clone + 'a,
{
    #[must_use]
    pub fn orientation(mut self, orientation: SliderOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Reverses the visual axis and arrow direction. Use this for RTL sliders.
    #[must_use]
    pub fn reversed(mut self, reversed: bool) -> Self {
        self.reversed = reversed;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    #[must_use]
    pub fn page_step(mut self, page_step: f32) -> Self {
        self.spec = self.spec.page_step(page_step);
        self
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    pub fn into_widget(self) -> impl Widget<Message, iced::Theme, iced::Renderer> + 'a {
        let values = normalize_values(self.values, self.spec);
        let width = self.width.unwrap_or(match self.orientation {
            SliderOrientation::Horizontal => Length::Fill,
            SliderOrientation::Vertical => Length::Fixed(DEFAULT_CROSS),
        });
        let height = self.height.unwrap_or(match self.orientation {
            SliderOrientation::Horizontal => Length::Fixed(DEFAULT_CROSS),
            SliderOrientation::Vertical => Length::Fixed(DEFAULT_LENGTH),
        });
        let visual_style = style(&self.theme, self.disabled, self.invalid);
        let mut thumbs = Vec::with_capacity(values.len());

        for index in 0..values.len() {
            let activate = (self.on_change)(reduce_thumb(
                &values,
                index,
                SliderCommand::Increment,
                self.spec,
            ));
            let key_values = values.clone();
            let key_on_change = Rc::clone(&self.on_change);
            let spec = self.spec;
            let orientation = self.orientation;
            let reversed = self.reversed;
            let disabled = self.disabled;

            let thumb = FocusControl::new(
                slider_thumb_id(&self.id, index),
                iced::widget::Space::new()
                    .width(THUMB_DIAMETER)
                    .height(THUMB_DIAMETER),
                activate,
                &self.theme,
            )
            .disabled(disabled)
            .on_key_press(move |key, _modifiers| {
                let command = keyboard_command(&key, orientation, reversed)?;
                Some(key_on_change(reduce_thumb(
                    &key_values,
                    index,
                    command,
                    spec,
                )))
            })
            .style(move |_iced_theme, status| thumb_style(visual_style, status));

            thumbs.push(thumb.into());
        }

        SliderWidget {
            values,
            spec: self.spec,
            on_change: self.on_change,
            orientation: self.orientation,
            reversed: self.reversed,
            disabled: self.disabled,
            width,
            height,
            visual_style,
            thumbs,
        }
    }
}

impl<'a, Message> From<Slider<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(slider: Slider<'a, Message>) -> Self {
        Element::new(slider.into_widget())
    }
}

/// Stable ID for focusing or querying a particular thumb.
pub fn slider_thumb_id(slider_id: &str, index: usize) -> iced::widget::Id {
    iced::widget::Id::from(format!("ducktape-slider:{slider_id}:{index}"))
}

pub fn focus_slider_thumb<Message>(slider_id: &str, index: usize) -> Task<Message> {
    iced::widget::operation::focus(slider_thumb_id(slider_id, index))
}

pub fn keyboard_command(
    key: &keyboard::Key,
    orientation: SliderOrientation,
    reversed: bool,
) -> Option<SliderCommand> {
    let along_axis = match (orientation, key) {
        (SliderOrientation::Horizontal, keyboard::Key::Named(Named::ArrowLeft))
        | (SliderOrientation::Vertical, keyboard::Key::Named(Named::ArrowDown)) => {
            Some(SliderCommand::Decrement)
        }
        (SliderOrientation::Horizontal, keyboard::Key::Named(Named::ArrowRight))
        | (SliderOrientation::Vertical, keyboard::Key::Named(Named::ArrowUp)) => {
            Some(SliderCommand::Increment)
        }
        _ => None,
    };

    if let Some(command) = along_axis {
        return Some(if reversed {
            match command {
                SliderCommand::Decrement => SliderCommand::Increment,
                SliderCommand::Increment => SliderCommand::Decrement,
                _ => unreachable!(),
            }
        } else {
            command
        });
    }

    match key {
        keyboard::Key::Named(Named::PageDown) => Some(SliderCommand::PageDecrement),
        keyboard::Key::Named(Named::PageUp) => Some(SliderCommand::PageIncrement),
        keyboard::Key::Named(Named::Home) => Some(SliderCommand::Minimum),
        keyboard::Key::Named(Named::End) => Some(SliderCommand::Maximum),
        _ => None,
    }
}

pub fn normalize_values(values: impl Into<Vec<f32>>, spec: SliderSpec) -> Vec<f32> {
    let mut values = values.into();

    if values.is_empty() {
        values.push(spec.min);
    }

    values
        .iter_mut()
        .for_each(|value| *value = snap(*value, spec));
    values.sort_by(f32::total_cmp);
    values
}

pub fn set_thumb(values: &[f32], index: usize, value: f32, spec: SliderSpec) -> Vec<f32> {
    let mut values = normalize_values(values.to_vec(), spec);
    let index = index.min(values.len() - 1);
    let lower = index
        .checked_sub(1)
        .map_or(spec.min, |previous| values[previous]);
    let upper = values.get(index + 1).copied().unwrap_or(spec.max);
    values[index] = snap(value, spec).clamp(lower, upper);
    values
}

pub fn reduce_thumb(
    values: &[f32],
    index: usize,
    command: SliderCommand,
    spec: SliderSpec,
) -> Vec<f32> {
    let values = normalize_values(values.to_vec(), spec);
    let index = index.min(values.len() - 1);
    let current = values[index];
    let target = match command {
        SliderCommand::Decrement => current - spec.step,
        SliderCommand::Increment => current + spec.step,
        SliderCommand::PageDecrement => current - spec.page_step,
        SliderCommand::PageIncrement => current + spec.page_step,
        SliderCommand::Minimum => spec.min,
        SliderCommand::Maximum => spec.max,
    };
    set_thumb(&values, index, target, spec)
}

fn snap(value: f32, spec: SliderSpec) -> f32 {
    if spec.min == spec.max {
        return spec.min;
    }

    if value.is_nan() || value <= spec.min {
        return spec.min;
    }
    if value >= spec.max {
        return spec.max;
    }

    let steps = ((value - spec.min) / spec.step).round();
    (spec.min + steps * spec.step).clamp(spec.min, spec.max)
}

fn thumb_style(style: SliderStyle, status: Status) -> focus_control::Style {
    let hovered = matches!(status, Status::Hovered | Status::Pressed);
    let mut shell = focus_control::Style {
        background: Some(Background::Color(style.thumb)),
        text_color: None,
        border: Border {
            color: if hovered {
                style.focus_ring
            } else {
                style.thumb_border
            },
            width: if hovered { 2.0 } else { 1.5 },
            radius: 999.0.into(),
        },
        shadow: Shadow::default(),
        focus_ring: Border {
            color: style.focus_ring,
            width: 2.0,
            radius: 999.0.into(),
        },
        focus_offset: 2.0,
    };

    if status == Status::Disabled {
        shell.focus_ring.width = 0.0;
    }
    shell
}

struct SliderWidget<'a, Message> {
    values: Vec<f32>,
    spec: SliderSpec,
    on_change: Rc<dyn Fn(Vec<f32>) -> Message + 'a>,
    orientation: SliderOrientation,
    reversed: bool,
    disabled: bool,
    width: Length,
    height: Length,
    visual_style: SliderStyle,
    thumbs: Vec<Element<'a, Message>>,
}

#[derive(Debug, Clone, Copy, Default)]
struct State {
    active_thumb: usize,
    dragging: Option<Drag>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Drag {
    thumb: usize,
    source: DragSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragSource {
    Mouse,
    Touch(touch::Finger),
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for SliderWidget<'_, Message>
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
        self.thumbs.iter().map(widget::Tree::new).collect()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&self.thumbs);
        let state = tree.state.downcast_mut::<State>();
        state.active_thumb = state.active_thumb.min(self.values.len() - 1);
        if state
            .dragging
            .is_some_and(|drag| drag.thumb >= self.values.len())
        {
            state.dragging = None;
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
        let intrinsic = match self.orientation {
            SliderOrientation::Horizontal => Size::new(DEFAULT_LENGTH, DEFAULT_CROSS),
            SliderOrientation::Vertical => Size::new(DEFAULT_CROSS, DEFAULT_LENGTH),
        };
        let size = limits.resolve(self.width, self.height, intrinsic);
        let bounds = Rectangle::new(Point::ORIGIN, size);
        let thumb_limits = layout::Limits::new(
            Size::new(THUMB_DIAMETER, THUMB_DIAMETER),
            Size::new(THUMB_DIAMETER, THUMB_DIAMETER),
        );
        let children = self
            .thumbs
            .iter_mut()
            .zip(&mut tree.children)
            .zip(&self.values)
            .map(|((thumb, tree), value)| {
                let center =
                    point_for_value(*value, bounds, self.spec, self.orientation, self.reversed);
                thumb
                    .as_widget_mut()
                    .layout(tree, renderer, &thumb_limits)
                    .move_to(Point::new(
                        center.x - THUMB_DIAMETER / 2.0,
                        center.y - THUMB_DIAMETER / 2.0,
                    ))
            })
            .collect();

        layout::Node::with_children(size, children)
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
            self.thumbs
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((thumb, tree), layout)| {
                    thumb
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
        if let Some(focused) = focused_thumb(&tree.children) {
            tree.state.downcast_mut::<State>().active_thumb = focused;
        }

        if matches!(event, Event::Keyboard(_)) {
            for ((thumb, tree), layout) in self
                .thumbs
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                thumb.as_widget_mut().update(
                    tree, event, layout, cursor, renderer, clipboard, shell, viewport,
                );

                if shell.is_event_captured() {
                    break;
                }
            }
            return;
        }

        if self.disabled {
            tree.state.downcast_mut::<State>().dragging = None;
            return;
        }

        let bounds = layout.bounds();
        let press = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => cursor
                .land()
                .position()
                .map(|point| (DragSource::Mouse, point)),
            Event::Touch(touch::Event::FingerPressed { id, position }) => {
                Some((DragSource::Touch(*id), *position))
            }
            _ => None,
        };

        if let Some((source, point)) = press {
            if bounds.contains(point) {
                let value =
                    value_from_point(point, bounds, self.spec, self.orientation, self.reversed);
                let active = tree.state.downcast_ref::<State>().active_thumb;
                let thumb = nearest_thumb(&self.values, value, active);
                let next = set_thumb(&self.values, thumb, value, self.spec);

                {
                    let state = tree.state.downcast_mut::<State>();
                    state.active_thumb = thumb;
                    state.dragging = Some(Drag { thumb, source });
                }
                focus_thumb(&mut tree.children, Some(thumb));

                if next != self.values {
                    shell.publish((self.on_change)(next));
                }
                shell.capture_event();
                shell.request_redraw();
            } else {
                tree.state.downcast_mut::<State>().dragging = None;
                focus_thumb(&mut tree.children, None);
            }
            return;
        }

        let dragging = tree.state.downcast_ref::<State>().dragging;
        let movement = match (event, dragging) {
            (
                Event::Mouse(mouse::Event::CursorMoved { position }),
                Some(Drag {
                    source: DragSource::Mouse,
                    ..
                }),
            ) => Some(*position),
            (
                Event::Touch(touch::Event::FingerMoved { id, position }),
                Some(Drag {
                    source: DragSource::Touch(active),
                    ..
                }),
            ) if *id == active => Some(*position),
            _ => None,
        };

        if let (Some(point), Some(drag)) = (movement, dragging) {
            let value = value_from_point(point, bounds, self.spec, self.orientation, self.reversed);
            let next = set_thumb(&self.values, drag.thumb, value, self.spec);

            if next != self.values {
                shell.publish((self.on_change)(next));
            }
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        let released = matches!(
            (event, dragging),
            (
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                Some(Drag {
                    source: DragSource::Mouse,
                    ..
                })
            )
        ) || matches!(
            (event, dragging),
            (
                Event::Touch(touch::Event::FingerLifted { id, .. })
                    | Event::Touch(touch::Event::FingerLost { id, .. }),
                Some(Drag { source: DragSource::Touch(active), .. })
            ) if *id == active
        );

        if released {
            tree.state.downcast_mut::<State>().dragging = None;
            shell.capture_event();
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if self.disabled || !cursor.is_over(layout.bounds()) {
            mouse::Interaction::None
        } else if tree.state.downcast_ref::<State>().dragging.is_some() {
            mouse::Interaction::Grabbing
        } else {
            mouse::Interaction::Grab
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let geometry = track_geometry(
            layout.bounds(),
            &self.values,
            self.spec,
            self.orientation,
            self.reversed,
        );
        let border = Border {
            radius: 999.0.into(),
            ..Default::default()
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: geometry.track,
                border,
                ..renderer::Quad::default()
            },
            Background::Color(self.visual_style.track),
        );
        if geometry.range.width > 0.0 && geometry.range.height > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: geometry.range,
                    border,
                    ..renderer::Quad::default()
                },
                Background::Color(self.visual_style.range),
            );
        }

        // Focused thumbs are drawn last so equal-valued range handles stay distinguishable.
        for focused_pass in [false, true] {
            for ((thumb, tree), layout) in self
                .thumbs
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
            {
                let focused = tree
                    .state
                    .downcast_ref::<focus_control::State>()
                    .is_focused();
                if focused == focused_pass {
                    thumb.as_widget().draw(
                        tree,
                        renderer,
                        theme,
                        renderer_style,
                        layout,
                        cursor,
                        viewport,
                    );
                }
            }
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
            &mut self.thumbs,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

fn focused_thumb(children: &[widget::Tree]) -> Option<usize> {
    children.iter().position(|tree| {
        tree.state
            .downcast_ref::<focus_control::State>()
            .is_focused()
    })
}

fn focus_thumb(children: &mut [widget::Tree], target: Option<usize>) {
    children.iter_mut().enumerate().for_each(|(index, tree)| {
        let state = tree.state.downcast_mut::<focus_control::State>();
        if target == Some(index) {
            state.focus();
        } else {
            state.unfocus();
        }
    });
}

fn nearest_thumb(values: &[f32], value: f32, active: usize) -> usize {
    values
        .iter()
        .enumerate()
        .min_by(|(left_index, left), (right_index, right)| {
            let left_distance = (**left - value).abs();
            let right_distance = (**right - value).abs();
            left_distance.total_cmp(&right_distance).then_with(|| {
                let left_active = *left_index == active;
                let right_active = *right_index == active;
                right_active.cmp(&left_active)
            })
        })
        .map_or(0, |(index, _)| index)
}

fn normalized(value: f32, spec: SliderSpec) -> f32 {
    if spec.min == spec.max {
        0.0
    } else {
        ((value - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0)
    }
}

fn axis_fraction(
    value: f32,
    spec: SliderSpec,
    orientation: SliderOrientation,
    reversed: bool,
) -> f32 {
    let fraction = normalized(value, spec);
    match (orientation, reversed) {
        (SliderOrientation::Horizontal, false) | (SliderOrientation::Vertical, true) => fraction,
        (SliderOrientation::Horizontal, true) | (SliderOrientation::Vertical, false) => {
            1.0 - fraction
        }
    }
}

fn point_for_value(
    value: f32,
    bounds: Rectangle,
    spec: SliderSpec,
    orientation: SliderOrientation,
    reversed: bool,
) -> Point {
    let fraction = axis_fraction(value, spec, orientation, reversed);
    match orientation {
        SliderOrientation::Horizontal => Point::new(
            bounds.x + THUMB_DIAMETER / 2.0 + fraction * (bounds.width - THUMB_DIAMETER).max(0.0),
            bounds.center_y(),
        ),
        SliderOrientation::Vertical => Point::new(
            bounds.center_x(),
            bounds.y + THUMB_DIAMETER / 2.0 + fraction * (bounds.height - THUMB_DIAMETER).max(0.0),
        ),
    }
}

fn value_from_point(
    point: Point,
    bounds: Rectangle,
    spec: SliderSpec,
    orientation: SliderOrientation,
    reversed: bool,
) -> f32 {
    let (position, start, span) = match orientation {
        SliderOrientation::Horizontal => (
            point.x,
            bounds.x + THUMB_DIAMETER / 2.0,
            (bounds.width - THUMB_DIAMETER).max(0.0),
        ),
        SliderOrientation::Vertical => (
            point.y,
            bounds.y + THUMB_DIAMETER / 2.0,
            (bounds.height - THUMB_DIAMETER).max(0.0),
        ),
    };
    let physical = if span == 0.0 {
        0.0
    } else {
        ((position - start) / span).clamp(0.0, 1.0)
    };
    let logical = match (orientation, reversed) {
        (SliderOrientation::Horizontal, false) | (SliderOrientation::Vertical, true) => physical,
        (SliderOrientation::Horizontal, true) | (SliderOrientation::Vertical, false) => {
            1.0 - physical
        }
    };
    snap(spec.min + logical * (spec.max - spec.min), spec)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TrackGeometry {
    track: Rectangle,
    range: Rectangle,
}

fn track_geometry(
    bounds: Rectangle,
    values: &[f32],
    spec: SliderSpec,
    orientation: SliderOrientation,
    reversed: bool,
) -> TrackGeometry {
    let first = point_for_value(values[0], bounds, spec, orientation, reversed);
    let last = point_for_value(
        *values.last().expect("normalized sliders have a thumb"),
        bounds,
        spec,
        orientation,
        reversed,
    );
    let origin = point_for_value(spec.min, bounds, spec, orientation, reversed);
    let (range_start, range_end) = if values.len() == 1 {
        match orientation {
            SliderOrientation::Horizontal => (origin.x.min(first.x), origin.x.max(first.x)),
            SliderOrientation::Vertical => (origin.y.min(first.y), origin.y.max(first.y)),
        }
    } else {
        match orientation {
            SliderOrientation::Horizontal => (first.x.min(last.x), first.x.max(last.x)),
            SliderOrientation::Vertical => (first.y.min(last.y), first.y.max(last.y)),
        }
    };

    match orientation {
        SliderOrientation::Horizontal => {
            let start = bounds.x + THUMB_DIAMETER / 2.0;
            TrackGeometry {
                track: Rectangle {
                    x: start,
                    y: bounds.center_y() - TRACK_THICKNESS / 2.0,
                    width: (bounds.width - THUMB_DIAMETER).max(0.0),
                    height: TRACK_THICKNESS,
                },
                range: Rectangle {
                    x: range_start,
                    y: bounds.center_y() - TRACK_THICKNESS / 2.0,
                    width: range_end - range_start,
                    height: TRACK_THICKNESS,
                },
            }
        }
        SliderOrientation::Vertical => {
            let start = bounds.y + THUMB_DIAMETER / 2.0;
            TrackGeometry {
                track: Rectangle {
                    x: bounds.center_x() - TRACK_THICKNESS / 2.0,
                    y: start,
                    width: TRACK_THICKNESS,
                    height: (bounds.height - THUMB_DIAMETER).max(0.0),
                },
                range: Rectangle {
                    x: bounds.center_x() - TRACK_THICKNESS / 2.0,
                    y: range_start,
                    width: TRACK_THICKNESS,
                    height: range_end - range_start,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{DARK, LIGHT};

    fn spec() -> SliderSpec {
        SliderSpec::new(0.0..=100.0, 5.0)
    }

    #[test]
    fn values_snap_clamp_sort_and_do_not_cross() {
        assert_eq!(
            normalize_values(vec![102.0, f32::NAN, 48.0], spec()),
            vec![0.0, 50.0, 100.0]
        );
        assert_eq!(set_thumb(&[20.0, 80.0], 0, 95.0, spec()), vec![80.0, 80.0]);
        assert_eq!(set_thumb(&[20.0, 80.0], 1, 5.0, spec()), vec![20.0, 20.0]);
        assert_eq!(normalize_values(Vec::new(), spec()), vec![0.0]);
        assert_eq!(
            reduce_thumb(
                &[90.0],
                0,
                SliderCommand::Maximum,
                SliderSpec::new(0.0..=100.0, 30.0)
            ),
            vec![100.0]
        );
    }

    #[test]
    fn invalid_numeric_inputs_have_boring_safe_defaults() {
        let reversed = SliderSpec::new(100.0..=0.0, -1.0).page_step(f32::NAN);
        assert_eq!(
            (reversed.min, reversed.max, reversed.step),
            (0.0, 100.0, 1.0)
        );
        assert_eq!(snap(f32::NEG_INFINITY, reversed), 0.0);
        assert_eq!(snap(f32::INFINITY, reversed), 100.0);

        let non_finite = SliderSpec::new(f32::NAN..=10.0, 2.0);
        assert_eq!((non_finite.min, non_finite.max), (0.0, 100.0));
    }

    #[test]
    fn keyboard_math_respects_steps_pages_edges_and_neighbors() {
        let spec = spec().page_step(25.0);
        assert_eq!(
            reduce_thumb(&[20.0, 80.0], 0, SliderCommand::Increment, spec),
            vec![25.0, 80.0]
        );
        assert_eq!(
            reduce_thumb(&[20.0, 80.0], 1, SliderCommand::PageDecrement, spec),
            vec![20.0, 55.0]
        );
        assert_eq!(
            reduce_thumb(&[20.0, 80.0], 0, SliderCommand::Maximum, spec),
            vec![80.0, 80.0]
        );
        assert_eq!(
            reduce_thumb(&[20.0, 80.0], 1, SliderCommand::Minimum, spec),
            vec![20.0, 20.0]
        );
    }

    #[test]
    fn key_mapping_follows_axis_and_reversal() {
        let right = keyboard::Key::Named(Named::ArrowRight);
        let up = keyboard::Key::Named(Named::ArrowUp);
        let page_up = keyboard::Key::Named(Named::PageUp);

        assert_eq!(
            keyboard_command(&right, SliderOrientation::Horizontal, false),
            Some(SliderCommand::Increment)
        );
        assert_eq!(
            keyboard_command(&right, SliderOrientation::Horizontal, true),
            Some(SliderCommand::Decrement)
        );
        assert_eq!(
            keyboard_command(&right, SliderOrientation::Vertical, false),
            None
        );
        assert_eq!(
            keyboard_command(&up, SliderOrientation::Vertical, false),
            Some(SliderCommand::Increment)
        );
        assert_eq!(
            keyboard_command(&up, SliderOrientation::Vertical, true),
            Some(SliderCommand::Decrement)
        );
        assert_eq!(
            keyboard_command(&page_up, SliderOrientation::Horizontal, true),
            Some(SliderCommand::PageIncrement)
        );
    }

    #[test]
    fn horizontal_vertical_and_reversed_geometry_stays_centered() {
        let horizontal = Rectangle::new(Point::ORIGIN, Size::new(200.0, 32.0));
        let geometry = track_geometry(
            horizontal,
            &[50.0],
            spec(),
            SliderOrientation::Horizontal,
            false,
        );
        assert_eq!(
            geometry.track,
            Rectangle {
                x: 8.0,
                y: 14.0,
                width: 184.0,
                height: 4.0
            }
        );
        assert_eq!(
            geometry.range,
            Rectangle {
                x: 8.0,
                y: 14.0,
                width: 92.0,
                height: 4.0
            }
        );
        let reversed = track_geometry(
            horizontal,
            &[50.0],
            spec(),
            SliderOrientation::Horizontal,
            true,
        );
        assert_eq!(
            reversed.range,
            Rectangle {
                x: 100.0,
                y: 14.0,
                width: 92.0,
                height: 4.0
            }
        );

        let vertical = Rectangle::new(Point::ORIGIN, Size::new(32.0, 200.0));
        let geometry = track_geometry(
            vertical,
            &[25.0],
            spec(),
            SliderOrientation::Vertical,
            false,
        );
        assert_eq!(
            geometry.track,
            Rectangle {
                x: 14.0,
                y: 8.0,
                width: 4.0,
                height: 184.0
            }
        );
        assert_eq!(
            geometry.range,
            Rectangle {
                x: 14.0,
                y: 146.0,
                width: 4.0,
                height: 46.0
            }
        );
    }

    #[test]
    fn pointer_mapping_and_overlap_choice_are_deterministic() {
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(200.0, 32.0));
        assert_eq!(
            value_from_point(
                Point::new(100.0, 16.0),
                bounds,
                spec(),
                SliderOrientation::Horizontal,
                false
            ),
            50.0
        );
        assert_eq!(
            value_from_point(
                Point::new(100.0, 16.0),
                bounds,
                spec(),
                SliderOrientation::Horizontal,
                true
            ),
            50.0
        );
        assert_eq!(nearest_thumb(&[50.0, 50.0], 50.0, 1), 1);
        assert_eq!(nearest_thumb(&[20.0, 80.0], 30.0, 1), 0);
    }

    #[test]
    fn semantic_styles_hold_in_light_and_dark() {
        for theme in [LIGHT, DARK] {
            let normal = style(&theme, false, false);
            let disabled = style(&theme, true, false);
            let invalid = style(&theme, false, true);
            assert_eq!(normal.range, theme.palette.primary);
            assert!(disabled.range.a < normal.range.a);
            assert_eq!(invalid.range, theme.palette.destructive);
            assert_eq!(invalid.focus_ring, theme.palette.destructive);
        }
    }

    #[test]
    fn multi_thumb_and_vertical_sliders_build_focusable_children() {
        let horizontal: Element<'_, Vec<f32>> = slider(
            "range",
            vec![20.0, 80.0],
            0.0..=100.0,
            1.0,
            |values| values,
            &LIGHT,
        )
        .into();
        let vertical: Element<'_, Vec<f32>> = slider(
            "volume",
            vec![50.0],
            0.0..=100.0,
            1.0,
            |values| values,
            &DARK,
        )
        .orientation(SliderOrientation::Vertical)
        .reversed(true)
        .invalid(true)
        .into();

        assert_eq!(horizontal.as_widget().children().len(), 2);
        assert_eq!(vertical.as_widget().children().len(), 1);
    }
}
