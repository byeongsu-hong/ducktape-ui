//! The wire between a host and an Ice app running in wasm.
//!
//! The guest ships a WIDGET TREE, not a picture: every tick it returns the
//! [`Node`] its view built, with every value inlined — text, colours, sizes —
//! and the host's own toolkit does layout, render, fonts, IME, clipboard and
//! scroll. The guest never learns where anything landed, which is the point:
//! there is nothing in it to draw with.
//!
//! Interaction goes back as MEANING, not input. A button carries the index of
//! the message the guest queued for it this frame ([`Node::Button`]'s
//! `on_press`); the host sends [`Event::Message`] with that index and the
//! guest runs its own handler. A text field carries a handler index; the host
//! owns the text and sends [`Event::Input`] with what it now reads.
//!
//! The types here are the one definition of the format: the guest serializes
//! them and the host deserializes the same code, so a field neither side can
//! drop silently. A host that reads a frame from an untrusted module runs
//! [`sanitize`] first.

use serde::{Deserialize, Serialize};

/// Something the host tells the guest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    /// The user activated the widget the guest gave this message index to
    /// (a button press, an input submit). Indices are per frame: they name
    /// entries in the table the guest filled while building the tree it
    /// last sent.
    Message(u32),
    /// A text field's content changed. `handler` indexes the guest's
    /// per-frame input-handler table; `text` is the whole value the host now
    /// holds.
    Input { handler: u32, text: String },
    /// One answer to a [`Request`]. A one-shot request gets exactly one with
    /// `done`; a subscription gets many, the last one `done`.
    Response {
        id: u64,
        result: Result<Vec<u8>, String>,
        done: bool,
    },
}

/// Something the guest asked the host for. The guest never blocks on it: a
/// future (or stream) inside the guest waits for the matching
/// [`Event::Response`]s, which the host delivers on its own schedule.
///
/// `kind` is `<capability>.<operation>`; the host refuses a capability the
/// app's manifest did not declare.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    pub kind: String,
    pub payload: Vec<u8>,
}

/// What one tick of the guest produced.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The tree to show. `None` with `unchanged` set means "what you have".
    pub root: Option<Node>,
    /// What the guest asked for while producing this frame.
    pub requests: Vec<Request>,
    /// Requests the guest stopped waiting on — a dropped future or stream.
    /// The host frees whatever it kept for them and sends no more answers.
    pub cancels: Vec<u64>,
    /// `root` is `None` because the tree is the one the guest sent last:
    /// the host keeps what it has instead of decoding it again. Requests and
    /// cancels still cross.
    pub unchanged: bool,
}

/// Red, green, blue, alpha in `0.0..=1.0`. The guest resolves its own
/// palette; the host paints what it is told.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rgba(pub [f32; 4]);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Length {
    Fill,
    FillPortion(u16),
    Shrink,
    Fixed(f32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub const fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Border {
    pub color: Rgba,
    pub width: f32,
    /// top-left, top-right, bottom-right, bottom-left.
    pub radius: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AlignX {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AlignY {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Axis {
    Column,
    Row,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Weight {
    #[default]
    Normal,
    Medium,
    Semibold,
    Bold,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Font {
    pub monospace: bool,
    pub weight: Weight,
}

/// One state of a button or input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Face {
    pub background: Option<Rgba>,
    pub text: Option<Rgba>,
    pub border: Option<Border>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ButtonStyle {
    pub active: Face,
    pub hovered: Option<Face>,
    pub pressed: Option<Face>,
    pub disabled: Option<Face>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputFace {
    pub background: Option<Rgba>,
    pub border: Option<Border>,
    pub value: Option<Rgba>,
    pub placeholder: Option<Rgba>,
    pub selection: Option<Rgba>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputStyle {
    pub active: InputFace,
    pub hovered: Option<InputFace>,
    pub focused: Option<InputFace>,
    pub disabled: Option<InputFace>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ButtonContent {
    Label(String),
    Child(Box<Node>),
}

/// One widget. `key` is the node's identity across frames — the
/// accessibility path the compiler already computes (`App/content/count`)
/// — which the host uses for widget state (focus, caret, scroll) and for
/// the accessibility tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Node {
    Container {
        key: String,
        width: Option<Length>,
        height: Option<Length>,
        padding: Option<Edges>,
        align_x: Option<AlignX>,
        align_y: Option<AlignY>,
        background: Option<Rgba>,
        border: Option<Border>,
        content: Box<Node>,
    },
    Linear {
        key: String,
        axis: Axis,
        spacing: Option<f32>,
        padding: Option<Edges>,
        width: Option<Length>,
        height: Option<Length>,
        /// Cross-axis alignment of the children.
        align: Option<AlignX>,
        children: Vec<Node>,
    },
    Scroll {
        key: String,
        direction: ScrollDirection,
        width: Option<Length>,
        height: Option<Length>,
        content: Box<Node>,
    },
    Text {
        key: String,
        content: String,
        size: Option<f32>,
        color: Option<Rgba>,
        font: Font,
        width: Option<Length>,
        align_x: Option<AlignX>,
    },
    Input {
        key: String,
        placeholder: String,
        /// The guest's copy of the text. The host owns the live value and
        /// adopts this only when it differs from what the guest reported
        /// last frame.
        value: String,
        on_input: u32,
        on_submit: Option<u32>,
        width: Option<Length>,
        secure: bool,
        style: InputStyle,
    },
    Button {
        key: String,
        content: ButtonContent,
        /// `None` is a disabled button.
        on_press: Option<u32>,
        width: Option<Length>,
        height: Option<Length>,
        padding: Option<Edges>,
        style: ButtonStyle,
    },
    Space {
        width: Option<Length>,
        height: Option<Length>,
    },
    Rule {
        key: String,
        axis: Axis,
        thickness: f32,
        color: Option<Rgba>,
    },
}

/// Spends generic parameters on nothing: `<(&'a (), M, T) as Erase>::Node`
/// is [`Node`] for every `'a`, `M` and `T`. Generated code names its element
/// type `__IceElement<'a, Message, Theme>` for both targets, and a type
/// alias may not drop a parameter, so the tree target's alias projects
/// through this instead.
pub trait Erase {
    type Node;
}

impl<T: ?Sized> Erase for T {
    type Node = Node;
}

impl Node {
    /// The node an empty view renders as.
    pub fn empty() -> Self {
        Self::Space {
            width: None,
            height: None,
        }
    }

    pub fn key(&self) -> Option<&str> {
        match self {
            Self::Container { key, .. }
            | Self::Linear { key, .. }
            | Self::Scroll { key, .. }
            | Self::Text { key, .. }
            | Self::Input { key, .. }
            | Self::Button { key, .. }
            | Self::Rule { key, .. } => Some(key),
            Self::Space { .. } => None,
        }
    }

    fn children_mut(&mut self) -> Vec<&mut Node> {
        match self {
            Self::Container { content, .. } | Self::Scroll { content, .. } => vec![content],
            Self::Linear { children, .. } => children.iter_mut().collect(),
            Self::Button {
                content: ButtonContent::Child(child),
                ..
            } => vec![child],
            Self::Button { .. }
            | Self::Text { .. }
            | Self::Input { .. }
            | Self::Space { .. }
            | Self::Rule { .. } => Vec::new(),
        }
    }

    /// Every node in the tree, depth first, this one included.
    pub fn count(&self) -> usize {
        1 + match self {
            Self::Container { content, .. } | Self::Scroll { content, .. } => content.count(),
            Self::Linear { children, .. } => children.iter().map(Node::count).sum(),
            Self::Button {
                content: ButtonContent::Child(child),
                ..
            } => child.count(),
            Self::Button { .. }
            | Self::Text { .. }
            | Self::Input { .. }
            | Self::Space { .. }
            | Self::Rule { .. } => 0,
        }
    }
}

/// A tree deeper than this is cut off: a guest cannot make the host's
/// layout recurse without bound.
pub const MAX_DEPTH: usize = 64;
/// More nodes than this and the host stops reading: the widget tree of a
/// screen, not of a spreadsheet.
pub const MAX_NODES: usize = 8_192;
/// The longest string a single node may carry (text, placeholder, key).
pub const MAX_STRING_BYTES: usize = 64 << 10;
/// Text and spacing sizes are pixels; nothing on a screen needs more.
const MAX_PIXELS: f32 = 8192.0;

/// Pulls a frame from an untrusted module into what the host is willing to
/// lay out: the tree is truncated past [`MAX_DEPTH`] and [`MAX_NODES`],
/// strings past [`MAX_STRING_BYTES`], and every size, colour and spacing
/// clamped to a finite range. A frame from a well-behaved guest passes
/// through unchanged.
pub fn sanitize(frame: &mut Frame) {
    let mut budget = MAX_NODES;
    if let Some(root) = &mut frame.root {
        sanitize_node(root, 0, &mut budget);
    }
    for request in &mut frame.requests {
        truncate(&mut request.kind);
    }
}

fn sanitize_node(node: &mut Node, depth: usize, budget: &mut usize) {
    // The caller guarantees one node of budget; a node too deep spends it
    // on the empty node that stands in for it.
    *budget -= 1;
    if depth >= MAX_DEPTH {
        *node = Node::empty();
        return;
    }
    match node {
        Node::Container {
            key,
            padding,
            border,
            background,
            ..
        } => {
            truncate(key);
            bound_edges(padding);
            bound_border(border);
            bound_color(background);
        }
        Node::Linear {
            key,
            spacing,
            padding,
            ..
        } => {
            truncate(key);
            bound_optional(spacing);
            bound_edges(padding);
        }
        Node::Scroll { key, .. } => truncate(key),
        Node::Text {
            key,
            content,
            size,
            color,
            ..
        } => {
            truncate(key);
            truncate(content);
            bound_optional(size);
            bound_color(color);
        }
        Node::Input {
            key,
            placeholder,
            value,
            style,
            ..
        } => {
            truncate(key);
            truncate(placeholder);
            truncate(value);
            for face in [
                Some(&mut style.active),
                style.hovered.as_mut(),
                style.focused.as_mut(),
                style.disabled.as_mut(),
            ]
            .into_iter()
            .flatten()
            {
                bound_color(&mut face.background);
                bound_border(&mut face.border);
                bound_color(&mut face.value);
                bound_color(&mut face.placeholder);
                bound_color(&mut face.selection);
            }
        }
        Node::Button {
            key,
            content,
            padding,
            style,
            ..
        } => {
            truncate(key);
            if let ButtonContent::Label(label) = content {
                truncate(label);
            }
            bound_edges(padding);
            for face in [
                Some(&mut style.active),
                style.hovered.as_mut(),
                style.pressed.as_mut(),
                style.disabled.as_mut(),
            ]
            .into_iter()
            .flatten()
            {
                bound_color(&mut face.background);
                bound_color(&mut face.text);
                bound_border(&mut face.border);
            }
        }
        Node::Space { .. } => {}
        Node::Rule {
            key,
            thickness,
            color,
            ..
        } => {
            truncate(key);
            *thickness = bounded(*thickness);
            bound_color(color);
        }
    }
    for length in lengths_mut(node) {
        if let Length::Fixed(value) = length {
            *value = bounded(*value);
        }
    }
    // Children past the budget are dropped, not stood in for: a layout of
    // ten thousand rows becomes its first rows, which is what a host can
    // lay out, rather than ten thousand empty nodes it still has to walk.
    if let Node::Linear { children, .. } = node {
        let mut kept = 0;
        for child in children.iter_mut() {
            if *budget == 0 {
                break;
            }
            sanitize_node(child, depth + 1, budget);
            kept += 1;
        }
        children.truncate(kept);
        return;
    }
    for child in node.children_mut() {
        if *budget == 0 {
            *child = Node::empty();
            continue;
        }
        sanitize_node(child, depth + 1, budget);
    }
}

fn lengths_mut(node: &mut Node) -> Vec<&mut Length> {
    let slots: Vec<&mut Option<Length>> = match node {
        Node::Container { width, height, .. }
        | Node::Linear { width, height, .. }
        | Node::Scroll { width, height, .. }
        | Node::Button { width, height, .. }
        | Node::Space { width, height } => vec![width, height],
        Node::Text { width, .. } | Node::Input { width, .. } => vec![width],
        Node::Rule { .. } => Vec::new(),
    };
    slots.into_iter().flatten().collect()
}

fn truncate(text: &mut String) {
    if text.len() <= MAX_STRING_BYTES {
        return;
    }
    let mut end = MAX_STRING_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
}

fn bounded(value: f32) -> f32 {
    match value.is_nan() {
        true => 0.0,
        false => value.clamp(0.0, MAX_PIXELS),
    }
}

fn bound_optional(value: &mut Option<f32>) {
    if let Some(value) = value {
        *value = bounded(*value);
    }
}

fn bound_edges(edges: &mut Option<Edges>) {
    if let Some(edges) = edges {
        edges.top = bounded(edges.top);
        edges.right = bounded(edges.right);
        edges.bottom = bounded(edges.bottom);
        edges.left = bounded(edges.left);
    }
}

fn bound_color(color: &mut Option<Rgba>) {
    if let Some(Rgba(channels)) = color {
        for channel in channels {
            *channel = match channel.is_nan() {
                true => 0.0,
                false => channel.clamp(0.0, 1.0),
            };
        }
    }
}

fn bound_border(border: &mut Option<Border>) {
    if let Some(border) = border {
        let mut color = Some(border.color);
        bound_color(&mut color);
        border.color = color.expect("kept");
        border.width = bounded(border.width);
        for radius in &mut border.radius {
            *radius = bounded(*radius);
        }
    }
}

pub fn encode<T: Serialize>(value: &T) -> Vec<u8> {
    bincode::serialize(value).expect("wire types are plain data")
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, String> {
    bincode::deserialize(bytes).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(content: &str) -> Node {
        Node::Text {
            key: "App/t".into(),
            content: content.into(),
            size: Some(16.0),
            color: Some(Rgba([0.1, 0.2, 0.3, 1.0])),
            font: Font::default(),
            width: None,
            align_x: None,
        }
    }

    fn column(children: Vec<Node>) -> Node {
        Node::Linear {
            key: "App/col".into(),
            axis: Axis::Column,
            spacing: Some(8.0),
            padding: None,
            width: Some(Length::Fill),
            height: None,
            align: None,
            children,
        }
    }

    #[test]
    fn a_frame_round_trips() {
        let frame = Frame {
            root: Some(column(vec![
                text("hello"),
                Node::Button {
                    key: "App/b".into(),
                    content: ButtonContent::Label("Go".into()),
                    on_press: Some(3),
                    width: None,
                    height: None,
                    padding: Some(Edges::all(4.0)),
                    style: ButtonStyle::default(),
                },
                Node::Input {
                    key: "App/i".into(),
                    placeholder: "Name".into(),
                    value: "x".into(),
                    on_input: 0,
                    on_submit: Some(4),
                    width: Some(Length::Fixed(200.0)),
                    secure: false,
                    style: InputStyle::default(),
                },
            ])),
            requests: vec![Request {
                id: 1,
                kind: "host.echo".into(),
                payload: b"hi".to_vec(),
            }],
            cancels: vec![2],
            unchanged: false,
        };
        assert_eq!(decode::<Frame>(&encode(&frame)).unwrap(), frame);
        let events = vec![
            Event::Message(3),
            Event::Input {
                handler: 0,
                text: "xy".into(),
            },
            Event::Response {
                id: 1,
                result: Err("nope".into()),
                done: true,
            },
        ];
        assert_eq!(decode::<Vec<Event>>(&encode(&events)).unwrap(), events);
    }

    #[test]
    fn a_well_behaved_frame_is_untouched() {
        let mut frame = Frame {
            root: Some(column(vec![text("hello")])),
            ..Frame::default()
        };
        let before = frame.clone();
        sanitize(&mut frame);
        assert_eq!(frame, before);
    }

    #[test]
    fn a_hostile_frame_is_pulled_into_range() {
        let mut deep = text("leaf");
        for _ in 0..MAX_DEPTH + 10 {
            deep = column(vec![deep]);
        }
        let wide = column((0..MAX_NODES + 5).map(|_| text("x")).collect());
        let mut frame = Frame {
            root: Some(column(vec![
                Node::Text {
                    key: "k".repeat(MAX_STRING_BYTES + 3),
                    content: "é".repeat(MAX_STRING_BYTES),
                    size: Some(f32::NAN),
                    color: Some(Rgba([2.0, -1.0, f32::INFINITY, 0.5])),
                    font: Font::default(),
                    width: Some(Length::Fixed(-5.0)),
                    align_x: None,
                },
                deep,
                wide,
            ])),
            ..Frame::default()
        };
        sanitize(&mut frame);
        let root = frame.root.unwrap();
        // A container whose child fell past the budget keeps an empty
        // stand-in, one per level at most.
        assert!(root.count() <= MAX_NODES + MAX_DEPTH, "{}", root.count());
        let Node::Linear { children, .. } = &root else {
            panic!()
        };
        let Node::Text {
            key,
            content,
            size,
            color,
            width,
            ..
        } = &children[0]
        else {
            panic!("{:?}", children[0])
        };
        assert_eq!(key.len(), MAX_STRING_BYTES);
        assert!(content.len() <= MAX_STRING_BYTES && content.is_char_boundary(content.len()));
        assert_eq!(*size, Some(0.0));
        assert_eq!(*color, Some(Rgba([1.0, 0.0, 1.0, 0.5])));
        assert_eq!(*width, Some(Length::Fixed(0.0)));
    }

    #[test]
    fn depth_is_cut_before_the_host_recurses_into_it() {
        let mut deep = text("leaf");
        for _ in 0..MAX_DEPTH * 2 {
            deep = column(vec![deep]);
        }
        let mut frame = Frame {
            root: Some(deep),
            ..Frame::default()
        };
        sanitize(&mut frame);
        let mut depth = 0;
        let mut node = frame.root.as_ref().unwrap();
        while let Node::Linear { children, .. } = node {
            depth += 1;
            node = &children[0];
        }
        assert!(depth <= MAX_DEPTH, "{depth}");
    }
}
