use crate::lower::{
    ResolvedLayoutMode, ResolvedLinearAxis, ResolvedMediaKind, ResolvedView, ResolvedViewKind,
};
use crate::{analyze_file, analyze_file_with_overlays};
use std::collections::{BTreeSet, HashMap};
use std::fs;
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
    "match",
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
    let path = examples.join("render_surface.ice");
    let program = crate::lower::lower(analyze_file(&path).unwrap()).unwrap();
    crate::codegen::generate(&program, path.to_str().unwrap())
        .expect("every normalized render node dispatches through codegen");
    let covered = program
        .resolved_views()
        .map(|view| normalized_render_kind(&program, view))
        .collect::<BTreeSet<_>>();

    assert_eq!(covered, ALL_RENDER_NODES.iter().copied().collect());
}

#[test]
fn dynamic_identity_analysis_is_separate_from_every_render_family() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/iced-app/src/ui");
    let path = examples.join("render_surface.ice").canonicalize().unwrap();
    let source = fs::read_to_string(&path).unwrap();
    let source = source
        .split_once("\ntest renders_every_node\n")
        .map_or(source.as_str(), |(application, _)| application);
    let source = source.replacen(
        "state\n",
        "state\n  identity_key = \"canonical-view-key\"\n",
        1,
    );
    let mut in_app_view = false;
    let mut dynamic_identities = 0usize;
    let mut preexisting_dynamic_identities = 0usize;
    let source = source
        .lines()
        .map(|line| {
            if line == "view" {
                in_app_view = true;
                return line.to_owned();
            }
            if !in_app_view || line.trim_start().starts_with("panes #") {
                return line.to_owned();
            }
            let Some(marker) = line.find(" #") else {
                return line.to_owned();
            };
            let token_start = marker + 2;
            let token_end = line[token_start..]
                .find(char::is_whitespace)
                .map_or(line.len(), |offset| token_start + offset);
            if line[token_start..token_end].contains('(') {
                preexisting_dynamic_identities += 1;
                return line.to_owned();
            }
            dynamic_identities += 1;
            format!(
                "{}{}(identity_key){}",
                &line[..token_start],
                &line[token_start..token_end],
                &line[token_end..]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(dynamic_identities >= 40);

    let checked = analyze_file_with_overlays(&path, &HashMap::from([(path.clone(), source)]))
        .expect("dynamic identities remain independently checked across the complete view surface");
    let program = crate::lower::lower(checked)
        .expect("identity expressions do not leak into widget-family analysis guards");
    crate::codegen::generate(&program, path.to_str().unwrap())
        .expect("every dynamic identity is emitted from canonical view HIR");
    assert_eq!(
        program
            .resolved_views()
            .filter_map(|view| view.identity.as_ref())
            .filter(|identity| identity.key.is_some())
            .count(),
        dynamic_identities + preexisting_dynamic_identities
    );
}

fn normalized_render_kind(
    program: &crate::lower::LoweredProgram,
    view: &ResolvedView,
) -> &'static str {
    match &view.kind {
        ResolvedViewKind::Layout { .. } => match &program.resolved_layout(view.id).unwrap().mode {
            ResolvedLayoutMode::Linear(layout) => match layout.axis {
                ResolvedLinearAxis::Column => "col",
                ResolvedLinearAxis::Row => "row",
            },
            ResolvedLayoutMode::Grid(_) => "grid",
            ResolvedLayoutMode::Stack(_) => "stack",
            ResolvedLayoutMode::Hover(_) => "hover",
            ResolvedLayoutMode::Flex(_) => "flex",
            ResolvedLayoutMode::Scroll(_) => "scroll",
        },
        ResolvedViewKind::Container { .. } => "box",
        ResolvedViewKind::Overlay { .. } => "overlay",
        ResolvedViewKind::PaneGrid { .. } => "panes",
        ResolvedViewKind::Text => "text",
        ResolvedViewKind::RichText => "rich-text",
        ResolvedViewKind::Input => "input",
        ResolvedViewKind::Button { .. } => "button",
        ResolvedViewKind::Checkbox => "checkbox",
        ResolvedViewKind::Toggler => "toggler",
        ResolvedViewKind::Slider => "slider",
        ResolvedViewKind::Progress => "progress",
        ResolvedViewKind::Radio => "radio",
        ResolvedViewKind::PickList => "pick",
        ResolvedViewKind::ComboBox => "combo",
        ResolvedViewKind::Rule => "rule",
        ResolvedViewKind::QrCode => "qr",
        ResolvedViewKind::Space => "space",
        ResolvedViewKind::If { .. } => "if",
        ResolvedViewKind::Match { .. } => "match",
        ResolvedViewKind::For { .. } => "for",
        ResolvedViewKind::KeyedColumn { .. } => "keyed",
        ResolvedViewKind::Lazy { .. } => "lazy",
        ResolvedViewKind::Markdown => "markdown",
        ResolvedViewKind::TextEditor => "editor",
        ResolvedViewKind::Table { .. } => "table",
        ResolvedViewKind::Component { .. } => "component",
        ResolvedViewKind::Slot { .. } => "slot",
        ResolvedViewKind::ExternComponent => "extern",
        ResolvedViewKind::Themer => "themer",
        ResolvedViewKind::Shader => "shader",
        ResolvedViewKind::Media => match program.resolved_media(view.id).unwrap().kind {
            ResolvedMediaKind::Image => "image",
            ResolvedMediaKind::Svg => "svg",
            ResolvedMediaKind::Viewer => "viewer",
        },
        ResolvedViewKind::Tooltip { .. } => "tooltip",
        ResolvedViewKind::MouseArea { .. } => "mouse",
        ResolvedViewKind::ResizeHandle { .. } => "resize",
        ResolvedViewKind::Canvas => "canvas",
        ResolvedViewKind::Theme { .. } => "theme",
        ResolvedViewKind::Float { .. } => "float",
        ResolvedViewKind::Pin { .. } => "pin",
        ResolvedViewKind::Sensor { .. } => "sensor",
        ResolvedViewKind::ResponsiveBreakpoint { .. } | ResolvedViewKind::ResponsiveSize { .. } => {
            "responsive"
        }
    }
}
