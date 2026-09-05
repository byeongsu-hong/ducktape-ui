//! Renders the node tree a view module sends over `ui_lang_wire` with the
//! host's own widgets.
//!
//! The guest decided WHAT is on screen; everything about HOW is the host's:
//! layout, fonts, IME, the caret, scroll position, focus. A text field's
//! live value in particular lives here, in [`Inputs`] keyed by node key —
//! the guest only ever sees a whole-string [`wire::Event::Input`] after the
//! fact, and gets to overwrite the host's copy only by reporting a value
//! that differs from the one it reported last frame (its own handler
//! cleared or set the field).
//!
//! The rendered element speaks [`Output`]; the host turns each one into the
//! wire event with [`Inputs::apply`] and hands it to the guest.

use std::collections::HashMap;

use iced::alignment::{Horizontal, Vertical};
use iced::{Background, Color, Element, Length, widget};
use ui_lang_wire as wire;

use crate::{Role, StableId, accessible, bounded_fill_element, bounded_padding, bounded_spacing};

pub type IceElement<'a, Message> = Element<'a, Message, iced::Theme, iced::Renderer>;

/// What the user did to a rendered tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    /// A button was pressed or an input submitted: the guest's message
    /// table index the node carried.
    Activate(u32),
    /// An input's text changed. `key` is the node's key, `handler` the
    /// guest's input-handler table index, `text` the whole new value.
    Edit {
        key: String,
        handler: u32,
        text: String,
    },
}

#[derive(Debug)]
struct Field {
    /// What the host shows and edits.
    text: String,
    /// What the guest said the value was, last frame.
    reported: String,
}

/// The live text of every input in a tree, by node key.
#[derive(Debug, Default)]
pub struct Inputs {
    fields: HashMap<String, Field>,
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
        collect_inputs(root, &mut seen);
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
    }

    /// The text to show for an input; `fallback` is the guest's value for a
    /// key the host has not adopted yet.
    pub fn text<'a>(&'a self, key: &str, fallback: &'a str) -> &'a str {
        self.fields
            .get(key)
            .map_or(fallback, |field| field.text.as_str())
    }

    /// Records what the user did and returns the event the guest hears.
    ///
    /// An edit is bounded to [`wire::MAX_STRING_BYTES`] here, the one place
    /// every host goes through before a keystroke or paste reaches a guest:
    /// a wasm component's whole memory is bounded, and a pasted-in string
    /// it decodes and keeps is copies of copies of whatever the clipboard
    /// held. The host's own copy of the field is cut the same way, so the
    /// widget shows exactly what the guest was told — a paste past the
    /// bound is cut, not refused, and the next render paints the cut value.
    pub fn apply(&mut self, output: Output) -> wire::Event {
        match output {
            Output::Activate(index) => wire::Event::Message(index),
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
        }
    }
}

fn collect_inputs(node: &wire::Node, into: &mut HashMap<String, String>) {
    match node {
        wire::Node::Input { key, value, .. } => {
            into.insert(key.clone(), value.clone());
        }
        wire::Node::Container { content, .. } | wire::Node::Scroll { content, .. } => {
            collect_inputs(content, into);
        }
        wire::Node::Linear { children, .. } => {
            for child in children {
                collect_inputs(child, into);
            }
        }
        wire::Node::Button {
            content: wire::ButtonContent::Child(child),
            ..
        } => collect_inputs(child, into),
        wire::Node::Button { .. }
        | wire::Node::Text { .. }
        | wire::Node::Space { .. }
        | wire::Node::Rule { .. } => {}
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
    let [top_left, top_right, bottom_right, bottom_left] =
        border.radius.map(|radius| radius.max(0.0));
    iced::Border {
        color: color(border.color),
        width: border.width.max(0.0),
        radius: iced::border::Radius {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        },
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
        style.border = self::border(border);
    }
}

fn button_style(
    style: wire::ButtonStyle,
    theme: &iced::Theme,
    status: widget::button::Status,
) -> widget::button::Style {
    let mut resolved = widget::button::primary(theme, status);
    apply_face(style.active, &mut resolved);
    let state = match status {
        widget::button::Status::Active => None,
        widget::button::Status::Hovered => style.hovered,
        widget::button::Status::Pressed => style.pressed,
        widget::button::Status::Disabled => style.disabled,
    };
    if let Some(face) = state {
        apply_face(face, &mut resolved);
    }
    resolved
}

fn apply_input_face(face: wire::InputFace, style: &mut widget::text_input::Style) {
    if let Some(background) = face.background {
        style.background = Background::Color(color(background));
    }
    if let Some(border) = face.border {
        style.border = self::border(border);
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
    apply_input_face(style.active, &mut resolved);
    let state = match status {
        widget::text_input::Status::Active => None,
        widget::text_input::Status::Hovered => style.hovered,
        widget::text_input::Status::Focused { .. } => style.focused,
        widget::text_input::Status::Disabled => style.disabled,
    };
    if let Some(face) = state {
        apply_input_face(face, &mut resolved);
    }
    resolved
}

/// Renders a tree. Strings are cloned out of it, so the element outlives the
/// frame it came from; the next frame's tree can replace it freely.
pub fn render(root: &wire::Node, inputs: &Inputs) -> IceElement<'static, Output> {
    render_node(root, inputs)
}

fn render_node(node: &wire::Node, inputs: &Inputs) -> IceElement<'static, Output> {
    match node {
        wire::Node::Container {
            key,
            width,
            height,
            padding: edges,
            align_x,
            align_y,
            background,
            border: edge,
            content,
        } => {
            let mut container =
                widget::container(render_node(content, inputs)).id(widget::Id::from(key.clone()));
            if let Some(edges) = edges {
                container = container.padding(padding(*edges));
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
            container = container.style(move |_theme| widget::container::Style {
                background: background.map(Background::Color),
                border: edge.unwrap_or_default(),
                ..widget::container::Style::default()
            });
            accessible(container, StableId::new(key), Role::GenericContainer)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .into()
        }
        wire::Node::Linear {
            key,
            axis,
            spacing,
            padding: edges,
            width,
            height,
            align,
            children,
        } => {
            let is_row = matches!(axis, wire::Axis::Row);
            let count = children.len();
            let rendered = children
                .iter()
                .map(|child| bounded_fill_element(render_node(child, inputs), count, is_row))
                .collect::<Vec<_>>();
            let spacing = bounded_spacing(f64::from(spacing.unwrap_or(0.0)), count);
            let layout: IceElement<'static, Output> = match axis {
                wire::Axis::Column => {
                    let mut column = widget::column(rendered).spacing(spacing);
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
                    column.into()
                }
                wire::Axis::Row => {
                    let mut row = widget::row(rendered).spacing(spacing);
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
                    row.into()
                }
            };
            accessible(
                widget::container(layout),
                StableId::new(key),
                Role::GenericContainer,
            )
            .logical_id_maybe(cfg!(test).then_some(key.as_str()))
            .into()
        }
        wire::Node::Scroll {
            key,
            direction,
            width,
            height,
            content,
        } => {
            let scrollbar = widget::scrollable::Scrollbar::new();
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
            let mut scroll = widget::scrollable(render_node(content, inputs))
                .id(widget::Id::from(key.clone()))
                .direction(direction);
            if let Some(width) = width {
                scroll = scroll.width(length(*width));
            }
            if let Some(height) = height {
                scroll = scroll.height(length(*height));
            }
            accessible(scroll, StableId::new(key), Role::ScrollView)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .into()
        }
        wire::Node::Text {
            key,
            content,
            size,
            color: fg,
            font: face,
            width,
            align_x,
        } => {
            let mut text = widget::text(content.clone()).font(font(*face));
            if let Some(size) = size {
                text = text.size(size.max(f32::EPSILON));
            }
            if let Some(fg) = fg {
                text = text.color(color(*fg));
            }
            if let Some(width) = width {
                text = text.width(length(*width));
            }
            if let Some(align) = align_x {
                text = text.align_x(horizontal(*align));
            }
            accessible(
                crate::selectable_text(text),
                StableId::new(key),
                Role::Label,
            )
            .logical_id_maybe(cfg!(test).then_some(key.as_str()))
            .value(content.clone())
            .into()
        }
        wire::Node::Input {
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
            let style = *style;
            let mut input = widget::text_input(placeholder, current)
                .id(widget::Id::from(key.clone()))
                .secure(*secure)
                .on_input(edit)
                .on_submit_maybe(on_submit.map(Output::Activate))
                .style(move |theme, status| input_style(style, theme, status));
            if let Some(width) = width {
                input = input.width(length(*width));
            }
            accessible(input, StableId::new(key), role)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .focus_id(widget::Id::from(key.clone()))
                .label(placeholder.clone())
                .value_maybe((!secure).then(|| current.to_owned()))
                .disabled(false)
                .into()
        }
        wire::Node::Button {
            key,
            content,
            label: name,
            on_press,
            width,
            height,
            padding: edges,
            style,
        } => {
            // The label fallback is only cloned into an owned `String` when
            // an explicit accessible `name` is absent — `name.clone()` wins
            // over it via `.or_else` whenever one is set, so a button that
            // names its own accessible label never pays for the fallback.
            let (label_fallback, inner): (Option<&str>, IceElement<'static, Output>) = match content
            {
                wire::ButtonContent::Label(label) => {
                    (Some(label.as_str()), widget::text(label.clone()).into())
                }
                wire::ButtonContent::Child(child) => (None, render_node(child, inputs)),
            };
            let label = name.clone().or_else(|| label_fallback.map(str::to_owned));
            let activate = on_press.map(Output::Activate);
            let style = *style;
            let mut button = widget::button(inner)
                .on_press_maybe(activate.clone())
                .style(move |theme, status| button_style(style, theme, status));
            if let Some(width) = width {
                button = button.width(length(*width));
            }
            if let Some(height) = height {
                button = button.height(length(*height));
            }
            if let Some(edges) = edges {
                button = button.padding(padding(*edges));
            }
            accessible(button, StableId::new(key), Role::Button)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .focus_id(widget::Id::from(key.clone()))
                .label(label.unwrap_or_default())
                .disabled(on_press.is_none())
                .on_activate_maybe(activate)
                .into()
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
        } => {
            let thickness = thickness.max(0.0);
            let fg = fg.map(color);
            let styled = move |theme: &iced::Theme| {
                let mut style = widget::rule::default(theme);
                if let Some(fg) = fg {
                    style.color = fg;
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(value: &str) -> wire::Node {
        wire::Node::Input {
            key: "App/draft".into(),
            placeholder: "What needs doing?".into(),
            value: value.into(),
            on_input: 0,
            on_submit: Some(1),
            width: None,
            secure: false,
            style: wire::InputStyle::default(),
        }
    }

    #[test]
    fn the_host_keeps_what_the_user_typed_until_the_guest_moves_the_value() {
        let mut inputs = Inputs::default();
        inputs.adopt(&input(""));
        let event = inputs.apply(Output::Edit {
            key: "App/draft".into(),
            handler: 0,
            text: "milk".into(),
        });
        assert_eq!(
            event,
            wire::Event::Input {
                handler: 0,
                text: "milk".into()
            }
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

    #[test]
    fn an_edit_inside_the_bound_is_untouched() {
        let mut inputs = Inputs::default();
        inputs.adopt(&input(""));
        let text = "milk and éclairs".to_string();
        let event = inputs.apply(Output::Edit {
            key: "App/draft".into(),
            handler: 0,
            text: text.clone(),
        });
        assert_eq!(
            event,
            wire::Event::Input {
                handler: 0,
                text: text.clone()
            }
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
        let event = inputs.apply(Output::Edit {
            key: "App/draft".into(),
            handler: 0,
            text: pasted,
        });
        let wire::Event::Input { text, .. } = event else {
            panic!("expected an Input event");
        };
        assert_eq!(text.len(), wire::MAX_STRING_BYTES - 1);
        assert!(text.is_char_boundary(text.len()));
        assert_eq!(text, prefix);
        // The host's own copy of the field agrees with what the guest heard.
        assert_eq!(inputs.text("App/draft", ""), prefix);
    }

    #[test]
    fn every_node_kind_renders() {
        let tree = wire::Node::Container {
            key: "App".into(),
            width: Some(wire::Length::Fill),
            height: Some(wire::Length::Fill),
            padding: Some(wire::Edges::all(8.0)),
            align_x: Some(wire::AlignX::Center),
            align_y: Some(wire::AlignY::Center),
            background: Some(wire::Rgba([0.0, 0.0, 0.0, 1.0])),
            border: None,
            content: Box::new(wire::Node::Linear {
                key: "App/content".into(),
                axis: wire::Axis::Column,
                spacing: Some(4.0),
                padding: None,
                width: None,
                height: None,
                align: Some(wire::AlignX::Left),
                children: vec![
                    wire::Node::Text {
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
                    wire::Node::Button {
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
                    },
                    wire::Node::Scroll {
                        key: "App/content/list".into(),
                        direction: wire::ScrollDirection::Vertical,
                        width: None,
                        height: None,
                        content: Box::new(wire::Node::Space {
                            width: None,
                            height: Some(wire::Length::Fixed(10.0)),
                        }),
                    },
                ],
            }),
        };
        let mut inputs = Inputs::default();
        inputs.adopt(&tree);
        let _element: IceElement<'static, Output> = render(&tree, &inputs);
    }
}
