//! The published form of an Ice view.
//!
//! A view compiles to two halves. The static half — widget structure, literals,
//! style tables, geometry, accessibility segments and source coordinates —
//! becomes the [`Node`] tree in this crate, published as JSON rather than Rust.
//! The dynamic half — anything that reads application state or names a message
//! — stays compiled and reaches the renderer through positional slot tables,
//! one per kind of value, each addressed by its own index type.
//!
//! This crate is the one definition of that format. The code generator writes
//! it and the runtime reads it, so a field neither side can drop silently: the
//! producer and the consumer are the same types. Nothing here depends on a
//! widget toolkit; rendering lives in `ui_lang_runtime::template`.
//!
//! The modelled vocabulary is layouts, containers, text, inputs, and buttons.
//! Anything else becomes a [`Node::Subtree`] hole the compiler fills through
//! the slot table, so an unmodelled construct costs only its own subtree its
//! reloadability rather than costing the whole view its template. The `if`,
//! `for` and `match` that a layout splices into its child list get a
//! [`Node::Group`] instead, which holds however many children the frame built
//! rather than exactly one.

use serde::{Deserialize, Serialize};

/// A position in the table of strings the view computes each frame.
///
/// The five slot kinds below index five separate tables rather than one, and
/// each is a distinct type, so a node that names the wrong kind of slot fails
/// to compile. When they shared a table an emitter could point a button's
/// activation at a string, and nothing said so until someone noticed the
/// button had stopped working.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TextSlot(pub usize);

/// A position in the table of strings borrowed from application state.
///
/// Separate from [`TextSlot`] because the lifetime differs, not just the
/// contents: a text input's value is borrowed for as long as the element
/// lives, so it cannot come from a string this frame computed and dropped.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StateSlot(pub usize);

/// A position in the table of messages delivered on activation.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MessageSlot(pub usize);

/// A position in the table of message constructors for widgets that report a
/// value, such as the two-way binding behind `<->`.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct HandlerSlot(pub usize);

/// A position in the table of compiled subtrees that fill the template's holes.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SubtreeSlot(pub usize);

/// A position in the table of compiled *child lists*.
///
/// Separate from [`SubtreeSlot`] because the count is not known until the
/// frame runs: `if`, `for` and `match` contribute however many children their
/// condition, iteration or arm produces. A subtree hole holds exactly one
/// element and cannot express that, which is why a layout containing one used
/// to become the hole itself.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GroupSlot(pub usize);

/// How many slots of each kind a view expects.
///
/// This is the reload contract. A running process can accept any template
/// whose tables its compiled `__view` still fills; the moment a view wants a
/// slot the binary does not compute, only a rebuild can supply it.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct SlotCounts {
    #[serde(default)]
    pub texts: usize,
    #[serde(default)]
    pub states: usize,
    #[serde(default)]
    pub messages: usize,
    #[serde(default)]
    pub handlers: usize,
    #[serde(default)]
    pub subtrees: usize,
    #[serde(default)]
    pub groups: usize,
}

/// A string that is either baked into the template or supplied by a slot.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Value {
    Literal(String),
    Slot(TextSlot),
}

/// A color drawn from the app's compiled palette, optionally faded.
///
/// Palettes stay compiled: they are fixed-size arrays whose type changes with
/// the token list, so the template refers to them by index the same way the
/// inline path does.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct ColorRef {
    pub index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f32>,
}

/// A width or height. Mirrors the `w=`/`h=` vocabulary.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Size {
    Fill,
    Shrink,
    Fixed(f32),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlignX {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlignY {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Edges {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

/// The visual properties a button status can override.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ButtonFace {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<ColorRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<ColorRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f32>,
}

/// Per-status button styling, flattened the same way the inline path flattens
/// `active`/`hovered`/`pressed` into one closure.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ButtonStyle {
    #[serde(default)]
    pub active: ButtonFace,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hovered: Option<ButtonFace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

impl A11y {
    /// This node's own accessibility path.
    pub fn key(&self, parent: &str) -> String {
        format!("{parent}/{}", self.segment)
    }

    /// The path descendants hang off.
    pub fn scope<'a>(&self, parent: &'a str, key: &'a str) -> &'a str {
        if self.named { key } else { parent }
    }
}

/// A `.ice` coordinate, with its file named indirectly.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct Source {
    pub path: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Node {
    Container {
        a11y: A11y,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<Size>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<Size>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        padding: Option<Edges>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        align_x: Option<AlignX>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        align_y: Option<AlignY>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        background: Option<ColorRef>,
        content: Box<Node>,
    },
    Linear {
        a11y: A11y,
        axis: Axis,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        spacing: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        padding: Option<Edges>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<Size>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<Size>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        align_x: Option<AlignX>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        align_y: Option<AlignY>,
        children: Vec<Node>,
    },
    Text {
        a11y: A11y,
        value: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<ColorRef>,
    },
    Input {
        a11y: A11y,
        label: String,
        /// The state this input edits, borrowed for the element's lifetime.
        value: StateSlot,
        /// The constructor an edit routes through.
        on_input: HandlerSlot,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<Size>,
        #[serde(default)]
        secure: bool,
    },
    Button {
        a11y: A11y,
        label: String,
        /// The message an activation delivers.
        on_press: MessageSlot,
        #[serde(default)]
        style: ButtonStyle,
    },
    /// A hole the compiler fills, holding a construct the template vocabulary
    /// does not model. Everything around it still reloads; what is inside
    /// changes only when the binary does.
    Subtree { slot: SubtreeSlot },
    /// A hole that contributes however many children the frame produces, for
    /// the `if`, `for` and `match` that a layout splices into its own child
    /// list.
    ///
    /// Only [`Node::Linear`] expands one: it is the only node whose child
    /// count is a list rather than a single content. Anywhere else a group
    /// renders as nothing, which is the same treatment an out-of-range slot
    /// gets — those states mean a stale template mid-reload, not a view worth
    /// taking the window down for.
    Group { slot: GroupSlot },
}

/// Stands in for a compiled subtree, which composes its own accessibility path
/// and pushes its own source location.
static EMPTY_A11Y: A11y = A11y {
    segment: String::new(),
    named: false,
    source: None,
};

impl Node {
    /// This node's accessibility identity.
    pub fn a11y(&self) -> &A11y {
        match self {
            Self::Container { a11y, .. }
            | Self::Linear { a11y, .. }
            | Self::Text { a11y, .. }
            | Self::Input { a11y, .. }
            | Self::Button { a11y, .. } => a11y,
            // Compiled children carry their own identity and provenance.
            Self::Subtree { .. } | Self::Group { .. } => &EMPTY_A11Y,
        }
    }
}

/// A whole view: the node tree plus the slots it expects of each kind.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Template {
    pub root: Node,
    pub slots: SlotCounts,
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

/// Reports whether a compiled process can accept `candidate` without being
/// rebuilt: the slot table it fills each frame must still line up.
///
/// This is the whole reload decision. Structure, literals, colors, spacing and
/// accessibility segments may all change freely; the moment a view needs a
/// slot the binary does not have, only a rebuild can supply it.
pub fn accepts(compiled: &Template, candidate: &Template) -> bool {
    compiled.slots == candidate.slots && slot_uses(&candidate.root, compiled.slots)
}

fn slot_uses(node: &Node, available: SlotCounts) -> bool {
    let value_ok = |value: &Value| match value {
        Value::Literal(_) => true,
        Value::Slot(TextSlot(index)) => *index < available.texts,
    };
    match node {
        Node::Container { content, .. } => slot_uses(content, available),
        Node::Linear { children, .. } => children.iter().all(|child| slot_uses(child, available)),
        Node::Text { value, .. } => value_ok(value),
        Node::Input {
            value: StateSlot(value),
            on_input: HandlerSlot(on_input),
            ..
        } => *value < available.states && *on_input < available.handlers,
        Node::Button {
            on_press: MessageSlot(on_press),
            ..
        } => *on_press < available.messages,
        Node::Subtree {
            slot: SubtreeSlot(slot),
        } => *slot < available.subtrees,
        Node::Group {
            slot: GroupSlot(slot),
        } => *slot < available.groups,
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

    fn template(root: Node, texts: usize) -> Template {
        Template {
            root,
            slots: SlotCounts {
                texts,
                ..SlotCounts::default()
            },
        }
    }

    #[test]
    fn json_round_trips() {
        let original = template(
            Node::Linear {
                a11y: a11y("content"),
                axis: Axis::Column,
                spacing: Some(16.0),
                padding: None,
                width: Some(Size::Fill),
                height: None,
                align_x: Some(AlignX::Center),
                align_y: None,
                children: vec![
                    text_node("title", Value::Literal("Ice".into())),
                    text_node("count", Value::Slot(TextSlot(0))),
                ],
            },
            1,
        );
        let json = original.to_json().expect("template serializes");
        assert_eq!(Template::from_json(&json).expect("parses"), original);
    }

    #[test]
    fn unset_options_are_omitted_rather_than_written_null() {
        // Codegen emits one of these per view, so an absent option costs no
        // bytes. Deserialization has to supply the default in its place, which
        // is what lets the two sides share one definition.
        let json = template(text_node("count", Value::Slot(TextSlot(0))), 1)
            .to_json()
            .expect("template serializes");
        assert!(!json.contains("null"), "{json}");
        assert!(!json.contains("size"), "{json}");
    }

    #[test]
    fn reload_accepts_only_a_satisfiable_slot_table() {
        let compiled = template(text_node("count", Value::Slot(TextSlot(0))), 1);

        // Restructuring and re-literalling the same slots is reloadable.
        let restructured = template(
            Node::Linear {
                a11y: a11y("content"),
                axis: Axis::Row,
                spacing: Some(8.0),
                padding: None,
                width: None,
                height: None,
                align_x: None,
                align_y: None,
                children: vec![
                    text_node("label", Value::Literal("Total".into())),
                    text_node("count", Value::Slot(TextSlot(0))),
                ],
            },
            1,
        );
        assert!(accepts(&compiled, &restructured));

        // Needing a slot the binary does not fill requires a rebuild.
        let extra_slot = template(text_node("count", Value::Slot(TextSlot(1))), 2);
        assert!(!accepts(&compiled, &extra_slot));
    }

    #[test]
    fn slot_kinds_are_counted_separately() {
        // One flat count could be satisfied by the wrong kinds adding up to the
        // right total. Per-kind counts cannot: a view wanting a message where
        // the binary computes a string is refused even though both tables hold
        // one slot between them.
        let compiled = Template {
            root: text_node("count", Value::Slot(TextSlot(0))),
            slots: SlotCounts {
                texts: 1,
                ..SlotCounts::default()
            },
        };
        let wants_a_message = Template {
            root: Node::Button {
                a11y: a11y("bump"),
                label: "Bump".into(),
                on_press: MessageSlot(0),
                style: ButtonStyle::default(),
            },
            slots: SlotCounts {
                messages: 1,
                ..SlotCounts::default()
            },
        };
        assert!(!accepts(&compiled, &wants_a_message));
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
}
