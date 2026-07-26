use crate::{Layout, MediaKind, ResponsiveContent, ViewNode, analyze_file};
use std::collections::BTreeSet;
use std::path::Path;

const ALL_RENDER_NODES: &[&str] = &[
    "box",
    "button",
    "canvas",
    "checkbox",
    "col",
    "combo",
    "component",
    "editor",
    "extern",
    "flex",
    "float",
    "for",
    "grid",
    "if",
    "image",
    "input",
    "keyed",
    "lazy",
    "markdown",
    "mouse",
    "overlay",
    "panes",
    "pick",
    "pin",
    "progress",
    "qr",
    "radio",
    "resize",
    "responsive",
    "rich-text",
    "row",
    "rule",
    "scroll",
    "sensor",
    "shader",
    "slider",
    "slot",
    "space",
    "stack",
    "svg",
    "table",
    "text",
    "theme",
    "themer",
    "toggler",
    "tooltip",
    "viewer",
];

#[test]
fn render_contract_covers_every_render_node() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/iced-app/src/ui");
    let mut covered = BTreeSet::new();

    let path = examples.join("render_surface.ice");
    let document = analyze_file(&path).unwrap();
    collect(
        &document.view,
        &document,
        &mut covered,
        &mut BTreeSet::new(),
    );

    assert_eq!(covered, ALL_RENDER_NODES.iter().copied().collect());
}

fn collect(
    node: &ViewNode,
    document: &crate::Document,
    covered: &mut BTreeSet<&'static str>,
    visited_components: &mut BTreeSet<String>,
) {
    let kind = match node {
        ViewNode::Layout { kind, options, .. } => {
            if options.flexbox.is_some() {
                "flex"
            } else {
                match kind {
                    Layout::Column => "col",
                    Layout::Row => "row",
                    Layout::Scroll => "scroll",
                    Layout::Grid => "grid",
                    Layout::Stack => "stack",
                }
            }
        }
        ViewNode::Container { .. } => "box",
        ViewNode::Overlay { .. } => "overlay",
        ViewNode::PaneGrid { .. } => "panes",
        ViewNode::Text { .. } => "text",
        ViewNode::RichText { .. } => "rich-text",
        ViewNode::Input { .. } => "input",
        ViewNode::Button { .. } => "button",
        ViewNode::Checkbox { .. } => "checkbox",
        ViewNode::Toggler { .. } => "toggler",
        ViewNode::Slider { .. } => "slider",
        ViewNode::Progress { .. } => "progress",
        ViewNode::Radio { .. } => "radio",
        ViewNode::PickList { .. } => "pick",
        ViewNode::ComboBox { .. } => "combo",
        ViewNode::Rule { .. } => "rule",
        ViewNode::QrCode { .. } => "qr",
        ViewNode::Space { .. } => "space",
        ViewNode::If { .. } => "if",
        ViewNode::For { .. } => "for",
        ViewNode::KeyedColumn { .. } => "keyed",
        ViewNode::Lazy { .. } => "lazy",
        ViewNode::Markdown { .. } => "markdown",
        ViewNode::TextEditor { .. } => "editor",
        ViewNode::Table { .. } => "table",
        ViewNode::Component { .. } => "component",
        ViewNode::Slot { .. } => "slot",
        ViewNode::ExternComponent { .. } => "extern",
        ViewNode::Themer { .. } => "themer",
        ViewNode::Shader { .. } => "shader",
        ViewNode::Media { kind, .. } => match kind {
            MediaKind::Image => "image",
            MediaKind::Svg => "svg",
            MediaKind::Viewer => "viewer",
        },
        ViewNode::Tooltip { .. } => "tooltip",
        ViewNode::MouseArea { .. } => "mouse",
        ViewNode::ResizeHandle { .. } => "resize",
        ViewNode::Canvas { .. } => "canvas",
        ViewNode::Theme { .. } => "theme",
        ViewNode::Float { .. } => "float",
        ViewNode::Pin { .. } => "pin",
        ViewNode::Sensor { .. } => "sensor",
        ViewNode::Responsive { .. } => "responsive",
    };
    covered.insert(kind);

    match node {
        ViewNode::Layout { children, .. }
        | ViewNode::If { children, .. }
        | ViewNode::For { children, .. } => {
            children
                .iter()
                .for_each(|child| collect(child, document, covered, visited_components));
        }
        ViewNode::Container { content, .. }
        | ViewNode::MouseArea { content, .. }
        | ViewNode::ResizeHandle { content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. }
        | ViewNode::Sensor { content, .. } => {
            collect(content, document, covered, visited_components);
        }
        ViewNode::Overlay { content, layer, .. }
        | ViewNode::Tooltip {
            content,
            tip: layer,
            ..
        } => {
            collect(content, document, covered, visited_components);
            collect(layer, document, covered, visited_components);
        }
        ViewNode::PaneGrid {
            panes, templates, ..
        } => {
            for pane in panes {
                pane.nodes().for_each(|node| {
                    collect(node, document, covered, visited_components);
                });
            }
            for template in templates {
                template.pane.nodes().for_each(|node| {
                    collect(node, document, covered, visited_components);
                });
            }
        }
        ViewNode::Table { columns, .. } => {
            for column in columns {
                collect(&column.header, document, covered, visited_components);
                collect(&column.cell, document, covered, visited_components);
            }
        }
        ViewNode::Component { name, slots, .. } => {
            slots.iter().for_each(|slot| {
                collect(&slot.content, document, covered, visited_components);
            });
            if visited_components.insert(name.clone()) {
                let component = document
                    .components
                    .iter()
                    .find(|component| component.name == *name)
                    .expect("checked component call");
                collect(&component.root, document, covered, visited_components);
            }
        }
        ViewNode::Button {
            content: Some(content),
            ..
        }
        | ViewNode::KeyedColumn { child: content, .. }
        | ViewNode::Lazy { child: content, .. } => {
            collect(content, document, covered, visited_components);
        }
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                collect(narrow, document, covered, visited_components);
                collect(wide, document, covered, visited_components);
            }
            ResponsiveContent::Size { content, .. } => {
                collect(content, document, covered, visited_components);
            }
        },
        ViewNode::Text { .. }
        | ViewNode::RichText { .. }
        | ViewNode::Input { .. }
        | ViewNode::Button { content: None, .. }
        | ViewNode::Checkbox { .. }
        | ViewNode::Toggler { .. }
        | ViewNode::Slider { .. }
        | ViewNode::Progress { .. }
        | ViewNode::Radio { .. }
        | ViewNode::PickList { .. }
        | ViewNode::ComboBox { .. }
        | ViewNode::Rule { .. }
        | ViewNode::QrCode { .. }
        | ViewNode::Space { .. }
        | ViewNode::Markdown { .. }
        | ViewNode::TextEditor { .. }
        | ViewNode::Slot { .. }
        | ViewNode::ExternComponent { .. }
        | ViewNode::Themer { .. }
        | ViewNode::Shader { .. }
        | ViewNode::Media { .. }
        | ViewNode::Canvas { .. } => {}
    }
}
