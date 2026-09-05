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
    #[serde(deserialize_with = "decode_child")]
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
        #[serde(deserialize_with = "decode_child")]
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
        #[serde(deserialize_with = "decode_children")]
        children: Vec<Node>,
    },
    Scroll {
        key: String,
        direction: ScrollDirection,
        width: Option<Length>,
        height: Option<Length>,
        #[serde(deserialize_with = "decode_child")]
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
        /// The accessible name of a button whose content is not a plain
        /// label.
        label: Option<String>,
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
/// A text size, which is not a length: every glyph at it is rasterized and
/// cached, so a screenful of 8192 px text is an atlas no screen asked for.
const MAX_TEXT_PIXELS: f32 = 512.0;

/// Pulls a frame from an untrusted module into what the host is willing to
/// lay out: the tree is truncated past [`MAX_DEPTH`] and [`MAX_NODES`],
/// strings past [`MAX_STRING_BYTES`], text sizes to [`MAX_TEXT_PIXELS`],
/// every other size, colour and spacing clamped to a finite range, and a key
/// used twice moved off the one already taken. A frame from a well-behaved
/// guest passes through unchanged.
///
/// A frame that arrived as bytes has passed [`decode`] first, which refuses
/// one nested deeper than this walk goes.
pub fn sanitize(frame: &mut Frame) {
    let mut budget = MAX_NODES;
    let mut taken = Taken::new();
    if let Some(root) = &mut frame.root {
        sanitize_node(root, 0, &mut budget, &mut taken);
    }
    for request in &mut frame.requests {
        truncate(&mut request.kind);
    }
}

/// Every key claimed in one tree, each with the suffix its next duplicate
/// will try. Remembering the suffix is what keeps a tree of one key linear:
/// searching upward from `#2` on every duplicate walks past every earlier
/// one, and a screen of eight thousand nodes sharing a key — the guest's to
/// send — took nine seconds of the window thread that way.
type Taken = std::collections::HashMap<String, usize>;

/// A key already used in this tree, made unique. A key is the node's
/// identity — its widget state, its focus target, its accessibility id, and
/// the [`Node::Input`] whose text the host owns — so two nodes sharing one
/// share all of that: typing in either edits both. The tree the guest sent
/// is kept, with the later node moved off the taken key.
fn claim(key: &mut String, taken: &mut Taken) {
    truncate(key);
    let Some(mut nth) = taken.get(key.as_str()).copied() else {
        taken.insert(key.clone(), 2);
        return;
    };
    // The guest may itself have sent `key#2`: a suffix already taken is
    // skipped, and the count moves past it for good.
    let mut unique = format!("{key}#{nth}");
    while taken.contains_key(unique.as_str()) {
        nth += 1;
        unique = format!("{key}#{nth}");
    }
    taken.insert(std::mem::replace(key, unique.clone()), nth + 1);
    taken.insert(unique, 2);
}

fn sanitize_node(node: &mut Node, depth: usize, budget: &mut usize, taken: &mut Taken) {
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
            claim(key, taken);
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
            claim(key, taken);
            bound_optional(spacing);
            bound_edges(padding);
        }
        Node::Scroll { key, .. } => claim(key, taken),
        Node::Text {
            key,
            content,
            size,
            color,
            ..
        } => {
            claim(key, taken);
            truncate(content);
            if let Some(size) = size {
                *size = bounded(*size).min(MAX_TEXT_PIXELS);
            }
            bound_color(color);
        }
        Node::Input {
            key,
            placeholder,
            value,
            style,
            ..
        } => {
            claim(key, taken);
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
            label,
            padding,
            style,
            ..
        } => {
            claim(key, taken);
            if let ButtonContent::Label(label) = content {
                truncate(label);
            }
            if let Some(label) = label {
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
            claim(key, taken);
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
            sanitize_node(child, depth + 1, budget, taken);
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
        sanitize_node(child, depth + 1, budget, taken);
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

/// What one `decode` may build before it is refused: enough that
/// [`sanitize`]'s truncation still shapes any tree a real view sends, and
/// few enough that a hostile one cannot make the host allocate its way
/// through a frame's worth of nodes every tick.
const MAX_DECODED_NODES: usize = 16 * MAX_NODES;

/// Bounds what a decode may descend into, since decoding is recursive: a
/// [`Node`] holds its children and serde builds them from the inside out, so
/// a chain of containers is a chain of stack frames. The tree the host walks
/// afterwards — [`sanitize`], the renderer, `Drop` — recurses the same way,
/// which is why the limit is the door rather than each walk.
///
/// [`MAX_FRAME_BYTES`-sized](Frame) input is no protection: a chain deep
/// enough to overflow a host thread's stack is a few tens of kilobytes.
mod budget {
    use std::cell::Cell;

    use super::{MAX_DECODED_NODES, MAX_DEPTH};

    thread_local! {
        static DEPTH: Cell<usize> = const { Cell::new(0) };
        static NODES: Cell<usize> = const { Cell::new(0) };
    }

    /// One node being decoded. Descending past what the host walks, or
    /// building more nodes than it will hold, refuses the whole frame:
    /// there is no partial tree to keep, and a truncated one would be a
    /// tree the guest did not write.
    pub(super) struct Node(());

    impl Node {
        pub(super) fn enter() -> Result<Self, &'static str> {
            let depth = DEPTH.get() + 1;
            if depth > MAX_DEPTH {
                return Err("a tree deeper than the host renders");
            }
            let nodes = NODES.get() + 1;
            if nodes > MAX_DECODED_NODES {
                return Err("more nodes than the host holds");
            }
            DEPTH.set(depth);
            NODES.set(nodes);
            Ok(Self(()))
        }
    }

    impl Drop for Node {
        fn drop(&mut self) {
            DEPTH.set(DEPTH.get().saturating_sub(1));
        }
    }

    /// A fresh budget for one top-level [`decode`](super::decode). The depth
    /// unwinds itself; the node count is what one frame may spend.
    pub(super) fn reset() {
        NODES.set(0);
    }
}

fn decode_child<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Box<Node>, D::Error> {
    let _node = budget::Node::enter().map_err(serde::de::Error::custom)?;
    Box::<Node>::deserialize(deserializer)
}

fn decode_children<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Node>, D::Error> {
    struct Children;

    impl<'de> serde::de::Visitor<'de> for Children {
        type Value = Vec<Node>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a list of nodes")
        }

        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut children: A,
        ) -> Result<Self::Value, A::Error> {
            let mut nodes = Vec::new();
            // The guard is held while one child is built and dropped before
            // the next: siblings share a depth, and each costs a node.
            while let Some(child) = {
                let _node = budget::Node::enter().map_err(serde::de::Error::custom)?;
                children.next_element::<Node>()?
            } {
                nodes.push(child);
            }
            Ok(nodes)
        }
    }

    deserializer.deserialize_seq(Children)
}

pub fn encode<T: Serialize>(value: &T) -> Vec<u8> {
    bincode::serialize(value).expect("wire types are plain data")
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, String> {
    budget::reset();
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
                    label: None,
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

    fn button(content: ButtonContent) -> Node {
        Node::Button {
            key: "App/b".into(),
            content,
            label: None,
            on_press: Some(1),
            width: None,
            height: None,
            padding: None,
            style: ButtonStyle::default(),
        }
    }

    /// A button holding a node is the third way the tree recurses, and the
    /// only one that hangs off an enum's field rather than a struct's.
    #[test]
    fn a_button_holding_a_node_round_trips_and_counts_as_a_child() {
        let frame = Frame {
            root: Some(button(ButtonContent::Child(Box::new(text("inside"))))),
            ..Frame::default()
        };
        assert_eq!(decode::<Frame>(&encode(&frame)).unwrap(), frame);

        let mut nested = Node::empty();
        for _ in 0..MAX_DEPTH + 1 {
            nested = button(ButtonContent::Child(Box::new(nested)));
        }
        let bytes = encode(&Frame {
            root: Some(nested),
            ..Frame::default()
        });
        assert!(decode::<Frame>(&bytes).is_err());
    }

    /// Building and encoding a chain this deep recurses as far as decoding
    /// it would, so the hostile frame is made where there is stack for it.
    fn deep_chain_bytes(depth: usize) -> Vec<u8> {
        std::thread::Builder::new()
            .stack_size(512 << 20)
            .spawn(move || {
                let mut node = Node::empty();
                for _ in 0..depth {
                    node = column(vec![node]);
                }
                let frame = Frame {
                    root: Some(node),
                    ..Frame::default()
                };
                let bytes = encode(&frame);
                // Dropping it recurses too, and this thread is the one with
                // the stack to do it.
                drop(frame);
                bytes
            })
            .expect("hostile frame thread")
            .join()
            .expect("hostile frame")
    }

    #[test]
    fn a_tree_the_host_would_not_walk_is_refused_before_it_is_built() {
        assert!(decode::<Frame>(&deep_chain_bytes(MAX_DEPTH - 1)).is_ok());
        let refused = decode::<Frame>(&deep_chain_bytes(MAX_DEPTH + 1)).unwrap_err();
        assert!(
            refused.contains("deeper than the host renders"),
            "{refused}"
        );
    }

    /// The bug this guards: a chain of a few thousand containers is a frame
    /// of ~100 KB — far inside any byte cap a host sets — and decoding it
    /// walked a host thread off its stack, aborting the process. A refusal
    /// is a message in one app's window; an overflow is every window gone.
    #[test]
    fn a_chain_that_overflowed_the_host_stack_is_an_error_not_a_crash() {
        let bytes = deep_chain_bytes(5_000);
        assert!(bytes.len() < 1 << 20, "{} bytes", bytes.len());
        assert!(decode::<Frame>(&bytes).is_err());
    }

    #[test]
    fn more_nodes_than_the_host_holds_is_refused() {
        let wide = column((0..MAX_DECODED_NODES + 2).map(|_| Node::empty()).collect());
        let bytes = encode(&Frame {
            root: Some(wide),
            ..Frame::default()
        });
        let refused = decode::<Frame>(&bytes).unwrap_err();
        assert!(
            refused.contains("more nodes than the host holds"),
            "{refused}"
        );
    }

    /// A frame is bytes a module wrote, so every byte of it is the guest's
    /// to choose. Whatever they say, `decode` answers rather than aborts.
    #[test]
    fn bytes_a_hostile_guest_could_write_are_answered_not_survived() {
        let sound = encode(&Frame {
            root: Some(column(vec![text("hello"), Node::empty()])),
            requests: vec![Request {
                id: 7,
                kind: "host.echo".into(),
                payload: b"hi".to_vec(),
            }],
            cancels: vec![1, 2],
            unchanged: false,
        });
        for cut in 0..sound.len() {
            let _ = decode::<Frame>(&sound[..cut]);
        }
        for at in 0..sound.len() {
            for bit in 0..8 {
                let mut flipped = sound.clone();
                flipped[at] ^= 1 << bit;
                let _ = decode::<Frame>(&flipped);
            }
        }
    }

    #[test]
    fn a_key_used_twice_is_moved_off_the_one_already_taken() {
        let mut frame = Frame {
            root: Some(column(vec![text("one"), text("two"), text("three")])),
            ..Frame::default()
        };
        sanitize(&mut frame);
        let Some(Node::Linear { children, .. }) = &frame.root else {
            panic!()
        };
        let keys: Vec<&str> = children.iter().filter_map(Node::key).collect();
        assert_eq!(keys, ["App/t", "App/t#2", "App/t#3"]);
    }

    /// A screen where every node claims the same key, and one where the
    /// guest pre-empted the suffixes: each duplicate must cost one lookup,
    /// not a walk past every earlier one. Measured, not asserted, in debug;
    /// in release a quadratic claim took nine seconds here and a linear one
    /// takes a few milliseconds.
    #[test]
    fn a_screen_of_one_key_is_claimed_in_linear_time() {
        let mut same = text("x");
        let Node::Text { key, .. } = &mut same else {
            panic!()
        };
        *key = "App/t".into();
        let mut children: Vec<Node> = (0..MAX_NODES - 1).map(|_| same.clone()).collect();
        // The guest sent `App/t#2` and `App/t#3` itself: the count skips them.
        for (child, taken) in children.iter_mut().zip(["App/t#2", "App/t#3"]) {
            let Node::Text { key, .. } = child else {
                panic!()
            };
            *key = taken.into();
        }
        let mut frame = Frame {
            root: Some(column(children)),
            ..Frame::default()
        };
        let started = std::time::Instant::now();
        sanitize(&mut frame);
        let took = started.elapsed();
        let Some(Node::Linear { children, .. }) = &frame.root else {
            panic!()
        };
        let keys: std::collections::HashSet<&str> = children.iter().filter_map(Node::key).collect();
        assert_eq!(keys.len(), children.len(), "every key unique");
        assert_eq!(children[2].key(), Some("App/t"));
        assert_eq!(children[3].key(), Some("App/t#4"));
        if cfg!(not(debug_assertions)) {
            assert!(took < std::time::Duration::from_millis(200), "{took:?}");
        }
    }

    #[test]
    fn a_text_size_is_capped_where_a_length_is_not() {
        let mut huge = text("huge");
        let Node::Text { size, width, .. } = &mut huge else {
            panic!()
        };
        *size = Some(f32::MAX);
        *width = Some(Length::Fixed(f32::MAX));
        let mut frame = Frame {
            root: Some(huge),
            ..Frame::default()
        };
        sanitize(&mut frame);
        let Some(Node::Text { size, width, .. }) = &frame.root else {
            panic!()
        };
        assert_eq!(*size, Some(MAX_TEXT_PIXELS));
        assert_eq!(*width, Some(Length::Fixed(MAX_PIXELS)));
    }
}
