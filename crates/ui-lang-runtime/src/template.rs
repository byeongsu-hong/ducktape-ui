//! View-as-data: the template renderer.
//!
//! An Ice view compiles to two halves. The static half — widget structure,
//! literals, style tables, accessibility segments — is the node tree defined in
//! `ui_lang_template` and published as data rather than Rust. The dynamic half
//! — anything that reads application state or names a message — stays compiled,
//! and reaches the renderer as the positional [`Slots`] tables the generated
//! `__view` fills in each frame.
//!
//! The split is what makes a running app reloadable: replacing the node tree
//! needs no compiler as long as the slot tables still satisfy it.
//!
//! This module is the reading half only. The format itself — and the
//! compatibility rule that decides whether an edited template can be accepted
//! without a rebuild — lives in `ui_lang_template`, so the generator that
//! writes a template and the runtime that renders it share one definition
//! rather than two that can drift apart. It is re-exported here, which is why
//! `ui_lang_runtime::template::Template` still names it.

use iced::{
    Background, Color, Element, Length, Padding,
    alignment::{Horizontal, Vertical},
    widget,
};

pub use ui_lang_template::*;

use crate::{Role, StableId, accessible, bounded_fill_element, bounded_padding, bounded_spacing};

/// The element type generated applications render into.
pub type IceElement<'a, Message> = Element<'a, Message, iced::Theme, iced::Renderer>;

/// The dynamic half of a view, evaluated fresh by generated code each frame.
///
/// One table per kind of value, each addressed by its own index type from
/// `ui_lang_template`. That is the whole point of the split: a node cannot
/// name a slot of the wrong kind, because the index it holds does not type as
/// anything else. When these were one table of a tagged enum, the renderer had
/// to check the tag and had nothing useful to do when it did not match — a
/// mis-emitted button silently lost its message and still drew.
///
/// What remains is out-of-range, which is not a codegen mistake but a stale
/// template mid-reload. Those still resolve to something harmless rather than
/// panicking: a half-written save must not take the window down.
pub struct Slots<'a, Message> {
    /// Strings computed this frame — `self.count.to_string()` and friends.
    /// Cloned into the widget, so they need not outlive the call.
    pub texts: Vec<String>,
    /// Strings borrowed straight out of application state. Text inputs need
    /// this: iced borrows the edited value for the element's lifetime, which
    /// is why it cannot read `texts`.
    pub states: Vec<&'a str>,
    /// Message values delivered on activation.
    pub messages: Vec<Message>,
    /// Message constructors for widgets that report a value.
    pub handlers: Vec<fn(String) -> Message>,
    /// Whole subtrees the compiler built, for the constructs the template
    /// vocabulary does not model — control flow, components, native surfaces.
    ///
    /// Taken rather than cloned: an element is not `Clone`, and a template
    /// references each subtree exactly once. A stale template that names one
    /// twice gets an empty second reading rather than a panic.
    pub subtrees: Vec<std::cell::RefCell<Option<IceElement<'a, Message>>>>,
}

impl<Message> Default for Slots<'_, Message> {
    fn default() -> Self {
        Self {
            texts: Vec::new(),
            states: Vec::new(),
            messages: Vec::new(),
            handlers: Vec::new(),
            subtrees: Vec::new(),
        }
    }
}

impl<'a, Message> Slots<'a, Message> {
    /// Allocates each table to the size the template declares, so filling them
    /// costs no reallocation on a frame.
    pub fn with_capacity(counts: SlotCounts) -> Self {
        Self {
            texts: Vec::with_capacity(counts.texts),
            states: Vec::with_capacity(counts.states),
            messages: Vec::with_capacity(counts.messages),
            handlers: Vec::with_capacity(counts.handlers),
            subtrees: Vec::with_capacity(counts.subtrees),
        }
    }

    /// Appends a compiled subtree to the hole table.
    pub fn push_subtree(&mut self, element: impl Into<IceElement<'a, Message>>) {
        self.subtrees
            .push(std::cell::RefCell::new(Some(element.into())));
    }
}

// The template's vocabulary, converted into iced's. These are free functions
// rather than `From` impls because both sides are foreign to this crate: the
// format belongs to `ui_lang_template` and the widget types to `iced`.

fn length(size: Size) -> Length {
    match size {
        Size::Fill => Length::Fill,
        Size::Shrink => Length::Shrink,
        Size::Fixed(value) => Length::Fixed(value),
    }
}

fn horizontal(align: AlignX) -> Horizontal {
    match align {
        AlignX::Left => Horizontal::Left,
        AlignX::Center => Horizontal::Center,
        AlignX::Right => Horizontal::Right,
    }
}

fn vertical(align: AlignY) -> Vertical {
    match align {
        AlignY::Top => Vertical::Top,
        AlignY::Center => Vertical::Center,
        AlignY::Bottom => Vertical::Bottom,
    }
}

fn edge_padding(edges: Edges) -> Padding {
    bounded_padding(edges.top, edges.right, edges.bottom, edges.left)
}

/// Resolves a template string to an owned one.
///
/// Literals are cloned out of the template so the returned element never
/// borrows it, which is what lets a reload swap the template while the previous
/// frame's element is still alive.
fn resolve_value<Message>(value: &Value, slots: &Slots<'_, Message>) -> String {
    match value {
        Value::Literal(value) => value.clone(),
        Value::Slot(TextSlot(index)) => slots.texts.get(*index).cloned().unwrap_or_default(),
    }
}

fn resolve_color(reference: ColorRef, palette: &[Color]) -> Color {
    let mut color = palette
        .get(reference.index)
        .copied()
        .unwrap_or(Color::BLACK);
    if let Some(alpha) = reference.alpha {
        color.a = alpha;
    }
    color
}

fn source_location(source: Source, paths: &[&'static str]) -> Option<crate::testing::Location> {
    Some(crate::testing::Location::new(
        paths.get(source.path)?,
        source.line,
        source.column,
        "rendered view node",
    ))
}

/// Renders a template against this frame's slot table.
///
/// The returned element borrows only from the slots that borrow application
/// state; template strings are cloned. That is deliberate — it keeps the
/// element independent of the template, so the next reload can drop the tree
/// this frame was built from.
pub fn render<'a, Message>(
    template: &Template,
    slots: &Slots<'a, Message>,
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
    slots: &Slots<'a, Message>,
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
        .and_then(|source| source_location(source, paths))
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
                container = container.padding(edge_padding(*padding));
            }
            if let Some(width) = width {
                container = container.width(length(*width));
            }
            if let Some(height) = height {
                container = container.height(length(*height));
            }
            if let Some(align) = align_x {
                container = container.align_x(horizontal(*align));
            }
            if let Some(align) = align_y {
                container = container.align_y(vertical(*align));
            }
            if let Some(background) = background {
                let color = resolve_color(*background, palette);
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
            let is_row = matches!(axis, Axis::Row);
            let count = children.len();
            let rendered = children
                .iter()
                .map(|child| {
                    bounded_fill_element(
                        render_node(child, slots, palette, scope, paths),
                        count,
                        is_row,
                    )
                })
                .collect::<Vec<_>>();
            let spacing = bounded_spacing(spacing.unwrap_or(0.0), count);
            let layout: IceElement<'a, Message> = match axis {
                Axis::Column => {
                    let mut column = widget::column(rendered).spacing(spacing);
                    if let Some(width) = width {
                        column = column.width(length(*width));
                    }
                    if let Some(height) = height {
                        column = column.height(length(*height));
                    }
                    if let Some(align) = align_x {
                        column = column.align_x(horizontal(*align));
                    }
                    column.into()
                }
                Axis::Row => {
                    let mut row = widget::row(rendered).spacing(spacing);
                    if let Some(width) = width {
                        row = row.width(length(*width));
                    }
                    if let Some(height) = height {
                        row = row.height(length(*height));
                    }
                    if let Some(align) = align_y {
                        row = row.align_y(vertical(*align));
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
            let resolved = resolve_value(value, slots);
            let mut text = widget::text(resolved.clone());
            if let Some(size) = size {
                text = text.size(size.clamp(f32::EPSILON, f32::MAX));
            }
            if let Some(color) = color {
                text = text.color(resolve_color(*color, palette));
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
            let StateSlot(value) = value;
            let HandlerSlot(on_input) = on_input;
            let current = slots.states.get(*value).copied().unwrap_or("");
            let handler = slots.handlers.get(*on_input).copied();
            let mut input = widget::text_input("", current)
                .id(widget::Id::from(key.clone()))
                .secure(secure);
            if let Some(width) = width {
                input = input.width(length(*width));
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
        Node::Subtree {
            slot: SubtreeSlot(slot),
        } => slots
            .subtrees
            .get(*slot)
            .and_then(|cell| cell.borrow_mut().take())
            .unwrap_or_else(|| widget::Space::new().into()),
        Node::Button {
            a11y,
            label,
            on_press,
            style,
        } => {
            let key = a11y.key(parent_key);
            let id = StableId::new(&key);
            let MessageSlot(on_press) = on_press;
            let activate = slots.messages.get(*on_press).cloned();
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
        background: face.background.map(|color| resolve_color(color, palette)),
        text_color: face.text_color.map(|color| resolve_color(color, palette)),
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

/// Emits an event whenever the published template file changes.
///
/// Without this a rewritten template would sit unread: iced rebuilds the view
/// only when something asks it to, and an idle window never asks.
///
/// The watch runs on a plain OS thread rather than a timer subscription, for
/// two reasons. `iced::time::every` exists only when the application enables
/// `tokio` or `smol`, and requiring that of every Ice application to support a
/// development feature is the wrong trade. And a thread can compare timestamps
/// itself, so it sends only when the file actually moves — an idle window
/// rebuilds its view zero times rather than once per tick.
///
/// The subscription is inert when no dev runner has set `ICE_TEMPLATE_PATH`,
/// so a release build spawns nothing and subscribes to nothing.
pub fn changes() -> iced::Subscription<()> {
    if std::env::var_os("ICE_TEMPLATE_PATH").is_none() {
        return iced::Subscription::none();
    }
    iced::Subscription::run(|| {
        let (mut sender, receiver) = iced::futures::channel::mpsc::channel(1);
        std::thread::spawn(move || {
            let Some(path) = std::env::var_os("ICE_TEMPLATE_PATH").map(std::path::PathBuf::from)
            else {
                return;
            };
            let stamp = |path: &std::path::Path| {
                std::fs::metadata(path)
                    .and_then(|meta| meta.modified())
                    .ok()
            };
            let mut seen = stamp(&path);
            loop {
                std::thread::sleep(std::time::Duration::from_millis(150));
                let current = stamp(&path);
                if current == seen {
                    continue;
                }
                seen = current;
                // A full channel means the previous change has not been
                // rendered yet, and a second notification would tell the view
                // nothing new. A closed one means the window is gone.
                match sender.try_send(()) {
                    Ok(()) => {}
                    Err(error) if error.is_full() => {}
                    Err(_) => return,
                }
            }
        });
        receiver
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_and_slots_resolve() {
        let slots = Slots::<()> {
            texts: vec!["42".into()],
            ..Slots::default()
        };
        assert_eq!(resolve_value(&Value::Literal("Ice".into()), &slots), "Ice");
        assert_eq!(resolve_value(&Value::Slot(TextSlot(0)), &slots), "42");
        // An out-of-range slot renders empty rather than panicking: a stale
        // template must not take the window down mid-reload.
        assert_eq!(resolve_value(&Value::Slot(TextSlot(7)), &slots), "");
    }

    #[test]
    fn palette_reference_applies_alpha() {
        let palette = [Color::from_rgba(0.2, 0.4, 0.6, 1.0)];
        let faded = resolve_color(
            ColorRef {
                index: 0,
                alpha: Some(0.5),
            },
            &palette,
        );
        assert_eq!(faded.a, 0.5);
        assert_eq!(faded.r, 0.2);
        // An index the palette does not have must not panic during a reload.
        let missing = resolve_color(
            ColorRef {
                index: 9,
                alpha: None,
            },
            &palette,
        );
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
