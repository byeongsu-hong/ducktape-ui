//! View-as-data: the template renderer.
//!
//! An Ice view compiles to two halves. The static half — widget structure,
//! literals, style tables, accessibility segments — is this module's [`Node`]
//! tree, published as data rather than Rust. The dynamic half — anything that
//! reads application state or names a message — stays compiled, and reaches
//! the renderer as a positional [`Slot`] table the generated `__view` fills in
//! each frame.
//!
//! The split is what makes a running app reloadable: replacing the [`Node`]
//! tree needs no compiler as long as the slot table still satisfies it.
//!
//! The modelled vocabulary is layouts, containers, text, inputs, and buttons.
//! Anything else becomes a [`Node::Subtree`] hole that the compiler fills
//! through the slot table, so an unmodelled construct costs only its own
//! subtree its reloadability rather than costing the whole view its template.

use std::borrow::Cow;

use iced::{
    Background, Color, Element, Length, Padding,
    alignment::{Horizontal, Vertical},
    widget,
};
use serde::{Deserialize, Serialize};

use crate::{Role, StableId, accessible, bounded_fill_element, bounded_padding, bounded_spacing};

/// The element type generated applications render into.
pub type IceElement<'a, Message> = Element<'a, Message, iced::Theme, iced::Renderer>;

/// The dynamic half of a view, evaluated fresh by generated code each frame.
///
/// Slots are positional: a template's `Value::Slot(i)` names index `i` of the
/// table the caller passes to [`render`].
pub enum Slot<'a, Message> {
    /// A string computed this frame — `self.count.to_string()` and friends.
    /// Cloned into the widget, so it need not outlive the call.
    Owned(String),
    /// A string borrowed straight out of application state. Text inputs need
    /// this: iced borrows the edited value for the element's lifetime.
    Borrowed(&'a str),
    /// A message value delivered on activation.
    Message(Message),
    /// A message constructor for widgets that report a value, such as the
    /// two-way binding behind `<->`.
    TextHandler(fn(String) -> Message),
    /// A whole subtree the compiler built, for the constructs the template
    /// vocabulary does not model — control flow, components, native surfaces.
    ///
    /// Taken rather than cloned: an element is not `Clone`, and a template
    /// references each subtree exactly once. A stale template that names one
    /// twice gets an empty second reading rather than a panic.
    Subtree(std::cell::RefCell<Option<IceElement<'a, Message>>>),
}

impl<'a, Message> Slot<'a, Message> {
    /// Wraps a compiled subtree for the slot table.
    pub fn subtree(element: impl Into<IceElement<'a, Message>>) -> Self {
        Self::Subtree(std::cell::RefCell::new(Some(element.into())))
    }
}

impl<Message> Slot<'_, Message> {
    fn as_str(&self) -> &str {
        match self {
            Self::Owned(value) => value,
            Self::Borrowed(value) => value,
            Self::Message(_) | Self::TextHandler(_) | Self::Subtree(_) => "",
        }
    }
}

/// A string that is either baked into the template or supplied by a slot.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Value {
    Literal(String),
    Slot(usize),
}

impl Value {
    /// Resolves to an owned string. Literals are cloned out of the template so
    /// the returned element never borrows it, which is what lets a reload
    /// swap the template while the previous frame's element is still alive.
    fn resolve<Message>(&self, slots: &[Slot<'_, Message>]) -> String {
        match self {
            Self::Literal(value) => value.clone(),
            Self::Slot(index) => slots.get(*index).map(Slot::as_str).unwrap_or("").to_owned(),
        }
    }
}

/// A color drawn from the app's compiled palette, optionally faded.
///
/// Palettes stay compiled: they are fixed-size arrays whose type changes with
/// the token list, so the template refers to them by index the same way the
/// inline path does.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct ColorRef {
    pub index: usize,
    #[serde(default)]
    pub alpha: Option<f32>,
}

impl ColorRef {
    fn resolve(&self, palette: &[Color]) -> Color {
        let mut color = palette.get(self.index).copied().unwrap_or(Color::BLACK);
        if let Some(alpha) = self.alpha {
            color.a = alpha;
        }
        color
    }
}

/// A width or height. Mirrors the `w=`/`h=` vocabulary.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Size {
    Fill,
    Shrink,
    Fixed(f32),
}

impl From<Size> for Length {
    fn from(size: Size) -> Self {
        match size {
            Size::Fill => Length::Fill,
            Size::Shrink => Length::Shrink,
            Size::Fixed(value) => Length::Fixed(value),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlignX {
    Left,
    Center,
    Right,
}

impl From<AlignX> for Horizontal {
    fn from(align: AlignX) -> Self {
        match align {
            AlignX::Left => Horizontal::Left,
            AlignX::Center => Horizontal::Center,
            AlignX::Right => Horizontal::Right,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlignY {
    Top,
    Center,
    Bottom,
}

impl From<AlignY> for Vertical {
    fn from(align: AlignY) -> Self {
        match align {
            AlignY::Top => Vertical::Top,
            AlignY::Center => Vertical::Center,
            AlignY::Bottom => Vertical::Bottom,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Edges {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl From<Edges> for Padding {
    fn from(edges: Edges) -> Self {
        bounded_padding(edges.top, edges.right, edges.bottom, edges.left)
    }
}

/// The visual properties a button status can override.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ButtonFace {
    #[serde(default)]
    pub background: Option<ColorRef>,
    #[serde(default)]
    pub text_color: Option<ColorRef>,
    #[serde(default)]
    pub radius: Option<f32>,
}

/// Per-status button styling, flattened the same way the inline path flattens
/// `active`/`hovered`/`pressed` into one closure.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ButtonStyle {
    #[serde(default)]
    pub active: ButtonFace,
    #[serde(default)]
    pub hovered: Option<ButtonFace>,
    #[serde(default)]
    pub pressed: Option<ButtonFace>,
}

/// Which direction a linear layout stacks, and how a single child fills it.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    Column,
    Row,
}

/// The accessibility identity of a node.
///
/// `segment` is appended to the parent's path at render time rather than
/// stored whole, so a subtree keeps its identity when an ancestor moves.
///
/// Only an author's `#name` opens a scope for descendants. An unnamed node
/// still gets a path of its own, built from its source line, but its children
/// hang off the nearest named ancestor — so inserting a bare wrapper around a
/// widget does not rename every selector beneath it.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct A11y {
    pub segment: String,
    #[serde(default)]
    pub named: bool,
    /// Where this node was written, so a rendered widget can still be traced
    /// back to its `.ice` line the way the compiled path traces it.
    ///
    /// The file is an index into the caller's path table rather than a string:
    /// paths must be `&'static str`, and a reload therefore cannot introduce a
    /// file the compiled binary does not already name.
    #[serde(default)]
    pub source: Option<Source>,
}

/// A `.ice` coordinate, with its file named indirectly.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct Source {
    pub path: usize,
    pub line: usize,
    pub column: usize,
}

impl Source {
    fn location(&self, paths: &[&'static str]) -> Option<crate::testing::Location> {
        Some(crate::testing::Location::new(
            paths.get(self.path)?,
            self.line,
            self.column,
            "rendered view node",
        ))
    }
}

impl A11y {
    /// This node's own accessibility path.
    fn key(&self, parent: &str) -> String {
        format!("{parent}/{}", self.segment)
    }

    /// The path descendants hang off.
    fn scope<'a>(&self, parent: &'a str, key: &'a str) -> &'a str {
        if self.named { key } else { parent }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Node {
    Container {
        a11y: A11y,
        #[serde(default)]
        width: Option<Size>,
        #[serde(default)]
        height: Option<Size>,
        #[serde(default)]
        padding: Option<Edges>,
        #[serde(default)]
        align_x: Option<AlignX>,
        #[serde(default)]
        align_y: Option<AlignY>,
        #[serde(default)]
        background: Option<ColorRef>,
        content: Box<Node>,
    },
    Linear {
        a11y: A11y,
        axis: Axis,
        #[serde(default)]
        spacing: Option<f64>,
        #[serde(default)]
        width: Option<Size>,
        #[serde(default)]
        height: Option<Size>,
        #[serde(default)]
        align_x: Option<AlignX>,
        #[serde(default)]
        align_y: Option<AlignY>,
        children: Vec<Node>,
    },
    Text {
        a11y: A11y,
        value: Value,
        #[serde(default)]
        size: Option<f32>,
        #[serde(default)]
        color: Option<ColorRef>,
    },
    Input {
        a11y: A11y,
        label: String,
        /// Slot holding the current value; must be [`Slot::Borrowed`].
        value: usize,
        /// Slot holding the `fn(String) -> Message` the edit routes through.
        on_input: usize,
        #[serde(default)]
        width: Option<Size>,
        #[serde(default)]
        secure: bool,
    },
    Button {
        a11y: A11y,
        label: String,
        /// Slot holding the message an activation delivers.
        on_press: usize,
        #[serde(default)]
        style: ButtonStyle,
    },
    /// A hole the compiler fills, holding a construct the template vocabulary
    /// does not model. Everything around it still reloads; what is inside
    /// changes only when the binary does.
    Subtree { slot: usize },
}

/// Stands in for a compiled subtree, which composes its own accessibility path
/// and pushes its own source location.
static EMPTY_A11Y: A11y = A11y {
    segment: String::new(),
    named: false,
    source: None,
};

impl Node {
    fn a11y(&self) -> &A11y {
        match self {
            Self::Container { a11y, .. }
            | Self::Linear { a11y, .. }
            | Self::Text { a11y, .. }
            | Self::Input { a11y, .. }
            | Self::Button { a11y, .. } => a11y,
            // A compiled subtree carries its own identity and provenance.
            Self::Subtree { .. } => &EMPTY_A11Y,
        }
    }
}

/// A whole view: the node tree plus the slot count it expects.
///
/// `slots` is the compatibility contract. A running process can accept any
/// template whose slot count and kinds its compiled `__view` still satisfies;
/// anything else needs a rebuild.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Template {
    pub root: Node,
    pub slots: usize,
}

impl Template {
    /// Parses a template from the JSON codegen publishes.
    pub fn from_json(source: &str) -> Result<Self, String> {
        serde_json::from_str(source).map_err(|error| error.to_string())
    }

    /// Serializes a template to the JSON form codegen publishes.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }
}

/// Renders a template against this frame's slot table.
///
/// The returned element borrows only from the slots that borrow application
/// state; template strings are cloned. That is deliberate — it keeps the
/// element independent of the template, so the next reload can drop the tree
/// this frame was built from.
pub fn render<'a, Message>(
    template: &Template,
    slots: &[Slot<'a, Message>],
    palette: &[Color],
    root_scope: &str,
    paths: &[&'static str],
) -> IceElement<'a, Message>
where
    Message: Clone + 'static,
{
    render_node(&template.root, slots, palette, root_scope, paths)
}

fn render_node<'a, Message>(
    node: &Node,
    slots: &[Slot<'a, Message>],
    palette: &[Color],
    parent_key: &str,
    paths: &[&'static str],
) -> IceElement<'a, Message>
where
    Message: Clone + 'static,
{
    // Held for the whole of this node's construction, so every widget the
    // accessibility layer records below inherits this `.ice` coordinate.
    let _source = node
        .a11y()
        .source
        .and_then(|source| source.location(paths))
        .map(crate::testing::push_render_source);
    match node {
        Node::Container {
            a11y,
            width,
            height,
            padding,
            align_x,
            align_y,
            background,
            content,
        } => {
            let key = a11y.key(parent_key);
            let inner = render_node(content, slots, palette, a11y.scope(parent_key, &key), paths);
            let mut container = widget::container(inner).id(widget::Id::from(key.clone()));
            if let Some(padding) = padding {
                container = container.padding(Padding::from(*padding));
            }
            if let Some(width) = width {
                container = container.width(Length::from(*width));
            }
            if let Some(height) = height {
                container = container.height(Length::from(*height));
            }
            if let Some(align) = align_x {
                container = container.align_x(Horizontal::from(*align));
            }
            if let Some(align) = align_y {
                container = container.align_y(Vertical::from(*align));
            }
            if let Some(background) = background {
                let color = background.resolve(palette);
                container = container.style(move |_theme| widget::container::Style {
                    background: Some(Background::Color(color)),
                    ..widget::container::Style::default()
                });
            }
            accessible(container, StableId::new(&key), Role::GenericContainer)
                .logical_id(key)
                .into()
        }
        Node::Linear {
            a11y,
            axis,
            spacing,
            width,
            height,
            align_x,
            align_y,
            children,
        } => {
            let key = a11y.key(parent_key);
            let scope = a11y.scope(parent_key, &key);
            let horizontal = matches!(axis, Axis::Row);
            let count = children.len();
            let rendered = children
                .iter()
                .map(|child| {
                    bounded_fill_element(
                        render_node(child, slots, palette, scope, paths),
                        count,
                        horizontal,
                    )
                })
                .collect::<Vec<_>>();
            let spacing = bounded_spacing(spacing.unwrap_or(0.0), count);
            let layout: IceElement<'a, Message> = match axis {
                Axis::Column => {
                    let mut column = widget::column(rendered).spacing(spacing);
                    if let Some(width) = width {
                        column = column.width(Length::from(*width));
                    }
                    if let Some(height) = height {
                        column = column.height(Length::from(*height));
                    }
                    if let Some(align) = align_x {
                        column = column.align_x(Horizontal::from(*align));
                    }
                    column.into()
                }
                Axis::Row => {
                    let mut row = widget::row(rendered).spacing(spacing);
                    if let Some(width) = width {
                        row = row.width(Length::from(*width));
                    }
                    if let Some(height) = height {
                        row = row.height(Length::from(*height));
                    }
                    if let Some(align) = align_y {
                        row = row.align_y(Vertical::from(*align));
                    }
                    row.into()
                }
            };
            accessible(
                widget::container(layout),
                StableId::new(&key),
                Role::GenericContainer,
            )
            .logical_id(key)
            .into()
        }
        Node::Text {
            a11y,
            value,
            size,
            color,
        } => {
            let key = a11y.key(parent_key);
            let resolved = value.resolve(slots);
            let mut text = widget::text(resolved.clone());
            if let Some(size) = size {
                text = text.size(size.clamp(f32::EPSILON, f32::MAX));
            }
            if let Some(color) = color {
                text = text.color(color.resolve(palette));
            }
            accessible(
                crate::selectable_text(text),
                StableId::new(&key),
                Role::Label,
            )
            .logical_id(key)
            .value(resolved)
            .into()
        }
        Node::Input {
            a11y,
            label,
            value,
            on_input,
            width,
            secure,
        } => {
            let key = a11y.key(parent_key);
            let id = StableId::new(&key);
            let secure = *secure;
            let role = if secure {
                Role::PasswordInput
            } else {
                Role::TextInput
            };
            let current = match slots.get(*value) {
                Some(Slot::Borrowed(value)) => *value,
                // An input's value must borrow state so iced can edit it in
                // place; an owned slot means codegen mis-emitted this node.
                _ => "",
            };
            let handler = match slots.get(*on_input) {
                Some(Slot::TextHandler(handler)) => Some(*handler),
                _ => None,
            };
            let mut input = widget::text_input("", current)
                .id(widget::Id::from(key.clone()))
                .secure(secure);
            if let Some(width) = width {
                input = input.width(Length::from(*width));
            }
            let input = accessible(input.on_input_maybe(handler), id, role)
                .logical_id(key.clone())
                .focus_id(widget::Id::from(key))
                .label(label.clone())
                .value_maybe((!secure).then(|| current.to_owned()))
                .disabled(false);
            widget::column![widget::text(label.clone()), input]
                .spacing(6)
                .into()
        }
        Node::Subtree { slot } => match slots.get(*slot) {
            Some(Slot::Subtree(cell)) => cell
                .borrow_mut()
                .take()
                .unwrap_or_else(|| widget::Space::new().into()),
            _ => widget::Space::new().into(),
        },
        Node::Button {
            a11y,
            label,
            on_press,
            style,
        } => {
            let key = a11y.key(parent_key);
            let id = StableId::new(&key);
            let activate = match slots.get(*on_press) {
                Some(Slot::Message(message)) => Some(message.clone()),
                _ => None,
            };
            let content: IceElement<'a, Message> = widget::text(label.clone()).into();
            let style = resolve_button_style(style, palette);
            let button = widget::button(content)
                .on_press_maybe(activate.clone())
                .style(move |theme, status| style.apply(theme, status));
            accessible(button, id, Role::Button)
                .logical_id(key.clone())
                .focus_id(widget::Id::from(key))
                .label(label.clone())
                .disabled(false)
                .on_activate_maybe(activate)
                .into()
        }
    }
}

/// A button style with its palette lookups already performed, so the draw-time
/// closure copies plain colors instead of borrowing the template.
#[derive(Clone, Copy)]
struct ResolvedButtonStyle {
    active: ResolvedFace,
    hovered: Option<ResolvedFace>,
    pressed: Option<ResolvedFace>,
}

#[derive(Clone, Copy, Default)]
struct ResolvedFace {
    background: Option<Color>,
    text_color: Option<Color>,
    radius: Option<f32>,
}

fn resolve_face(face: &ButtonFace, palette: &[Color]) -> ResolvedFace {
    ResolvedFace {
        background: face.background.map(|color| color.resolve(palette)),
        text_color: face.text_color.map(|color| color.resolve(palette)),
        radius: face.radius,
    }
}

fn resolve_button_style(style: &ButtonStyle, palette: &[Color]) -> ResolvedButtonStyle {
    ResolvedButtonStyle {
        active: resolve_face(&style.active, palette),
        hovered: style
            .hovered
            .as_ref()
            .map(|face| resolve_face(face, palette)),
        pressed: style
            .pressed
            .as_ref()
            .map(|face| resolve_face(face, palette)),
    }
}

impl ResolvedButtonStyle {
    fn apply(&self, theme: &iced::Theme, status: widget::button::Status) -> widget::button::Style {
        let mut style = widget::button::primary(theme, status);
        self.active.apply_to(&mut style);
        match status {
            widget::button::Status::Hovered => {
                if let Some(face) = self.hovered {
                    face.apply_to(&mut style);
                }
            }
            widget::button::Status::Pressed => {
                if let Some(face) = self.pressed {
                    face.apply_to(&mut style);
                }
            }
            _ => {}
        }
        style
    }
}

impl ResolvedFace {
    fn apply_to(&self, style: &mut widget::button::Style) {
        if let Some(background) = self.background {
            style.background = Some(Background::Color(background));
        }
        if let Some(text_color) = self.text_color {
            style.text_color = text_color;
        }
        if let Some(radius) = self.radius {
            let radius = radius.clamp(0.0, f32::MAX);
            style.border.radius = iced::border::Radius {
                top_left: radius,
                top_right: radius,
                bottom_right: radius,
                bottom_left: radius,
            };
        }
    }
}

/// Where a process reads its template from.
///
/// A release build carries the JSON codegen produced. `ICE_TEMPLATE_PATH`
/// overrides it with a file the dev runner rewrites, which is the entire
/// mechanism behind reloading a view without rebuilding.
pub struct TemplateSource {
    embedded: &'static str,
    path: Option<std::path::PathBuf>,
    current: std::cell::RefCell<Option<std::rc::Rc<Template>>>,
    stamp: std::cell::Cell<Option<std::time::SystemTime>>,
}

impl TemplateSource {
    /// Reads the compiled-in template, or the file `ICE_TEMPLATE_PATH` names
    /// when a dev runner has published one.
    pub fn new(embedded: &'static str) -> Self {
        Self::from_path(
            embedded,
            std::env::var_os("ICE_TEMPLATE_PATH").map(std::path::PathBuf::from),
        )
    }

    /// Reads a named template file, ignoring the environment.
    pub fn from_path(embedded: &'static str, path: Option<std::path::PathBuf>) -> Self {
        Self {
            embedded,
            path,
            current: std::cell::RefCell::new(None),
            stamp: std::cell::Cell::new(None),
        }
    }

    /// Returns the template to render this frame, picking up an edited file
    /// when one is newer than the last load.
    ///
    /// A file that fails to read or parse leaves the last good template in
    /// place: a half-written save during a reload must not blank the window.
    pub fn current(&self) -> std::rc::Rc<Template> {
        if let Some(path) = &self.path {
            let modified = std::fs::metadata(path)
                .and_then(|meta| meta.modified())
                .ok();
            if modified != self.stamp.get() || self.current.borrow().is_none() {
                if let Ok(source) = std::fs::read_to_string(path)
                    && let Ok(template) = Template::from_json(&source)
                {
                    self.stamp.set(modified);
                    let template = std::rc::Rc::new(template);
                    *self.current.borrow_mut() = Some(template.clone());
                    return template;
                }
                self.stamp.set(modified);
            }
        }
        if let Some(template) = self.current.borrow().clone() {
            return template;
        }
        let template = std::rc::Rc::new(
            Template::from_json(self.embedded)
                .expect("generated template JSON is written by codegen and always parses"),
        );
        *self.current.borrow_mut() = Some(template.clone());
        template
    }
}

/// Reports whether a compiled process can accept `candidate` without being
/// rebuilt: the slot table it fills each frame must still line up.
///
/// This is the whole reload decision. Structure, literals, colors, spacing and
/// accessibility segments may all change freely; the moment a view needs a
/// slot the binary does not have, only a rebuild can supply it.
pub fn accepts(compiled: &Template, candidate: &Template) -> bool {
    compiled.slots == candidate.slots && slot_uses(&candidate.root, compiled.slots)
}

fn slot_uses(node: &Node, available: usize) -> bool {
    let value_ok = |value: &Value| match value {
        Value::Literal(_) => true,
        Value::Slot(index) => *index < available,
    };
    match node {
        Node::Container { content, .. } => slot_uses(content, available),
        Node::Linear { children, .. } => children.iter().all(|child| slot_uses(child, available)),
        Node::Text { value, .. } => value_ok(value),
        Node::Input {
            value, on_input, ..
        } => *value < available && *on_input < available,
        Node::Button { on_press, .. } => *on_press < available,
        Node::Subtree { slot } => *slot < available,
    }
}

/// Borrowed form used when a caller wants to hand the renderer a template it
/// does not own.
pub type SharedTemplate = std::rc::Rc<Template>;

impl<'a, Message> From<&'a str> for Slot<'a, Message> {
    fn from(value: &'a str) -> Self {
        Self::Borrowed(value)
    }
}

impl<Message> From<String> for Slot<'_, Message> {
    fn from(value: String) -> Self {
        Self::Owned(value)
    }
}

/// Convenience for the common `Cow` shape codegen produces.
impl<'a, Message> From<Cow<'a, str>> for Slot<'a, Message> {
    fn from(value: Cow<'a, str>) -> Self {
        match value {
            Cow::Borrowed(value) => Self::Borrowed(value),
            Cow::Owned(value) => Self::Owned(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a11y(segment: &str) -> A11y {
        A11y {
            segment: segment.to_owned(),
            named: true,
            source: None,
        }
    }

    fn text_node(segment: &str, value: Value) -> Node {
        Node::Text {
            a11y: a11y(segment),
            value,
            size: None,
            color: None,
        }
    }

    fn template(root: Node, slots: usize) -> Template {
        Template { root, slots }
    }

    #[test]
    fn json_round_trips() {
        let original = template(
            Node::Linear {
                a11y: a11y("content"),
                axis: Axis::Column,
                spacing: Some(16.0),
                width: Some(Size::Fill),
                height: None,
                align_x: Some(AlignX::Center),
                align_y: None,
                children: vec![
                    text_node("title", Value::Literal("Ice".into())),
                    text_node("count", Value::Slot(0)),
                ],
            },
            1,
        );
        let json = original.to_json().expect("template serializes");
        assert_eq!(Template::from_json(&json).expect("parses"), original);
    }

    #[test]
    fn literals_and_slots_resolve() {
        let slots: Vec<Slot<'_, ()>> = vec![Slot::Owned("42".into())];
        assert_eq!(Value::Literal("Ice".into()).resolve(&slots), "Ice");
        assert_eq!(Value::Slot(0).resolve(&slots), "42");
        // An out-of-range slot renders empty rather than panicking: a stale
        // template must not take the window down mid-reload.
        assert_eq!(Value::Slot(7).resolve(&slots), "");
    }

    #[test]
    fn reload_accepts_only_a_satisfiable_slot_table() {
        let compiled = template(text_node("count", Value::Slot(0)), 1);

        // Restructuring and re-literalling the same slots is reloadable.
        let restructured = template(
            Node::Linear {
                a11y: a11y("content"),
                axis: Axis::Row,
                spacing: Some(8.0),
                width: None,
                height: None,
                align_x: None,
                align_y: None,
                children: vec![
                    text_node("label", Value::Literal("Total".into())),
                    text_node("count", Value::Slot(0)),
                ],
            },
            1,
        );
        assert!(accepts(&compiled, &restructured));

        // Needing a slot the binary does not fill requires a rebuild.
        let extra_slot = template(text_node("count", Value::Slot(1)), 2);
        assert!(!accepts(&compiled, &extra_slot));
    }

    #[test]
    fn only_named_nodes_open_a_scope_for_descendants() {
        let named = a11y("content");
        let key = named.key("Starter/app");
        assert_eq!(key, "Starter/app/content");
        assert_eq!(named.scope("Starter/app", &key), "Starter/app/content");

        // An unnamed wrapper still has a path, but its children keep hanging
        // off the nearest named ancestor.
        let unnamed = A11y {
            named: false,
            ..a11y("@layout:36")
        };
        let key = unnamed.key("Starter/app");
        assert_eq!(key, "Starter/app/@layout:36");
        assert_eq!(unnamed.scope("Starter/app", &key), "Starter/app");
    }

    #[test]
    fn palette_reference_applies_alpha() {
        let palette = [Color::from_rgba(0.2, 0.4, 0.6, 1.0)];
        let faded = ColorRef {
            index: 0,
            alpha: Some(0.5),
        }
        .resolve(&palette);
        assert_eq!(faded.a, 0.5);
        assert_eq!(faded.r, 0.2);
        // An index the palette does not have must not panic during a reload.
        let missing = ColorRef {
            index: 9,
            alpha: None,
        }
        .resolve(&palette);
        assert_eq!(missing, Color::BLACK);
    }

    #[test]
    fn hovered_face_cascades_over_active() {
        // `active` is the base every status starts from; a status face carries
        // only its overrides, so anything it omits keeps the base value. This
        // is why codegen can publish `hovered` with one property in it.
        let palette = [
            Color::from_rgb(0.1, 0.1, 0.1),
            Color::from_rgb(0.9, 0.9, 0.9),
            Color::from_rgb(0.0, 0.4, 1.0),
        ];
        let style = resolve_button_style(
            &ButtonStyle {
                active: ButtonFace {
                    background: Some(ColorRef {
                        index: 0,
                        alpha: None,
                    }),
                    text_color: Some(ColorRef {
                        index: 1,
                        alpha: None,
                    }),
                    radius: Some(8.0),
                },
                hovered: Some(ButtonFace {
                    text_color: Some(ColorRef {
                        index: 2,
                        alpha: None,
                    }),
                    ..ButtonFace::default()
                }),
                pressed: None,
            },
            &palette,
        );

        let theme = iced::Theme::Light;
        let active = style.apply(&theme, widget::button::Status::Active);
        let hovered = style.apply(&theme, widget::button::Status::Hovered);

        assert_eq!(active.text_color, palette[1]);
        assert_eq!(hovered.text_color, palette[2], "hovered overrides the base");
        assert_eq!(
            hovered.background, active.background,
            "what hovered omits keeps the base value"
        );
        assert_eq!(
            hovered.border.radius, active.border.radius,
            "the base radius survives the override"
        );

        // A status with no face of its own is entirely the base.
        let pressed = style.apply(&theme, widget::button::Status::Pressed);
        assert_eq!(pressed.text_color, active.text_color);
        assert_eq!(pressed.background, active.background);
    }
}
