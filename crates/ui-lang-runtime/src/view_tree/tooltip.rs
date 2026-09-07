//! Native tooltip rendering and its semantic description.
use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    position: wire::TooltipPosition,
    gap: f32,
    padding: f32,
    delay_ms: u64,
    snap: bool,
    style: wire::TooltipStyle,
    children: &[wire::Node],
    kept: &Kept<'_>,
) -> IceElement<'static, Output> {
    let Some(content) = children.first() else {
        return widget::Space::new().into();
    };
    let content = render_node(content, kept);
    let Some(tip) = children.get(1) else {
        return content;
    };
    let mut descriptions = Vec::new();
    collect_text(tip, kept, &mut descriptions);
    let content = if descriptions.is_empty() {
        content
    } else {
        crate::described(content, descriptions.join(" ")).into()
    };
    let position = match position {
        wire::TooltipPosition::Top => widget::tooltip::Position::Top,
        wire::TooltipPosition::Bottom => widget::tooltip::Position::Bottom,
        wire::TooltipPosition::Left => widget::tooltip::Position::Left,
        wire::TooltipPosition::Right => widget::tooltip::Position::Right,
        wire::TooltipPosition::FollowCursor => widget::tooltip::Position::FollowCursor,
    };
    widget::tooltip(content, render_node(tip, kept), position)
        .gap(gap)
        .padding(padding)
        .delay(std::time::Duration::from_millis(delay_ms))
        .snap_within_viewport(snap)
        .style(move |theme| native_style(style, theme))
        .into()
}

fn collect_text<'a>(node: &'a wire::Node, kept: &Kept<'_>, into: &mut Vec<&'a str>) {
    if let wire::Node::When { condition, .. } = node
        && !condition.matches(kept.containers)
    {
        return;
    }
    if let wire::Node::Text { content, .. } = node {
        into.push(content);
    } else {
        for child in selected_children(node.children(), kept) {
            collect_text(child, kept, into);
        }
    }
}

fn native_style(style: wire::TooltipStyle, theme: &iced::Theme) -> widget::container::Style {
    use widget::container as native;
    let mut out = match style.preset {
        wire::TooltipPreset::Transparent => native::transparent(theme),
        wire::TooltipPreset::Rounded => native::rounded_box(theme),
        wire::TooltipPreset::Bordered => native::bordered_box(theme),
        wire::TooltipPreset::Dark => native::dark(theme),
        wire::TooltipPreset::Primary => native::primary(theme),
        wire::TooltipPreset::Secondary => native::secondary(theme),
        wire::TooltipPreset::Success => native::success(theme),
        wire::TooltipPreset::Warning => native::warning(theme),
        wire::TooltipPreset::Danger => native::danger(theme),
    };
    if let Some(value) = style.background {
        out.background = Some(Background::Color(color(value)));
    }
    if let Some(value) = style.text {
        out.text_color = Some(color(value));
    }
    if let Some(value) = style.border {
        apply_border(value, &mut out.border);
    }
    if let Some(value) = style.shadow_color {
        out.shadow.color = color(value);
    }
    if let Some(value) = style.shadow_x {
        out.shadow.offset.x = value;
    }
    if let Some(value) = style.shadow_y {
        out.shadow.offset.y = value;
    }
    if let Some(value) = style.shadow_blur {
        out.shadow.blur_radius = value;
    }
    if let Some(value) = style.pixel_snap {
        out.snap = value;
    }
    out
}
