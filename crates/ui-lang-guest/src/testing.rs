//! Helpers for a guest's own tests: build events the host would send, and
//! read the tree a frame carries.

use crate::wire::{ButtonContent, Event, Frame, Node};

/// Every text the tree shows, depth first: text nodes, button labels, and
/// the value or placeholder of an input.
pub fn texts(frame: &Frame) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(root) = &frame.root {
        collect_texts(root, &mut out);
    }
    out
}

fn collect_texts(node: &Node, out: &mut Vec<String>) {
    match node {
        Node::Container { content, .. } | Node::Scroll { content, .. } => {
            collect_texts(content, out)
        }
        Node::Linear { children, .. } => children.iter().for_each(|child| collect_texts(child, out)),
        Node::Text { content, .. } => out.push(content.clone()),
        Node::Input {
            value, placeholder, ..
        } => out.push(if value.is_empty() {
            placeholder.clone()
        } else {
            value.clone()
        }),
        Node::Button { content, .. } => match content {
            ButtonContent::Label(label) => out.push(label.clone()),
            ButtonContent::Child(child) => collect_texts(child, out),
        },
        Node::Space { .. } | Node::Rule { .. } => {}
    }
}

pub fn has_text(frame: &Frame, content: &str) -> bool {
    texts(frame).iter().any(|text| text == content)
}

/// The node under `key` (`App/content/count`), if the tree has one.
pub fn find<'a>(frame: &'a Frame, key: &str) -> Option<&'a Node> {
    frame.root.as_ref().and_then(|root| find_in(root, key))
}

fn find_in<'a>(node: &'a Node, key: &str) -> Option<&'a Node> {
    if node.key() == Some(key) {
        return Some(node);
    }
    match node {
        Node::Container { content, .. } | Node::Scroll { content, .. } => find_in(content, key),
        Node::Linear { children, .. } => children.iter().find_map(|child| find_in(child, key)),
        Node::Button {
            content: ButtonContent::Child(child),
            ..
        } => find_in(child, key),
        Node::Button { .. }
        | Node::Text { .. }
        | Node::Input { .. }
        | Node::Space { .. }
        | Node::Rule { .. } => None,
    }
}

/// The events the host sends when the user presses the button under `key`.
pub fn press(frame: &Frame, key: &str) -> Vec<Event> {
    let Some(Node::Button { on_press, .. }) = find(frame, key) else {
        panic!("no button {key:?} in {:?}", keys(frame));
    };
    let Some(message) = on_press else {
        panic!("button {key:?} is disabled");
    };
    vec![Event::Message(*message)]
}

/// The events the host sends when the input under `key` now reads `text`.
pub fn type_into(frame: &Frame, key: &str, text: &str) -> Vec<Event> {
    let Some(Node::Input { on_input, .. }) = find(frame, key) else {
        panic!("no input {key:?} in {:?}", keys(frame));
    };
    vec![Event::Input {
        handler: *on_input,
        text: text.to_string(),
    }]
}

/// The events the host sends when the user submits the input under `key`.
pub fn submit(frame: &Frame, key: &str) -> Vec<Event> {
    let Some(Node::Input { on_submit, .. }) = find(frame, key) else {
        panic!("no input {key:?} in {:?}", keys(frame));
    };
    let Some(message) = on_submit else {
        panic!("input {key:?} has no submit route");
    };
    vec![Event::Message(*message)]
}

/// Every node key in the tree, depth first.
pub fn keys(frame: &Frame) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(root) = &frame.root {
        collect_keys(root, &mut out);
    }
    out
}

fn collect_keys(node: &Node, out: &mut Vec<String>) {
    if let Some(key) = node.key() {
        out.push(key.to_string());
    }
    match node {
        Node::Container { content, .. } | Node::Scroll { content, .. } => {
            collect_keys(content, out)
        }
        Node::Linear { children, .. } => children.iter().for_each(|child| collect_keys(child, out)),
        Node::Button {
            content: ButtonContent::Child(child),
            ..
        } => collect_keys(child, out),
        Node::Button { .. }
        | Node::Text { .. }
        | Node::Input { .. }
        | Node::Space { .. }
        | Node::Rule { .. } => {}
    }
}

pub fn answer(id: u64, payload: &[u8]) -> Event {
    Event::Response {
        id,
        result: Ok(payload.to_vec()),
        done: true,
    }
}

pub fn item(id: u64, payload: &[u8]) -> Event {
    Event::Response {
        id,
        result: Ok(payload.to_vec()),
        done: false,
    }
}

pub fn refuse(id: u64, message: &str) -> Event {
    Event::Response {
        id,
        result: Err(message.to_string()),
        done: true,
    }
}
