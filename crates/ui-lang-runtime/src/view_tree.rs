//! Renders the node tree a view module sends over `ui_lang_wire` with the
//! host's own widgets.
//!
//! The guest decided WHAT is on screen; everything about HOW is the host's:
//! layout, fonts, IME, the caret, scroll position, focus. A text field's
//! live value in particular lives here, in [`Inputs`] keyed by node key —
//! the guest only ever sees a whole-string [`wire::Event::Input`] after the
//! fact, and gets to overwrite the host's copy only by reporting a value
//! that differs from the one it reported last frame (its own handler
//! cleared or set the field). An editor's `text_editor::Content` lives
//! here the same way, and the guest hears its whole text as a
//! [`wire::Event::Edit`].
//!
//! The rendered element speaks [`Output`]; the host turns each one into the
//! wire event with [`Inputs::apply`] and hands it to the guest.
//!
//! A picture's bytes cross once (see [`wire::Node::Svg`]): [`Pictures`]
//! keeps every one a guest has sent, by its hash, for as long as the host
//! keeps the guest.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::text_editor;
use iced::{Background, Color, Element, Length, widget};
use ui_lang_wire as wire;

mod flex;
mod lists;

use crate::{Role, StableId, accessible, bounded_fill_element, bounded_padding, bounded_spacing};

mod memo;
mod qr;
mod rich_text;
mod text;
mod tooltip;
pub use text::register_font_family;
mod button;
mod canvas;
mod editor;
mod layers;
mod operations;
pub use operations::execute_widget_command;
#[cfg(feature = "markdown")]
mod markdown;
#[cfg(feature = "markdown")]
pub use markdown::markdown_surface;

pub type IceElement<'a, Message> = Element<'a, Message, iced::Theme, iced::Renderer>;

/// What the user did to a rendered tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    /// A native modal boundary consumed the event without a guest route.
    Ignore,
    /// A button was pressed or an input submitted: the guest's message
    /// table index the node carried.
    Activate(u32),
    /// A rich text link activated the guest's String handler.
    Link { handler: u32, text: String },
    /// A provider result; the renderer supplies the guest route, not the provider.
    Surface {
        handler: Option<u32>,
        value: wire::SurfaceValue,
    },
    /// An input's text changed. `key` is the node's key, `handler` the
    /// guest's input-handler table index, `text` the whole new value.
    Edit {
        key: String,
        handler: u32,
        text: String,
    },
    /// The user did `action` in an editor: a keystroke, a paste, a click,
    /// a caret move. The host performs it on the `Content` it holds and the
    /// guest hears the text, when it changed.
    EditorAction {
        key: String,
        handler: u32,
        action: text_editor::Action,
    },
    /// Accessibility asked the editor under `key` to move its caret.
    MoveCaret {
        key: String,
        line: usize,
        column: usize,
    },
    /// A checkbox or toggler was flipped to `on`.
    Toggle { handler: u32, on: bool },
    /// A slider moved to `value`.
    Slide { handler: u32, value: f32 },
    /// A pick list chose its option at `index`.
    Select { handler: u32, index: u32 },
    /// A sensor's child was shown at or resized to `width` by `height`.
    Size {
        handler: u32,
        width: f32,
        height: f32,
    },
    /// A left press landed at (`x`, `y`) inside a mouse area, in the area's
    /// own coordinates.
    Pointer { handler: u32, x: f32, y: f32 },
    /// The pointer moved to (`x`, `y`) inside a mouse area. Coalesced by
    /// [`Inputs::apply`]: one per handler per frame, the last position.
    Move { handler: u32, x: f32, y: f32 },
    /// Native scrollable offsets, measured from its configured anchors.
    ScrollOffset {
        handler: u32,
        x: f32,
        y: f32,
        relative_x: f32,
        relative_y: f32,
    },
    /// The wheel turned over a mouse area.
    Scroll {
        handler: u32,
        dx: f32,
        dy: f32,
        pixels: bool,
    },
}

#[derive(Clone, Debug)]
struct Field {
    /// What the host shows and edits.
    text: String,
    /// What the guest said the value was, last frame.
    reported: String,
}

/// What a [`wire::Node::Surface`] renders as: the host's own element for
/// the node's key and copied argument values. The host owns its state, clock
/// and redraws; the guest never sees inside.
pub type Surface = Arc<
    dyn Fn(&str, &[wire::SurfaceValue]) -> IceElement<'static, wire::SurfaceValue> + Send + Sync,
>;

/// The surfaces the embedding host paints, by the name a guest asks for.
/// A name not in here renders as a visible placeholder naming it.
pub type Surfaces = HashMap<String, Surface>;

/// The live content of every editor in a tree, by node key. The widget
/// shares it (`editor::Shared`) because the tree it sits in outlives the
/// lock on the guest that owns this.
#[derive(Clone, Debug)]
struct EditorField {
    content: editor::Shared,
    /// What the guest said the text was, last frame.
    reported: String,
}

/// The live text of every input and editor in a tree, by node key.
#[derive(Clone, Debug)]
pub struct Inputs {
    instance: u64,
    fields: HashMap<String, Field>,
    editors: HashMap<String, EditorField>,
}

impl Default for Inputs {
    fn default() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let instance = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .expect("view instance identities exhausted");
        Self {
            instance,
            fields: HashMap::new(),
            editors: HashMap::new(),
        }
    }
}

impl Inputs {
    /// Takes a new tree in. An input the guest now reports with a different
    /// value than last frame has been set by the guest — the host adopts
    /// it; one that reports the same value keeps whatever the user typed
    /// since. Inputs no longer in the tree are forgotten.
    pub fn adopt(&mut self, root: &wire::Node) {
        // A map, not a list: the retain below asks after every field this
        // holds, and a tree of a thousand inputs asking a list a thousand
        // times is a million comparisons on the window thread.
        let mut seen = HashMap::new();
        let mut editors = HashMap::new();
        collect_inputs(root, &mut seen, &mut editors);
        self.fields.retain(|key, _| seen.contains_key(key));
        for (key, value) in seen {
            match self.fields.get_mut(&key) {
                Some(field) if field.reported == value => {}
                Some(field) => {
                    field.text = value.clone();
                    field.reported = value;
                }
                None => {
                    self.fields.insert(
                        key,
                        Field {
                            text: value.clone(),
                            reported: value,
                        },
                    );
                }
            }
        }
        self.editors.retain(|key, _| editors.contains_key(key));
        for (key, text) in editors {
            match self.editors.get_mut(&key) {
                Some(field) if field.reported == text => {}
                Some(field) => {
                    // The guest set the text — unless it is echoing what the
                    // user typed, which the content already reads; rebuilding
                    // it then would put the caret back at the top on every
                    // keystroke.
                    let mut content = lock(&field.content);
                    if content.text() != text {
                        *content = text_editor::Content::with_text(&text);
                    }
                    drop(content);
                    field.reported = text;
                }
                None => {
                    self.editors.insert(
                        key,
                        EditorField {
                            content: Arc::new(Mutex::new(text_editor::Content::with_text(&text))),
                            reported: text,
                        },
                    );
                }
            }
        }
    }

    /// The text to show for an input; `fallback` is the guest's value for a
    /// key the host has not adopted yet.
    pub fn text<'a>(&'a self, key: &str, fallback: &'a str) -> &'a str {
        self.fields
            .get(key)
            .map_or(fallback, |field| field.text.as_str())
    }

    /// Records what the user did and queues the event the guest hears on
    /// `pending`, the events of the guest's next tick.
    ///
    /// A pointer move replaces the move already queued for the same handler
    /// instead of joining it: the guest hears one [`wire::Event::Pointer`]
    /// per handler per tick, at the last position — a browser's one
    /// `pointermove` per frame — where every move as its own event would be
    /// a guest tick per pixel. A press, a scroll and every other event queue
    /// in order.
    ///
    /// An edit is bounded to [`wire::MAX_STRING_BYTES`] here, the one place
    /// every host goes through before a keystroke or paste reaches a guest:
    /// a wasm component's whole memory is bounded, and a pasted-in string
    /// it decodes and keeps is copies of copies of whatever the clipboard
    /// held. The host's own copy of the field is cut the same way, so the
    /// widget shows exactly what the guest was told — a paste past the
    /// bound is cut, not refused, and the next render paints the cut value.
    ///
    /// An editor action is performed on the host's `Content` here, and the
    /// guest hears the whole text only when an action changed it: a caret
    /// move or a click is the host's alone and queues nothing, as does an
    /// action for an editor the tree no longer has.
    pub fn apply(&mut self, output: Output, pending: &mut Vec<wire::Event>) {
        let event = match output {
            Output::Ignore => return,
            Output::Activate(index) => wire::Event::Message(index),
            Output::Link { handler, mut text } => {
                wire::truncate_string(&mut text);
                wire::Event::Input { handler, text }
            }
            Output::Surface { handler, mut value } => {
                let Some(handler) = handler else {
                    return;
                };
                if !wire::sanitize_surface_event(&mut value) {
                    return;
                }
                wire::Event::Surface { handler, value }
            }
            Output::Edit {
                key,
                handler,
                mut text,
            } => {
                wire::truncate_string(&mut text);
                if let Some(field) = self.fields.get_mut(&key) {
                    field.text = text.clone();
                }
                wire::Event::Input { handler, text }
            }
            Output::EditorAction {
                key,
                handler,
                action,
            } => {
                let Some(field) = self.editors.get_mut(&key) else {
                    return;
                };
                let mut content = lock(&field.content);
                let before = content.text();
                content.perform(action);
                let mut text = content.text();
                if text == before {
                    return;
                }
                // A paste past the bound is cut like an input's, at the
                // cost of the caret: the content is rebuilt from the cut.
                if text.len() > wire::MAX_STRING_BYTES {
                    wire::truncate_string(&mut text);
                    *content = text_editor::Content::with_text(&text);
                }
                wire::Event::Edit { handler, text }
            }
            Output::MoveCaret { key, line, column } => {
                if let Some(field) = self.editors.get(&key) {
                    lock(&field.content).move_to(text_editor::Cursor {
                        position: text_editor::Position { line, column },
                        selection: None,
                    });
                }
                return;
            }
            Output::Toggle { handler, on } => wire::Event::Toggle { handler, on },
            Output::Slide { handler, value } => wire::Event::Slide { handler, value },
            Output::Select { handler, index } => wire::Event::Select { handler, index },
            Output::Size {
                handler,
                width,
                height,
            } => wire::Event::Size {
                handler,
                width,
                height,
            },
            Output::Pointer { handler, x, y } => wire::Event::Pointer { handler, x, y },
            Output::Move { handler, x, y } => {
                let queued = pending.iter_mut().rev().find(|event| {
                    matches!(event, wire::Event::Pointer { handler: queued, .. } if *queued == handler)
                });
                if let Some(wire::Event::Pointer {
                    x: at_x, y: at_y, ..
                }) = queued
                {
                    *at_x = x;
                    *at_y = y;
                    return;
                }
                wire::Event::Pointer { handler, x, y }
            }
            Output::ScrollOffset {
                handler,
                x,
                y,
                relative_x,
                relative_y,
            } => wire::Event::ScrollOffset {
                handler,
                x,
                y,
                relative_x,
                relative_y,
            },
            Output::Scroll {
                handler,
                dx,
                dy,
                pixels,
            } => wire::Event::Scroll {
                handler,
                dx,
                dy,
                pixels,
            },
        };
        pending.push(event);
    }

    /// The editor content under `key`; `None` for an editor the last
    /// [`Inputs::adopt`] did not see.
    fn editor(&self, key: &str) -> Option<&editor::Shared> {
        self.editors.get(key).map(|field| &field.content)
    }
}

fn lock(content: &editor::Shared) -> std::sync::MutexGuard<'_, text_editor::Content> {
    content
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn collect_inputs(
    node: &wire::Node,
    into: &mut HashMap<String, String>,
    editors: &mut HashMap<String, String>,
) {
    match node {
        wire::Node::Input { key, value, .. } => {
            into.insert(key.clone(), value.clone());
        }
        wire::Node::Editor { key, text, .. } => {
            editors.insert(key.clone(), text.clone());
        }
        wire::Node::Container { content, .. }
        | wire::Node::Sensor { child: content, .. }
        | wire::Node::Pin { content, .. }
        | wire::Node::Responsive { content, .. }
        | wire::Node::Lazy { content, .. }
        | wire::Node::MouseArea { content, .. }
        | wire::Node::Scroll { content, .. } => {
            collect_inputs(content, into, editors);
        }
        wire::Node::Linear { children, .. }
        | wire::Node::Grid { children, .. }
        | wire::Node::Stack { children, .. }
        | wire::Node::Hover { children, .. }
        | wire::Node::Tooltip { children, .. }
        | wire::Node::Overlay { children, .. }
        | wire::Node::KeyedColumn { children, .. }
        | wire::Node::Flex { children, .. }
        | wire::Node::When { children, .. } => {
            for child in children {
                collect_inputs(child, into, editors);
            }
        }
        wire::Node::Button {
            content: wire::ButtonContent::Child(child),
            ..
        } => collect_inputs(child, into, editors),
        wire::Node::Button { .. }
        | wire::Node::Qr { .. }
        | wire::Node::RichText { .. }
        | wire::Node::Text { .. }
        | wire::Node::Svg { .. }
        | wire::Node::Space { .. }
        | wire::Node::Rule { .. }
        | wire::Node::Toggle { .. }
        | wire::Node::Radio { .. }
        | wire::Node::Slider { .. }
        | wire::Node::PickList { .. }
        | wire::Node::Progress { .. }
        | wire::Node::Canvas { .. }
        | wire::Node::Surface { .. } => {}
    }
}

/// The most picture bytes one guest may leave with the host over its life.
/// A guest sends each picture once and never again, so nothing here is
/// evicted: past the cap a new picture is not kept and draws as empty
/// space, which is deterministic where an eviction would lose a picture the
/// guest believes the host has.
// ponytail: a hard cap, no eviction; an app that shows more than this in
// distinct pictures needs a re-ask protocol (host says "miss", guest resends).
pub const MAX_PICTURE_BYTES: usize = 8 * wire::MAX_SVG_BYTES_PER_FRAME;

/// Every picture a guest has sent, by the hash its nodes name it with.
#[derive(Clone, Debug, Default)]
pub struct Pictures {
    handles: HashMap<u64, widget::svg::Handle>,
    bytes: usize,
}

impl Pictures {
    /// Keeps every picture whose bytes this tree carries. Run on a frame
    /// that has passed [`wire::sanitize`], which bounds the bytes per frame.
    pub fn adopt(&mut self, root: &wire::Node) {
        collect_pictures(root, self);
    }

    fn keep(&mut self, hash: u64, bytes: &[u8]) {
        if self.handles.contains_key(&hash) || self.bytes + bytes.len() > MAX_PICTURE_BYTES {
            return;
        }
        self.bytes += bytes.len();
        self.handles
            .insert(hash, widget::svg::Handle::from_memory(bytes.to_vec()));
    }
}

fn collect_pictures(node: &wire::Node, into: &mut Pictures) {
    match node {
        wire::Node::Svg {
            hash,
            bytes: Some(bytes),
            ..
        } => into.keep(*hash, bytes),
        wire::Node::Container { content, .. }
        | wire::Node::Sensor { child: content, .. }
        | wire::Node::Pin { content, .. }
        | wire::Node::Responsive { content, .. }
        | wire::Node::Lazy { content, .. }
        | wire::Node::MouseArea { content, .. }
        | wire::Node::Scroll { content, .. } => {
            collect_pictures(content, into);
        }
        wire::Node::Linear { children, .. }
        | wire::Node::Grid { children, .. }
        | wire::Node::Stack { children, .. }
        | wire::Node::Hover { children, .. }
        | wire::Node::Tooltip { children, .. }
        | wire::Node::Overlay { children, .. }
        | wire::Node::KeyedColumn { children, .. }
        | wire::Node::Flex { children, .. }
        | wire::Node::When { children, .. } => {
            for child in children {
                collect_pictures(child, into);
            }
        }
        wire::Node::Button {
            content: wire::ButtonContent::Child(child),
            ..
        } => collect_pictures(child, into),
        wire::Node::Button { .. }
        | wire::Node::Svg { .. }
        | wire::Node::Qr { .. }
        | wire::Node::RichText { .. }
        | wire::Node::Text { .. }
        | wire::Node::Input { .. }
        | wire::Node::Editor { .. }
        | wire::Node::Space { .. }
        | wire::Node::Rule { .. }
        | wire::Node::Toggle { .. }
        | wire::Node::Radio { .. }
        | wire::Node::Slider { .. }
        | wire::Node::PickList { .. }
        | wire::Node::Progress { .. }
        | wire::Node::Canvas { .. }
        | wire::Node::Surface { .. } => {}
    }
}

// The wire's vocabulary, converted into iced's.

fn color(color: wire::Rgba) -> Color {
    let [r, g, b, a] = color.0.map(|channel| channel.clamp(0.0, 1.0));
    Color { r, g, b, a }
}

fn length(length: wire::Length) -> Length {
    match length {
        wire::Length::Fill => Length::Fill,
        wire::Length::FillPortion(factor) => Length::FillPortion(factor.max(1)),
        wire::Length::Shrink => Length::Shrink,
        wire::Length::Fixed(pixels) => Length::Fixed(pixels.max(0.0)),
    }
}

fn padding(edges: wire::Edges) -> iced::Padding {
    bounded_padding(
        f64::from(edges.top),
        f64::from(edges.right),
        f64::from(edges.bottom),
        f64::from(edges.left),
    )
}

fn border(border: wire::Border) -> iced::Border {
    let mut resolved = iced::Border::default();
    apply_border(border, &mut resolved);
    resolved
}

fn radius(value: [f32; 4]) -> iced::border::Radius {
    let [top_left, top_right, bottom_right, bottom_left] = value.map(|r| r.max(0.0));
    iced::border::Radius {
        top_left,
        top_right,
        bottom_right,
        bottom_left,
    }
}

fn apply_border(border: wire::Border, resolved: &mut iced::Border) {
    if let Some(value) = border.color {
        resolved.color = color(value);
    }
    if let Some(value) = border.width {
        resolved.width = value.max(0.0);
    }
    if let Some(value) = border.radius {
        resolved.radius = radius(value);
    }
}

fn apply_shadow(shadow: wire::Shadow, out: &mut iced::Shadow) {
    if let Some(value) = shadow.color {
        out.color = color(value);
    }
    if let Some(value) = shadow.x {
        out.offset.x = value;
    }
    if let Some(value) = shadow.y {
        out.offset.y = value;
    }
    if let Some(value) = shadow.blur {
        out.blur_radius = value;
    }
}

fn horizontal(align: wire::AlignX) -> Horizontal {
    match align {
        wire::AlignX::Left => Horizontal::Left,
        wire::AlignX::Center => Horizontal::Center,
        wire::AlignX::Right => Horizontal::Right,
    }
}

fn vertical(align: wire::AlignY) -> Vertical {
    match align {
        wire::AlignY::Top => Vertical::Top,
        wire::AlignY::Center => Vertical::Center,
        wire::AlignY::Bottom => Vertical::Bottom,
    }
}

/// A row's cross axis is vertical; the wire says `align` once for both.
fn cross(align: wire::AlignX) -> Vertical {
    match align {
        wire::AlignX::Left => Vertical::Top,
        wire::AlignX::Center => Vertical::Center,
        wire::AlignX::Right => Vertical::Bottom,
    }
}

fn font(font: wire::Font) -> iced::Font {
    iced::Font {
        family: match font.monospace {
            true => iced::font::Family::Monospace,
            false => iced::font::Family::SansSerif,
        },
        weight: match font.weight {
            wire::Weight::Thin => iced::font::Weight::Thin,
            wire::Weight::ExtraLight => iced::font::Weight::ExtraLight,
            wire::Weight::Light => iced::font::Weight::Light,
            wire::Weight::ExtraBold => iced::font::Weight::ExtraBold,
            wire::Weight::Black => iced::font::Weight::Black,
            wire::Weight::Normal => iced::font::Weight::Normal,
            wire::Weight::Medium => iced::font::Weight::Medium,
            wire::Weight::Semibold => iced::font::Weight::Semibold,
            wire::Weight::Bold => iced::font::Weight::Bold,
        },
        ..iced::Font::DEFAULT
    }
}

fn apply_face(face: wire::Face, style: &mut widget::button::Style) {
    if let Some(background) = face.background {
        style.background = Some(Background::Color(color(background)));
    }
    if let Some(text) = face.text {
        style.text_color = color(text);
    }
    if let Some(border) = face.border {
        apply_border(border, &mut style.border);
    }
}

fn button_style(
    style: &wire::ButtonStyle,
    theme: &iced::Theme,
    status: widget::button::Status,
) -> widget::button::Style {
    button::style(style, theme, status)
}

fn apply_input_face(face: wire::InputFace, style: &mut widget::text_input::Style) {
    if let Some(background) = face.background {
        style.background = Background::Color(color(background));
    }
    if let Some(border) = face.border {
        apply_border(border, &mut style.border);
    }
    if let Some(value) = face.value {
        style.value = color(value);
        style.icon = color(value);
    }
    if let Some(placeholder) = face.placeholder {
        style.placeholder = color(placeholder);
    }
    if let Some(selection) = face.selection {
        style.selection = color(selection);
    }
}

fn input_style(
    style: wire::InputStyle,
    theme: &iced::Theme,
    status: widget::text_input::Status,
) -> widget::text_input::Style {
    let mut resolved = widget::text_input::default(theme, status);
    apply_input_face(style.utility, &mut resolved);
    apply_input_face(style.active, &mut resolved);
    if matches!(status, widget::text_input::Status::Focused { .. })
        && let Some(color_value) = style.focus_border
    {
        resolved.border.color = color(color_value);
    }
    let state = match status {
        widget::text_input::Status::Active => None,
        widget::text_input::Status::Hovered => style.hovered,
        widget::text_input::Status::Focused { .. } => style.focused,
        widget::text_input::Status::Disabled => style.disabled,
    };
    if let Some(face) = state {
        apply_input_face(face, &mut resolved);
    }
    if matches!(
        status,
        widget::text_input::Status::Focused { is_hovered: true }
    ) && let Some(face) = style.focused_hovered
    {
        apply_input_face(face, &mut resolved);
    }
    resolved
}

/// A control face over the host's theme: the box or track, the mark, the
/// label and the border, each only where the face names one.
fn apply_control_face(
    face: wire::ControlFace,
    background: &mut Background,
    mark: &mut Color,
    text: &mut Option<Color>,
    edge: Option<&mut iced::Border>,
) {
    if let Some(fill) = face.background {
        *background = Background::Color(color(fill));
    }
    if let Some(fill) = face.mark {
        *mark = color(fill);
    }
    if let Some(fill) = face.text {
        *text = Some(color(fill));
    }
    if let (Some(edge), Some(border)) = (edge, face.border) {
        apply_border(border, edge);
    }
}

/// The faces a two-valued control paints for `status`: the active face of
/// its value, then the state's own face over it, as natively.
fn control_faces(
    style: wire::ToggleStyle,
    hovered: bool,
    disabled: bool,
    on: bool,
) -> impl Iterator<Item = wire::ControlFace> {
    let active = if on {
        style.active_on
    } else {
        style.active_off
    };
    let state = match (hovered, disabled, on) {
        (_, true, true) => style.disabled_on,
        (_, true, false) => style.disabled_off,
        (true, false, true) => style.hovered_on,
        (true, false, false) => style.hovered_off,
        (false, false, _) => None,
    };
    active.into_iter().chain(state)
}

fn checkbox_style(
    style: wire::ToggleStyle,
    theme: &iced::Theme,
    status: widget::checkbox::Status,
) -> widget::checkbox::Style {
    let preset = match style.tone {
        Some(wire::Tone::Secondary) => widget::checkbox::secondary,
        Some(wire::Tone::Success) => widget::checkbox::success,
        Some(wire::Tone::Danger) => widget::checkbox::danger,
        None | Some(wire::Tone::Primary | wire::Tone::Warning) => widget::checkbox::primary,
    };
    let mut resolved = preset(theme, status);
    let (hovered, disabled, on) = match status {
        widget::checkbox::Status::Active { is_checked } => (false, false, is_checked),
        widget::checkbox::Status::Hovered { is_checked } => (true, false, is_checked),
        widget::checkbox::Status::Disabled { is_checked } => (false, true, is_checked),
    };
    for face in control_faces(style, hovered, disabled, on) {
        apply_control_face(
            face,
            &mut resolved.background,
            &mut resolved.icon_color,
            &mut resolved.text_color,
            Some(&mut resolved.border),
        );
    }
    resolved
}

fn toggler_style(
    style: wire::ToggleStyle,
    theme: &iced::Theme,
    status: widget::toggler::Status,
) -> widget::toggler::Style {
    let mut resolved = widget::toggler::default(theme, status);
    let (hovered, disabled, on) = match status {
        widget::toggler::Status::Active { is_toggled } => (false, false, is_toggled),
        widget::toggler::Status::Hovered { is_toggled } => (true, false, is_toggled),
        widget::toggler::Status::Disabled { is_toggled } => (false, true, is_toggled),
    };
    for face in control_faces(style, hovered, disabled, on) {
        if let Some(fill) = face.background {
            resolved.background = Background::Color(color(fill));
        }
        if let Some(fill) = face.mark {
            resolved.foreground = Background::Color(color(fill));
        }
        if let Some(fill) = face.text {
            resolved.text_color = Some(color(fill));
        }
        if let Some(border) = face.border {
            if let Some(value) = border.color {
                resolved.background_border_color = color(value);
            }
            if let Some(value) = border.width {
                resolved.background_border_width = value.max(0.0);
            }
            if let Some(value) = border.radius {
                resolved.border_radius = Some(radius(value));
            }
        }
    }
    resolved
}

fn radio_style(
    style: wire::RadioStyle,
    theme: &iced::Theme,
    status: widget::radio::Status,
) -> widget::radio::Style {
    let mut resolved = widget::radio::default(theme, status);
    let (hovered, on) = match status {
        widget::radio::Status::Active { is_selected } => (false, is_selected),
        widget::radio::Status::Hovered { is_selected } => (true, is_selected),
    };
    let faces = control_faces(
        wire::ToggleStyle {
            tone: None,
            active_on: style.active_on,
            active_off: style.active_off,
            hovered_on: style.hovered_on,
            hovered_off: style.hovered_off,
            disabled_on: None,
            disabled_off: None,
        },
        hovered,
        false,
        on,
    );
    for face in faces {
        apply_control_face(
            face,
            &mut resolved.background,
            &mut resolved.dot_color,
            &mut resolved.text_color,
            None,
        );
        if let Some(border) = face.border {
            if let Some(value) = border.color {
                resolved.border_color = color(value);
            }
            if let Some(value) = border.width {
                resolved.border_width = value.max(0.0);
            }
        }
    }
    resolved
}

fn apply_slider_face(face: wire::SliderFace, style: &mut widget::slider::Style) {
    if let Some(fill) = face.rail_start {
        style.rail.backgrounds.0 = Background::Color(color(fill));
    }
    if let Some(fill) = face.rail_end {
        style.rail.backgrounds.1 = Background::Color(color(fill));
    }
    if let Some(width) = face.rail_width {
        style.rail.width = width.max(0.0);
    }
    if let Some(border) = face.rail_border {
        apply_border(border, &mut style.rail.border);
    }
    if let Some(fill) = face.handle {
        style.handle.background = Background::Color(color(fill));
    }
    if let Some(border) = face.handle_border {
        if let Some(value) = border.color {
            style.handle.border_color = color(value);
        }
        if let Some(value) = border.width {
            style.handle.border_width = value.max(0.0);
        }
    }
}

fn slider_style(
    style: wire::SliderStyle,
    theme: &iced::Theme,
    status: widget::slider::Status,
) -> widget::slider::Style {
    let mut resolved = widget::slider::default(theme, status);
    let state = match status {
        widget::slider::Status::Active => None,
        widget::slider::Status::Hovered => style.hovered,
        widget::slider::Status::Dragged => style.dragged,
    };
    for face in style.active.into_iter().chain(state) {
        apply_slider_face(face, &mut resolved);
    }
    resolved
}

fn pick_list_style(
    style: wire::PickListStyle,
    theme: &iced::Theme,
    status: widget::pick_list::Status,
) -> widget::pick_list::Style {
    let mut resolved = widget::pick_list::default(theme, status);
    let states = match status {
        widget::pick_list::Status::Active => [None, None],
        widget::pick_list::Status::Hovered => [style.hovered, None],
        widget::pick_list::Status::Opened { is_hovered: false } => [style.opened, None],
        widget::pick_list::Status::Opened { is_hovered: true } => {
            [style.opened, style.opened_hovered]
        }
    };
    for face in style.active.into_iter().chain(states.into_iter().flatten()) {
        if let Some(fill) = face.background {
            resolved.background = Background::Color(color(fill));
        }
        if let Some(fill) = face.text {
            resolved.text_color = color(fill);
        }
        if let Some(fill) = face.placeholder {
            resolved.placeholder_color = color(fill);
        }
        if let Some(fill) = face.handle {
            resolved.handle_color = color(fill);
        }
        if let Some(border) = face.border {
            apply_border(border, &mut resolved.border);
        }
    }
    resolved
}

fn menu_style(menu: wire::MenuFace, theme: &iced::Theme) -> iced::overlay::menu::Style {
    let mut resolved = iced::overlay::menu::default(theme);
    if let Some(fill) = menu.background {
        resolved.background = Background::Color(color(fill));
    }
    if let Some(fill) = menu.text {
        resolved.text_color = color(fill);
    }
    if let Some(border) = menu.border {
        apply_border(border, &mut resolved.border);
    }
    if let Some(fill) = menu.selected_text {
        resolved.selected_text_color = color(fill);
    }
    if let Some(fill) = menu.selected_background {
        resolved.selected_background = Background::Color(color(fill));
    }
    resolved
}

fn anchor(anchor: wire::ScrollAnchor) -> widget::scrollable::Anchor {
    match anchor {
        wire::ScrollAnchor::Start | wire::ScrollAnchor::Keep => widget::scrollable::Anchor::Start,
        wire::ScrollAnchor::End => widget::scrollable::Anchor::End,
    }
}

/// The box a layout's surface utilities draw around it; nothing when the
/// layout has none.
fn surfaced<'a>(
    content: impl Into<IceElement<'a, Output>>,
    background: Option<wire::Rgba>,
    edge: Option<wire::Border>,
) -> widget::Container<'a, Output, iced::Theme, iced::Renderer> {
    let mut layout = widget::container(content);
    if background.is_some() || edge.is_some() {
        let background = background.map(color);
        let edge = edge.map(border);
        layout = layout.style(move |_theme| widget::container::Style {
            background: background.map(Background::Color),
            border: edge.unwrap_or_default(),
            ..widget::container::Style::default()
        });
    }
    layout
}

/// One pick list option: its index is what crosses back, its text is what
/// iced shows and compares.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Choice(u32, String);

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

/// Renders a tree. Strings are cloned out of it, so the element outlives the
/// frame it came from; the next frame's tree can replace it freely.
pub fn render(
    root: &wire::Node,
    inputs: &Inputs,
    pictures: &Pictures,
    surfaces: &Surfaces,
) -> IceElement<'static, Output> {
    let handle = crate::MemoParkingHandle::unbound();
    let content = render_node(
        root,
        &Kept {
            button_ink: None,
            inputs,
            pictures,
            surfaces,
            canvas: std::rc::Rc::new(canvas::Cache::new(root)),
            containers: &HashMap::new(),
            memo: handle.clone(),
        },
    );
    memo::scope(content, inputs.instance, handle)
}

/// What the host keeps across frames, as one borrow for the render walk.
#[derive(Clone)]
struct Kept<'a> {
    memo: crate::MemoParkingHandle,
    button_ink: Option<crate::ButtonInk>,
    inputs: &'a Inputs,
    pictures: &'a Pictures,
    surfaces: &'a Surfaces,
    canvas: std::rc::Rc<canvas::Cache>,
    containers: &'a HashMap<String, [f64; 2]>,
}

fn render_node(node: &wire::Node, kept: &Kept<'_>) -> IceElement<'static, Output> {
    let inputs = kept.inputs;
    match node {
        wire::Node::KeyedColumn { .. } => lists::render(node, kept),
        wire::Node::Lazy {
            key,
            generation,
            content,
        } => memo::render(key, *generation, content, kept),
        wire::Node::Flex { .. } => flex::render(node, kept),
        wire::Node::Responsive {
            key,
            width,
            height,
            content,
        } => {
            let inputs = inputs.clone();
            let pictures = kept.pictures.clone();
            let surfaces = kept.surfaces.clone();
            let containers = kept.containers.clone();
            let canvas = kept.canvas.clone();
            let button_ink = kept.button_ink.clone();
            let memo = kept.memo.clone();
            let content = content.clone();
            let key = key.clone();
            let mut responsive = crate::responsive(move |size| {
                let mut containers = containers.clone();
                containers.insert(key.clone(), [size.width as f64, size.height as f64]);
                render_node(
                    &content,
                    &Kept {
                        memo: memo.clone(),
                        button_ink: button_ink.clone(),
                        inputs: &inputs,
                        pictures: &pictures,
                        surfaces: &surfaces,
                        containers: &containers,
                        canvas: canvas.clone(),
                    },
                )
            });
            if let Some(width) = width {
                responsive = responsive.width(length(*width));
            }
            if let Some(height) = height {
                responsive = responsive.height(length(*height));
            }
            responsive.into()
        }
        // Structural conditions are expanded by the surrounding layout.
        wire::Node::When { .. } => widget::Space::new().into(),
        wire::Node::Stack { .. } | wire::Node::Hover { .. } | wire::Node::Overlay { .. } => {
            layers::render(node, kept)
        }

        wire::Node::Container {
            shadow,
            max_width,
            max_height,
            clip,
            key,
            width,
            height,
            padding: edges,
            align_x,
            align_y,
            background,
            border: edge,
            snap,
            content,
        } => {
            let mut container =
                widget::container(render_node(content, kept)).id(widget::Id::from(key.clone()));
            if let Some(edges) = edges {
                container = container.padding(padding(*edges));
            }
            container = container.clip(*clip);
            if let Some(value) = max_width {
                container = container.max_width(*value);
            }
            if let Some(value) = max_height {
                container = container.max_height(*value);
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
            let background = background.map(color);
            let edge = edge.map(border);
            let mut shadow_value = iced::Shadow::default();
            apply_shadow(*shadow, &mut shadow_value);
            let snap = *snap;
            container = container.style(move |_theme| {
                let default = widget::container::Style::default();
                widget::container::Style {
                    background: background.map(Background::Color),
                    border: edge.unwrap_or_default(),
                    shadow: shadow_value,
                    snap: snap.unwrap_or(default.snap),
                    ..default
                }
            });
            accessible(container, StableId::new(key), Role::GenericContainer)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .into()
        }
        wire::Node::MouseArea {
            key,
            on_press,
            on_release,
            on_double_click,
            on_right_press,
            on_right_release,
            on_middle_press,
            on_middle_release,
            on_enter,
            on_exit,
            on_move,
            on_press_at,
            on_scroll,
            content,
        } => {
            let mut area = widget::mouse_area(render_node(content, kept));
            // iced's builders take a message each, so every route is set
            // only when the node carries it — `on_press` set would swallow
            // the press from the child under it.
            if let Some(index) = on_press {
                area = area.on_press(Output::Activate(*index));
            }
            if let Some(index) = on_release {
                area = area.on_release(Output::Activate(*index));
            }
            if let Some(index) = on_double_click {
                area = area.on_double_click(Output::Activate(*index));
            }
            if let Some(index) = on_right_press {
                area = area.on_right_press(Output::Activate(*index));
            }
            if let Some(index) = on_right_release {
                area = area.on_right_release(Output::Activate(*index));
            }
            if let Some(index) = on_middle_press {
                area = area.on_middle_press(Output::Activate(*index));
            }
            if let Some(index) = on_middle_release {
                area = area.on_middle_release(Output::Activate(*index));
            }
            if let Some(index) = on_enter {
                area = area.on_enter(Output::Activate(*index));
            }
            if let Some(index) = on_exit {
                area = area.on_exit(Output::Activate(*index));
            }
            if let Some(handler) = *on_move {
                // iced hands the position inside the area's bounds: local.
                area = area.on_move(move |point| Output::Move {
                    handler,
                    x: point.x,
                    y: point.y,
                });
            }
            if let Some(handler) = *on_scroll {
                area = area.on_scroll(move |delta| match delta {
                    iced::mouse::ScrollDelta::Lines { x, y } => Output::Scroll {
                        handler,
                        dx: x,
                        dy: y,
                        pixels: false,
                    },
                    iced::mouse::ScrollDelta::Pixels { x, y } => Output::Scroll {
                        handler,
                        dx: x,
                        dy: y,
                        pixels: true,
                    },
                });
            }
            let element: IceElement<'static, Output> = match *on_press_at {
                // The observer wraps the finished area, so it fires after
                // the child — captured or not — has seen the press.
                Some(handler) => crate::press_area(area)
                    .on_press_at(move |point: iced::Point| Output::Pointer {
                        handler,
                        x: point.x,
                        y: point.y,
                    })
                    .into(),
                None => area.into(),
            };
            accessible(
                widget::container(element),
                StableId::new(key),
                Role::GenericContainer,
            )
            .logical_id_maybe(cfg!(test).then_some(key.as_str()))
            .into()
        }
        wire::Node::Pin {
            x,
            y,
            width,
            height,
            content,
            ..
        } => {
            let mut pin = widget::pin(render_node(content, kept)).x(*x).y(*y);
            if let Some(width) = width {
                pin = pin.width(length(*width));
            }
            if let Some(height) = height {
                pin = pin.height(length(*height));
            }
            pin.into()
        }
        wire::Node::Tooltip {
            position,
            gap,
            padding,
            delay_ms,
            snap,
            style,
            children,
            ..
        } => tooltip::render(
            *position, *gap, *padding, *delay_ms, *snap, *style, children, kept,
        ),
        wire::Node::Linear {
            max_width,
            clip,
            key,
            wrap,
            axis,
            spacing,
            padding: edges,
            width,
            height,
            align,
            background,
            border: edge,
            children,
        } => {
            let children = selected_children(children, kept);
            let is_row = matches!(axis, wire::Axis::Row);
            let count = children.len();
            let rendered = children
                .iter()
                .map(|child| {
                    let element = render_node(child, kept);
                    if wrap.is_some() {
                        element
                    } else {
                        bounded_fill_element(element, count, is_row)
                    }
                })
                .collect::<Vec<_>>();
            let spacing = bounded_spacing(f64::from(spacing.unwrap_or(0.0)), count);
            let layout: IceElement<'static, Output> = match axis {
                wire::Axis::Column => {
                    let mut column = widget::column(rendered).spacing(spacing).clip(*clip);
                    if let Some(max_width) = max_width {
                        column = column.max_width(*max_width);
                    }
                    if let Some(edges) = edges {
                        column = column.padding(padding(*edges));
                    }
                    if let Some(width) = width {
                        column = column.width(length(*width));
                    }
                    if let Some(height) = height {
                        column = column.height(length(*height));
                    }
                    if let Some(align) = align {
                        column = column.align_x(horizontal(*align));
                    }
                    if let Some(wrap) = wrap {
                        let mut wrapped = column.wrap();
                        if let Some(gap) = wrap.spacing {
                            wrapped =
                                wrapped.horizontal_spacing(bounded_spacing(f64::from(gap), count));
                        }
                        if let Some(align) = wrap.align {
                            wrapped = wrapped.align_x(cross(align));
                        }
                        wrapped.into()
                    } else {
                        column.into()
                    }
                }
                wire::Axis::Row => {
                    let mut row = widget::row(rendered).spacing(spacing).clip(*clip);
                    if let Some(edges) = edges {
                        row = row.padding(padding(*edges));
                    }
                    if let Some(width) = width {
                        row = row.width(length(*width));
                    }
                    if let Some(height) = height {
                        row = row.height(length(*height));
                    }
                    if let Some(align) = align {
                        row = row.align_y(cross(*align));
                    }
                    if let Some(wrap) = wrap {
                        let mut wrapped = row.wrap();
                        if let Some(gap) = wrap.spacing {
                            wrapped =
                                wrapped.vertical_spacing(bounded_spacing(f64::from(gap), count));
                        }
                        if let Some(align) = wrap.align {
                            wrapped = wrapped.align_x(horizontal(align));
                        }
                        wrapped.into()
                    } else {
                        row.into()
                    }
                }
            };
            accessible(
                surfaced(layout, *background, *edge),
                StableId::new(key),
                Role::GenericContainer,
            )
            .logical_id_maybe(cfg!(test).then_some(key.as_str()))
            .into()
        }
        wire::Node::Grid {
            key,
            columns,
            fluid,
            spacing,
            padding: edges,
            width,
            height,
            aspect,
            background,
            border: edge,
            children,
        } => {
            let children = selected_children(children, kept);
            let columns = columns.map(|columns| columns.max(1) as usize);
            let rendered = children
                .iter()
                .map(|child| render_node(child, kept))
                .collect::<Vec<_>>();
            let mut grid = widget::grid(rendered).spacing(bounded_spacing(
                f64::from(spacing.unwrap_or(0.0)),
                children.len().max(columns.unwrap_or(0)),
            ));
            // Neither given leaves the widget's own default, as natively.
            grid = match (fluid, columns) {
                (Some(fluid), _) => grid.fluid(fluid.max(f32::EPSILON)),
                (None, Some(columns)) => grid.columns(columns),
                (None, None) => grid,
            };
            if let Some(aspect) = aspect {
                grid = grid.height(widget::grid::Sizing::AspectRatio(aspect.max(f32::EPSILON)));
            } else if let Some(height) = height {
                grid = grid.height(length(*height));
            }
            // The grid is sized in pixels; any other width is the wrapper's.
            let mut outer_width = None;
            match width {
                Some(wire::Length::Fixed(pixels)) => grid = grid.width(pixels.max(0.0)),
                Some(other) => outer_width = Some(length(*other)),
                None => {}
            }
            let mut layout = surfaced(grid, *background, *edge);
            if let Some(width) = outer_width {
                layout = layout.width(width);
            }
            if let Some(edges) = edges {
                layout = layout.padding(padding(*edges));
            }
            accessible(layout, StableId::new(key), Role::GenericContainer)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .into()
        }
        wire::Node::Sensor {
            key: _,
            on_show,
            on_resize,
            on_hide,
            anticipate,
            delay,
            child,
        } => {
            let size = |handler: u32| {
                move |size: iced::Size| Output::Size {
                    handler,
                    width: size.width,
                    height: size.height,
                }
            };
            let mut sensor = widget::sensor(render_node(child, kept));
            if let Some(handler) = on_show {
                sensor = sensor.on_show(size(*handler));
            }
            if let Some(handler) = on_resize {
                sensor = sensor.on_resize(size(*handler));
            }
            if let Some(message) = on_hide {
                sensor = sensor.on_hide(Output::Activate(*message));
            }
            if let Some(anticipate) = anticipate {
                sensor = sensor.anticipate(*anticipate);
            }
            if let Some(delay) = delay {
                sensor = sensor.delay(std::time::Duration::from_secs_f32(*delay / 1000.0));
            }
            sensor.into()
        }
        wire::Node::Scroll {
            on_scroll,
            virtual_rows,
            key,
            direction,
            width,
            height,
            bar_hidden,
            bar_width,
            bar_margin,
            scroller_width,
            bar_spacing,
            anchor_x,
            anchor_y,
            auto_scroll,
            background,
            border: edge,
            content,
        } => {
            let mut scrollbar = match bar_hidden {
                true => widget::scrollable::Scrollbar::hidden(),
                false => widget::scrollable::Scrollbar::new(),
            };
            if let Some(width) = bar_width {
                scrollbar = scrollbar.width(width.max(0.0));
            }
            if let Some(margin) = bar_margin {
                scrollbar = scrollbar.margin(margin.max(0.0));
            }
            if let Some(width) = scroller_width {
                scrollbar = scrollbar.scroller_width(width.max(0.0));
            }
            if let Some(spacing) = bar_spacing {
                scrollbar = scrollbar.spacing(spacing.max(0.0));
            }
            let direction = match direction {
                wire::ScrollDirection::Vertical => {
                    widget::scrollable::Direction::Vertical(scrollbar)
                }
                wire::ScrollDirection::Horizontal => {
                    widget::scrollable::Direction::Horizontal(scrollbar)
                }
                wire::ScrollDirection::Both => widget::scrollable::Direction::Both {
                    vertical: scrollbar,
                    horizontal: scrollbar,
                },
            };
            let mut scroll = widget::scrollable(render_node(content, kept))
                .id(widget::Id::from(key.clone()))
                .direction(direction)
                .anchor_x(anchor(*anchor_x))
                .anchor_y(anchor(*anchor_y))
                .auto_scroll(*auto_scroll);
            if let Some(width) = width {
                scroll = scroll.width(length(*width));
            }
            if let Some(height) = height {
                scroll = scroll.height(length(*height));
            }
            if let Some(handler) = *on_scroll {
                scroll = scroll.on_scroll(move |viewport| {
                    let absolute = viewport.absolute_offset();
                    let relative = viewport.relative_offset();
                    Output::ScrollOffset {
                        handler,
                        x: absolute.x,
                        y: absolute.y,
                        relative_x: if relative.x.is_finite() {
                            relative.x.clamp(0.0, 1.0)
                        } else {
                            0.0
                        },
                        relative_y: if relative.y.is_finite() {
                            relative.y.clamp(0.0, 1.0)
                        } else {
                            0.0
                        },
                    }
                });
            }
            let scroll: IceElement<'static, Output> = if *virtual_rows {
                crate::virtual_scroll(scroll).into()
            } else {
                scroll.into()
            };
            // `Keep` wraps the scrollable alone, as natively, so the
            // wrapper's operation walk reaches it first.
            let scroll: IceElement<'static, Output> = match anchor_y {
                wire::ScrollAnchor::Keep => crate::scroll_anchor(scroll).into(),
                _ => scroll,
            };
            let scroll: IceElement<'static, Output> = match (background, edge) {
                (None, None) => scroll,
                _ => surfaced(scroll, *background, *edge).into(),
            };
            accessible(scroll, StableId::new(key), Role::ScrollView)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .into()
        }
        wire::Node::Text { .. } => text::render(node),
        wire::Node::Qr { key, code } => qr::render(key, code),
        wire::Node::RichText { .. } => rich_text::render(node),
        wire::Node::Svg {
            inherit_button_ink,
            key,
            hash,
            label,
            color: tint,
            hover,
            fit,
            rotation,
            opacity,
            width,
            height,
            ..
        } => {
            // A picture the host does not hold — never sent, or sent past
            // the cap — is the space it would take.
            let picture: IceElement<'static, Output> = match kept.pictures.handles.get(hash) {
                Some(handle) => {
                    let mut svg = widget::svg(handle.clone());
                    if let Some(width) = width {
                        svg = svg.width(length(*width));
                    }
                    if let Some(height) = height {
                        svg = svg.height(length(*height));
                    }
                    if let Some(fit) = fit {
                        svg = svg.content_fit(match fit {
                            wire::ContentFit::Contain => iced::ContentFit::Contain,
                            wire::ContentFit::Cover => iced::ContentFit::Cover,
                            wire::ContentFit::Fill => iced::ContentFit::Fill,
                            wire::ContentFit::None => iced::ContentFit::None,
                            wire::ContentFit::ScaleDown => iced::ContentFit::ScaleDown,
                        });
                    }
                    if let Some(rotation) = rotation {
                        let finite = |radians: f32| {
                            iced::Radians(if radians.is_finite() { radians } else { 0.0 })
                        };
                        svg = svg.rotation(match *rotation {
                            wire::Rotation::Floating(radians) => {
                                iced::Rotation::Floating(finite(radians))
                            }
                            wire::Rotation::Solid(radians) => {
                                iced::Rotation::Solid(finite(radians))
                            }
                        });
                    }
                    if let Some(opacity) = opacity {
                        svg = svg.opacity(opacity.clamp(0.0, 1.0));
                    }
                    let ink = inherit_button_ink
                        .then(|| kept.button_ink.clone())
                        .flatten();
                    let tint = tint.map(color);
                    let hover = hover.map(|hover| hover.map(color)).unwrap_or(tint);
                    svg.style(move |_theme, status| widget::svg::Style {
                        color: ink.as_ref().map(|ink| ink.get()).or(match status {
                            widget::svg::Status::Idle => tint,
                            widget::svg::Status::Hovered => hover,
                        }),
                    })
                    .into()
                }
                None => {
                    let mut space = widget::Space::new();
                    if let Some(width) = width {
                        space = space.width(length(*width));
                    }
                    if let Some(height) = height {
                        space = space.height(length(*height));
                    }
                    space.into()
                }
            };
            accessible(picture, StableId::new(key), Role::Image)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .label(label.clone().unwrap_or_default())
                .into()
        }
        wire::Node::Input {
            options,
            key,
            placeholder,
            value,
            on_input,
            on_submit,
            width,
            secure,
            style,
        } => {
            // Borrowed for the widget, which copies it into its own owned
            // state right away (`TextInput::new` builds a `String` and a
            // `Value` from these references) — an owned copy here is only
            // ever needed for the accessible value below, and a secure field
            // never reports one.
            let current = inputs.text(key, value);
            let role = match secure {
                true => Role::PasswordInput,
                false => Role::TextInput,
            };
            let edit = {
                let key = key.clone();
                let handler = *on_input;
                move |text| Output::Edit {
                    key: key.clone(),
                    handler,
                    text,
                }
            };
            let style = **style;
            let mut input = widget::text_input(placeholder, current)
                .id(widget::Id::from(key.clone()))
                .secure(*secure)
                .on_input_maybe((!options.disabled).then_some(edit))
                .on_submit_maybe(
                    on_submit
                        .filter(|_| !options.disabled)
                        .map(Output::Activate),
                )
                .style(move |theme, status| input_style(style, theme, status));
            if let Some(width) = width {
                input = input.width(length(*width));
            }
            if let Some(edges) = options.padding {
                input = input.padding(padding(edges));
            }
            if let Some(size) = options.text_size {
                input = input.size(size);
            }
            if let Some(height) = options.line_height {
                input = input.line_height(widget::text::LineHeight::Relative(height));
            }
            if let Some(align) = options.align {
                input = input.align_x(horizontal(align));
            }
            if let Some(font) = &options.font {
                input = input.font(text::named_font(font));
            }
            let mut accessible = accessible(input, StableId::new(key), role)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .focus_id(widget::Id::from(key.clone()))
                .label(options.label.clone())
                .value_maybe((!secure).then(|| current.to_owned()))
                .disabled(options.disabled);
            if let Some(description) = &options.description {
                accessible = accessible.description(description.clone());
            }
            accessible.into()
        }
        wire::Node::Editor {
            key,
            placeholder,
            text,
            on_edit,
            ..
        } => {
            // A key the host has not adopted yet (a render before the frame
            // was taken in) shows the guest's text and keeps nothing.
            let content = inputs
                .editor(key)
                .cloned()
                .unwrap_or_else(|| Arc::new(Mutex::new(text_editor::Content::with_text(text))));
            let (value, cursor) = {
                let content = lock(&content);
                (content.text(), content.cursor())
            };
            let move_to = {
                let key = key.clone();
                move |line, column| Output::MoveCaret {
                    key: key.clone(),
                    line,
                    column,
                }
            };
            accessible(
                editor::HostEditor::new(node, content),
                StableId::new(key),
                Role::MultilineTextInput,
            )
            .logical_id_maybe(cfg!(test).then_some(key.as_str()))
            .focus_id(widget::Id::from(key.clone()))
            .label(placeholder.clone())
            .value(value)
            .editor_caret(cursor)
            .on_move_to(move_to)
            .disabled(on_edit.is_none())
            .into()
        }
        wire::Node::Button {
            key,
            content,
            label: name,
            checked,
            expanded,
            description,
            on_press,
            width,
            height,
            padding: edges,
            style,
        } => {
            let ink = match content {
                wire::ButtonContent::Child(child) if uses_button_ink(child) => {
                    Some(crate::button_ink())
                }
                _ => None,
            };
            // The label fallback is only cloned into an owned `String` when
            // an explicit accessible `name` is absent — `name.clone()` wins
            // over it via `.or_else` whenever one is set, so a button that
            // names its own accessible label never pays for the fallback.
            let (label_fallback, inner): (Option<&str>, IceElement<'static, Output>) = match content
            {
                wire::ButtonContent::Label(label) => (
                    Some(label.as_str()),
                    button::label(label, style.recipe.as_ref()),
                ),
                wire::ButtonContent::Child(child) => {
                    let inner = if let Some(ink) = &ink {
                        render_node(
                            child,
                            &Kept {
                                button_ink: Some(ink.clone()),
                                ..kept.clone()
                            },
                        )
                    } else {
                        render_node(child, kept)
                    };
                    (None, inner)
                }
            };
            let center_x = matches!(width, Some(wire::Length::Fixed(_)));
            let center_y = matches!(height, Some(wire::Length::Fixed(_)));
            let inner = if center_x || center_y {
                let mut centered = widget::container(inner);
                if center_x {
                    centered = centered.width(iced::Fill).align_x(Horizontal::Center);
                }
                if center_y {
                    centered = centered.height(iced::Fill).align_y(Vertical::Center);
                }
                IceElement::from(centered)
            } else {
                inner
            };
            let label = name.clone().or_else(|| label_fallback.map(str::to_owned));
            let activate = on_press.map(Output::Activate);
            let style = style.clone();
            let focus_ring = style.recipe.as_ref().and_then(|recipe| {
                recipe.focus_ring.map(|fg| {
                    (
                        fg,
                        recipe
                            .base
                            .border
                            .and_then(|b| b.radius)
                            .map_or(0.0, |r| r[0]),
                    )
                })
            });
            let mut button = widget::button(inner)
                .on_press_maybe(activate.clone())
                .style(move |theme, status| {
                    let resolved = button_style(&style, theme, status);
                    if let Some(ink) = &ink {
                        ink.set(resolved.text_color);
                    }
                    resolved
                });
            if let Some(width) = width {
                button = button.width(length(*width));
            }
            if let Some(height) = height {
                button = button.height(length(*height));
            }
            if let Some(edges) = edges {
                button = button.padding(padding(*edges));
            }
            let mut accessible = accessible(button, StableId::new(key), Role::Button)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .focus_id(widget::Id::from(key.clone()))
                .label(label.unwrap_or_default())
                .disabled(on_press.is_none())
                .on_activate_maybe(activate);
            if let Some(value) = checked {
                accessible = accessible.checked(*value);
            }
            if let Some(value) = expanded {
                accessible = accessible.expanded(*value);
            }
            if let Some(value) = description {
                accessible = accessible.description(value.clone());
            }
            if let Some((fg, radius)) = focus_ring {
                accessible = accessible.focus_ring(color(fg), radius);
            }
            accessible.into()
        }
        wire::Node::Space { width, height } => {
            let mut space = widget::Space::new();
            if let Some(width) = width {
                space = space.width(length(*width));
            }
            if let Some(height) = height {
                space = space.height(length(*height));
            }
            space.into()
        }
        wire::Node::Rule {
            key,
            axis,
            thickness,
            color: fg,
            weak,
            radius,
            snap,
        } => {
            let thickness = thickness.max(0.0);
            let fg = fg.map(color);
            let (weak, radius, snap) = (*weak, *radius, *snap);
            let styled = move |theme: &iced::Theme| {
                let mut style = match weak {
                    true => widget::rule::weak(theme),
                    false => widget::rule::default(theme),
                };
                if let Some(fg) = fg {
                    style.color = fg;
                }
                if let Some([top_left, top_right, bottom_right, bottom_left]) =
                    radius.map(|radius| radius.map(|corner| corner.max(0.0)))
                {
                    style.radius = iced::border::Radius {
                        top_left,
                        top_right,
                        bottom_right,
                        bottom_left,
                    };
                }
                if let Some(snap) = snap {
                    style.snap = snap;
                }
                style
            };
            let rule: IceElement<'static, Output> = match axis {
                wire::Axis::Row => widget::rule::horizontal(thickness).style(styled).into(),
                wire::Axis::Column => widget::rule::vertical(thickness).style(styled).into(),
            };
            accessible(widget::container(rule), StableId::new(key), Role::Splitter)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .into()
        }
        wire::Node::Toggle {
            key,
            kind,
            label,
            checked,
            on_toggle,
            width,
            style,
        } => {
            let checked = *checked;
            let style = *style;
            let flip = on_toggle.map(|handler| move |on| Output::Toggle { handler, on });
            let activate = on_toggle.map(|handler| Output::Toggle {
                handler,
                on: !checked,
            });
            let (control, role): (IceElement<'static, Output>, Role) = match kind {
                wire::ToggleKind::Checkbox => {
                    let mut checkbox = widget::checkbox(checked)
                        .label(label.clone())
                        .on_toggle_maybe(flip)
                        .style(move |theme, status| checkbox_style(style, theme, status));
                    if let Some(width) = width {
                        checkbox = checkbox.width(length(*width));
                    }
                    (checkbox.into(), Role::CheckBox)
                }
                wire::ToggleKind::Switch => {
                    let mut toggler = widget::toggler(checked)
                        .label(label.clone())
                        .on_toggle_maybe(flip)
                        .style(move |theme, status| toggler_style(style, theme, status));
                    if let Some(width) = width {
                        toggler = toggler.width(length(*width));
                    }
                    (toggler.into(), Role::Switch)
                }
            };
            accessible(control, StableId::new(key), role)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .focus_id(widget::Id::from(key.clone()))
                .label(label.clone())
                .checked(checked)
                .disabled(on_toggle.is_none())
                .on_activate_maybe(activate)
                .into()
        }
        wire::Node::Radio {
            key,
            label,
            selected,
            on_select,
            width,
            style,
        } => {
            let activate = Output::Activate(*on_select);
            let choose = activate.clone();
            let style = *style;
            let mut radio =
                widget::radio(label.clone(), true, selected.then_some(true), move |_| {
                    choose.clone()
                })
                .style(move |theme, status| radio_style(style, theme, status));
            if let Some(width) = width {
                radio = radio.width(length(*width));
            }
            accessible(radio, StableId::new(key), Role::RadioButton)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .focus_id(widget::Id::from(key.clone()))
                .label(label.clone())
                .checked(*selected)
                .selected(*selected)
                .on_activate_maybe(Some(activate))
                .into()
        }
        wire::Node::Slider {
            key,
            value,
            min,
            max,
            step,
            on_change,
            on_release,
            axis,
            width,
            height,
            style,
        } => {
            let style = *style;
            // iced divides by the step and by the range: a step of zero or
            // an empty range would put the handle at NaN.
            let (min, max) = match *max > *min {
                true => (*min, *max),
                false => (*min, *min + 1.0),
            };
            let value = value.clamp(min, max);
            let step = match *step > 0.0 {
                true => *step,
                false => 1.0,
            };
            let handler = *on_change;
            let change = move |value| Output::Slide { handler, value };
            let release = on_release.map(Output::Activate);
            let up = (value + step <= max).then(|| change(value + step));
            let down = (value - step >= min).then(|| change(value - step));
            let slider: IceElement<'static, Output> = match axis {
                wire::Axis::Row => {
                    let mut slider = widget::slider(min..=max, value, change)
                        .step(step)
                        .style(move |theme, status| slider_style(style, theme, status));
                    if let Some(release) = release.clone() {
                        slider = slider.on_release(release);
                    }
                    if let Some(width) = width {
                        slider = slider.width(length(*width));
                    }
                    if let Some(Length::Fixed(pixels)) = height.map(length) {
                        slider = slider.height(pixels);
                    }
                    slider.into()
                }
                wire::Axis::Column => {
                    let mut slider = widget::vertical_slider(min..=max, value, change)
                        .step(step)
                        .style(move |theme, status| slider_style(style, theme, status));
                    if let Some(release) = release {
                        slider = slider.on_release(release);
                    }
                    if let Some(height) = height {
                        slider = slider.height(length(*height));
                    }
                    if let Some(Length::Fixed(pixels)) = width.map(length) {
                        slider = slider.width(pixels);
                    }
                    slider.into()
                }
            };
            accessible(slider, StableId::new(key), Role::Slider)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .label("Slider")
                .value(format!("{value}"))
                .numeric(value.into(), min.into(), max.into(), Some(step.into()))
                .on_increment_maybe(up)
                .on_decrement_maybe(down)
                .into()
        }
        wire::Node::PickList {
            key,
            options,
            selected,
            placeholder,
            on_select,
            width,
            style,
        } => {
            let style = *style;
            let choices: Vec<Choice> = options
                .iter()
                .enumerate()
                .map(|(index, option)| Choice(index as u32, option.clone()))
                .collect();
            let chosen = selected.and_then(|index| choices.get(index as usize).cloned());
            let handler = *on_select;
            let mut pick = widget::pick_list(choices, chosen.clone(), move |choice: Choice| {
                Output::Select {
                    handler,
                    index: choice.0,
                }
            })
            .style(move |theme, status| pick_list_style(style, theme, status));
            if let Some(menu) = style.menu {
                pick = pick.menu_style(move |theme| menu_style(menu, theme));
            }
            if let Some(placeholder) = placeholder {
                pick = pick.placeholder(placeholder.clone());
            }
            if let Some(width) = width {
                pick = pick.width(length(*width));
            }
            accessible(pick, StableId::new(key), Role::ComboBox)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .label(placeholder.clone().unwrap_or_default())
                .value(chosen.map(|choice| choice.1).unwrap_or_default())
                .into()
        }
        wire::Node::Progress {
            key,
            value,
            min,
            max,
            axis,
            length: along,
            girth,
            tone,
            background,
            bar: fill,
            border: edge,
        } => {
            let (range, value) =
                crate::progress_range((*min).into(), (*max).into(), (*value).into());
            let (min, max) = (*range.start(), *range.end());
            let preset = match tone {
                None | Some(wire::Tone::Primary) => widget::progress_bar::primary,
                Some(wire::Tone::Secondary) => widget::progress_bar::secondary,
                Some(wire::Tone::Success) => widget::progress_bar::success,
                Some(wire::Tone::Warning) => widget::progress_bar::warning,
                Some(wire::Tone::Danger) => widget::progress_bar::danger,
            };
            let (background, fill, edge) = (background.map(color), fill.map(color), *edge);
            let mut bar = widget::progress_bar(range, value).style(move |theme| {
                let mut style = preset(theme);
                if let Some(background) = background {
                    style.background = Background::Color(background);
                }
                if let Some(fill) = fill {
                    style.bar = Background::Color(fill);
                }
                if let Some(edge) = edge {
                    apply_border(edge, &mut style.border);
                }
                style
            });
            if let Some(along) = along {
                bar = bar.length(length(*along));
            }
            if let Some(girth) = girth {
                bar = bar.girth(length(*girth));
            }
            if matches!(axis, wire::Axis::Column) {
                bar = bar.vertical();
            }
            accessible(bar, StableId::new(key), Role::ProgressIndicator)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .label("Progress")
                .value(format!("{value}"))
                .numeric(value.into(), min.into(), max.into(), None)
                .into()
        }
        wire::Node::Canvas {
            key, width, height, ..
        } => {
            let mut canvas = widget::canvas(kept.canvas.get(key));
            if let Some(width) = width {
                canvas = canvas.width(length(*width));
            }
            if let Some(height) = height {
                canvas = canvas.height(length(*height));
            }
            accessible(canvas, StableId::new(key), Role::Group)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .label("Canvas")
                .into()
        }
        wire::Node::Surface {
            key,
            name,
            args,
            on_event,
        } => {
            let content = match kept.surfaces.get(name) {
                Some(surface) => {
                    let handler = *on_event;
                    surface(key, args).map(move |value| Output::Surface { handler, value })
                }
                // Loud, not silent: a guest built against a surface this
                // host does not paint shows the gap where it would be.
                None => widget::container(
                    widget::text(format!("no host surface named {name:?}")).size(13),
                )
                .padding(8)
                .style(|theme: &iced::Theme| widget::container::Style {
                    border: iced::Border {
                        color: theme.palette().danger,
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..widget::container::Style::default()
                })
                .into(),
            };
            accessible(widget::container(content), StableId::new(key), Role::Group)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .label(name.clone())
                .into()
        }
    }
}

fn uses_button_ink(node: &wire::Node) -> bool {
    match node {
        wire::Node::Svg {
            inherit_button_ink, ..
        } => *inherit_button_ink,
        wire::Node::Button { .. } => false,
        _ => node.children().iter().any(uses_button_ink),
    }
}

fn selected_children<'a>(children: &'a [wire::Node], kept: &Kept<'_>) -> Vec<&'a wire::Node> {
    fn append<'a>(
        children: &'a [wire::Node],
        containers: &HashMap<String, [f64; 2]>,
        selected: &mut Vec<&'a wire::Node>,
    ) {
        for child in children {
            match child {
                wire::Node::When {
                    condition,
                    children,
                    ..
                } => {
                    if condition.matches(containers) {
                        append(children, containers, selected);
                    }
                }
                _ => selected.push(child),
            }
        }
    }
    let mut selected = Vec::new();
    append(children, kept.containers, &mut selected);
    selected
}

#[cfg(test)]
mod tests {
    #[test]
    fn pin_keeps_default_fill_and_explicit_dimensions() {
        use iced::advanced::{layout::Limits, renderer::Headless, widget::Tree};
        let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        for (width, height, expected) in [
            (None, None, iced::Size::new(200.0, 100.0)),
            (
                Some(wire::Length::Fixed(80.0)),
                Some(wire::Length::Fixed(50.0)),
                iced::Size::new(80.0, 50.0),
            ),
            (
                Some(wire::Length::Shrink),
                Some(wire::Length::Shrink),
                iced::Size::new(20.0, 10.0),
            ),
        ] {
            let node = wire::Node::Pin {
                key: "pin".into(),
                x: 4.0,
                y: 6.0,
                width,
                height,
                content: Box::new(wire::Node::Space {
                    width: Some(wire::Length::Fixed(20.0)),
                    height: Some(wire::Length::Fixed(10.0)),
                }),
            };
            let mut element = render(
                &node,
                &Inputs::default(),
                &Pictures::default(),
                &Surfaces::new(),
            );
            let mut tree = Tree::new(&element);
            let layout = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &Limits::new(iced::Size::ZERO, iced::Size::new(200.0, 100.0)),
            );
            assert_eq!(layout.size(), expected);
            assert_eq!(
                layout.children()[0].bounds().position(),
                iced::Point::new(4.0, 6.0)
            );
        }
    }

    #[test]
    fn input_styles_preserve_utility_active_focus_and_hover_precedence() {
        let red = wire::Rgba([1.0, 0.0, 0.0, 1.0]);
        let green = wire::Rgba([0.0, 1.0, 0.0, 1.0]);
        let blue = wire::Rgba([0.0, 0.0, 1.0, 1.0]);
        let edge = |color| wire::InputFace {
            border: Some(wire::Border {
                color: Some(color),
                width: None,
                radius: None,
            }),
            ..Default::default()
        };
        let style = wire::InputStyle {
            utility: wire::InputFace {
                background: Some(red),
                ..Default::default()
            },
            active: edge(red),
            focus_border: Some(green),
            focused_hovered: Some(edge(blue)),
            ..Default::default()
        };
        use widget::text_input::Status;
        let active = input_style(style, &iced::Theme::Light, Status::Active);
        assert_eq!(
            active.background,
            Background::Color(Color::from_rgb(1.0, 0.0, 0.0))
        );
        assert_eq!(active.border.color, Color::from_rgb(1.0, 0.0, 0.0));
        assert_eq!(
            input_style(
                style,
                &iced::Theme::Light,
                Status::Focused { is_hovered: false }
            )
            .border
            .color,
            Color::from_rgb(0.0, 1.0, 0.0)
        );
        assert_eq!(
            input_style(
                style,
                &iced::Theme::Light,
                Status::Focused { is_hovered: true }
            )
            .border
            .color,
            Color::from_rgb(0.0, 0.0, 1.0)
        );
        let explicit = wire::InputStyle {
            focused: Some(edge(red)),
            ..style
        };
        assert_eq!(
            input_style(
                explicit,
                &iced::Theme::Light,
                Status::Focused { is_hovered: false }
            )
            .border
            .color,
            Color::from_rgb(1.0, 0.0, 0.0)
        );
    }
    use super::*;

    // Claim: Tree layouts retain native width limits and clip only their paint viewport.
    // Counterexamples: ignoring max_width or clip must change geometry or pixels.
    #[test]
    fn linear_max_width_and_clip_reach_native_layout_and_paint() {
        use iced::advanced::{layout::Limits, renderer::Headless, widget::Tree};
        use iced_test::runtime::{UserInterface, user_interface};
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        for axis in [wire::Axis::Column, wire::Axis::Row] {
            for clip in [false, true] {
                let node = wire::Node::Linear {
                    key: "bounded".into(),
                    max_width: Some(80.0),
                    clip,
                    axis,
                    wrap: None,
                    spacing: None,
                    padding: None,
                    width: Some(wire::Length::Fill),
                    height: Some(wire::Length::Fixed(20.0)),
                    align: None,
                    background: None,
                    border: None,
                    children: vec![wire::Node::Text {
                        key: "overflow".into(),
                        content: "WWWWWWWWWWWWWWWW".into(),
                        options: wire::TextOptions {
                            wrapping: Some(wire::Wrapping::None),
                            ..Default::default()
                        },
                        size: Some(28.0),
                        color: Some(wire::Rgba([0.0, 0.0, 0.0, 1.0])),
                        font: Default::default(),
                        width: None,
                        align_x: None,
                    }],
                };
                for available in [60.0, 200.0] {
                    let mut element = render(
                        &node,
                        &Inputs::default(),
                        &Pictures::default(),
                        &Surfaces::new(),
                    );
                    let mut tree = Tree::new(&element);
                    let layout = element.as_widget_mut().layout(
                        &mut tree,
                        &renderer,
                        &Limits::new(iced::Size::ZERO, iced::Size::new(available, 100.0)),
                    );
                    assert_eq!(
                        layout.children()[0].size().width,
                        if axis == wire::Axis::Column {
                            available.min(80.0)
                        } else {
                            available
                        }
                    );
                }
                let mut ui = UserInterface::build(
                    render(
                        &node,
                        &Inputs::default(),
                        &Pictures::default(),
                        &Surfaces::new(),
                    ),
                    iced::Size::new(200.0, 100.0),
                    user_interface::Cache::default(),
                    &mut renderer,
                );
                ui.draw(
                    &mut renderer,
                    &iced::Theme::Light,
                    &iced::advanced::renderer::Style {
                        text_color: iced::Color::BLACK,
                    },
                    iced::mouse::Cursor::Unavailable,
                );
                let pixels =
                    renderer.screenshot(iced::Size::new(200, 100), 1.0, iced::Color::WHITE);
                let ink_below = (20..40).any(|y| (0..80).any(|x| pixels[(y * 200 + x) * 4] < 200));
                assert_eq!(
                    ink_below, !clip,
                    "{axis:?}: clipping must constrain overflowing glyph paint"
                );
                assert!(
                    (0..20).any(|y| (0..80).any(|x| pixels[(y * 200 + x) * 4] < 200)),
                    "visible glyph paint establishes the fixture"
                );
            }
        }
    }

    // Claim: copied box shadows paint outside the box with native offset, blur, and alpha.
    // Counterexample: omitting the shadow leaves the sampled exterior pixels white.
    #[test]
    fn container_shadows_paint_offset_blur_and_transparency() {
        use iced::advanced::renderer::Headless;
        use iced_test::runtime::{UserInterface, user_interface};
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        for (x, y, blur, alpha, sample_x, sample_y) in [
            (12.0, 0.0, 0.0, 0.5, 55, 40),
            (-12.0, 0.0, 0.0, 0.5, 23, 40),
            (0.0, 12.0, 0.0, 0.5, 40, 55),
            (0.0, -12.0, 0.0, 0.5, 40, 23),
            (12.0, 0.0, 8.0, 0.5, 64, 40),
            (12.0, 0.0, 0.0, 0.0, 55, 40),
        ] {
            let node = wire::Node::Pin {
                key: "position".into(),
                x: 30.0,
                y: 30.0,
                width: None,
                height: None,
                content: Box::new(wire::Node::Container {
                    shadow: wire::Shadow {
                        color: Some(wire::Rgba([1.0, 0.0, 0.0, alpha])),
                        x: Some(x),
                        y: Some(y),
                        blur: Some(blur),
                    },
                    max_width: None,
                    max_height: None,
                    clip: false,
                    key: "shadow".into(),
                    width: Some(wire::Length::Fixed(20.0)),
                    height: Some(wire::Length::Fixed(20.0)),
                    padding: None,
                    align_x: None,
                    align_y: None,
                    background: Some(wire::Rgba([1.0; 4])),
                    border: None,
                    snap: None,
                    content: Box::new(wire::Node::Space {
                        width: None,
                        height: None,
                    }),
                }),
            };
            let mut ui = UserInterface::build(
                render(
                    &node,
                    &Inputs::default(),
                    &Pictures::default(),
                    &Surfaces::new(),
                ),
                iced::Size::new(100.0, 100.0),
                user_interface::Cache::default(),
                &mut renderer,
            );
            ui.draw(
                &mut renderer,
                &iced::Theme::Light,
                &iced::advanced::renderer::Style {
                    text_color: iced::Color::BLACK,
                },
                iced::mouse::Cursor::Unavailable,
            );
            let pixels = renderer.screenshot(iced::Size::new(100, 100), 1.0, iced::Color::WHITE);
            let pixel = &pixels[(sample_y * 100 + sample_x) * 4..][..3];
            if alpha == 0.0 {
                assert_eq!(pixel, &[255, 255, 255], "transparent shadow paints nothing");
            } else {
                assert!(
                    pixel[0] > pixel[1] && pixel[1] > 0 && pixel[1] < 255,
                    "offset={x}, blur={blur}: translucent red shadow outside box, got {pixel:?}"
                );
            }
            assert_eq!(
                &pixels[(40 * 100 + 40) * 4..][..3],
                &[255, 255, 255],
                "box face covers its shadow"
            );
        }
    }

    #[test]
    fn wrapping_rows_and_columns_reflow_at_host_limits() {
        use iced::advanced::{layout::Limits, renderer::Headless, widget::Tree};
        let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        for axis in [wire::Axis::Row, wire::Axis::Column] {
            let row = axis == wire::Axis::Row;
            let node = wire::Node::Linear {
                max_width: None,
                clip: false,
                key: "wrap".into(),
                axis,
                wrap: Some(wire::Wrap {
                    spacing: Some(6.0),
                    align: Some(wire::AlignX::Right),
                }),
                spacing: Some(8.0),
                padding: None,
                width: Some(if row {
                    wire::Length::Fill
                } else {
                    wire::Length::Shrink
                }),
                height: Some(if row {
                    wire::Length::Shrink
                } else {
                    wire::Length::Fill
                }),
                align: None,
                background: None,
                border: None,
                children: (0..3)
                    .map(|_| wire::Node::Space {
                        width: Some(wire::Length::Fixed(if row { 60.0 } else { 20.0 })),
                        height: Some(wire::Length::Fixed(if row { 20.0 } else { 60.0 })),
                    })
                    .collect(),
            };
            for (available, expected_cross) in [(100.0, 72.0), (200.0, 20.0)] {
                let mut element = render(
                    &node,
                    &Inputs::default(),
                    &Pictures::default(),
                    &Surfaces::new(),
                );
                let mut tree = Tree::new(&element);
                let layout = element.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &Limits::new(iced::Size::ZERO, iced::Size::new(available, available)),
                );
                let size = layout.size();
                assert_eq!(
                    if row { size.height } else { size.width },
                    expected_cross,
                    "wrapping {axis:?} reflows at {available}"
                );
            }
        }
    }

    #[test]
    fn wire_button_accessibility_distinguishes_false_from_absence() {
        use iced::advanced::renderer::Headless;
        use iced::advanced::widget::operation::{Operation, Outcome};
        use iced_test::runtime::{UserInterface, user_interface};
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let mut cache = user_interface::Cache::default();
        for state in [Some(true), Some(false), None] {
            let node = wire::Node::Button {
                key: "toggle".into(),
                content: wire::ButtonContent::Label("Toggle".into()),
                label: None,
                checked: state,
                expanded: state,
                description: Some("Details".into()),
                on_press: Some(0),
                width: None,
                height: None,
                padding: None,
                style: wire::ButtonStyle::default(),
            };
            let root = render(
                &node,
                &Inputs::default(),
                &Pictures::default(),
                &Surfaces::new(),
            );
            let mut ui =
                UserInterface::build(root, iced::Size::new(400.0, 240.0), cache, &mut renderer);
            let mut operation = crate::SnapshotOperation::<Output>::named("Test");
            ui.operate(
                &renderer,
                &mut iced::advanced::widget::operation::black_box(&mut operation),
            );
            let Outcome::Some(snapshot) = operation.finish() else {
                panic!("snapshot")
            };
            let (_, node) = snapshot
                .update
                .nodes
                .iter()
                .find(|(id, _)| *id == StableId::new("toggle").node_id())
                .unwrap();
            assert_eq!(
                node.toggled(),
                state.map(|value| if value {
                    accesskit::Toggled::True
                } else {
                    accesskit::Toggled::False
                })
            );
            assert_eq!(node.is_expanded(), state);
            assert_eq!(node.description(), Some("Details"));
            cache = ui.into_cache();
        }
    }

    fn input(value: &str) -> wire::Node {
        wire::Node::Input {
            options: Default::default(),
            key: "App/draft".into(),
            placeholder: "What needs doing?".into(),
            value: value.into(),
            on_input: 0,
            on_submit: Some(1),
            width: None,
            secure: false,
            style: Box::default(),
        }
    }

    fn partial_border() -> wire::Border {
        wire::Border {
            color: None,
            width: None,
            radius: None,
        }
    }

    #[test]
    fn pick_list_border_faces_follow_native_state_inheritance() {
        let active = wire::PickFace {
            border: Some(wire::Border {
                color: Some(wire::Rgba([1.0, 0.0, 0.0, 1.0])),
                width: Some(3.0),
                radius: Some([7.0; 4]),
            }),
            ..wire::PickFace::default()
        };
        let hovered = wire::PickFace {
            border: Some(wire::Border {
                color: Some(wire::Rgba([0.0, 1.0, 0.0, 1.0])),
                ..partial_border()
            }),
            ..wire::PickFace::default()
        };
        let opened = wire::PickFace {
            border: Some(wire::Border {
                width: Some(5.0),
                ..partial_border()
            }),
            ..wire::PickFace::default()
        };
        let style = wire::PickListStyle {
            active: Some(active),
            hovered: Some(hovered),
            opened: Some(opened),
            opened_hovered: Some(hovered),
            ..wire::PickListStyle::default()
        };
        for (status, expected_width) in [
            (widget::pick_list::Status::Hovered, 3.0),
            (widget::pick_list::Status::Opened { is_hovered: true }, 5.0),
        ] {
            let resolved = pick_list_style(style, &iced::Theme::Light, status);
            assert_eq!(resolved.border.radius, radius([7.0; 4]));
            assert_eq!(resolved.border.width, expected_width);
            assert_eq!(resolved.border.color, Color::from_rgb(0.0, 1.0, 0.0));
        }
    }

    #[test]
    fn checkbox_border_color_keeps_native_rounding_and_width() {
        let theme = iced::Theme::Light;
        let status = widget::checkbox::Status::Active { is_checked: false };
        let native = widget::checkbox::primary(&theme, status);
        assert!(native.border.radius.top_left > 0.0);
        let face = wire::ControlFace {
            border: Some(wire::Border {
                color: Some(wire::Rgba([1.0, 0.0, 0.0, 1.0])),
                ..partial_border()
            }),
            ..wire::ControlFace::default()
        };
        let resolved = checkbox_style(
            wire::ToggleStyle {
                active_off: Some(face),
                ..wire::ToggleStyle::default()
            },
            &theme,
            status,
        );
        assert_eq!(resolved.border.radius, native.border.radius);
        assert_eq!(resolved.border.width, native.border.width);
        assert_eq!(resolved.border.color, Color::from_rgb(1.0, 0.0, 0.0));
    }

    #[test]
    fn border_faces_inherit_omissions_but_explicit_zero_clears() {
        let active = wire::Face {
            border: Some(wire::Border {
                color: Some(wire::Rgba([1.0, 0.0, 0.0, 1.0])),
                width: Some(3.0),
                radius: Some([2.0, 4.0, 6.0, 8.0]),
            }),
            ..wire::Face::default()
        };
        let hovered = wire::Face {
            border: Some(wire::Border {
                width: Some(5.0),
                ..partial_border()
            }),
            ..wire::Face::default()
        };
        let style = wire::ButtonStyle {
            preset: wire::ButtonPreset::default(),
            recipe: None,
            active,
            hovered: Some(hovered),
            ..wire::ButtonStyle::default()
        };
        let resolved = button_style(&style, &iced::Theme::Light, widget::button::Status::Hovered);
        assert_eq!(resolved.border.radius, radius([2.0, 4.0, 6.0, 8.0]));
        assert_eq!(resolved.border.color, Color::from_rgb(1.0, 0.0, 0.0));
        assert_eq!(resolved.border.width, 5.0);
        let cleared = wire::Face {
            border: Some(wire::Border {
                color: Some(wire::Rgba([0.0; 4])),
                width: Some(0.0),
                radius: Some([0.0; 4]),
            }),
            ..wire::Face::default()
        };
        let resolved = button_style(
            &wire::ButtonStyle {
                hovered: Some(cleared),
                ..style
            },
            &iced::Theme::Light,
            widget::button::Status::Hovered,
        );
        assert_eq!(resolved.border.radius, iced::border::Radius::default());
        assert_eq!(resolved.border.width, 0.0);
        assert_eq!(resolved.border.color, Color::TRANSPARENT);
    }

    #[test]
    fn toggler_partial_border_keeps_automatic_rounding() {
        let theme = iced::Theme::Light;
        let status = widget::toggler::Status::Active { is_toggled: true };
        let native = widget::toggler::default(&theme, status);
        let resolved = toggler_style(
            wire::ToggleStyle {
                active_on: Some(wire::ControlFace {
                    border: Some(wire::Border {
                        width: Some(2.0),
                        ..partial_border()
                    }),
                    ..wire::ControlFace::default()
                }),
                ..wire::ToggleStyle::default()
            },
            &theme,
            status,
        );
        assert_eq!(resolved.border_radius, native.border_radius);
        assert_eq!(
            resolved.background_border_color,
            native.background_border_color
        );
        assert_eq!(resolved.background_border_width, 2.0);
    }

    #[test]
    fn a_rendered_surface_routes_its_value_and_an_unrouted_one_stays_quiet() {
        use iced::advanced::renderer::Headless;
        use iced::{Event, Font, Pixels, Point, Size, mouse};
        use iced_test::runtime::{UserInterface, user_interface};
        use wire::SurfaceValue as V;

        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let mut surfaces = Surfaces::new();
        surfaces.insert(
            "link".into(),
            Arc::new(|key, args| {
                assert_eq!(key, "App/link");
                assert_eq!(args.len(), 2);
                assert_eq!(args[1], V::Bool(true));
                widget::button("Open")
                    .on_press(args[0].clone())
                    .width(100)
                    .height(40)
                    .into()
            }),
        );
        for handler in [Some(27), None] {
            for value in [
                V::Str("duck://pages/example".into()),
                V::List(vec![V::Record {
                    name: "Link".into(),
                    fields: vec![(
                        "target".into(),
                        V::Option(Some(Box::new(V::Str("duck://pages/example".into())))),
                    )],
                }]),
            ] {
                let node = wire::Node::Surface {
                    key: "App/link".into(),
                    name: "link".into(),
                    args: vec![value.clone(), V::Bool(true)],
                    on_event: handler,
                };
                let mut inputs = Inputs::default();
                let element = render(&node, &inputs, &Pictures::default(), &surfaces);
                let mut ui = UserInterface::build(
                    element,
                    Size::new(200.0, 100.0),
                    user_interface::Cache::default(),
                    &mut renderer,
                );
                let mut messages = vec![];
                let _ = ui.update(
                    &[
                        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                    ],
                    mouse::Cursor::Available(Point::new(20.0, 20.0)),
                    &mut renderer,
                    &mut iced::advanced::clipboard::Null,
                    &mut messages,
                );
                assert_eq!(messages.len(), 1, "the provider button was clicked");
                let mut pending = vec![];
                inputs.apply(messages.remove(0), &mut pending);
                let expected: Vec<_> = handler
                    .into_iter()
                    .map(|handler| wire::Event::Surface {
                        handler,
                        value: value.clone(),
                    })
                    .collect();
                assert_eq!(pending, expected);
            }
        }
    }

    #[test]
    fn bounded_host_surfaces_keep_shader_region_dimensions() {
        use iced::advanced::{layout, renderer::Headless, widget::Tree};
        use iced::{Font, Pixels, Size};
        let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let mut surfaces = Surfaces::new();
        surfaces.insert(
            "shader".into(),
            Arc::new(|_, _| {
                widget::button("Host shader region")
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            }),
        );
        let missing = Surfaces::new();
        // Providers fill the region. Missing providers retain the same
        // bounds; their diagnostic label must not supply intrinsic size.
        for registry in [&surfaces, &missing] {
            for (width, height, expected) in [
                (wire::Length::Fill, 24.0, Size::new(300.0, 24.0)),
                (wire::Length::Fixed(100.0), 100.0, Size::new(100.0, 100.0)),
                (wire::Length::Fixed(0.0), 0.0, Size::ZERO),
            ] {
                let node = wire::Node::Container {
                    shadow: Default::default(),
                    max_width: None,
                    max_height: None,
                    clip: false,
                    key: "shader/@bounds".into(),
                    width: Some(width),
                    height: Some(wire::Length::Fixed(height)),
                    padding: None,
                    align_x: None,
                    align_y: None,
                    background: None,
                    border: None,
                    snap: None,
                    content: Box::new(wire::Node::Surface {
                        key: "shader".into(),
                        name: "shader".into(),
                        args: vec![],
                        on_event: None,
                    }),
                };
                let inputs = Inputs::default();
                let mut element = render(&node, &inputs, &Pictures::default(), registry);
                let mut tree = Tree::new(&element);
                let layout = element.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
                );
                assert_eq!(layout.size(), expected, "shader surface region dimensions");
            }
        }
    }

    #[test]
    fn surface_events_bound_strings_and_drop_nonfinite_numbers() {
        use wire::SurfaceValue as V;
        let mut inputs = Inputs::default();
        let prefix = "a".repeat(wire::MAX_STRING_BYTES - 1);
        assert_eq!(
            applied(
                &mut inputs,
                Output::Surface {
                    handler: Some(4),
                    value: V::Str(format!("{prefix}€€")),
                }
            ),
            Some(wire::Event::Surface {
                handler: 4,
                value: V::Str(prefix)
            })
        );
        assert_eq!(
            applied(
                &mut inputs,
                Output::Surface {
                    handler: Some(4),
                    value: V::F64(f64::INFINITY)
                }
            ),
            None
        );
        assert_eq!(
            applied(
                &mut inputs,
                Output::Surface {
                    handler: Some(4),
                    value: V::List(vec![V::Option(Some(Box::new(V::F64(f64::NAN))))]),
                }
            ),
            None
        );
    }

    #[test]
    fn the_host_keeps_what_the_user_typed_until_the_guest_moves_the_value() {
        let mut inputs = Inputs::default();
        inputs.adopt(&input(""));
        let mut pending = Vec::new();
        inputs.apply(
            Output::Edit {
                key: "App/draft".into(),
                handler: 0,
                text: "milk".into(),
            },
            &mut pending,
        );
        assert_eq!(
            pending,
            [wire::Event::Input {
                handler: 0,
                text: "milk".into()
            }]
        );
        // The guest has not caught up yet: it still reports the old value.
        inputs.adopt(&input(""));
        assert_eq!(inputs.text("App/draft", ""), "milk");
        // It echoes what it was told: still the host's text.
        inputs.adopt(&input("milk"));
        assert_eq!(inputs.text("App/draft", ""), "milk");
        // Its handler cleared the field: the host follows.
        inputs.adopt(&input(""));
        assert_eq!(inputs.text("App/draft", "x"), "");
        // Gone from the tree, gone from the host.
        inputs.adopt(&wire::Node::empty());
        assert_eq!(inputs.text("App/draft", "fallback"), "fallback");
    }

    fn editor_node(text: &str) -> wire::Node {
        wire::Node::Editor {
            key: "App/notes".into(),
            placeholder: "Notes".into(),
            text: text.into(),
            on_edit: Some(3),
            width: None,
            height: Some(wire::Length::Fill),
            min_height: Some(80.0),
            max_height: None,
        }
    }

    fn editor_text(inputs: &Inputs) -> String {
        lock(inputs.editor("App/notes").expect("adopted")).text()
    }

    /// What `apply` queues for one output.
    fn applied(inputs: &mut Inputs, output: Output) -> Option<wire::Event> {
        let mut pending = Vec::new();
        inputs.apply(output, &mut pending);
        assert!(pending.len() <= 1, "{pending:?}");
        pending.pop()
    }

    #[test]
    fn an_editor_action_edits_the_hosts_content_and_only_a_text_change_reaches_the_guest() {
        use text_editor::{Action, Edit, Motion};
        let mut inputs = Inputs::default();
        inputs.adopt(&editor_node("ab"));
        let insert = |c| Output::EditorAction {
            key: "App/notes".into(),
            handler: 3,
            action: Action::Edit(Edit::Insert(c)),
        };
        // The caret starts at the top: the host's content, not the guest's.
        assert_eq!(
            applied(&mut inputs, insert('x')),
            Some(wire::Event::Edit {
                handler: 3,
                text: "xab".into()
            })
        );
        // A caret move is the host's alone.
        assert_eq!(
            applied(
                &mut inputs,
                Output::EditorAction {
                    key: "App/notes".into(),
                    handler: 3,
                    action: Action::Move(Motion::End),
                }
            ),
            None
        );
        assert_eq!(
            applied(&mut inputs, insert('y')),
            Some(wire::Event::Edit {
                handler: 3,
                text: "xaby".into()
            })
        );
        // The guest has not caught up: it still reports the old text.
        inputs.adopt(&editor_node("ab"));
        assert_eq!(editor_text(&inputs), "xaby");
        // It echoes what it was told: the host's content, caret and all.
        inputs.adopt(&editor_node("xaby"));
        assert_eq!(editor_text(&inputs), "xaby");
        assert_eq!(
            applied(&mut inputs, insert('z')),
            Some(wire::Event::Edit {
                handler: 3,
                text: "xabyz".into()
            })
        );
        // Its handler set the text: the host follows.
        inputs.adopt(&editor_node(""));
        assert_eq!(editor_text(&inputs), "");
        // Accessibility moves the caret without a word to the guest.
        inputs.adopt(&editor_node("one\ntwo"));
        assert_eq!(
            applied(
                &mut inputs,
                Output::MoveCaret {
                    key: "App/notes".into(),
                    line: 1,
                    column: 1,
                }
            ),
            None
        );
        assert_eq!(
            applied(&mut inputs, insert('!')),
            Some(wire::Event::Edit {
                handler: 3,
                text: "one\nt!wo".into()
            })
        );
        // Gone from the tree, gone from the host: an action for it is dropped.
        inputs.adopt(&wire::Node::empty());
        assert_eq!(applied(&mut inputs, insert('q')), None);
    }

    #[test]
    fn a_paste_into_an_editor_past_the_bound_is_cut() {
        use text_editor::{Action, Edit};
        let mut inputs = Inputs::default();
        inputs.adopt(&editor_node(""));
        let prefix = "a".repeat(wire::MAX_STRING_BYTES - 1);
        let event = applied(
            &mut inputs,
            Output::EditorAction {
                key: "App/notes".into(),
                handler: 3,
                action: Action::Edit(Edit::Paste(Arc::new(format!("{prefix}€€€")))),
            },
        );
        let Some(wire::Event::Edit { text, .. }) = event else {
            panic!("expected an Edit event, got {event:?}");
        };
        assert_eq!(text, prefix);
        assert_eq!(editor_text(&inputs), prefix);
    }

    #[test]
    fn an_edit_inside_the_bound_is_untouched() {
        let mut inputs = Inputs::default();
        inputs.adopt(&input(""));
        let text = "milk and éclairs".to_string();
        let mut pending = Vec::new();
        inputs.apply(
            Output::Edit {
                key: "App/draft".into(),
                handler: 0,
                text: text.clone(),
            },
            &mut pending,
        );
        assert_eq!(
            pending,
            [wire::Event::Input {
                handler: 0,
                text: text.clone()
            }]
        );
        assert_eq!(inputs.text("App/draft", ""), text);
    }

    #[test]
    fn a_paste_past_the_bound_reaches_the_guest_cut_on_a_char_boundary() {
        // A 3-byte char straddles the cut: MAX_STRING_BYTES itself lands
        // mid-character, so the real cut must back up to the char before it.
        let prefix = "a".repeat(wire::MAX_STRING_BYTES - 1);
        let pasted = format!("{prefix}€€€");
        let mut inputs = Inputs::default();
        inputs.adopt(&input(""));
        let mut pending = Vec::new();
        inputs.apply(
            Output::Edit {
                key: "App/draft".into(),
                handler: 0,
                text: pasted,
            },
            &mut pending,
        );
        let [wire::Event::Input { text, .. }] = pending.as_slice() else {
            panic!("expected an Input event, got {pending:?}");
        };
        assert_eq!(text.len(), wire::MAX_STRING_BYTES - 1);
        assert!(text.is_char_boundary(text.len()));
        assert_eq!(*text, prefix);
        // The host's own copy of the field agrees with what the guest heard.
        assert_eq!(inputs.text("App/draft", ""), prefix);
    }

    /// Moves are one event per handler per tick, at the last position; a
    /// press at a position, a scroll and another handler's move are not
    /// folded into it.
    #[test]
    fn pointer_moves_coalesce_per_handler_and_nothing_else_does() {
        let mut inputs = Inputs::default();
        let mut pending = Vec::new();
        let mv = |handler, x, y| Output::Move { handler, x, y };
        inputs.apply(mv(1, 1.0, 1.0), &mut pending);
        inputs.apply(mv(2, 5.0, 5.0), &mut pending);
        inputs.apply(
            Output::Scroll {
                handler: 3,
                dx: 0.0,
                dy: 1.0,
                pixels: false,
            },
            &mut pending,
        );
        inputs.apply(mv(1, 2.0, 3.0), &mut pending);
        inputs.apply(
            Output::Pointer {
                handler: 4,
                x: 9.0,
                y: 9.0,
            },
            &mut pending,
        );
        inputs.apply(
            Output::Pointer {
                handler: 4,
                x: 8.0,
                y: 8.0,
            },
            &mut pending,
        );
        let pointer = |handler, x, y| wire::Event::Pointer { handler, x, y };
        assert_eq!(
            pending,
            [
                pointer(1, 2.0, 3.0),
                pointer(2, 5.0, 5.0),
                wire::Event::Scroll {
                    handler: 3,
                    dx: 0.0,
                    dy: 1.0,
                    pixels: false,
                },
                pointer(4, 9.0, 9.0),
                pointer(4, 8.0, 8.0),
            ]
        );
    }

    #[test]
    fn scroll_offsets_follow_native_anchors_and_only_emit_on_change() {
        use iced::advanced::renderer::Headless;
        use iced::{Event, Point, Size, mouse};
        use iced_test::runtime::{UserInterface, user_interface};
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        for anchor_y in [wire::ScrollAnchor::Start, wire::ScrollAnchor::End] {
            for on_scroll in [Some(42), None] {
                let node = wire::Node::Scroll {
                    on_scroll,
                    virtual_rows: false,
                    key: "viewport".into(),
                    direction: wire::ScrollDirection::Vertical,
                    width: Some(wire::Length::Fixed(100.0)),
                    height: Some(wire::Length::Fixed(100.0)),
                    bar_hidden: true,
                    bar_width: None,
                    bar_margin: None,
                    scroller_width: None,
                    bar_spacing: None,
                    anchor_x: wire::ScrollAnchor::Start,
                    anchor_y,
                    auto_scroll: false,
                    background: None,
                    border: None,
                    content: Box::new(wire::Node::Space {
                        width: Some(wire::Length::Fixed(100.0)),
                        height: Some(wire::Length::Fixed(600.0)),
                    }),
                };
                let mut inputs = Inputs::default();
                let mut ui = UserInterface::build(
                    render(&node, &inputs, &Pictures::default(), &Surfaces::new()),
                    Size::new(100.0, 100.0),
                    user_interface::Cache::default(),
                    &mut renderer,
                );
                let mut messages = vec![];
                let delta = if anchor_y == wire::ScrollAnchor::Start {
                    -50.0
                } else {
                    50.0
                };
                ui.update(
                    &[Event::Mouse(mouse::Event::WheelScrolled {
                        delta: mouse::ScrollDelta::Pixels { x: 0.0, y: delta },
                    })],
                    mouse::Cursor::Available(Point::new(25.0, 25.0)),
                    &mut renderer,
                    &mut iced::advanced::clipboard::Null,
                    &mut messages,
                );
                let mut pending = vec![];
                for message in messages.drain(..) {
                    inputs.apply(message, &mut pending);
                }
                if on_scroll.is_some() {
                    assert_eq!(
                        pending,
                        vec![wire::Event::ScrollOffset {
                            handler: 42,
                            x: 0.0,
                            y: 50.0,
                            relative_x: 0.0,
                            relative_y: 0.1
                        }],
                        "native anchor-relative scroll payload"
                    );
                } else {
                    assert!(pending.is_empty(), "scroll telemetry is opt-in");
                }
                ui.update(
                    &[Event::Mouse(mouse::Event::WheelScrolled {
                        delta: mouse::ScrollDelta::Pixels { x: 0.0, y: 0.0 },
                    })],
                    mouse::Cursor::Available(Point::new(25.0, 25.0)),
                    &mut renderer,
                    &mut iced::advanced::clipboard::Null,
                    &mut messages,
                );
                assert!(
                    messages.is_empty(),
                    "unchanged native viewport sends no duplicate event"
                );
            }
        }
    }

    #[test]
    fn every_node_kind_renders() {
        let tree = wire::Node::Container {
            shadow: Default::default(),
            max_width: None,
            max_height: None,
            clip: false,
            key: "App".into(),
            width: Some(wire::Length::Fill),
            height: Some(wire::Length::Fill),
            padding: Some(wire::Edges::all(8.0)),
            align_x: Some(wire::AlignX::Center),
            align_y: Some(wire::AlignY::Center),
            background: Some(wire::Rgba([0.0, 0.0, 0.0, 1.0])),
            border: None,
            snap: Some(true),
            content: Box::new(wire::Node::Linear {
                max_width: None,
                clip: false,
                wrap: None,
                key: "App/content".into(),
                axis: wire::Axis::Column,
                spacing: Some(4.0),
                padding: None,
                width: None,
                height: None,
                align: Some(wire::AlignX::Left),
                background: Some(wire::Rgba([0.1, 0.1, 0.1, 1.0])),
                border: None,
                children: vec![
                    wire::Node::Text {
                        options: Default::default(),
                        key: "App/content/title".into(),
                        content: "Todo".into(),
                        size: Some(28.0),
                        color: Some(wire::Rgba([1.0, 1.0, 1.0, 1.0])),
                        font: wire::Font {
                            monospace: false,
                            weight: wire::Weight::Bold,
                        },
                        width: None,
                        align_x: None,
                    },
                    input("milk"),
                    wire::Node::MouseArea {
                        key: "App/content/pad".into(),
                        on_press: Some(8),
                        on_release: Some(9),
                        on_double_click: Some(10),
                        on_right_press: Some(11),
                        on_right_release: Some(12),
                        on_middle_press: Some(13),
                        on_middle_release: Some(14),
                        on_enter: Some(15),
                        on_exit: Some(16),
                        on_move: Some(8),
                        on_press_at: Some(9),
                        on_scroll: Some(10),
                        content: Box::new(wire::Node::Space {
                            width: Some(wire::Length::Fixed(40.0)),
                            height: Some(wire::Length::Fixed(40.0)),
                        }),
                    },
                    editor_node("notes"),
                    wire::Node::Button {
                        checked: None,
                        expanded: None,
                        description: None,
                        key: "App/content/add".into(),
                        content: wire::ButtonContent::Label("Add".into()),
                        label: None,
                        on_press: Some(2),
                        width: None,
                        height: None,
                        padding: None,
                        style: wire::ButtonStyle::default(),
                    },
                    wire::Node::Rule {
                        key: "App/content/@rule:1".into(),
                        axis: wire::Axis::Row,
                        thickness: 1.0,
                        color: None,
                        weak: true,
                        radius: Some([1.0, -1.0, 0.0, f32::NAN]),
                        snap: Some(false),
                    },
                    wire::Node::Sensor {
                        key: "App/content/watch".into(),
                        on_show: Some(0),
                        on_resize: Some(1),
                        on_hide: Some(2),
                        anticipate: Some(48.0),
                        delay: Some(16.0),
                        child: Box::new(wire::Node::Scroll {
                            on_scroll: None,
                            virtual_rows: false,
                            key: "App/content/list".into(),
                            direction: wire::ScrollDirection::Vertical,
                            width: None,
                            height: None,
                            bar_hidden: true,
                            bar_width: Some(-4.0),
                            bar_margin: Some(2.0),
                            scroller_width: Some(0.0),
                            bar_spacing: Some(1.0),
                            anchor_x: wire::ScrollAnchor::End,
                            anchor_y: wire::ScrollAnchor::Keep,
                            auto_scroll: true,
                            background: None,
                            border: Some(wire::Border {
                                color: Some(wire::Rgba([1.0, 0.0, 0.0, 1.0])),
                                width: Some(1.0),
                                radius: Some([2.0; 4]),
                            }),
                            content: Box::new(wire::Node::Space {
                                width: None,
                                height: Some(wire::Length::Fixed(10.0)),
                            }),
                        }),
                    },
                    wire::Node::Grid {
                        key: "App/content/cells".into(),
                        columns: Some(0),
                        fluid: None,
                        spacing: Some(4.0),
                        padding: Some(wire::Edges::all(2.0)),
                        width: Some(wire::Length::Fill),
                        height: Some(wire::Length::Fixed(40.0)),
                        aspect: None,
                        background: None,
                        border: None,
                        children: vec![
                            wire::Node::Space {
                                width: None,
                                height: None,
                            },
                            wire::Node::Space {
                                width: None,
                                height: None,
                            },
                        ],
                    },
                    wire::Node::Grid {
                        key: "App/content/tiles".into(),
                        columns: None,
                        fluid: Some(0.0),
                        spacing: None,
                        padding: None,
                        width: Some(wire::Length::Fixed(120.0)),
                        height: None,
                        aspect: Some(-1.0),
                        background: Some(wire::Rgba([0.2, 0.2, 0.2, 1.0])),
                        border: None,
                        children: vec![wire::Node::Space {
                            width: None,
                            height: None,
                        }],
                    },
                    wire::Node::Toggle {
                        key: "App/content/hide".into(),
                        kind: wire::ToggleKind::Switch,
                        label: "Hide done".into(),
                        checked: false,
                        on_toggle: Some(3),
                        width: None,
                        style: wire::ToggleStyle {
                            tone: None,
                            active_on: Some(face()),
                            active_off: None,
                            hovered_on: None,
                            hovered_off: Some(face()),
                            disabled_on: None,
                            disabled_off: None,
                        },
                    },
                    wire::Node::Toggle {
                        key: "App/content/agree".into(),
                        kind: wire::ToggleKind::Checkbox,
                        label: "Agree".into(),
                        checked: true,
                        on_toggle: None,
                        width: Some(wire::Length::Fill),
                        style: wire::ToggleStyle {
                            tone: Some(wire::Tone::Warning),
                            disabled_on: Some(face()),
                            ..wire::ToggleStyle::default()
                        },
                    },
                    wire::Node::Radio {
                        key: "App/content/first".into(),
                        label: "First".into(),
                        selected: true,
                        on_select: 4,
                        width: None,
                        style: wire::RadioStyle {
                            active_on: Some(face()),
                            ..wire::RadioStyle::default()
                        },
                    },
                    // A hostile slider: empty range, zero step, value outside.
                    wire::Node::Slider {
                        key: "App/content/volume".into(),
                        value: 7.0,
                        min: 1.0,
                        max: 1.0,
                        step: 0.0,
                        on_change: 5,
                        on_release: Some(6),
                        axis: wire::Axis::Column,
                        width: Some(wire::Length::Fixed(20.0)),
                        height: Some(wire::Length::Fill),
                        style: wire::SliderStyle {
                            active: Some(wire::SliderFace {
                                rail_start: Some(wire::Rgba([1.0, 0.0, 0.0, 1.0])),
                                rail_end: None,
                                rail_width: Some(-3.0),
                                rail_border: None,
                                handle: Some(wire::Rgba([0.0, 1.0, 0.0, 1.0])),
                                handle_border: Some(wire::Border {
                                    color: Some(wire::Rgba([0.0, 0.0, 1.0, 1.0])),
                                    width: Some(-1.0),
                                    radius: Some([0.0; 4]),
                                }),
                            }),
                            hovered: None,
                            dragged: Some(wire::SliderFace::default()),
                        },
                    },
                    wire::Node::PickList {
                        key: "App/content/mode".into(),
                        options: vec!["Light".into(), "Dark".into()],
                        selected: Some(9),
                        placeholder: Some("Mode".into()),
                        on_select: 7,
                        width: None,
                        style: wire::PickListStyle {
                            active: Some(wire::PickFace {
                                background: Some(wire::Rgba([0.0, 0.0, 0.0, 1.0])),
                                text: Some(wire::Rgba([1.0, 1.0, 1.0, 1.0])),
                                placeholder: None,
                                handle: None,
                                border: None,
                            }),
                            menu: Some(wire::MenuFace {
                                selected_background: Some(wire::Rgba([0.0, 0.5, 0.5, 1.0])),
                                ..wire::MenuFace::default()
                            }),
                            ..wire::PickListStyle::default()
                        },
                    },
                    wire::Node::Progress {
                        key: "App/content/done".into(),
                        value: 0.5,
                        min: 1.0,
                        max: 0.0,
                        axis: wire::Axis::Row,
                        length: Some(wire::Length::Fill),
                        girth: Some(wire::Length::Fixed(6.0)),
                        tone: Some(wire::Tone::Success),
                        background: None,
                        bar: Some(wire::Rgba([0.0, 1.0, 0.0, 1.0])),
                        border: None,
                    },
                    picture(Some(b"<svg xmlns='http://www.w3.org/2000/svg'/>".to_vec())),
                    // A hash the host never saw: empty space of the size.
                    wire::Node::Svg {
                        inherit_button_ink: false,
                        key: "App/content/unseen".into(),
                        hash: 99,
                        bytes: None,
                        label: None,
                        color: None,
                        hover: None,
                        fit: None,
                        rotation: None,
                        opacity: None,
                        width: None,
                        height: None,
                    },
                    wire::Node::Surface {
                        key: "App/content/tile".into(),
                        name: "tile".into(),
                        args: vec![wire::SurfaceValue::Str("camera 1".into())],
                        on_event: None,
                    },
                    // A surface this host does not paint: a placeholder,
                    // not a panic.
                    wire::Node::Surface {
                        key: "App/content/missing".into(),
                        name: "nobody".into(),
                        args: vec![],
                        on_event: None,
                    },
                ],
            }),
        };
        let mut inputs = Inputs::default();
        inputs.adopt(&tree);
        let mut pictures = Pictures::default();
        pictures.adopt(&tree);
        let painted = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = painted.clone();
        let mut surfaces = Surfaces::new();
        surfaces.insert(
            "tile".into(),
            Arc::new(move |key: &str, args: &[wire::SurfaceValue]| {
                assert_eq!(key, "App/content/tile");
                seen.lock().unwrap().extend_from_slice(args);
                widget::text("camera 1").into()
            }),
        );
        let _element: IceElement<'static, Output> = render(&tree, &inputs, &pictures, &surfaces);
        assert_eq!(
            *painted.lock().unwrap(),
            [wire::SurfaceValue::Str("camera 1".into())]
        );
    }

    fn picture(bytes: Option<Vec<u8>>) -> wire::Node {
        wire::Node::Svg {
            inherit_button_ink: false,
            key: "App/content/icon".into(),
            hash: 7,
            bytes,
            label: Some("Icon".into()),
            color: Some(wire::Rgba([1.0, 0.0, 0.0, 1.0])),
            hover: Some(None),
            fit: Some(wire::ContentFit::Cover),
            rotation: Some(wire::Rotation::Solid(f32::NAN)),
            opacity: Some(2.0),
            width: Some(wire::Length::Fixed(24.0)),
            height: Some(wire::Length::Fixed(24.0)),
        }
    }

    fn face() -> wire::ControlFace {
        wire::ControlFace {
            background: Some(wire::Rgba([0.0, 0.0, 0.0, 1.0])),
            mark: Some(wire::Rgba([1.0, 1.0, 1.0, 1.0])),
            text: None,
            border: Some(wire::Border {
                color: Some(wire::Rgba([1.0, 0.0, 0.0, 1.0])),
                width: Some(1.0),
                radius: Some([2.0; 4]),
            }),
        }
    }

    /// The bytes cross once: a later frame naming the hash alone still
    /// finds the picture, and a picture past the cap is not kept.
    #[test]
    fn a_picture_is_kept_by_hash_after_the_frame_that_carried_it() {
        let mut pictures = Pictures::default();
        pictures.adopt(&picture(None));
        assert!(
            pictures.handles.is_empty(),
            "a hash alone is nothing to keep"
        );
        pictures.adopt(&picture(Some(b"<svg/>".to_vec())));
        assert!(pictures.handles.contains_key(&7));
        pictures.adopt(&picture(None));
        assert!(pictures.handles.contains_key(&7));
        assert_eq!(pictures.bytes, 6);

        let mut full = Pictures::default();
        full.adopt(&wire::Node::Svg {
            inherit_button_ink: false,
            key: "App/content/big".into(),
            hash: 8,
            bytes: Some(vec![b' '; MAX_PICTURE_BYTES + 1]),
            label: None,
            color: None,
            hover: None,
            fit: None,
            rotation: None,
            opacity: None,
            width: None,
            height: None,
        });
        assert!(full.handles.is_empty());
        assert_eq!(full.bytes, 0);
    }
}
