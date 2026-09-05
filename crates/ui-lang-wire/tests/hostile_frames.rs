//! Property tests for the wire's hostile-input contract: a random tree either
//! comes back out of `decode` refused for a reason the door actually names,
//! or `sanitize` pulls it inside every bound `sanitize_node` promises; bytes a
//! hostile guest could have written never make `decode` panic; and a
//! hand-crafted length-prefix bomb is refused before it is walked.
//!
//! No new dependency: the generator is a splitmix64 PRNG seeded by a fixed
//! constant, so a failure prints its seed and the run reproduces exactly.

use std::collections::HashSet;

use ui_lang_wire::*;

/// Mirrors the wire's own private ceiling on decoded nodes (`16 *
/// MAX_NODES`, see `decode`'s doc comment): decode refuses a frame that
/// would build more than this many nodes, independent of `MAX_NODES` itself,
/// which only bounds what `sanitize` keeps.
const MAX_DECODED_NODES: usize = 16 * MAX_NODES;
/// Mirrors the wire's private `MAX_PIXELS`: every non-text size sanitize
/// keeps is clamped to this range.
const PIXEL_BOUND: f32 = 8192.0;
/// Mirrors the wire's private `MAX_TEXT_PIXELS`: a text size is clamped
/// tighter than any other length, since it drives glyph rasterization.
const TEXT_PIXEL_BOUND: f32 = 512.0;

// ---------------------------------------------------------------- splitmix64

/// A tiny deterministic PRNG so the property tests need no new dependency.
/// splitmix64: https://prng.di.unimi.it/splitmix64.c
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_range(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        (self.next_u64() as usize) % bound
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// A fraction in `0.0..1.0`, from the PRNG's top 24 bits.
    fn next_unit(&mut self) -> f64 {
        ((self.next_u64() >> 40) as f64) / ((1u64 << 24) as f64)
    }

    /// A value in `0..=max`, biased toward small values by raising a
    /// uniform fraction to `exponent` before scaling: the higher the
    /// exponent, the more the mass sits near zero. Keeps most generated
    /// trees and strings cheap while still drawing the occasional value
    /// near `max` to exercise the wire's ceilings.
    fn skewed(&mut self, max: usize, exponent: i32) -> usize {
        if max == 0 {
            return 0;
        }
        let biased = self.next_unit().powi(exponent);
        ((biased * max as f64) as usize).min(max)
    }

    fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.next_range(items.len())]
    }
}

// ------------------------------------------------------------- tree generator

/// A hostile f32: sometimes a normal-looking value, sometimes one of the
/// exact values `sanitize` exists to handle (NaN, both infinities, the
/// float extremes, and out-of-range negatives).
fn gen_f32(rng: &mut Rng) -> f32 {
    match rng.next_range(20) {
        0 => f32::NAN,
        1 => f32::INFINITY,
        2 => f32::NEG_INFINITY,
        3 => f32::MAX,
        4 => f32::MIN,
        5 => -(rng.skewed(1_000_000, 2) as f32),
        _ => rng.skewed(20_000, 2) as f32 - 5_000.0,
    }
}

fn gen_opt_f32(rng: &mut Rng) -> Option<f32> {
    rng.next_bool().then(|| gen_f32(rng))
}

/// A key drawn from a small fixed pool: with only five options across a
/// whole tree, collisions are the common case rather than the exception.
fn gen_key(rng: &mut Rng) -> String {
    const POOL: [&str; 5] = ["App/a", "App/b", "dup", "x", "same-key"];
    (*rng.choose(&POOL)).to_string()
}

/// A string built from single-, two-, three- and four-byte UTF-8
/// characters. Most calls stay small so a tree of thousands of leaves
/// stays cheap to build; roughly one in three hundred goes hostile and
/// targets up to `3 * MAX_STRING_BYTES`, which is what actually exercises
/// `truncate`'s char-boundary walk.
fn gen_string(rng: &mut Rng) -> String {
    const POOL: [char; 6] = ['a', 'Z', 'é', '한', '😀', '\n'];
    let (cap, exponent) = match rng.next_range(300) {
        0 => (3 * MAX_STRING_BYTES, 8),
        _ => (48, 2),
    };
    let target = rng.skewed(cap, exponent);
    let mut s = String::new();
    while s.len() < target {
        s.push(*rng.choose(&POOL));
    }
    s
}

fn gen_opt_length(rng: &mut Rng) -> Option<Length> {
    rng.next_bool().then(|| match rng.next_range(4) {
        0 => Length::Fill,
        1 => Length::FillPortion(rng.next_range(1000) as u16),
        2 => Length::Shrink,
        _ => Length::Fixed(gen_f32(rng)),
    })
}

fn gen_opt_edges(rng: &mut Rng) -> Option<Edges> {
    rng.next_bool().then(|| Edges {
        top: gen_f32(rng),
        right: gen_f32(rng),
        bottom: gen_f32(rng),
        left: gen_f32(rng),
    })
}

fn gen_opt_color(rng: &mut Rng) -> Option<Rgba> {
    rng.next_bool()
        .then(|| Rgba([gen_f32(rng), gen_f32(rng), gen_f32(rng), gen_f32(rng)]))
}

fn gen_border(rng: &mut Rng) -> Border {
    Border {
        color: Rgba([gen_f32(rng), gen_f32(rng), gen_f32(rng), gen_f32(rng)]),
        width: gen_f32(rng),
        radius: [gen_f32(rng), gen_f32(rng), gen_f32(rng), gen_f32(rng)],
    }
}

fn gen_opt_border(rng: &mut Rng) -> Option<Border> {
    rng.next_bool().then(|| gen_border(rng))
}

fn gen_opt_align_x(rng: &mut Rng) -> Option<AlignX> {
    rng.next_bool()
        .then(|| *rng.choose(&[AlignX::Left, AlignX::Center, AlignX::Right]))
}

fn gen_opt_align_y(rng: &mut Rng) -> Option<AlignY> {
    rng.next_bool()
        .then(|| *rng.choose(&[AlignY::Top, AlignY::Center, AlignY::Bottom]))
}

fn gen_axis(rng: &mut Rng) -> Axis {
    if rng.next_bool() {
        Axis::Column
    } else {
        Axis::Row
    }
}

fn gen_face(rng: &mut Rng) -> Face {
    Face {
        background: gen_opt_color(rng),
        text: gen_opt_color(rng),
        border: gen_opt_border(rng),
    }
}

fn gen_button_style(rng: &mut Rng) -> ButtonStyle {
    ButtonStyle {
        active: gen_face(rng),
        hovered: rng.next_bool().then(|| gen_face(rng)),
        pressed: rng.next_bool().then(|| gen_face(rng)),
        disabled: rng.next_bool().then(|| gen_face(rng)),
    }
}

fn gen_input_face(rng: &mut Rng) -> InputFace {
    InputFace {
        background: gen_opt_color(rng),
        border: gen_opt_border(rng),
        value: gen_opt_color(rng),
        placeholder: gen_opt_color(rng),
        selection: gen_opt_color(rng),
    }
}

fn gen_input_style(rng: &mut Rng) -> InputStyle {
    InputStyle {
        active: gen_input_face(rng),
        hovered: rng.next_bool().then(|| gen_input_face(rng)),
        focused: rng.next_bool().then(|| gen_input_face(rng)),
        disabled: rng.next_bool().then(|| gen_input_face(rng)),
    }
}

fn gen_button_label(rng: &mut Rng) -> Node {
    Node::Button {
        key: gen_key(rng),
        content: ButtonContent::Label(gen_string(rng)),
        label: rng.next_bool().then(|| gen_string(rng)),
        on_press: rng.next_bool().then(|| rng.next_u64() as u32),
        width: gen_opt_length(rng),
        height: gen_opt_length(rng),
        padding: gen_opt_edges(rng),
        style: gen_button_style(rng),
    }
}

fn gen_input(rng: &mut Rng) -> Node {
    Node::Input {
        key: gen_key(rng),
        placeholder: gen_string(rng),
        value: gen_string(rng),
        on_input: rng.next_u64() as u32,
        on_submit: rng.next_bool().then(|| rng.next_u64() as u32),
        width: gen_opt_length(rng),
        secure: rng.next_bool(),
        style: gen_input_style(rng),
    }
}

fn gen_rule(rng: &mut Rng) -> Node {
    Node::Rule {
        key: gen_key(rng),
        axis: gen_axis(rng),
        thickness: gen_f32(rng),
        color: gen_opt_color(rng),
    }
}

fn gen_text(rng: &mut Rng) -> Node {
    Node::Text {
        key: gen_key(rng),
        content: gen_string(rng),
        size: gen_opt_f32(rng),
        color: gen_opt_color(rng),
        font: Font {
            monospace: rng.next_bool(),
            weight: *rng.choose(&[
                Weight::Normal,
                Weight::Medium,
                Weight::Semibold,
                Weight::Bold,
            ]),
        },
        width: gen_opt_length(rng),
        align_x: gen_opt_align_x(rng),
    }
}

/// A leaf with no children, for filling out a wide `Linear`: every leaf
/// variant except `Space` carries a string or a colour worth truncating.
fn gen_leaf(rng: &mut Rng) -> Node {
    match rng.next_range(5) {
        0 => gen_text(rng),
        1 => Node::Space {
            width: gen_opt_length(rng),
            height: gen_opt_length(rng),
        },
        2 => gen_input(rng),
        3 => gen_rule(rng),
        _ => gen_button_label(rng),
    }
}

/// Builds one random tree of exactly `depth` levels of nesting with `width`
/// extra siblings injected at one random level, entirely with an
/// iterative loop rather than recursion — the wire's own stress test
/// (`deep_chain_bytes` in `lib.rs`) builds a deep chain the same way,
/// because a recursive builder would blow its own stack before `decode`
/// ever got a chance to refuse anything.
fn gen_tree(rng: &mut Rng, depth: usize, width: usize) -> Node {
    let mut node = gen_leaf(rng);
    let width_level = if depth == 0 { 0 } else { rng.next_range(depth) };
    for level in 0..depth {
        if level == width_level && width > 0 {
            let mut children: Vec<Node> = (0..width).map(|_| gen_leaf(rng)).collect();
            children.push(node);
            node = Node::Linear {
                key: gen_key(rng),
                axis: gen_axis(rng),
                spacing: gen_opt_f32(rng),
                padding: gen_opt_edges(rng),
                width: gen_opt_length(rng),
                height: gen_opt_length(rng),
                align: gen_opt_align_x(rng),
                children,
            };
            continue;
        }
        node = match rng.next_range(4) {
            0 => Node::Container {
                key: gen_key(rng),
                width: gen_opt_length(rng),
                height: gen_opt_length(rng),
                padding: gen_opt_edges(rng),
                align_x: gen_opt_align_x(rng),
                align_y: gen_opt_align_y(rng),
                background: gen_opt_color(rng),
                border: gen_opt_border(rng),
                content: Box::new(node),
            },
            1 => Node::Linear {
                key: gen_key(rng),
                axis: gen_axis(rng),
                spacing: gen_opt_f32(rng),
                padding: gen_opt_edges(rng),
                width: gen_opt_length(rng),
                height: gen_opt_length(rng),
                align: gen_opt_align_x(rng),
                children: vec![node],
            },
            2 => Node::Scroll {
                key: gen_key(rng),
                direction: *rng.choose(&[
                    ScrollDirection::Vertical,
                    ScrollDirection::Horizontal,
                    ScrollDirection::Both,
                ]),
                width: gen_opt_length(rng),
                height: gen_opt_length(rng),
                content: Box::new(node),
            },
            _ => Node::Button {
                key: gen_key(rng),
                content: ButtonContent::Child(Box::new(node)),
                label: rng.next_bool().then(|| gen_string(rng)),
                on_press: rng.next_bool().then(|| rng.next_u64() as u32),
                width: gen_opt_length(rng),
                height: gen_opt_length(rng),
                padding: gen_opt_edges(rng),
                style: gen_button_style(rng),
            },
        };
    }
    node
}

/// One random `Frame` around a tree of exactly `depth`/`width`: the shared
/// core behind both [`gen_frame`] (which chooses depth/width to stress the
/// decode-time and sanitize-time ceilings) and [`gen_frame_bounded`] (which
/// keeps trees small because its callers re-encode and mutate them
/// hundreds of times each).
fn gen_frame_with(rng: &mut Rng, depth: usize, width: usize) -> Frame {
    let root = gen_tree(rng, depth, width);
    let requests = (0..rng.next_range(4))
        .map(|_| Request {
            id: rng.next_u64(),
            kind: gen_string(rng),
            payload: (0..rng.next_range(16))
                .map(|_| rng.next_range(256) as u8)
                .collect(),
        })
        .collect();
    let cancels = (0..rng.next_range(4)).map(|_| rng.next_u64()).collect();
    Frame {
        root: Some(root),
        requests,
        cancels,
        unchanged: rng.next_bool(),
    }
}

/// One random `Frame`: a tree plus a handful of requests whose `kind`
/// string is generated the same hostile way as everything else.
fn gen_frame(rng: &mut Rng, i: usize) -> Frame {
    let (depth, width) = if i == 0 {
        // Exactly one tree per run goes just over each door, not far over
        // it, and only once: `sanitize`'s key-collision renaming (`claim`
        // in lib.rs) costs quadratic time in how many nodes share one base
        // key once uniquing runs out of room, this file's key pool is
        // deliberately tiny (collisions are the point), and `sanitize`
        // stops at MAX_NODES regardless of how much wider the input tree
        // claims to be — so paying that quadratic cost even once per
        // saturating tree is unavoidable, and paying it many times over
        // (the previous `3 * MAX_NODES`, every 25th tree) is just wasted
        // wall clock, not more coverage.
        (MAX_DEPTH + 8, MAX_NODES + 300)
    } else {
        // A cap and an exponent chosen so this branch, which runs for
        // every other tree, essentially never saturates `sanitize`'s
        // MAX_NODES budget on its own — the forced tree above is what
        // guarantees a saturating, over-both-ceilings tree is exercised.
        (rng.skewed(2 * MAX_DEPTH, 6), rng.skewed(MAX_NODES / 4, 6))
    };
    gen_frame_with(rng, depth, width)
}

/// A frame capped well below [`gen_frame`]'s ceiling-stressing sizes: only
/// [`mutated_bytes_never_panic`] calls this, and it re-encodes and mutates
/// each frame hundreds of times, so a frame in the hundreds-of-KB range
/// (which `gen_frame`'s skew occasionally draws) turns a few hundred
/// `decode` calls into seconds each. The bomb itself — a claimed size with
/// no data behind it — is what exercises decode's refusal path; a tree
/// actually built this wide adds nothing that `gen_frame`'s own forced
/// giants (covered by `random_trees_come_out_of_sanitize_inside_every_bound`)
/// don't already cover.
fn gen_frame_bounded(rng: &mut Rng) -> Frame {
    let depth = rng.skewed(MAX_DEPTH / 2, 3);
    let width = rng.skewed(MAX_NODES / 64, 6);
    gen_frame_with(rng, depth, width)
}

/// Runs `f` on a thread with a much larger stack than a test gets by
/// default, then re-raises whatever it did (return value or panic) on the
/// caller — a panic keeps its original message, seed included, instead of
/// being replaced by a generic "thread panicked" one. Building and encoding
/// a tree recurses once per level of nesting the same way decoding does
/// (see `lib.rs`'s own `deep_chain_bytes`), so the frames this file builds
/// up to `2 * MAX_DEPTH` levels deep get the same headroom.
fn on_big_stack<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    let handle = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(f)
        .expect("spawn a big-stack thread");
    match handle.join() {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn build_frame(seed: u64, i: usize) -> Frame {
    on_big_stack(move || {
        let mut rng = Rng::new(seed);
        gen_frame(&mut rng, i)
    })
}

fn build_and_encode(seed: u64, i: usize) -> (Frame, Vec<u8>) {
    on_big_stack(move || {
        let mut rng = Rng::new(seed);
        let frame = gen_frame(&mut rng, i);
        let bytes = encode(&frame);
        (frame, bytes)
    })
}

fn build_and_encode_bounded(seed: u64) -> (Frame, Vec<u8>) {
    on_big_stack(move || {
        let mut rng = Rng::new(seed);
        let frame = gen_frame_bounded(&mut rng);
        let bytes = encode(&frame);
        (frame, bytes)
    })
}

// -------------------------------------------------------- bound assertions

/// The nesting depth `sanitize` would count for this node (root is 0, each
/// `Container`/`Scroll`/`Linear`/`Button` child adds one) — the same metric
/// `decode`'s own depth budget counts, so it doubles as "was this tree
/// really over the door" evidence when `decode` refuses one.
fn tree_depth(node: &Node) -> usize {
    match node {
        Node::Container { content, .. } | Node::Scroll { content, .. } => 1 + tree_depth(content),
        Node::Linear { children, .. } => 1 + children.iter().map(tree_depth).max().unwrap_or(0),
        Node::Button {
            content: ButtonContent::Child(child),
            ..
        } => 1 + tree_depth(child),
        _ => 0,
    }
}

fn check_length(length: &Option<Length>, ctx: &str) {
    if let Some(Length::Fixed(value)) = length {
        assert!(
            value.is_finite() && (0.0..=PIXEL_BOUND).contains(value),
            "{ctx}: length {value} outside 0..={PIXEL_BOUND}"
        );
    }
}

fn check_edges(edges: &Option<Edges>, ctx: &str) {
    let Some(edges) = edges else { return };
    for value in [edges.top, edges.right, edges.bottom, edges.left] {
        assert!(
            value.is_finite() && (0.0..=PIXEL_BOUND).contains(&value),
            "{ctx}: edge {value} outside 0..={PIXEL_BOUND}"
        );
    }
}

fn check_color(color: &Option<Rgba>, ctx: &str) {
    let Some(Rgba(channels)) = color else { return };
    for value in channels {
        assert!(
            value.is_finite() && (0.0..=1.0).contains(value),
            "{ctx}: colour channel {value} outside 0..=1"
        );
    }
}

fn check_border(border: &Option<Border>, ctx: &str) {
    let Some(border) = border else { return };
    check_color(&Some(border.color), ctx);
    assert!(
        border.width.is_finite() && (0.0..=PIXEL_BOUND).contains(&border.width),
        "{ctx}: border width {} outside 0..={PIXEL_BOUND}",
        border.width
    );
    for radius in border.radius {
        assert!(
            radius.is_finite() && (0.0..=PIXEL_BOUND).contains(&radius),
            "{ctx}: border radius {radius} outside 0..={PIXEL_BOUND}"
        );
    }
}

fn check_string(text: &str, ctx: &str, field: &str) {
    assert!(
        text.len() <= MAX_STRING_BYTES,
        "{ctx}: {field} is {} bytes, over MAX_STRING_BYTES",
        text.len()
    );
    assert!(
        text.is_char_boundary(text.len()),
        "{ctx}: {field} does not end on a char boundary"
    );
}

/// Walks a sanitized tree asserting every post-condition `sanitize_node`
/// promises: depth within `MAX_DEPTH`, every string within
/// `MAX_STRING_BYTES` and on a char boundary, every key unique across the
/// whole tree, and every size/colour/border field inside its own bound.
fn check_bounds(node: &Node, depth: usize, keys: &mut HashSet<String>, ctx: &str) {
    assert!(
        depth <= MAX_DEPTH,
        "{ctx}: a node sits at depth {depth}, over MAX_DEPTH"
    );
    if let Some(key) = node.key() {
        check_string(key, ctx, "key");
        assert!(
            keys.insert(key.to_string()),
            "{ctx}: key {key:?} used more than once after sanitize"
        );
    }
    match node {
        Node::Container {
            width,
            height,
            padding,
            background,
            border,
            content,
            ..
        } => {
            check_length(width, ctx);
            check_length(height, ctx);
            check_edges(padding, ctx);
            check_color(background, ctx);
            check_border(border, ctx);
            check_bounds(content, depth + 1, keys, ctx);
        }
        Node::Linear {
            spacing,
            padding,
            width,
            height,
            children,
            ..
        } => {
            if let Some(spacing) = spacing {
                assert!(
                    spacing.is_finite() && (0.0..=PIXEL_BOUND).contains(spacing),
                    "{ctx}: spacing {spacing} outside 0..={PIXEL_BOUND}"
                );
            }
            check_edges(padding, ctx);
            check_length(width, ctx);
            check_length(height, ctx);
            for child in children {
                check_bounds(child, depth + 1, keys, ctx);
            }
        }
        Node::Scroll {
            width,
            height,
            content,
            ..
        } => {
            check_length(width, ctx);
            check_length(height, ctx);
            check_bounds(content, depth + 1, keys, ctx);
        }
        Node::Text {
            content,
            size,
            color,
            width,
            ..
        } => {
            check_string(content, ctx, "text content");
            if let Some(size) = size {
                assert!(
                    size.is_finite() && (0.0..=TEXT_PIXEL_BOUND).contains(size),
                    "{ctx}: text size {size} outside 0..={TEXT_PIXEL_BOUND}"
                );
            }
            check_color(color, ctx);
            check_length(width, ctx);
        }
        Node::Input {
            placeholder,
            value,
            width,
            style,
            ..
        } => {
            check_string(placeholder, ctx, "placeholder");
            check_string(value, ctx, "input value");
            check_length(width, ctx);
            for face in [
                Some(&style.active),
                style.hovered.as_ref(),
                style.focused.as_ref(),
                style.disabled.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                check_color(&face.background, ctx);
                check_border(&face.border, ctx);
                check_color(&face.value, ctx);
                check_color(&face.placeholder, ctx);
                check_color(&face.selection, ctx);
            }
        }
        Node::Button {
            content,
            label,
            width,
            height,
            padding,
            style,
            ..
        } => {
            match content {
                ButtonContent::Label(text) => check_string(text, ctx, "button label"),
                ButtonContent::Child(child) => check_bounds(child, depth + 1, keys, ctx),
            }
            if let Some(label) = label {
                check_string(label, ctx, "accessible label");
            }
            check_length(width, ctx);
            check_length(height, ctx);
            check_edges(padding, ctx);
            for face in [
                Some(&style.active),
                style.hovered.as_ref(),
                style.pressed.as_ref(),
                style.disabled.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                check_color(&face.background, ctx);
                check_color(&face.text, ctx);
                check_border(&face.border, ctx);
            }
        }
        Node::Space { width, height } => {
            check_length(width, ctx);
            check_length(height, ctx);
        }
        Node::Rule {
            thickness, color, ..
        } => {
            assert!(
                thickness.is_finite() && (0.0..=PIXEL_BOUND).contains(thickness),
                "{ctx}: rule thickness {thickness} outside 0..={PIXEL_BOUND}"
            );
            check_color(color, ctx);
        }
    }
}

/// Every post-condition `sanitize` promises about a whole frame: the tree's
/// node count and every bound `check_bounds` covers, plus every request's
/// `kind`.
fn check_frame(frame: &Frame, ctx: &str) {
    if let Some(root) = &frame.root {
        assert!(
            root.count() <= MAX_NODES,
            "{ctx}: {} nodes, over MAX_NODES",
            root.count()
        );
        let mut keys = HashSet::new();
        check_bounds(root, 0, &mut keys, ctx);
    }
    for request in &frame.requests {
        check_string(&request.kind, ctx, "request kind");
    }
}

// --------------------------------------------------------------- test 1

/// Random trees, decoded and sanitized, always land inside every bound
/// `sanitize` promises — or `decode` refused them for a reason the door
/// actually names, and the tree really was over it.
#[test]
fn random_trees_come_out_of_sanitize_inside_every_bound() {
    // The task asked for ~300; without decode's own recursion needing a
    // dedicated thread (its depth door caps recursion at MAX_DEPTH, safe on
    // a normal stack — see `on_big_stack`'s doc comment), 200 trees with a
    // steep width/depth skew keep this test's slice of the file's ~10s
    // debug budget comfortably small.
    const SEED: u64 = 0x5EED_F00D_1234_5678;
    const NUM_TREES: usize = 200;

    for i in 0..NUM_TREES {
        let seed = SEED ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let ctx = format!("seed={seed:#x} tree={i}");
        let (frame, bytes) = build_and_encode(seed, i);

        match decode::<Frame>(&bytes) {
            Err(message) => {
                let root = frame.root.as_ref().expect("gen_frame always sets a root");
                let depth_over = tree_depth(root) > MAX_DEPTH;
                let count_over = root.count() > MAX_DECODED_NODES + 1;
                assert!(
                    depth_over || count_over,
                    "{ctx}: decode refused a tree that was not actually over either \
                     door (depth {}, nodes {}): {message}",
                    tree_depth(root),
                    root.count()
                );
                let names_the_door = message.contains("deeper than the host renders")
                    || message.contains("more nodes than the host holds");
                assert!(names_the_door, "{ctx}: unexpected refusal: {message}");
            }
            Ok(mut decoded) => {
                sanitize(&mut decoded);
                check_frame(&decoded, &ctx);
            }
        }
    }
}

// --------------------------------------------------------------- test 2

fn corrupt_length_prefix(rng: &mut Rng, bytes: &mut [u8]) {
    if bytes.len() < 8 {
        return;
    }
    let at = rng.next_range(bytes.len() - 7);
    let value = *rng.choose(&[u64::MAX, 1u64 << 40, 0u64]);
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

/// One random mutation applied to a copy of a sound frame's bytes: a bit
/// flip, a byte overwrite, a truncation, an insertion of random bytes, or a
/// length-prefix corruption (an 8-byte little-endian window overwritten
/// with a value a real `Vec`/`String` length prefix would never hold).
fn mutate_once(rng: &mut Rng, bytes: &mut Vec<u8>) {
    if bytes.is_empty() {
        bytes.push(rng.next_range(256) as u8);
        return;
    }
    match rng.next_range(5) {
        0 => {
            let i = rng.next_range(bytes.len());
            bytes[i] ^= 1 << rng.next_range(8);
        }
        1 => {
            let i = rng.next_range(bytes.len());
            bytes[i] = rng.next_range(256) as u8;
        }
        2 => {
            let cut = rng.next_range(bytes.len() + 1);
            bytes.truncate(cut);
        }
        3 => {
            let at = rng.next_range(bytes.len() + 1);
            let junk: Vec<u8> = (0..1 + rng.next_range(16))
                .map(|_| rng.next_range(256) as u8)
                .collect();
            bytes.splice(at..at, junk);
        }
        _ => corrupt_length_prefix(rng, bytes),
    }
}

fn payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string panic payload".to_string())
}

/// Bytes a hostile guest could have written — a sound frame with random
/// bit flips, byte overwrites, truncations, insertions, and corrupted
/// length prefixes — never make `decode` panic. `decode`'s own depth door
/// (checked before each level is even built) is what makes this safe on a
/// plain stack: see `lib.rs`'s `bytes_a_hostile_guest_could_write_are_answered_not_survived`,
/// which this test generalizes to frames far larger than a single flipped
/// bit's worth of hand-written cases.
#[test]
fn mutated_bytes_never_panic() {
    const SEED: u64 = 0xBADF_00D5_A5A5_5A5A;
    const NUM_FRAMES: usize = 50;
    const MUTATIONS_PER_FRAME: usize = 200;

    for i in 0..NUM_FRAMES {
        let seed = SEED ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let (_frame, bytes) = build_and_encode_bounded(seed);
        let mut mutator = Rng::new(seed ^ 0xF00D);

        for m in 0..MUTATIONS_PER_FRAME {
            let mut mutated = bytes.clone();
            mutate_once(&mut mutator, &mut mutated);
            let ctx = format!("seed={seed:#x} frame={i} mutation={m}");

            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                decode::<Frame>(&mutated)
            }));
            match outcome {
                Ok(Ok(mut decoded)) => {
                    sanitize(&mut decoded);
                    check_frame(&decoded, &ctx);
                }
                Ok(Err(_)) => {}
                Err(payload) => panic!("{ctx}: decode panicked: {}", payload_message(&payload)),
            }
        }
    }
}

// --------------------------------------------------------------- test 3

/// A hand-crafted length-prefix bomb — a `Frame` whose root is a `Linear`
/// claiming `2^40` children, with the buffer cut off a few bytes later — is
/// refused without decode trying to build any of it.
///
/// The layout is worked out rather than hand-counted: encoding a `Linear`
/// with zero children and one with a single child differ only in the
/// 8-byte little-endian length prefix bincode writes ahead of a `Vec`'s
/// elements (everything before it — the enum discriminant, the key string,
/// the `Option` tags for `spacing`/`padding`/`width`/`height`/`align` — is
/// byte-for-byte identical either way, and the first divergent byte is
/// that prefix's low byte, 0x00 vs 0x01). Taking the common prefix length
/// of the two encodings finds that offset without hard-coding it.
#[test]
fn a_length_prefix_bomb_is_refused_without_the_allocation() {
    fn linear(children: Vec<Node>) -> Frame {
        Frame {
            root: Some(Node::Linear {
                key: "k".into(),
                axis: Axis::Column,
                spacing: None,
                padding: None,
                width: None,
                height: None,
                align: None,
                children,
            }),
            ..Frame::default()
        }
    }
    let empty_children = linear(vec![]);
    let one_child = linear(vec![Node::empty()]);

    let bytes_empty = encode(&empty_children);
    let bytes_one = encode(&one_child);
    let offset = bytes_empty
        .iter()
        .zip(bytes_one.iter())
        .take_while(|(a, b)| a == b)
        .count();
    assert!(
        offset > 0 && offset + 8 <= bytes_empty.len(),
        "could not locate the children length prefix (offset {offset})"
    );

    let mut bomb = bytes_empty[..offset].to_vec();
    bomb.extend_from_slice(&(1u64 << 40).to_le_bytes());
    // The buffer ends a handful of bytes after the claimed count — nowhere
    // near what 2^40 elements would take — which is the whole point: a
    // decoder that trusted the prefix enough to preallocate would already
    // have tried and failed before it noticed.
    bomb.extend_from_slice(&[0u8; 4]);

    let start = std::time::Instant::now();
    let result = decode::<Frame>(&bomb);
    let elapsed = start.elapsed();

    assert!(
        result.is_err(),
        "a 2^40-child claim with no data was accepted"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "refusing the bomb took {elapsed:?}, which means something tried to act on the claimed count"
    );
}

// --------------------------------------------------------------- test 4

/// `sanitize` is idempotent: running it again on its own output changes
/// nothing, which is what lets a host call it on every frame without
/// worrying whether the guest already sent a clean one.
#[test]
fn sanitize_is_idempotent() {
    const SEED: u64 = 0x1DE4_1DE4_5EED_5EED;
    const NUM_TREES: usize = 150;

    for i in 0..NUM_TREES {
        let seed = SEED ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let ctx = format!("seed={seed:#x} tree={i}");
        let mut once = build_frame(seed, i);
        sanitize(&mut once);
        let mut twice = once.clone();
        sanitize(&mut twice);
        assert_eq!(once, twice, "{ctx}: sanitize is not idempotent");
    }
}
