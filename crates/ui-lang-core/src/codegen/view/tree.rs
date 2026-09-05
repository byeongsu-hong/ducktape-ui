//! The `tree` target's view emitter: a node becomes Rust that builds a
//! `ui_lang_wire::Node` with every value inlined, instead of an iced widget.
//!
//! Only the leaves and layouts differ from the native emitter. Control flow
//! (`if`, `for`, `match`), components and slots are emitted by the shared
//! code, which pushes whatever `render_node` returns — here a `Node` — into
//! the parent's child list; so a `for` over a list of rows compiles to the
//! same loop for both targets and only the row inside changes.
//!
//! A construct the tree does not model (`grid`, `markdown`, an extern
//! widget, a gradient background...) fails the build, naming the construct
//! and its `.ice` line, rather than rendering as something else. The host
//! has a fixed vocabulary; a view module is written to it.
//!
//! Interaction: a button's message goes into the guest's per-frame table
//! (`ui_lang_guest::slots::message`) and the node carries the index; an
//! input's `String -> Message` constructor likewise (`slots::handler`).
//! Colours are resolved through the app's palette here and cross as RGBA.

use super::*;

// Reached through the guest crate, which is the app's one dependency: it
// re-exports the wire so a module never names `ui_lang_wire` itself.
const WIRE: &str = "::ui_lang_guest::wire";
const SLOTS: &str = "::ui_lang_guest::slots";

/// The tree rendering of `node`, or `None` when the target is native or
/// the node is one the shared emitters render for both targets.
pub(in crate::codegen) fn render_tree_node(
    node: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    if document.target() != Target::Tree {
        return Ok(None);
    }
    let view = document.resolved_view(node)?;
    let identity = view.identity.as_ref();
    let rendered = match &view.kind {
        ResolvedViewKind::Layout { children } => layout(
            node, identity, children, document, message, env, scope, slot,
        )?,
        ResolvedViewKind::Container { content } => container(
            node, identity, *content, document, message, env, scope, slot,
        )?,
        ResolvedViewKind::Text => text(node, identity, document, env, scope)?,
        ResolvedViewKind::Input => input(node, identity, document, message, env, scope)?,
        ResolvedViewKind::Button { content } => button(
            node,
            identity,
            content.as_ref(),
            document,
            message,
            env,
            scope,
            slot,
        )?,
        ResolvedViewKind::Space => space(node, document, env)?,
        ResolvedViewKind::Rule => rule(node, identity, document, env, scope)?,
        // Rendered by the shared emitters: their code is target-neutral.
        ResolvedViewKind::Component { .. }
        | ResolvedViewKind::Slot { .. }
        | ResolvedViewKind::If { .. }
        | ResolvedViewKind::For { .. }
        | ResolvedViewKind::Match { .. } => return Ok(None),
        other => {
            return Err(refused(document, view.origin, kind_name(other)));
        }
    };
    Ok(Some(rendered))
}

fn kind_name(kind: &ResolvedViewKind) -> &'static str {
    match kind {
        ResolvedViewKind::Layout { .. } => "layout",
        ResolvedViewKind::Container { .. } => "box",
        ResolvedViewKind::Overlay { .. } => "overlay",
        ResolvedViewKind::PaneGrid { .. } => "pane grid",
        ResolvedViewKind::Text => "text",
        ResolvedViewKind::RichText => "rich text",
        ResolvedViewKind::Input => "input",
        ResolvedViewKind::Button { .. } => "button",
        ResolvedViewKind::Checkbox => "checkbox",
        ResolvedViewKind::Toggler => "toggler",
        ResolvedViewKind::Slider => "slider",
        ResolvedViewKind::Progress => "progress",
        ResolvedViewKind::Radio => "radio",
        ResolvedViewKind::PickList => "pick list",
        ResolvedViewKind::ComboBox => "combo box",
        ResolvedViewKind::Rule => "rule",
        ResolvedViewKind::QrCode => "qr code",
        ResolvedViewKind::Space => "space",
        ResolvedViewKind::If { .. } => "if",
        ResolvedViewKind::Match { .. } => "match",
        ResolvedViewKind::For { .. } => "for",
        ResolvedViewKind::KeyedColumn { .. } => "keyed column",
        ResolvedViewKind::Lazy { .. } => "lazy",
        ResolvedViewKind::Component { .. } => "component",
        ResolvedViewKind::Slot { .. } => "slot",
        ResolvedViewKind::MouseArea { .. } => "mouse area",
        ResolvedViewKind::ResizeHandle { .. } => "resize handle",
        ResolvedViewKind::Theme { .. } => "theme",
        ResolvedViewKind::Float { .. } => "float",
        ResolvedViewKind::Pin { .. } => "pin",
        ResolvedViewKind::Sensor { .. } => "sensor",
        ResolvedViewKind::Tooltip { .. } => "tooltip",
        ResolvedViewKind::ResponsiveSize { .. } => "responsive size",
        ResolvedViewKind::Table { .. } => "table",
        ResolvedViewKind::Markdown => "markdown",
        ResolvedViewKind::TextEditor => "editor",
        ResolvedViewKind::ExternComponent => "extern widget",
        ResolvedViewKind::Themer => "themer",
        ResolvedViewKind::Shader => "shader",
        ResolvedViewKind::Media => "media",
        ResolvedViewKind::Canvas => "canvas",
    }
}

/// The build error for a construct the tree does not carry.
fn refused(program: &LoweredProgram, origin: OriginId, what: &str) -> Error {
    program.error_at_origin(
        "E190",
        origin,
        format!("`{what}` is not available in a view module: the tree wire does not carry it"),
    )
}

fn refuse_when(
    program: &LoweredProgram,
    origin: OriginId,
    condition: bool,
    what: &str,
) -> Result<(), Error> {
    if condition {
        return Err(refused(program, origin, what));
    }
    Ok(())
}

// ---- values -------------------------------------------------------------

fn key_code(
    identity: Option<&ResolvedViewIdentity>,
    kind: &str,
    origin: OriginId,
    scope: &str,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    owned_accessibility_key_code(identity, kind, origin, scope, env, program)
}

fn rgba_code(color: &ResolvedThemeColor) -> String {
    format!(
        "{{ let __color: ::iced::Color = {}; {WIRE}::Rgba([__color.r, __color.g, __color.b, __color.a]) }}",
        resolved_theme_color(color)
    )
}

fn option_code(value: Option<String>) -> String {
    match value {
        Some(value) => format!("::std::option::Option::Some({value})"),
        None => "::std::option::Option::None".into(),
    }
}

fn length_code(
    length: &ResolvedContainerLength,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    origin: OriginId,
) -> Result<String, Error> {
    Ok(match length {
        ResolvedContainerLength::Fill => format!("{WIRE}::Length::Fill"),
        ResolvedContainerLength::FillPortion(portion) => {
            format!("{WIRE}::Length::FillPortion({portion})")
        }
        ResolvedContainerLength::Shrink => format!("{WIRE}::Length::Shrink"),
        ResolvedContainerLength::FixedF64(expression) => format!(
            "{WIRE}::Length::Fixed(({}) as f32)",
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
        ),
        ResolvedContainerLength::FixedLength(_) => {
            return Err(refused(program, origin, "a `length` value"));
        }
    })
}

/// A dimension: the explicit length, else the `w=fill`/`h=fill` utility.
fn dimension_code(
    length: Option<&ResolvedContainerLength>,
    utility_fill: bool,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    origin: OriginId,
) -> Result<String, Error> {
    Ok(option_code(match (length, utility_fill) {
        (Some(length), _) => Some(length_code(length, program, env, origin)?),
        (None, true) => Some(format!("{WIRE}::Length::Fill")),
        (None, false) => None,
    }))
}

fn edges_code(
    padding: &ResolvedContainerPadding,
    utility: [u16; 4],
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let explicit = padding.all.is_some()
        || padding.x.is_some()
        || padding.y.is_some()
        || padding.top.is_some()
        || padding.right.is_some()
        || padding.bottom.is_some()
        || padding.left.is_some();
    if !explicit {
        if utility == [0; 4] {
            return Ok(option_code(None));
        }
        let [top, right, bottom, left] = utility;
        return Ok(option_code(Some(format!(
            "{WIRE}::Edges {{ top: {top}.0, right: {right}.0, bottom: {bottom}.0, left: {left}.0 }}"
        ))));
    }
    let value = |expression: Option<ResolvedExpressionId>| {
        expression
            .map(|expression| resolved_expr_use_code(program, expression, env, ValueMode::Owned))
            .transpose()
    };
    let all = value(padding.all)?.unwrap_or_else(|| "0.0".into());
    let x = value(padding.x)?.unwrap_or_else(|| all.clone());
    let y = value(padding.y)?.unwrap_or_else(|| all.clone());
    let top = value(padding.top)?.unwrap_or_else(|| y.clone());
    let right = value(padding.right)?.unwrap_or_else(|| x.clone());
    let bottom = value(padding.bottom)?.unwrap_or(y);
    let left = value(padding.left)?.unwrap_or(x);
    Ok(option_code(Some(format!(
        "{WIRE}::Edges {{ top: ({top}) as f32, right: ({right}) as f32, bottom: ({bottom}) as f32, left: ({left}) as f32 }}"
    ))))
}

fn radius_code(
    radius: &ResolvedContainerRadius,
    utility: u16,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    let explicit = radius.all.is_some()
        || radius.top_left.is_some()
        || radius.top_right.is_some()
        || radius.bottom_right.is_some()
        || radius.bottom_left.is_some();
    if !explicit {
        return Ok((utility != 0).then(|| format!("[{utility}.0; 4]")));
    }
    let corner = |value: Option<ResolvedExpressionId>| {
        value
            .map(|value| clamped_f32_code(value, "0.0", "f32::MAX", program, env))
            .transpose()
    };
    let base = corner(radius.all)?.unwrap_or_else(|| "0.0".into());
    let top_left = corner(radius.top_left)?.unwrap_or_else(|| base.clone());
    let top_right = corner(radius.top_right)?.unwrap_or_else(|| base.clone());
    let bottom_right = corner(radius.bottom_right)?.unwrap_or_else(|| base.clone());
    let bottom_left = corner(radius.bottom_left)?.unwrap_or(base);
    Ok(Some(format!(
        "[{top_left}, {top_right}, {bottom_right}, {bottom_left}]"
    )))
}

/// A border from a surface's border colour, width and radius, plus the
/// utility equivalents. A radius alone still needs a border (the host rounds
/// the background through it), drawn transparent and zero wide.
fn border_code(
    surface: &ResolvedContainerSurface,
    style: &ResolvedStyle,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let color = surface
        .border_color
        .as_ref()
        .or(style.border_color.as_ref());
    let width = match surface.border_width {
        Some(width) => Some(clamped_f32_code(width, "0.0", "f32::MAX", program, env)?),
        None => (style.border_width != 0).then(|| format!("{}.0", style.border_width)),
    };
    let radius = radius_code(&surface.radius, style.radius, program, env)?;
    if color.is_none() && width.is_none() && radius.is_none() {
        return Ok(option_code(None));
    }
    let color = color
        .map(rgba_code)
        .unwrap_or_else(|| format!("{WIRE}::Rgba([0.0, 0.0, 0.0, 0.0])"));
    let width = width.unwrap_or_else(|| "0.0".into());
    let radius = radius.unwrap_or_else(|| "[0.0; 4]".into());
    Ok(option_code(Some(format!(
        "{WIRE}::Border {{ color: {color}, width: {width}, radius: {radius} }}"
    ))))
}

fn background_code(
    surface: &ResolvedContainerSurface,
    style: &ResolvedStyle,
    program: &LoweredProgram,
    origin: OriginId,
) -> Result<String, Error> {
    refuse_when(
        program,
        origin,
        surface.background_alpha.is_some(),
        "a background alpha",
    )?;
    Ok(option_code(
        match (&surface.background, &style.background) {
            (Some(ResolvedContainerBackground::Color(color)), _) => Some(rgba_code(color)),
            (Some(ResolvedContainerBackground::Linear { .. }), _) => {
                return Err(refused(program, origin, "a gradient background"));
            }
            (None, Some(color)) => Some(rgba_code(color)),
            (None, None) => None,
        },
    ))
}

fn refuse_surface_extras(
    surface: &ResolvedContainerSurface,
    program: &LoweredProgram,
    origin: OriginId,
) -> Result<(), Error> {
    refuse_when(
        program,
        origin,
        surface.shadow_color.is_some()
            || surface.shadow_x.is_some()
            || surface.shadow_y.is_some()
            || surface.shadow_blur.is_some(),
        "a shadow",
    )?;
    refuse_when(
        program,
        origin,
        surface.pixel_snap.is_some(),
        "`px-snap` on a surface",
    )
}

fn align_x_code(align: ResolvedContainerAlignment) -> String {
    let name = match align {
        ResolvedContainerAlignment::Start => "Left",
        ResolvedContainerAlignment::Center => "Center",
        ResolvedContainerAlignment::End => "Right",
    };
    format!("{WIRE}::AlignX::{name}")
}

fn align_y_code(align: ResolvedContainerAlignment) -> String {
    let name = match align {
        ResolvedContainerAlignment::Start => "Top",
        ResolvedContainerAlignment::Center => "Center",
        ResolvedContainerAlignment::End => "Bottom",
    };
    format!("{WIRE}::AlignY::{name}")
}

/// The utility styles a box may carry: geometry and surface. The text and
/// interaction utilities belong to leaves and are refused here.
fn refuse_box_utilities(
    style: &ResolvedStyle,
    program: &LoweredProgram,
    origin: OriginId,
) -> Result<(), Error> {
    refuse_when(
        program,
        origin,
        style.max_width.is_some()
            || style.self_center
            || style.clip
            || style.text_size.is_some()
            || style.text_line_height.is_some()
            || style.font_monospace
            || style.font_weight.is_some()
            || style.text_color.is_some()
            || style.hover_background.is_some()
            || style.pressed_background.is_some()
            || style.disabled_background.is_some()
            || style.disabled_text_color.is_some()
            || style.focus_border_color.is_some()
            || style.focus_visible_border_color.is_some()
            || style.disabled_opacity.is_some(),
        "this utility style on a box",
    )
}

/// A layout carries geometry only. Natively a surface utility (`@bg-…`,
/// `@border-…`, `@r-…`) wraps the row or column in a styled container, and
/// the wire has no node for that wrapper: a `Linear` paints nothing. Wrap the
/// layout in a `box` to paint it.
fn refuse_layout_utilities(
    style: &ResolvedStyle,
    program: &LoweredProgram,
    origin: OriginId,
) -> Result<(), Error> {
    refuse_box_utilities(style, program, origin)?;
    refuse_when(
        program,
        origin,
        style.background.is_some()
            || style.border_color.is_some()
            || style.border_width != 0
            || style.radius != 0,
        "a surface utility style on a layout",
    )
}

// ---- nodes --------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn layout(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    children: &[ViewId],
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let layout = program.resolved_layout(id)?;
    let origin = layout.origin;
    let style = &layout.utility_style;
    refuse_layout_utilities(style, program, origin)?;
    let key = key_code(identity, "layout", origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    match &layout.mode {
        ResolvedLayoutMode::Linear(linear) => {
            refuse_when(program, origin, linear.wrap, "a wrapping layout")?;
            refuse_when(
                program,
                origin,
                linear.virtual_row.is_some(),
                "a virtual-row layout",
            )?;
            refuse_when(program, origin, linear.max_width.is_some(), "`max-w`")?;
            refuse_when(program, origin, linear.clip.is_some(), "`clip`")?;
            let axis = match linear.axis {
                ResolvedLinearAxis::Column => "Column",
                ResolvedLinearAxis::Row => "Row",
            };
            let spacing = match linear.spacing {
                Some(spacing) => Some(format!(
                    "({}) as f32",
                    resolved_expr_use_code(program, spacing, env, ValueMode::Owned)?
                )),
                None => style.gap.map(|gap| format!("{gap}.0")),
            };
            let align = match linear.align {
                Some(align) => Some(align_x_code(align)),
                None => style
                    .items_center
                    .then(|| format!("{WIRE}::AlignX::Center")),
            };
            let mut body = format!(
                "{{ let mut __children: ::std::vec::Vec<__IceElement<'_, {message}>> = ::std::vec::Vec::new();"
            );
            render_children(
                &mut body,
                children,
                program,
                message,
                env,
                &child_scope,
                slot,
            )?;
            write!(
                body,
                " {WIRE}::Node::Linear {{ key: {key}, axis: {WIRE}::Axis::{axis}, spacing: {}, padding: {}, width: {}, height: {}, align: {}, children: __children }} }}",
                option_code(spacing),
                edges_code(&linear.padding, style.padding, program, env)?,
                dimension_code(linear.width.as_ref(), style.width_fill, program, env, origin)?,
                dimension_code(linear.height.as_ref(), style.height_fill, program, env, origin)?,
                option_code(align),
            )
            .unwrap();
            Ok(body)
        }
        ResolvedLayoutMode::Scroll(scroll) => {
            refuse_when(
                program,
                origin,
                scroll.route.is_some()
                    || scroll.viewport_route.is_some()
                    || scroll.auto_scroll.is_some(),
                "a scroll route",
            )?;
            refuse_when(
                program,
                origin,
                scroll.hidden_bar
                    || scroll.bar_width.is_some()
                    || scroll.bar_margin.is_some()
                    || scroll.scroller_width.is_some()
                    || scroll.bar_spacing.is_some(),
                "a scroll bar option",
            )?;
            refuse_when(
                program,
                origin,
                scroll.anchor_x != ResolvedScrollAnchor::Start
                    || scroll.anchor_y != ResolvedScrollAnchor::Start,
                "a scroll anchor",
            )?;
            refuse_when(
                program,
                origin,
                scroll.custom_style.is_some() || !scroll.styles.is_empty(),
                "a scroll style",
            )?;
            let direction = match scroll.direction {
                ResolvedScrollDirection::Vertical => "Vertical",
                ResolvedScrollDirection::Horizontal => "Horizontal",
                ResolvedScrollDirection::Both => "Both",
            };
            let content = render_node(children[0], program, message, env, &child_scope, slot)?;
            Ok(format!(
                "{WIRE}::Node::Scroll {{ key: {key}, direction: {WIRE}::ScrollDirection::{direction}, width: {}, height: {}, content: ::std::boxed::Box::new({content}) }}",
                dimension_code(
                    scroll.width.as_ref(),
                    style.width_fill,
                    program,
                    env,
                    origin
                )?,
                dimension_code(
                    scroll.height.as_ref(),
                    style.height_fill,
                    program,
                    env,
                    origin
                )?,
            ))
        }
        ResolvedLayoutMode::Grid(_) => Err(refused(program, origin, "grid")),
        ResolvedLayoutMode::Stack(_) => Err(refused(program, origin, "stack")),
        ResolvedLayoutMode::Hover(_) => Err(refused(program, origin, "hover")),
        ResolvedLayoutMode::Flex(_) => Err(refused(program, origin, "flex")),
    }
}

#[allow(clippy::too_many_arguments)]
fn container(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    content: ViewId,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let container = program.resolved_container(id)?;
    let origin = container.origin;
    let style = &container.utility_style;
    refuse_box_utilities(style, program, origin)?;
    refuse_when(
        program,
        origin,
        container.max_width.is_some() || container.max_height.is_some(),
        "`max-w`/`max-h`",
    )?;
    refuse_when(program, origin, container.clip.is_some(), "`clip`")?;
    refuse_when(
        program,
        origin,
        container.custom_style.is_some(),
        "a custom style",
    )?;
    refuse_when(
        program,
        origin,
        !container.border_dash.is_empty(),
        "a dashed border",
    )?;
    refuse_when(
        program,
        origin,
        container.surface.text_color.is_some(),
        "`text=` on a box",
    )?;
    refuse_surface_extras(&container.surface, program, origin)?;
    let key = key_code(identity, "container", origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    let content = render_node(content, program, message, env, &child_scope, slot)?;
    Ok(format!(
        "{WIRE}::Node::Container {{ key: {key}, width: {}, height: {}, padding: {}, align_x: {}, align_y: {}, background: {}, border: {}, content: ::std::boxed::Box::new({content}) }}",
        dimension_code(
            container.width.as_ref(),
            style.width_fill,
            program,
            env,
            origin
        )?,
        dimension_code(
            container.height.as_ref(),
            style.height_fill,
            program,
            env,
            origin
        )?,
        edges_code(&container.padding, style.padding, program, env)?,
        option_code(container.align_x.map(align_x_code)),
        option_code(container.align_y.map(align_y_code)),
        background_code(&container.surface, style, program, origin)?,
        border_code(&container.surface, style, program, env)?,
    ))
}

fn text(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let text = program.resolved_text(id)?;
    let origin = text.origin;
    let options = &text.options;
    let style = &text.utility_style;
    let ResolvedTextContent::Plain { value } = &text.content else {
        return Err(refused(program, origin, "rich text"));
    };
    refuse_when(program, origin, options.height.is_some(), "`h=` on text")?;
    refuse_when(
        program,
        origin,
        options.line_height.is_some() || style.text_line_height.is_some(),
        "a line height",
    )?;
    refuse_when(
        program,
        origin,
        options.align_y.is_some(),
        "`align-y` on text",
    )?;
    refuse_when(
        program,
        origin,
        options.shaping.is_some() || options.wrapping.is_some(),
        "shaping or wrapping",
    )?;
    refuse_when(
        program,
        origin,
        options.tracking.is_some_and(|tracking| tracking != 0.0),
        "tracking",
    )?;
    refuse_when(program, origin, options.live.is_some(), "a live region")?;
    refuse_when(
        program,
        origin,
        options.custom_style.is_some(),
        "a custom text style",
    )?;
    refuse_when(
        program,
        origin,
        options.underline.is_some() || options.strikethrough.is_some(),
        "underline or strikethrough",
    )?;
    refuse_when(
        program,
        origin,
        style.background.is_some()
            || style.border_color.is_some()
            || style.border_width != 0
            || style.radius != 0
            || style.padding != [0; 4]
            || style.gap.is_some()
            || style.max_width.is_some()
            || style.items_center
            || style.self_center
            || style.clip
            || style.height_fill,
        "this utility style on text",
    )?;
    let key = key_code(identity, "text", origin, scope, env, program)?;
    let content = format!(
        "({}).to_string()",
        resolved_expr_use_code(program, *value, env, ValueMode::Owned)?
    );
    let size = match options.size {
        Some(size) => Some(clamped_f32_code(
            size,
            "f32::EPSILON",
            "f32::MAX",
            program,
            env,
        )?),
        None => style.text_size.map(|size| format!("{size:?}f32")),
    };
    let monospace = match &options.font {
        Some(ResolvedTextFont::Monospace) => true,
        Some(ResolvedTextFont::Default) => false,
        Some(ResolvedTextFont::Named(_)) => {
            return Err(refused(program, origin, "a named font"));
        }
        None => style.font_monospace,
    };
    let weight = match style.font_weight {
        Some(ResolvedStyleFontWeight::Medium) => "Medium",
        Some(ResolvedStyleFontWeight::Semibold) => "Semibold",
        Some(ResolvedStyleFontWeight::Bold) => "Bold",
        None => "Normal",
    };
    let align_x = match options.align_x {
        None | Some(ResolvedTextAlignment::Default) => None,
        Some(ResolvedTextAlignment::Left) => Some(format!("{WIRE}::AlignX::Left")),
        Some(ResolvedTextAlignment::Center) => Some(format!("{WIRE}::AlignX::Center")),
        Some(ResolvedTextAlignment::Right) => Some(format!("{WIRE}::AlignX::Right")),
        Some(ResolvedTextAlignment::Justified) => {
            return Err(refused(program, origin, "justified text"));
        }
    };
    Ok(format!(
        "{WIRE}::Node::Text {{ key: {key}, content: {content}, size: {}, color: {}, font: {WIRE}::Font {{ monospace: {monospace}, weight: {WIRE}::Weight::{weight} }}, width: {}, align_x: {} }}",
        option_code(size),
        option_code(style.text_color.as_ref().map(rgba_code)),
        dimension_code(
            options.width.as_ref(),
            style.width_fill,
            program,
            env,
            origin
        )?,
        option_code(align_x),
    ))
}

fn input_face_code(
    style: Option<&ResolvedInputStatusStyle>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    origin: OriginId,
) -> Result<Option<String>, Error> {
    let Some(style) = style else {
        return Ok(None);
    };
    refuse_when(
        program,
        origin,
        style.icon_color.is_some(),
        "an input icon colour",
    )?;
    refuse_when(
        program,
        origin,
        style.surface.text_color.is_some(),
        "`text=` on an input state (use `value=`)",
    )?;
    refuse_surface_extras(&style.surface, program, origin)?;
    let plain = ResolvedStyle::default();
    Ok(Some(format!(
        "{WIRE}::InputFace {{ background: {}, border: {}, value: {}, placeholder: {}, selection: {} }}",
        background_code(&style.surface, &plain, program, origin)?,
        border_code(&style.surface, &plain, program, env)?,
        option_code(style.value_color.as_ref().map(rgba_code)),
        option_code(style.placeholder_color.as_ref().map(rgba_code)),
        option_code(style.selection_color.as_ref().map(rgba_code)),
    )))
}

fn input(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let input = program.resolved_input(id)?;
    let origin = input.origin;
    refuse_when(
        program,
        origin,
        input.binding.secret().is_some(),
        "a secret input",
    )?;
    refuse_when(program, origin, input.hint.is_some(), "an input hint")?;
    refuse_when(
        program,
        origin,
        input.disabled.is_some(),
        "`disabled=` on an input",
    )?;
    refuse_when(
        program,
        origin,
        input.accessibility_label.is_some() || input.accessibility_description.is_some(),
        "an accessibility label on an input",
    )?;
    refuse_when(program, origin, input.paste.is_some(), "a paste route")?;
    refuse_when(
        program,
        origin,
        input.padding.is_some()
            || input.text_size.is_some()
            || input.line_height.is_some()
            || input.align.is_some()
            || input.font.is_some()
            || input.icon.is_some()
            || input.custom_style.is_some(),
        "this input option",
    )?;
    refuse_when(
        program,
        origin,
        input.styles.focused_hovered.is_some(),
        "a `focused hovered` input state",
    )?;
    refuse_when(
        program,
        origin,
        !input.utility_style.is_empty(),
        "a utility style on an input",
    )?;
    let state = resolved_input_state(input, env, program)?;
    let binding_constructor = match &state.state {
        Some(StateBinding::App(name)) => {
            let variant = binding_variant(name);
            format!("{message}::{variant} as fn(::std::string::String) -> {message}")
        }
        Some(StateBinding::Component {
            component,
            name,
            scope,
        }) => {
            let variant = component_binding_variant(component, name);
            format!(
                "{{ let __scope = ({}).clone(); move |__value| {message}::{variant}(__scope.clone(), __value) }}",
                borrowed_scope(scope)
            )
        }
        None => {
            return Err(program.invariant_at_origin(
                origin,
                "normalized input binding is absent from the state environment",
            ));
        }
    };
    let constructor = input
        .change
        .as_ref()
        .map(|route| {
            resolved_interaction_route_callback_code(
                route,
                "__value",
                &["__value"],
                env,
                program,
                message,
            )
        })
        .transpose()?
        .unwrap_or(binding_constructor);
    let on_submit = input
        .submit
        .as_ref()
        .map(|route| resolved_interaction_route_code(route, &[], env, program, message))
        .transpose()?
        .map(|activate| format!("{SLOTS}::message({activate})"));
    let secure = match input.secure {
        Some(expression) => resolved_expr_use_code(program, expression, env, ValueMode::Owned)?,
        None => "false".into(),
    };
    let key = key_code(identity, "input", origin, scope, env, program)?;
    let active = input_face_code(input.styles.active.as_ref(), program, env, origin)?
        .unwrap_or_else(|| format!("{WIRE}::InputFace::default()"));
    Ok(format!(
        "{WIRE}::Node::Input {{ key: {key}, placeholder: ::std::string::String::from({}), value: ({}).to_string(), on_input: {SLOTS}::handler(::std::boxed::Box::new({constructor})), on_submit: {}, width: {}, secure: ({secure}), style: {WIRE}::InputStyle {{ active: {active}, hovered: {}, focused: {}, disabled: {} }} }}",
        rust_string(&input.label),
        state.code,
        option_code(on_submit),
        dimension_code(input.width.as_ref(), false, program, env, origin)?,
        option_code(input_face_code(
            input.styles.hovered.as_ref(),
            program,
            env,
            origin
        )?),
        option_code(input_face_code(
            input.styles.focused.as_ref(),
            program,
            env,
            origin
        )?),
        option_code(input_face_code(
            input.styles.disabled.as_ref(),
            program,
            env,
            origin
        )?),
    ))
}

fn button_face_code(
    style: Option<&ResolvedButtonStatusStyle>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    origin: OriginId,
) -> Result<Option<String>, Error> {
    let Some(style) = style else {
        return Ok(None);
    };
    refuse_surface_extras(&style.surface, program, origin)?;
    let plain = ResolvedStyle::default();
    Ok(Some(format!(
        "{WIRE}::Face {{ background: {}, text: {}, border: {} }}",
        background_code(&style.surface, &plain, program, origin)?,
        option_code(style.surface.text_color.as_ref().map(rgba_code)),
        border_code(&style.surface, &plain, program, env)?,
    )))
}

#[allow(clippy::too_many_arguments)]
fn button(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    content: Option<&ViewId>,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let button = program.resolved_button(id)?;
    let origin = button.origin;
    refuse_when(
        program,
        origin,
        button.checked.is_some() || button.expanded.is_some(),
        "`checked=`/`expanded=` on a button",
    )?;
    refuse_when(
        program,
        origin,
        button.accessibility_description.is_some(),
        "an accessibility description on a button",
    )?;
    let label = match button.accessibility_label {
        Some(label) => Some(format!(
            "::std::string::String::from({})",
            resolved_expr_use_code(program, label, env, ValueMode::Owned)?
        )),
        None => None,
    };
    refuse_when(program, origin, button.clip.is_some(), "`clip` on a button")?;
    refuse_when(
        program,
        origin,
        button.custom_style.is_some(),
        "a custom button style",
    )?;
    refuse_when(
        program,
        origin,
        button.preset != ResolvedButtonPreset::Primary,
        "a button preset (give the states explicitly)",
    )?;
    refuse_when(
        program,
        origin,
        !button.utility_style.is_empty(),
        "a utility style on a button",
    )?;
    let key = key_code(identity, "button", origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    let content = match (&button.content, content) {
        (ResolvedButtonContent::Label(label), _) => format!(
            "{WIRE}::ButtonContent::Label(::std::string::String::from({}))",
            rust_string(label)
        ),
        (ResolvedButtonContent::Child(_), Some(child)) => format!(
            "{WIRE}::ButtonContent::Child(::std::boxed::Box::new({}))",
            render_node(*child, program, message, env, &child_scope, slot)?
        ),
        (ResolvedButtonContent::Child(_), None) => {
            return Err(program.invariant_at_origin(origin, "button child is absent from the HIR"));
        }
    };
    let activate = resolved_interaction_route_code(&button.route, &[], env, program, message)?;
    let on_press = match button.disabled {
        Some(disabled) => format!(
            "if ({}) {{ ::std::option::Option::None }} else {{ ::std::option::Option::Some({SLOTS}::message({activate})) }}",
            resolved_expr_use_code(program, disabled, env, ValueMode::Owned)?
        ),
        None => format!("::std::option::Option::Some({SLOTS}::message({activate}))"),
    };
    let padding = match button.padding {
        Some(padding) => Some(format!(
            "{WIRE}::Edges::all(({}) as f32)",
            resolved_expr_use_code(program, padding, env, ValueMode::Owned)?
        )),
        None => None,
    };
    let active = button_face_code(button.styles.active.as_ref(), program, env, origin)?
        .unwrap_or_else(|| format!("{WIRE}::Face::default()"));
    Ok(format!(
        "{WIRE}::Node::Button {{ key: {key}, content: {content}, label: {}, on_press: {on_press}, width: {}, height: {}, padding: {}, style: {WIRE}::ButtonStyle {{ active: {active}, hovered: {}, pressed: {}, disabled: {} }} }}",
        option_code(label),
        dimension_code(button.width.as_ref(), false, program, env, origin)?,
        dimension_code(button.height.as_ref(), false, program, env, origin)?,
        option_code(padding),
        option_code(button_face_code(
            button.styles.hovered.as_ref(),
            program,
            env,
            origin
        )?),
        option_code(button_face_code(
            button.styles.pressed.as_ref(),
            program,
            env,
            origin
        )?),
        option_code(button_face_code(
            button.styles.disabled.as_ref(),
            program,
            env,
            origin
        )?),
    ))
}

fn space(
    id: ViewId,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let space = program.resolved_space(id)?;
    Ok(format!(
        "{WIRE}::Node::Space {{ width: {}, height: {} }}",
        dimension_code(space.width.as_ref(), false, program, env, space.origin)?,
        dimension_code(space.height.as_ref(), false, program, env, space.origin)?,
    ))
}

fn rule(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let rule = program.resolved_rule(id)?;
    let origin = rule.origin;
    refuse_when(program, origin, rule.fill.is_some(), "a rule fill")?;
    refuse_when(
        program,
        origin,
        rule.preset != ResolvedRulePreset::Default,
        "a rule style preset",
    )?;
    refuse_when(program, origin, rule.snap.is_some(), "`snap` on a rule")?;
    let rounded = rule.radius.all.is_some()
        || rule.radius.top_left.is_some()
        || rule.radius.top_right.is_some()
        || rule.radius.bottom_right.is_some()
        || rule.radius.bottom_left.is_some();
    refuse_when(program, origin, rounded, "a rule radius")?;
    let axis = match rule.axis {
        ResolvedRuleAxis::Horizontal => "Row",
        ResolvedRuleAxis::Vertical => "Column",
    };
    Ok(format!(
        "{WIRE}::Node::Rule {{ key: {}, axis: {WIRE}::Axis::{axis}, thickness: ({}) as f32, color: {} }}",
        key_code(identity, "rule", origin, scope, env, program)?,
        resolved_expr_use_code(program, rule.thickness, env, ValueMode::Owned)?,
        option_code(rule.color.as_ref().map(rgba_code)),
    ))
}
