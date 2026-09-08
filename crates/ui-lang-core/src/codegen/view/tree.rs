//! The `tree` target's view emitter: a node becomes Rust that builds a
//! `ui_lang_wire::Node` with every value inlined, instead of an iced widget.
//!
//! Only the leaves and layouts differ from the native emitter. Control flow
//! (`if`, `for`, `match`), components and slots are emitted by the shared
//! code, which pushes whatever `render_node` returns — here a `Node` — into
//! the parent's child list; so a `for` over a list of rows compiles to the
//! same loop for both targets and only the row inside changes.
//!
//! A construct the tree does not model (`float`, table, a gradient
//! background...) fails the build, naming the construct and its `.ice`
//! line, rather than rendering as something else. The host has a fixed
//! vocabulary; a view module is written to it.
//!
//! Extern widgets and shaders reach past that vocabulary:
//! each becomes a `Surface` node the host paints itself, by the extern's name,
//! with the call's typed data arguments copied across the wire. No Rust
//! function is called on the guest side; the declaration only types the call.
//!
//! An `svg` crosses as bytes the guest holds — an embedded asset or a
//! `memory` source — under a content hash, and only the first frame that
//! shows it carries the bytes (`ui_lang_guest::slots::picture`). A path the
//! guest would read at runtime is refused: a module has no filesystem.
//!
//! Interaction: a button's message goes into the guest's per-frame table
//! (`ui_lang_guest::slots::message`) and the node carries the index; an
//! input's `String -> Message` constructor likewise (`slots::handler`), as
//! do an editor's, a checkbox's `bool`, a slider's `f32`, a pick list's
//! option index, a mouse area's pointer position and scroll delta. An
//! `editor` state field is a `String` on this target (`editor_type_code`):
//! the host owns the `text_editor::Content`, and the guest hears the whole
//! text back like an input's. Colours are resolved through the app's
//! palette here and cross as RGBA.
//!
//! A per-state style given as literal colours crosses as a set of faces the
//! host paints over its own theme; a style given as a Rust callback
//! (`style=some_fn(…)`) is refused, since no Rust runs on the host's side.

use super::*;
mod button;
mod canvas;
mod flex;
mod lists;
pub(super) use flex::item_code as flex_item_code;
pub(super) use lists::keyed_column;
mod pin;
mod qr;
mod responsive;
mod rich_text;
mod text;
mod tooltip;
pub(in crate::codegen) use responsive::render_container_condition;

// Reached through the guest crate, which is the app's one dependency: it
// re-exports the wire so a module never names `ui_lang_wire` itself.
const WIRE: &str = "::ui_lang_guest::wire";
mod layers;

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
        ResolvedViewKind::Sensor { content } => sensor(
            node, identity, *content, document, message, env, scope, slot,
        )?,
        ResolvedViewKind::MouseArea { content } => mouse_area(
            node, identity, *content, document, message, env, scope, slot,
        )?,
        ResolvedViewKind::Canvas => canvas::render(node, identity, document, env, scope)?,
        ResolvedViewKind::ResponsiveSize { content } => responsive::render(
            node, identity, *content, document, message, env, scope, slot,
        )?,
        ResolvedViewKind::Text | ResolvedViewKind::RichText => {
            text(node, identity, document, message, env, scope)?
        }
        ResolvedViewKind::QrCode => qr::render(
            document.resolved_qr_code(node)?,
            identity,
            document,
            env,
            scope,
        )?,
        ResolvedViewKind::Media => svg(node, identity, document, env, scope)?,
        ResolvedViewKind::Input => input(node, identity, document, message, env, scope)?,
        ResolvedViewKind::TextEditor => editor(node, identity, document, message, env, scope)?,
        ResolvedViewKind::Markdown => {
            markdown_surface(node, identity, document, message, env, scope)?
        }
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
        ResolvedViewKind::Pin { content } => pin::render(
            node, identity, *content, document, message, env, scope, slot,
        )?,
        ResolvedViewKind::Tooltip { content, tip } => tooltip::render(
            node, identity, *content, *tip, document, message, env, scope, slot,
        )?,
        ResolvedViewKind::Overlay { content, layer } => layers::overlay(
            node, identity, *content, *layer, document, message, env, scope, slot,
        )?,
        ResolvedViewKind::Space => space(node, document, env)?,
        ResolvedViewKind::Rule => rule(node, identity, document, env, scope)?,
        ResolvedViewKind::Checkbox | ResolvedViewKind::Toggler | ResolvedViewKind::Radio => {
            boolean(node, identity, document, message, env, scope)?
        }
        ResolvedViewKind::Slider => slider(node, identity, document, message, env, scope)?,
        ResolvedViewKind::PickList => pick_list(node, identity, document, message, env, scope)?,
        ResolvedViewKind::Progress => progress(node, identity, document, env, scope)?,
        ResolvedViewKind::ExternComponent | ResolvedViewKind::Shader => {
            surface(node, identity, document, message, env, scope)?
        }
        // Rendered by the shared emitters: their code is target-neutral.
        ResolvedViewKind::Component { .. }
        | ResolvedViewKind::Slot { .. }
        | ResolvedViewKind::KeyedColumn { .. }
        | ResolvedViewKind::Lazy { .. }
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

fn radius_explicit(radius: &ResolvedContainerRadius) -> bool {
    radius.all.is_some()
        || radius.top_left.is_some()
        || radius.top_right.is_some()
        || radius.bottom_right.is_some()
        || radius.bottom_left.is_some()
}

fn radius_code(
    radius: &ResolvedContainerRadius,
    utility: u16,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    if !radius_explicit(radius) {
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

/// A border from its parts, any of which may be absent. A radius alone still
/// needs a border (the host rounds the background through it); omitted
/// fields must retain the host style, including earlier state faces.
fn border_parts_code(
    color: Option<&ResolvedThemeColor>,
    width: Option<String>,
    radius: Option<String>,
) -> String {
    if color.is_none() && width.is_none() && radius.is_none() {
        return option_code(None);
    }
    let color = option_code(color.map(rgba_code));
    let width = option_code(width);
    let radius = option_code(radius);
    option_code(Some(format!(
        "{WIRE}::Border {{ color: {color}, width: {width}, radius: {radius} }}"
    )))
}

/// A border from a surface's border colour, width and radius, plus the
/// utility equivalents.
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
    Ok(border_parts_code(color, width, radius))
}

/// A border from a control's colour, width and radius options.
fn control_border_code(
    color: Option<&ResolvedThemeColor>,
    width: Option<CheckedExprUseId>,
    radius: &ResolvedContainerRadius,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let width = width
        .map(|width| clamped_f32_code(width, "0.0", "f32::MAX", program, env))
        .transpose()?;
    Ok(border_parts_code(
        color,
        width,
        radius_code(radius, 0, program, env)?,
    ))
}

/// A background that is one colour; a gradient is refused.
fn plain_background_code(
    background: Option<&ResolvedContainerBackground>,
    program: &LoweredProgram,
    origin: OriginId,
) -> Result<String, Error> {
    Ok(option_code(match background {
        Some(ResolvedContainerBackground::Color(color)) => Some(rgba_code(color)),
        Some(ResolvedContainerBackground::Linear { .. }) => {
            return Err(refused(program, origin, "a gradient background"));
        }
        None => None,
    }))
}

fn bool_option_code(
    value: Option<CheckedExprUseId>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(option_code(
        value
            .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
            .transpose()?,
    ))
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

fn refuse_shadow(
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
    )
}

/// A box carries `px-snap`; every other surface (a button's or input's
/// face, a pick list's) has no room for it.
fn refuse_surface_extras(
    surface: &ResolvedContainerSurface,
    program: &LoweredProgram,
    origin: OriginId,
) -> Result<(), Error> {
    refuse_shadow(surface, program, origin)?;
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

/// The surface a layout's utilities paint (`@bg-…`, `@border-…`, `@r-…`):
/// natively a styled container around the row or column, here the layout
/// node's own `background` and `border`, which the host draws around it.
fn layout_surface_code(style: &ResolvedStyle) -> (String, String) {
    let background = option_code(style.background.as_ref().map(rgba_code));
    let border = border_parts_code(
        style.border_color.as_ref(),
        (style.border_width != 0).then(|| format!("{}.0", style.border_width)),
        (style.radius != 0).then(|| format!("[{}.0; 4]", style.radius)),
    );
    (background, border)
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
    if matches!(
        layout.mode,
        ResolvedLayoutMode::Stack(_) | ResolvedLayoutMode::Hover(_)
    ) {
        return layers::layout(
            layout, identity, children, program, message, env, scope, slot,
        );
    }
    if let ResolvedLayoutMode::Flex(flex) = &layout.mode {
        return flex::render(
            layout, flex, identity, children, program, message, env, scope, slot,
        );
    }
    let origin = layout.origin;
    let style = &layout.utility_style;
    refuse_box_utilities(style, program, origin)?;
    let (background, border) = layout_surface_code(style);
    let key = key_code(identity, "layout", origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    match &layout.mode {
        ResolvedLayoutMode::Linear(linear) => {
            let wrap = if linear.wrap {
                let spacing = linear
                    .wrap_spacing
                    .map(|value| {
                        resolved_expr_use_code(program, value, env, ValueMode::Owned)
                            .map(|value| format!("({value}) as f32"))
                    })
                    .transpose()?;
                format!(
                    "Some({WIRE}::Wrap {{ spacing: {}, align: {} }})",
                    option_code(spacing),
                    option_code(linear.wrap_align.map(align_x_code))
                )
            } else {
                "None".into()
            };
            refuse_when(
                program,
                origin,
                linear.virtual_row.is_none() && linear.max_width.is_some(),
                "`max-w`",
            )?;
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
            if let Some(estimate) = linear.virtual_row {
                let estimate = resolved_expr_use_code(program, estimate, env, ValueMode::Owned)?;
                let max_width = option_code(
                    linear
                        .max_width
                        .map(|value| {
                            resolved_expr_use_code(program, value, env, ValueMode::Owned)
                                .map(|value| format!("({value}) as f32"))
                        })
                        .transpose()?,
                );
                write!(body, " {WIRE}::Node::KeyedColumn {{ key: {key}, keys: None, children: __children, spacing: {}, padding: {}, width: {}, height: {}, align: {}, max_width: {max_width}, virtual_row: Some(({estimate}) as f32), background: {background}, border: {border} }} }}",
                    option_code(spacing), edges_code(&linear.padding, style.padding, program, env)?,
                    dimension_code(linear.width.as_ref(), style.width_fill, program, env, origin)?,
                    dimension_code(linear.height.as_ref(), style.height_fill, program, env, origin)?, option_code(align),
                ).unwrap();
                return Ok(body);
            }
            write!(
                body,
                " {WIRE}::Node::Linear {{ key: {key}, wrap: {wrap}, axis: {WIRE}::Axis::{axis}, spacing: {}, padding: {}, width: {}, height: {}, align: {}, background: {background}, border: {border}, children: __children }} }}",
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
                scroll.route.is_some() || scroll.viewport_route.is_some(),
                "a scroll route",
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
            let pixels = |value: Option<CheckedExprUseId>| {
                value
                    .map(|value| clamped_f32_code(value, "0.0", "f32::MAX", program, env))
                    .transpose()
                    .map(option_code)
            };
            let anchor = |anchor| {
                let name = match anchor {
                    ResolvedScrollAnchor::Start => "Start",
                    ResolvedScrollAnchor::End => "End",
                    ResolvedScrollAnchor::Keep => "Keep",
                };
                format!("{WIRE}::ScrollAnchor::{name}")
            };
            let auto_scroll = match scroll.auto_scroll {
                Some(value) => resolved_expr_use_code(program, value, env, ValueMode::Owned)?,
                None => "false".into(),
            };
            let content = render_node(children[0], program, message, env, &child_scope, slot)?;
            let virtual_rows = children.iter().try_fold(false, |found, child| {
                super::layout::contains_virtual_rows(*child, program, slot)
                    .map(|value| found || value)
            })?;
            Ok(format!(
                "{WIRE}::Node::Scroll {{ virtual_rows: {virtual_rows}, key: {key}, direction: {WIRE}::ScrollDirection::{direction}, width: {}, height: {}, bar_hidden: {}, bar_width: {}, bar_margin: {}, scroller_width: {}, bar_spacing: {}, anchor_x: {}, anchor_y: {}, auto_scroll: ({auto_scroll}), background: {background}, border: {border}, content: ::std::boxed::Box::new({content}) }}",
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
                scroll.hidden_bar,
                pixels(scroll.bar_width)?,
                pixels(scroll.bar_margin)?,
                pixels(scroll.scroller_width)?,
                pixels(scroll.bar_spacing)?,
                anchor(scroll.anchor_x),
                anchor(scroll.anchor_y),
            ))
        }
        ResolvedLayoutMode::Grid(grid) => {
            let number = |expression: CheckedExprUseId| {
                resolved_expr_use_code(program, expression, env, ValueMode::Owned)
                    .map(|code| format!("({code}) as f32"))
            };
            let columns = grid
                .columns
                .map(|columns| {
                    resolved_expr_use_code(program, columns, env, ValueMode::Owned)
                        .map(|code| format!("u32::try_from({code}).unwrap_or(0)"))
                })
                .transpose()?;
            let spacing = match grid.spacing {
                Some(spacing) => Some(number(spacing)?),
                None => style.gap.map(|gap| format!("{gap}.0")),
            };
            let width = match grid.width {
                Some(width) => Some(format!("{WIRE}::Length::Fixed({})", number(width)?)),
                None => style.width_fill.then(|| format!("{WIRE}::Length::Fill")),
            };
            let (height, aspect) = match &grid.height {
                Some(ResolvedGridHeight::EvenlyDistribute(length)) => {
                    (Some(length_code(length, program, env, origin)?), None)
                }
                Some(ResolvedGridHeight::AspectRatio { width, height }) => (
                    None,
                    Some(format!("{} / {}", number(*width)?, number(*height)?)),
                ),
                None => (
                    style.height_fill.then(|| format!("{WIRE}::Length::Fill")),
                    None,
                ),
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
                " {WIRE}::Node::Grid {{ key: {key}, columns: {}, fluid: {}, spacing: {}, padding: {}, width: {}, height: {}, aspect: {}, background: {background}, border: {border}, children: __children }} }}",
                option_code(columns),
                option_code(grid.max_cell.map(number).transpose()?),
                option_code(spacing),
                edges_code(
                    &ResolvedContainerPadding::default(),
                    style.padding,
                    program,
                    env
                )?,
                option_code(width),
                option_code(height),
                option_code(aspect),
            )
            .unwrap();
            Ok(body)
        }
        ResolvedLayoutMode::Stack(_) | ResolvedLayoutMode::Hover(_) => {
            unreachable!("handled layered layout")
        }
        ResolvedLayoutMode::Flex(_) => unreachable!("handled flex layout"),
    }
}

/// A sensor's size routes cross as `(f32, f32)` handlers: the host answers
/// with the child's own laid-out size (see `wire::Event::Size`), never a
/// window position. `key=` is refused: the wire's node key is the node's
/// identity, and a second key that resets the sensor when it changes has no
/// field to cross in.
#[allow(clippy::too_many_arguments)]
fn sensor(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    content: ViewId,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let sensor = program.resolved_sensor(id)?;
    let origin = sensor.origin;
    refuse_when(program, origin, sensor.key.is_some(), "a sensor key")?;
    let key = key_code(identity, "sensor", origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    let child = render_node(content, program, message, env, &child_scope, slot)?;
    let mut size_handlers = Vec::new();
    for route in [&sensor.show, &sensor.resize] {
        let handler = route
            .as_ref()
            .map(|route| {
                let callback = snapshot_callback(
                    route,
                    "__size: (f64, f64)",
                    &["__size.0", "__size.1"],
                    env,
                    program,
                    message,
                )?;
                Ok::<_, Error>(handler_code(
                    "(f32, f32)",
                    message,
                    &callback,
                    "move |__sent: (f32, f32)| ::std::option::Option::Some(__route((f64::from(__sent.0), f64::from(__sent.1))))",
                ))
            })
            .transpose()?;
        size_handlers.push(option_code(handler));
    }
    let on_hide = sensor
        .hide
        .as_ref()
        .map(|route| resolved_interaction_route_code(route, &[], env, program, message))
        .transpose()?
        .map(|activate| format!("{SLOTS}::message({activate})"));
    let number = |expression: Option<CheckedExprUseId>| {
        expression
            .map(|expression| {
                resolved_expr_use_code(program, expression, env, ValueMode::Owned)
                    .map(|code| format!("({code}) as f32"))
            })
            .transpose()
    };
    Ok(format!(
        "{WIRE}::Node::Sensor {{ key: {key}, on_show: {}, on_resize: {}, on_hide: {}, anticipate: {}, delay: {}, child: ::std::boxed::Box::new({child}) }}",
        size_handlers[0],
        size_handlers[1],
        option_code(on_hide),
        option_code(number(sensor.anticipate)?),
        option_code(number(sensor.delay_ms)?),
    ))
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
    let mut allowed = style.clone();
    allowed.clip = false;
    allowed.max_width = None;
    refuse_box_utilities(&allowed, program, origin)?;
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
    refuse_shadow(&container.surface, program, origin)?;
    let key = key_code(identity, "container", origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    let content = render_node(content, program, message, env, &child_scope, slot)?;
    let pixels = |value: Option<CheckedExprUseId>| {
        value
            .map(|value| clamped_f32_code(value, "0.0", "f32::MAX", program, env))
            .transpose()
    };
    let max_width = option_code(
        pixels(container.max_width)?
            .or_else(|| style.max_width.map(|value| format!("{value:?}f32"))),
    );
    let max_height = option_code(pixels(container.max_height)?);
    let clip = container
        .clip
        .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| style.clip.to_string());
    Ok(format!(
        "{WIRE}::Node::Container {{ max_width: {max_width}, max_height: {max_height}, clip: {clip}, key: {key}, width: {}, height: {}, padding: {}, align_x: {}, align_y: {}, background: {}, border: {}, snap: {}, content: ::std::boxed::Box::new({content}) }}",
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
        bool_option_code(container.surface.pixel_snap, program, env)?,
    ))
}

/// A mouse area crosses with every route it names: a discrete one (a press,
/// an enter) as a message index, a positional one (`move=`, `press-at=`) as
/// a `(f32, f32)` handler the host answers with the pointer's position in
/// the area's own coordinates, and `scroll=` as a `(f32, f32, bool)` one.
/// The cursor is refused: the host paints its own.
#[allow(clippy::too_many_arguments)]
fn mouse_area(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    content: ViewId,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let mouse = program.resolved_mouse_area(id)?;
    let origin = mouse.origin;
    refuse_when(
        program,
        origin,
        mouse.interaction.is_some() || mouse.interaction_expression.is_some(),
        "a mouse cursor",
    )?;
    let key = key_code(identity, "mouse", origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    let content = render_node(content, program, message, env, &child_scope, slot)?;
    let discrete = |route: &Option<ResolvedInteractionRoute>| -> Result<String, Error> {
        Ok(option_code(
            route
                .as_ref()
                .map(|route| resolved_interaction_route_code(route, &[], env, program, message))
                .transpose()?
                .map(|activate| format!("{SLOTS}::message({activate})")),
        ))
    };
    let positional = |route: &Option<ResolvedInteractionRoute>| -> Result<String, Error> {
        Ok(option_code(match route {
            Some(route) => Some(handler_code(
                "(f32, f32)",
                message,
                &snapshot_callback(
                    route,
                    "__point: (f64, f64)",
                    &["__point.0", "__point.1"],
                    env,
                    program,
                    message,
                )?,
                "move |__sent: (f32, f32)| ::std::option::Option::Some(__route((f64::from(__sent.0), f64::from(__sent.1))))",
            )),
            None => None,
        }))
    };
    let on_scroll = match &mouse.scroll {
        Some(route) => Some(handler_code(
            "(f32, f32, bool)",
            message,
            &snapshot_callback(
                route,
                "__delta: (f64, f64, bool)",
                &["__delta.0", "__delta.1", "__delta.2"],
                env,
                program,
                message,
            )?,
            "move |__sent: (f32, f32, bool)| ::std::option::Option::Some(__route((f64::from(__sent.0), f64::from(__sent.1), __sent.2)))",
        )),
        None => None,
    };
    Ok(format!(
        "{WIRE}::Node::MouseArea {{ key: {key}, on_press: {}, on_release: {}, on_double_click: {}, on_right_press: {}, on_right_release: {}, on_middle_press: {}, on_middle_release: {}, on_enter: {}, on_exit: {}, on_move: {}, on_press_at: {}, on_scroll: {}, content: ::std::boxed::Box::new({content}) }}",
        discrete(&mouse.press)?,
        discrete(&mouse.release)?,
        discrete(&mouse.double_click)?,
        discrete(&mouse.right_press)?,
        discrete(&mouse.right_release)?,
        discrete(&mouse.middle_press)?,
        discrete(&mouse.middle_release)?,
        discrete(&mouse.enter)?,
        discrete(&mouse.exit)?,
        positional(&mouse.move_route)?,
        positional(&mouse.press_at)?,
        option_code(on_scroll),
    ))
}

fn text(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let text = program.resolved_text(id)?;
    let origin = text.origin;
    let options = &text.options;
    let style = &text.utility_style;
    refuse_when(program, origin, options.live.is_some(), "a live region")?;
    refuse_when(program, origin, options.heading.is_some(), "a heading")?;
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
            || style.clip,
        "this utility style on text",
    )?;
    let key = key_code(identity, "text", origin, scope, env, program)?;
    let size = match options.size {
        Some(size) => Some(clamped_f32_code(
            size,
            "f32::EPSILON",
            "f32::MAX",
            program,
            env,
        )?),
        None => style
            .text_size
            .map(|size| format!("{size:?}f32"))
            .or_else(|| {
                program
                    .settings()
                    .default_text_size
                    .map(|size| format!("{size:?}f32"))
            }),
    };
    let monospace = match &options.font {
        Some(ResolvedTextFont::Monospace) => true,
        Some(ResolvedTextFont::Default) => false,
        Some(ResolvedTextFont::Named(_)) => false,
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
    let text_options = text::options(text, program, env)?;
    let fields = format!(
        "options: {text_options}, key: {key}, size: {}, color: {}, font: {WIRE}::Font {{ monospace: {monospace}, weight: {WIRE}::Weight::{weight} }}, width: {}, align_x: {}",
        option_code(size),
        option_code(match &text.content {
            ResolvedTextContent::Rich {
                color: Some(color), ..
            } => Some(rgba_code(color)),
            _ => style.text_color.as_ref().map(rgba_code),
        }),
        dimension_code(
            options.width.as_ref(),
            style.width_fill,
            program,
            env,
            origin
        )?,
        option_code(align_x),
    );
    match &text.content {
        ResolvedTextContent::Plain { value } => {
            let value = resolved_expr_use_code(program, *value, env, ValueMode::Owned)?;
            Ok(format!(
                "{WIRE}::Node::Text {{ {fields}, content: ({value}).to_string() }}"
            ))
        }
        ResolvedTextContent::Rich {
            children, route, ..
        } => {
            refuse_when(
                program,
                origin,
                options.tracking.is_some() || options.shaping.is_some(),
                "tracking or shaping on rich text",
            )?;
            rich_text::render(children, route.as_ref(), &fields, program, message, env)
        }
    }
}

fn svg(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let media = program.resolved_media(id)?;
    let origin = media.origin;
    if media.kind != ResolvedMediaKind::Svg {
        return Err(refused(program, origin, "media"));
    }
    let options = &media.options;
    refuse_when(
        program,
        origin,
        options.accessibility_description.is_some(),
        "an accessibility description on an svg",
    )?;
    refuse_when(
        program,
        origin,
        options.svg_style.is_some(),
        "an svg style callback",
    )?;
    let inherit = options.svg_inherits_button_ink;
    let bytes = if options.svg_memory {
        match media.source_type {
            Type::Bytes => resolved_expr_use_code(program, media.source, env, ValueMode::Owned)?,
            _ => format!(
                "({}).as_bytes()",
                resolved_expr_use_code(program, media.source, env, ValueMode::Borrowed)?
            ),
        }
    } else {
        embedded_asset_bytes_code(program, media.source)
            .ok_or_else(|| refused(program, origin, "an svg read from a path (use `memory`)"))?
    };
    let dimension = |length: Option<&ResolvedMediaLength>| -> Result<String, Error> {
        Ok(option_code(match length {
            None => None,
            Some(ResolvedMediaLength::Fill) => Some(format!("{WIRE}::Length::Fill")),
            Some(ResolvedMediaLength::FillPortion(portion)) => {
                Some(format!("{WIRE}::Length::FillPortion({portion})"))
            }
            Some(ResolvedMediaLength::Shrink) => Some(format!("{WIRE}::Length::Shrink")),
            Some(ResolvedMediaLength::Fixed { source, .. }) if *source == Type::Length => {
                return Err(refused(program, origin, "a `length` value"));
            }
            Some(ResolvedMediaLength::Fixed { expression, .. }) => Some(format!(
                "{WIRE}::Length::Fixed(({}) as f32)",
                resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
            )),
        }))
    };
    let label = options
        .accessibility_label
        .map(|label| {
            resolved_expr_use_code(program, label, env, ValueMode::Owned)
                .map(|code| format!("::std::string::String::from({code})"))
        })
        .transpose()?;
    let color = options
        .svg_colors
        .as_ref()
        .and_then(|colors| colors.idle.as_ref())
        .map(rgba_code);
    // `hover=` defaults to the idle colour; only one that differs crosses.
    let hover = options
        .svg_colors
        .as_ref()
        .and_then(|colors| colors.hovered.as_ref())
        .filter(|hovered| {
            hovered.as_ref()
                != options
                    .svg_colors
                    .as_ref()
                    .and_then(|colors| colors.idle.as_ref())
        })
        .map(|hovered| option_code(hovered.as_ref().map(rgba_code)));
    let fit = options
        .fit
        .map(|fit| {
            resolved_expr_use_code(program, fit, env, ValueMode::Owned).map(|code| {
                format!(
                    "match ({code}) {{ ::iced::ContentFit::Contain => {WIRE}::ContentFit::Contain, ::iced::ContentFit::Cover => {WIRE}::ContentFit::Cover, ::iced::ContentFit::Fill => {WIRE}::ContentFit::Fill, ::iced::ContentFit::None => {WIRE}::ContentFit::None, ::iced::ContentFit::ScaleDown => {WIRE}::ContentFit::ScaleDown }}"
                )
            })
        })
        .transpose()?;
    let rotation = options
        .rotation
        .map(|rotation| {
            resolved_expr_use_code(program, rotation, env, ValueMode::Owned).map(|code| {
                format!(
                    "match ({code}) {{ ::iced::Rotation::Floating(__radians) => {WIRE}::Rotation::Floating(__radians.0), ::iced::Rotation::Solid(__radians) => {WIRE}::Rotation::Solid(__radians.0) }}"
                )
            })
        })
        .transpose()?;
    let opacity = options
        .opacity
        .map(|opacity| clamped_f32_code(opacity, "0.0", "1.0", program, env))
        .transpose()?;
    Ok(format!(
        "{{ let (__hash, __bytes) = {SLOTS}::picture({bytes}); {WIRE}::Node::Svg {{ inherit_button_ink: {inherit}, key: {}, hash: __hash, bytes: __bytes, label: {}, color: {}, hover: {}, fit: {}, rotation: {}, opacity: {}, width: {}, height: {} }} }}",
        key_code(identity, "media", origin, scope, env, program)?,
        option_code(label),
        option_code(color),
        option_code(hover),
        option_code(fit),
        option_code(rotation),
        option_code(opacity),
        dimension(options.width.as_ref())?,
        dimension(options.height.as_ref())?,
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
    refuse_when(program, origin, input.paste.is_some(), "a paste route")?;
    refuse_when(
        program,
        origin,
        input.icon.is_some() || input.custom_style.is_some(),
        "this input option",
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
    let string = |value| resolved_expr_use_code(program, value, env, ValueMode::Owned);
    let hint = input
        .hint
        .map(string)
        .transpose()?
        .unwrap_or_else(|| "\"\"".into());
    let label = input
        .accessibility_label
        .map(string)
        .transpose()?
        .unwrap_or_else(|| rust_string(&input.label));
    let description = option_code(
        input
            .accessibility_description
            .map(string)
            .transpose()?
            .map(|v| format!("({v}).to_string()")),
    );
    let disabled = input
        .disabled
        .map(string)
        .transpose()?
        .unwrap_or_else(|| "false".into());
    let utilities = &input.utility_style;
    let padding = option_code(match input.padding {
        Some(value) => Some(format!("{WIRE}::Edges::all(({}) as f32)", string(value)?)),
        None => utilities.has_padding().then(|| {
            format!(
                "{WIRE}::Edges {{ top: {}f32, right: {}f32, bottom: {}f32, left: {}f32 }}",
                utilities.padding[0],
                utilities.padding[1],
                utilities.padding[2],
                utilities.padding[3]
            )
        }),
    });
    let size = option_code(match input.text_size {
        Some(value) => Some(format!("({}) as f32", string(value)?)),
        None => program
            .settings()
            .default_text_size
            .map(|value| format!("{:?}f32", value.min(f64::from(f32::MAX)))),
    });
    let line_height = option_code(
        input
            .line_height
            .map(string)
            .transpose()?
            .map(|v| format!("({v}) as f32")),
    );
    let align = option_code(input.align.map(|a| format!("{WIRE}::AlignX::{a:?}")));
    let fallback = ResolvedDefaultFont {
        family: FontFamily::SansSerif,
        weight: FontWeight::Normal,
        stretch: FontStretch::Normal,
        style: FontStyle::Normal,
        origin,
    };
    let font = option_code(match &input.font {
        Some(ResolvedTextFont::Named(font)) => Some(text::named_font(font, None)),
        Some(ResolvedTextFont::Default) => Some(text::named_font(&fallback, None)),
        Some(ResolvedTextFont::Monospace) => Some(text::named_font(
            &ResolvedDefaultFont {
                family: FontFamily::Monospace,
                ..fallback
            },
            None,
        )),
        None => program
            .settings()
            .default_font
            .as_ref()
            .map(|font| text::named_font(font, None)),
    });
    let options = format!(
        "{WIRE}::InputOptions {{ label: ({label}).to_string(), description: {description}, disabled: {disabled}, padding: {padding}, text_size: {size}, line_height: {line_height}, align: {align}, font: {font} }}"
    );
    let utility = format!(
        "{WIRE}::InputFace {{ background: {}, border: Some({WIRE}::Border {{ color: {}, width: {}, radius: {} }}), ..Default::default() }}",
        option_code(utilities.background.as_ref().map(rgba_code)),
        option_code(utilities.border_color.as_ref().map(rgba_code)),
        option_code(
            (utilities.border_width != 0).then(|| format!("{}f32", utilities.border_width))
        ),
        option_code((utilities.radius != 0).then(|| format!("[{}f32; 4]", utilities.radius))),
    );
    let focus_border = option_code(utilities.focus_border_color.as_ref().map(rgba_code));
    let focused_hovered = option_code(input_face_code(
        input.styles.focused_hovered.as_ref(),
        program,
        env,
        origin,
    )?);
    let key = key_code(identity, "input", origin, scope, env, program)?;
    let active = input_face_code(input.styles.active.as_ref(), program, env, origin)?
        .unwrap_or_else(|| format!("{WIRE}::InputFace::default()"));
    Ok(format!(
        "{WIRE}::Node::Input {{ options: {options}, key: {key}, placeholder: ::std::string::String::from({}), value: ({}).to_string(), on_input: {}, on_submit: {}, width: {}, secure: ({secure}), style: ::std::boxed::Box::new({WIRE}::InputStyle {{ utility: {utility}, focus_border: {focus_border}, focused_hovered: {focused_hovered}, active: {active}, hovered: {}, focused: {}, disabled: {} }}) }}",
        hint,
        state.code,
        handler_code(
            "::std::string::String",
            message,
            &constructor,
            "move |__sent: ::std::string::String| ::std::option::Option::Some(__route(__sent))"
        ),
        option_code(on_submit),
        dimension_code(
            input.width.as_ref(),
            utilities.width_fill,
            program,
            env,
            origin
        )?,
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

fn editor(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let editor = program.resolved_text_editor(id)?;
    let origin = editor.origin;
    refuse_when(program, origin, editor.action.is_some(), "an editor action")?;
    refuse_when(
        program,
        origin,
        editor.key_binding.is_some(),
        "an editor key binding",
    )?;
    refuse_when(
        program,
        origin,
        editor.highlight.is_some() || editor.highlighter.is_some(),
        "an editor highlighter",
    )?;
    let styles = &editor.styles;
    refuse_when(
        program,
        origin,
        editor.custom_style.is_some()
            || styles.active.is_some()
            || styles.hovered.is_some()
            || styles.focused.is_some()
            || styles.focused_hovered.is_some()
            || styles.disabled.is_some(),
        "a style on an editor",
    )?;
    refuse_when(
        program,
        origin,
        editor.size.is_some()
            || editor.padding.is_some()
            || editor.line_height.is_some()
            || editor.wrapping.is_some()
            || editor.font.is_some(),
        "this editor option",
    )?;
    let state = resolved_editor_state(editor, env, program)?;
    let constructor = match &state.state {
        Some(StateBinding::App(name)) => {
            let variant = editor_variant(name);
            format!("{message}::{variant} as fn(::std::string::String) -> {message}")
        }
        Some(StateBinding::Component {
            component,
            name,
            scope,
        }) => {
            let variant = component_editor_variant(component, name);
            format!(
                "{{ let __scope = ({}).clone(); move |__value| {message}::{variant}(__scope.clone(), __value) }}",
                borrowed_scope(scope)
            )
        }
        None => {
            return Err(program.invariant_at_origin(
                origin,
                "normalized editor binding does not resolve to editor state",
            ));
        }
    };
    let handler = handler_code(
        "::std::string::String",
        message,
        &constructor,
        "move |__sent: ::std::string::String| ::std::option::Option::Some(__route(__sent))",
    );
    let on_edit = match editor.disabled {
        Some(disabled) => format!(
            "if ({}) {{ ::std::option::Option::None }} else {{ ::std::option::Option::Some({handler}) }}",
            resolved_expr_use_code(program, disabled, env, ValueMode::Owned)?
        ),
        None => format!("::std::option::Option::Some({handler})"),
    };
    let pixels = |value: Option<CheckedExprUseId>| -> Result<Option<String>, Error> {
        value
            .map(|value| {
                Ok(format!(
                    "({}) as f32",
                    resolved_expr_use_code(program, value, env, ValueMode::Owned)?
                ))
            })
            .transpose()
    };
    let key = key_code(identity, "editor", origin, scope, env, program)?;
    Ok(format!(
        "{WIRE}::Node::Editor {{ key: {key}, placeholder: {}, text: ({}).to_string(), on_edit: {on_edit}, width: {}, height: {}, min_height: {}, max_height: {} }}",
        editor
            .placeholder
            .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
            .transpose()?
            .unwrap_or_else(|| "::std::string::String::new()".into()),
        state.code,
        option_code(pixels(editor.width)?),
        dimension_code(editor.height.as_ref(), false, program, env, origin)?,
        option_code(pixels(editor.min_height)?),
        option_code(pixels(editor.max_height)?),
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
    let boolean = |value: Option<CheckedExprUseId>| -> Result<String, Error> {
        Ok(option_code(
            value
                .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
                .transpose()?,
        ))
    };
    let checked = boolean(button.checked)?;
    let expanded = boolean(button.expanded)?;
    let description = option_code(
        button
            .accessibility_description
            .map(|value| {
                resolved_expr_use_code(program, value, env, ValueMode::Owned)
                    .map(|value| format!("::std::string::String::from({value})"))
            })
            .transpose()?,
    );
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
    let recipe = button::recipe(button, program, env)?;
    let preset = format!("{WIRE}::ButtonPreset::{:?}", button.preset);
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
        None => button.utility_style.has_padding().then(|| {
            format!(
                "{WIRE}::Edges {{ top: {}f32, right: {}f32, bottom: {}f32, left: {}f32 }}",
                button.utility_style.padding[0],
                button.utility_style.padding[1],
                button.utility_style.padding[2],
                button.utility_style.padding[3],
            )
        }),
    };
    let active = button_face_code(button.styles.active.as_ref(), program, env, origin)?
        .unwrap_or_else(|| format!("{WIRE}::Face::default()"));
    Ok(format!(
        "{WIRE}::Node::Button {{ checked: {checked}, expanded: {expanded}, description: {description}, key: {key}, content: {content}, label: {}, on_press: {on_press}, width: {}, height: {}, padding: {}, style: {WIRE}::ButtonStyle {{ preset: {preset}, recipe: {recipe}, active: {active}, hovered: {}, pressed: {}, disabled: {} }} }}",
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
    let axis = match rule.axis {
        ResolvedRuleAxis::Horizontal => "Row",
        ResolvedRuleAxis::Vertical => "Column",
    };
    Ok(format!(
        "{WIRE}::Node::Rule {{ key: {}, axis: {WIRE}::Axis::{axis}, thickness: ({}) as f32, color: {}, weak: {}, radius: {}, snap: {} }}",
        key_code(identity, "rule", origin, scope, env, program)?,
        resolved_expr_use_code(program, rule.thickness, env, ValueMode::Owned)?,
        option_code(rule.color.as_ref().map(rgba_code)),
        rule.preset == ResolvedRulePreset::Weak,
        option_code(radius_code(&rule.radius, 0, program, env)?),
        bool_option_code(rule.snap, program, env)?,
    ))
}

/// A value-carrying handler in the guest's per-frame table, under the
/// argument type the host sends. `body` maps `__sent` to an
/// `Option<Message>` and may use `__route`, the route's callback.
///
/// The table outlives the view that filled it — the host answers a frame
/// later — so an entry must own everything it holds, where the callback the
/// native emitter builds may borrow a `for` binding or the state. A button
/// sidesteps this by building its message while the view runs; a control
/// with a finite set of answers precomputes one message per answer the
/// same way, and only a slider, whose answer is a number, hands the
/// callback itself over.
/// A route's callback for a value-carrying handler. It crosses as the
/// callback itself (see `handler_code`), so it may hold nothing borrowed:
/// each route argument is evaluated in the view, while the `for` binding
/// or the state it reads is alive, and the closure owns the values and
/// clones one out per answer.
fn snapshot_callback(
    route: &ResolvedInteractionRoute,
    pattern: &str,
    payloads: &[&str],
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    message: &str,
) -> Result<String, Error> {
    let arguments =
        route
            .args
            .iter()
            .filter_map(|arg| match arg {
                ResolvedInteractionRouteArg::Expression(expression) => Some(
                    resolved_expr_use_code(program, *expression, env, ValueMode::Owned),
                ),
                ResolvedInteractionRouteArg::Payload { .. } => None,
            })
            .collect::<Result<Vec<_>, _>>()?;
    let hoists = arguments
        .iter()
        .enumerate()
        .map(|(index, code)| format!("let __route_arg_{index} = {code};"))
        .collect::<String>();
    let snapshots = (0..arguments.len())
        .map(|index| format!("::std::clone::Clone::clone(&__route_arg_{index})"))
        .collect::<Vec<_>>();
    let callback = resolved_interaction_route_callback_with_snapshots(
        route, pattern, payloads, &snapshots, env, program, message,
    )?;
    Ok(format!("{{ {hoists} {callback} }}"))
}

fn handler_code(argument: &str, message: &str, callback: &str, body: &str) -> String {
    format!(
        "{SLOTS}::handler::<{argument}, {message}>(::std::boxed::Box::new({{ let __route = {callback}; {body} }}))"
    )
}

fn boolean(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let control = program.resolved_boolean_control(id)?;
    let origin = control.origin;
    let (name, callback) = match &control.style {
        ResolvedBooleanStyle::Checkbox(style) => ("checkbox", style.custom.is_some()),
        ResolvedBooleanStyle::Toggler(style) => ("toggler", style.custom.is_some()),
        ResolvedBooleanStyle::Radio(style) => ("radio", style.custom.is_some()),
    };
    refuse_when(
        program,
        origin,
        callback,
        &format!("a {name} style callback"),
    )?;
    let options = &control.options;
    refuse_when(
        program,
        origin,
        options.accessibility_label.is_some() || options.accessibility_description.is_some(),
        &format!("an accessibility label on a {name}"),
    )?;
    refuse_when(
        program,
        origin,
        options.size.is_some()
            || options.spacing.is_some()
            || options.text_size.is_some()
            || options.line_height.is_some()
            || options.shaping.is_some()
            || options.wrapping.is_some()
            || options.font.is_some()
            || options.alignment.is_some()
            || options.icon.is_some(),
        &format!("this {name} option"),
    )?;
    let key = key_code(identity, name, origin, scope, env, program)?;
    let label = format!(
        "({}).to_string()",
        resolved_expr_use_code(program, control.label, env, ValueMode::Owned)?
    );
    let checked = resolved_expr_use_code(program, control.checked, env, ValueMode::Owned)?;
    let width = dimension_code(options.width.as_ref(), false, program, env, origin)?;
    if control.kind == ResolvedBooleanKind::Radio {
        let value = control.value.ok_or_else(|| {
            program.invariant_at_origin(origin, "normalized radio value disappeared")
        })?;
        let value = resolved_expr_use_code(program, value, env, ValueMode::Owned)?;
        let activate =
            resolved_interaction_route_code(&control.route, &[&value], env, program, message)?;
        let ResolvedBooleanStyle::Radio(style) = &control.style else {
            return Err(program.invariant_at_origin(origin, "radio style HIR diverged"));
        };
        let face = |style: Option<&ResolvedRadioStatusStyle>| -> Result<String, Error> {
            let Some(style) = style else {
                return Ok(option_code(None));
            };
            Ok(option_code(Some(control_face_code(
                style
                    .background
                    .as_ref()
                    .map(|background| &background.value),
                style.dot_color.as_ref().map(|color| &color.value),
                style.text_color.as_ref().map(|color| &color.value),
                control_border_code(
                    style.border_color.as_ref().map(|color| &color.value),
                    style.border_width,
                    &ResolvedContainerRadius::default(),
                    program,
                    env,
                )?,
                program,
                origin,
            )?)))
        };
        return Ok(format!(
            "{WIRE}::Node::Radio {{ key: {key}, label: {label}, selected: ({checked}), on_select: {SLOTS}::message({activate}), width: {width}, style: {WIRE}::RadioStyle {{ active_on: {}, active_off: {}, hovered_on: {}, hovered_off: {} }} }}",
            face(style.active_selected.as_ref())?,
            face(style.active_unselected.as_ref())?,
            face(style.hovered_selected.as_ref())?,
            face(style.hovered_unselected.as_ref())?,
        ));
    }
    let (kind, style) = match (control.kind, &control.style) {
        (ResolvedBooleanKind::Checkbox, ResolvedBooleanStyle::Checkbox(style)) => (
            "Checkbox",
            checkbox_style_code(style, program, env, origin)?,
        ),
        (ResolvedBooleanKind::Toggler, ResolvedBooleanStyle::Toggler(style)) => {
            ("Switch", toggler_style_code(style, program, env, origin)?)
        }
        _ => {
            return Err(
                program.invariant_at_origin(origin, "boolean control kind and style HIR diverged")
            );
        }
    };
    let callback = resolved_interaction_route_callback_code(
        &control.route,
        "__value",
        &["__value"],
        env,
        program,
        message,
    )?;
    let handler = handler_code(
        "bool",
        message,
        &callback,
        "let __on = __route(true); let __off = __route(false); move |__sent: bool| ::std::option::Option::Some(if __sent { __on.clone() } else { __off.clone() })",
    );
    let on_toggle = match control.disabled {
        Some(disabled) => format!(
            "if ({}) {{ ::std::option::Option::None }} else {{ ::std::option::Option::Some({handler}) }}",
            resolved_expr_use_code(program, disabled, env, ValueMode::Owned)?
        ),
        None => format!("::std::option::Option::Some({handler})"),
    };
    Ok(format!(
        "{WIRE}::Node::Toggle {{ key: {key}, kind: {WIRE}::ToggleKind::{kind}, label: {label}, checked: ({checked}), on_toggle: {on_toggle}, width: {width}, style: {style} }}"
    ))
}

/// One face of a checkbox, toggler or radio: its box, mark, label colour and
/// border.
fn control_face_code(
    background: Option<&ResolvedContainerBackground>,
    mark: Option<&ResolvedThemeColor>,
    text: Option<&ResolvedThemeColor>,
    border: String,
    program: &LoweredProgram,
    origin: OriginId,
) -> Result<String, Error> {
    Ok(format!(
        "{WIRE}::ControlFace {{ background: {}, mark: {}, text: {}, border: {border} }}",
        plain_background_code(background, program, origin)?,
        option_code(mark.map(rgba_code)),
        option_code(text.map(rgba_code)),
    ))
}

fn tone_code(tone: &str) -> String {
    format!("::std::option::Option::Some({WIRE}::Tone::{tone})")
}

fn checkbox_style_code(
    styles: &ResolvedCheckboxStyleSet,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    origin: OriginId,
) -> Result<String, Error> {
    let tone = match styles.preset {
        None | Some(ResolvedCheckboxPreset::Primary) => option_code(None),
        Some(ResolvedCheckboxPreset::Secondary) => tone_code("Secondary"),
        Some(ResolvedCheckboxPreset::Success) => tone_code("Success"),
        Some(ResolvedCheckboxPreset::Danger) => tone_code("Danger"),
    };
    let face = |style: Option<&ResolvedCheckboxStatusStyle>| -> Result<String, Error> {
        let Some(style) = style else {
            return Ok(option_code(None));
        };
        Ok(option_code(Some(control_face_code(
            style
                .background
                .as_ref()
                .map(|background| &background.value),
            style.icon_color.as_ref().map(|color| &color.value),
            style.text_color.as_ref().map(|color| &color.value),
            control_border_code(
                style.border_color.as_ref().map(|color| &color.value),
                style.border_width,
                &style.radius,
                program,
                env,
            )?,
            program,
            origin,
        )?)))
    };
    Ok(format!(
        "{WIRE}::ToggleStyle {{ tone: {tone}, active_on: {}, active_off: {}, hovered_on: {}, hovered_off: {}, disabled_on: {}, disabled_off: {} }}",
        face(styles.active_checked.as_ref())?,
        face(styles.active_unchecked.as_ref())?,
        face(styles.hovered_checked.as_ref())?,
        face(styles.hovered_unchecked.as_ref())?,
        face(styles.disabled_checked.as_ref())?,
        face(styles.disabled_unchecked.as_ref())?,
    ))
}

/// A toggler's track is the face's background and border, its knob the
/// mark. The knob's own border and the padding ratio have no room.
fn toggler_style_code(
    styles: &ResolvedTogglerStyleSet,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    origin: OriginId,
) -> Result<String, Error> {
    let face = |style: Option<&ResolvedTogglerStatusStyle>| -> Result<String, Error> {
        let Some(style) = style else {
            return Ok(option_code(None));
        };
        refuse_when(
            program,
            origin,
            style.foreground_border_color.is_some()
                || style.foreground_border_width.is_some()
                || style.padding_ratio.is_some(),
            "a toggler knob border or padding ratio",
        )?;
        let mark = match &style.foreground {
            Some(foreground) => match &foreground.value {
                ResolvedContainerBackground::Color(color) => Some(color),
                ResolvedContainerBackground::Linear { .. } => {
                    return Err(refused(program, origin, "a gradient background"));
                }
            },
            None => None,
        };
        Ok(option_code(Some(control_face_code(
            style
                .background
                .as_ref()
                .map(|background| &background.value),
            mark,
            style.text_color.as_ref().map(|color| &color.value),
            control_border_code(
                style
                    .background_border_color
                    .as_ref()
                    .map(|color| &color.value),
                style.background_border_width,
                &style.radius,
                program,
                env,
            )?,
            program,
            origin,
        )?)))
    };
    Ok(format!(
        "{WIRE}::ToggleStyle {{ tone: ::std::option::Option::None, active_on: {}, active_off: {}, hovered_on: {}, hovered_off: {}, disabled_on: {}, disabled_off: {} }}",
        face(styles.active_checked.as_ref())?,
        face(styles.active_unchecked.as_ref())?,
        face(styles.hovered_checked.as_ref())?,
        face(styles.hovered_unchecked.as_ref())?,
        face(styles.disabled_checked.as_ref())?,
        face(styles.disabled_unchecked.as_ref())?,
    ))
}

fn slider(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let slider = program.resolved_slider(id)?;
    let origin = slider.origin;
    refuse_when(
        program,
        origin,
        slider.value_type != Type::F64,
        "a slider over a named number type",
    )?;
    refuse_when(
        program,
        origin,
        slider.default.is_some() || slider.shift_step.is_some(),
        "a slider default or shift step",
    )?;
    refuse_when(
        program,
        origin,
        slider.custom_style.is_some(),
        "a slider style callback",
    )?;
    let face = |style: Option<&ResolvedSliderStatusStyle>| -> Result<String, Error> {
        let Some(style) = style else {
            return Ok(option_code(None));
        };
        refuse_when(
            program,
            origin,
            style.handle_shape.is_some(),
            "a slider handle shape",
        )?;
        let rail_width = style
            .rail_width
            .map(|width| clamped_f32_code(width, "0.0", "f32::MAX", program, env))
            .transpose()?;
        Ok(option_code(Some(format!(
            "{WIRE}::SliderFace {{ rail_start: {}, rail_end: {}, rail_width: {}, rail_border: {}, handle: {}, handle_border: {} }}",
            plain_background_code(style.rail_start.as_ref(), program, origin)?,
            plain_background_code(style.rail_end.as_ref(), program, origin)?,
            option_code(rail_width),
            control_border_code(
                style.rail_border_color.as_ref(),
                style.rail_border_width,
                &style.rail_radius,
                program,
                env,
            )?,
            plain_background_code(style.handle_color.as_ref(), program, origin)?,
            control_border_code(
                style.handle_border_color.as_ref(),
                style.handle_border_width,
                &ResolvedContainerRadius::default(),
                program,
                env,
            )?,
        ))))
    };
    let style = format!(
        "{WIRE}::SliderStyle {{ active: {}, hovered: {}, dragged: {} }}",
        face(slider.styles.active.as_ref())?,
        face(slider.styles.hovered.as_ref())?,
        face(slider.styles.dragged.as_ref())?,
    );
    let number = |expression: CheckedExprUseId| {
        resolved_expr_use_code(program, expression, env, ValueMode::Owned)
            .map(|code| format!("({code}) as f32"))
    };
    let callback = snapshot_callback(
        &slider.change,
        "__value",
        &["__value"],
        env,
        program,
        message,
    )?;
    let on_release = slider
        .release
        .as_ref()
        .map(|route| resolved_interaction_route_code(route, &[], env, program, message))
        .transpose()?
        .map(|activate| format!("{SLOTS}::message({activate})"));
    let axis = match slider.axis {
        ResolvedRangeAxis::Horizontal => "Row",
        ResolvedRangeAxis::Vertical => "Column",
    };
    Ok(format!(
        "{WIRE}::Node::Slider {{ key: {}, value: {}, min: {}, max: {}, step: {}, on_change: {}, on_release: {}, axis: {WIRE}::Axis::{axis}, width: {}, height: {}, style: {style} }}",
        key_code(identity, "slider", origin, scope, env, program)?,
        number(slider.value)?,
        number(slider.min)?,
        number(slider.max)?,
        number(slider.step)?,
        handler_code(
            "f32",
            message,
            &callback,
            "move |__sent: f32| ::std::option::Option::Some(__route(f64::from(__sent)))"
        ),
        option_code(on_release),
        dimension_code(slider.width.as_ref(), false, program, env, origin)?,
        dimension_code(slider.height.as_ref(), false, program, env, origin)?,
    ))
}

fn pick_list(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let pick = program.resolved_pick_list(id)?;
    let origin = pick.origin;
    refuse_when(
        program,
        origin,
        pick.menu_height.is_some()
            || pick.padding.is_some()
            || pick.text_size.is_some()
            || pick.line_height.is_some()
            || pick.shaping.is_some()
            || pick.font.is_some()
            || pick.handle.is_some(),
        "this pick list option",
    )?;
    refuse_when(
        program,
        origin,
        pick.open.is_some() || pick.close.is_some(),
        "a pick list open or close route",
    )?;
    refuse_when(
        program,
        origin,
        pick.custom_style.is_some() || pick.menu.custom.is_some(),
        "a pick list style callback",
    )?;
    let plain = ResolvedStyle::default();
    let face = |style: Option<&ResolvedPickListStatusStyle>| -> Result<String, Error> {
        let Some(style) = style else {
            return Ok(option_code(None));
        };
        refuse_surface_extras(&style.surface, program, origin)?;
        Ok(option_code(Some(format!(
            "{WIRE}::PickFace {{ background: {}, text: {}, placeholder: {}, handle: {}, border: {} }}",
            background_code(&style.surface, &plain, program, origin)?,
            option_code(style.surface.text_color.as_ref().map(rgba_code)),
            option_code(style.placeholder_color.as_ref().map(rgba_code)),
            option_code(style.handle_color.as_ref().map(rgba_code)),
            border_code(&style.surface, &plain, program, env)?,
        ))))
    };
    let menu = if pick.menu.surface.is_none()
        && pick.menu.selected_text_color.is_none()
        && pick.menu.selected_background.is_none()
    {
        option_code(None)
    } else {
        let (background, text, border) = match &pick.menu.surface {
            Some(surface) => {
                refuse_surface_extras(surface, program, origin)?;
                (
                    background_code(surface, &plain, program, origin)?,
                    option_code(surface.text_color.as_ref().map(rgba_code)),
                    border_code(surface, &plain, program, env)?,
                )
            }
            None => (option_code(None), option_code(None), option_code(None)),
        };
        option_code(Some(format!(
            "{WIRE}::MenuFace {{ background: {background}, text: {text}, border: {border}, selected_text: {}, selected_background: {} }}",
            option_code(pick.menu.selected_text_color.as_ref().map(rgba_code)),
            plain_background_code(pick.menu.selected_background.as_ref(), program, origin)?,
        )))
    };
    let style = format!(
        "{WIRE}::PickListStyle {{ active: {}, hovered: {}, opened: {}, opened_hovered: {}, menu: {menu} }}",
        face(pick.styles.active.as_ref())?,
        face(pick.styles.hovered.as_ref())?,
        face(pick.styles.opened.as_ref())?,
        face(pick.styles.opened_hovered.as_ref())?,
    );
    let options = resolved_expr_use_code(program, pick.options, env, ValueMode::Owned)?;
    let selected = resolved_expr_use_code(program, pick.selected, env, ValueMode::Owned)?;
    let placeholder = pick
        .placeholder
        .map(|expression| resolved_expr_use_code(program, expression, env, ValueMode::Owned))
        .transpose()?
        .map(|code| format!("({code}).to_string()"));
    let callback = resolved_interaction_route_callback_code(
        &pick.selection,
        "__value",
        &["__value"],
        env,
        program,
        message,
    )?;
    // One message per option, built while the view runs; an index past the
    // list (the host raced a rebuild) is no message.
    let handler = handler_code(
        "u32",
        message,
        &callback,
        &format!(
            "let __table: ::std::vec::Vec<{message}> = __options.iter().cloned().map(__route).collect(); move |__sent: u32| __table.get(__sent as usize).cloned()"
        ),
    );
    Ok(format!(
        "{{ let __options = {options}; let __selected = {selected}; {WIRE}::Node::PickList {{ key: {}, options: __options.iter().map(|__option| __option.to_string()).collect(), selected: __selected.as_ref().and_then(|__chosen| __options.iter().position(|__option| __option == __chosen)).map(|__index| __index as u32), placeholder: {}, on_select: {handler}, width: {}, style: {style} }} }}",
        key_code(identity, "pick-list", origin, scope, env, program)?,
        option_code(placeholder),
        dimension_code(pick.width.as_ref(), false, program, env, origin)?,
    ))
}

fn progress(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let progress = program.resolved_progress(id)?;
    let origin = progress.origin;
    refuse_when(
        program,
        origin,
        progress.custom_style.is_some(),
        "a progress style callback",
    )?;
    let tone = match progress.style {
        None => option_code(None),
        Some(ResolvedProgressStyle::Primary) => tone_code("Primary"),
        Some(ResolvedProgressStyle::Secondary) => tone_code("Secondary"),
        Some(ResolvedProgressStyle::Success) => tone_code("Success"),
        Some(ResolvedProgressStyle::Warning) => tone_code("Warning"),
        Some(ResolvedProgressStyle::Danger) => tone_code("Danger"),
    };
    let number = |expression: CheckedExprUseId| {
        resolved_expr_use_code(program, expression, env, ValueMode::Owned)
            .map(|code| format!("({code}) as f32"))
    };
    let axis = match progress.axis {
        ResolvedRangeAxis::Horizontal => "Row",
        ResolvedRangeAxis::Vertical => "Column",
    };
    Ok(format!(
        "{WIRE}::Node::Progress {{ key: {}, value: {}, min: {}, max: {}, axis: {WIRE}::Axis::{axis}, length: {}, girth: {}, tone: {tone}, background: {}, bar: {}, border: {} }}",
        key_code(identity, "progress", origin, scope, env, program)?,
        number(progress.value)?,
        number(progress.min)?,
        number(progress.max)?,
        dimension_code(progress.length.as_ref(), false, program, env, origin)?,
        dimension_code(progress.girth.as_ref(), false, program, env, origin)?,
        plain_background_code(progress.background.as_ref(), program, origin)?,
        plain_background_code(progress.bar.as_ref(), program, origin)?,
        control_border_code(
            progress.border_color.as_ref(),
            progress.border_width,
            &progress.radius,
            program,
            env,
        )?,
    ))
}

/// The declaration types the wire values; no native adapter is called by
/// the guest. Routes own snapshots of their non-payload arguments.
fn surface(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let (name, origin, arguments, output, route, bounds) =
        if matches!(program.resolved_view(id)?.kind, ResolvedViewKind::Shader) {
            let shader = program.resolved_shader(id)?;
            let adapter = &shader.adapter;
            (
                &adapter.function.name,
                shader.origin,
                adapter
                    .arguments
                    .iter()
                    .map(|arg| (arg.expression, &arg.ty))
                    .collect::<Vec<_>>(),
                &adapter.output,
                &adapter.route,
                Some((&shader.width, &shader.height)),
            )
        } else {
            let component = program.resolved_extern_component(id)?;
            (
                &component.function.name,
                component.origin,
                component
                    .arguments
                    .iter()
                    .map(|arg| (arg.expression, &arg.ty))
                    .collect::<Vec<_>>(),
                &component.output,
                &component.route,
                None,
            )
        };
    let args = arguments
        .into_iter()
        .map(|(expression, ty)| {
            let value = resolved_expr_use_code(program, expression, env, ValueMode::Owned)?;
            let encoded =
                surface_value_code(ty, "__surface_arg", false, program, origin, &mut Vec::new())?;
            Ok(format!("{{ let __surface_arg = &({value}); {encoded} }}"))
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    let on_event = route
        .as_ref()
        .map(|route| {
            let decoded =
                surface_value_code(output, "__sent", true, program, origin, &mut Vec::new())?;
            let callback =
                snapshot_callback(route, "__value", &["__value"], env, program, message)?;
            Ok(handler_code(
                &format!("{WIRE}::SurfaceValue"),
                message,
                &callback,
                &format!("move |__sent| ({decoded}).map(&__route)"),
            ))
        })
        .transpose()?;
    let key = key_code(
        identity,
        if bounds.is_some() { "shader" } else { "extern" },
        origin,
        scope,
        env,
        program,
    )?;
    let surface_key = if bounds.is_some() {
        "__surface_key.clone()"
    } else {
        &key
    };
    let surface = format!(
        "{WIRE}::Node::Surface {{ key: {surface_key}, name: ::std::string::String::from({name:?}), args: ::std::vec![{args}], on_event: {} }}",
        option_code(on_event),
    );
    let content = if let Some((width, height)) = bounds {
        let dimension = |value: &Option<ResolvedContainerLength>| {
            match value {
                None => Ok(format!("{WIRE}::Length::Fixed(100.0)")),
                // Shader is atomic: unlike Container it has no intrinsic
                // child size, so Shrink resolves to zero on this axis.
                Some(ResolvedContainerLength::Shrink) => Ok(format!("{WIRE}::Length::Fixed(0.0)")),
                Some(value) => length_code(value, program, env, origin),
            }
        };
        // Iced Shader defaults to 100x100. The host element is constrained
        // by the same dimensions, including when its provider is missing.
        format!(
            "{WIRE}::Node::Container {{ max_width: None, max_height: None, clip: false, key: ::std::format!(\"{{}}/@bounds\", __surface_key), width: ::std::option::Option::Some({}), height: ::std::option::Option::Some({}), padding: ::std::option::Option::None, align_x: ::std::option::Option::None, align_y: ::std::option::Option::None, background: ::std::option::Option::None, border: ::std::option::Option::None, snap: ::std::option::Option::None, content: ::std::boxed::Box::new({surface}) }}",
            dimension(width)?,
            dimension(height)?,
        )
    } else {
        return Ok(surface);
    };
    Ok(format!("{{ let __surface_key = {key}; {content} }}"))
}

/// Encode a borrowed expression or decode an owned wire value. The generated
/// decoder returns Option<T>, so one mismatched nested field drops the event.
fn surface_value_code(
    ty: &Type,
    value: &str,
    decode: bool,
    program: &LoweredProgram,
    origin: OriginId,
    visiting: &mut Vec<String>,
) -> Result<String, Error> {
    let v = format!("{WIRE}::SurfaceValue");
    let scalar = match ty {
        Type::Unit => Some("Unit"),
        Type::Bool => Some("Bool"),
        Type::I64 => Some("I64"),
        Type::F64 => Some("F64"),
        Type::Str => Some("Str"),
        _ => None,
    };
    if let Some(tag) = scalar {
        return Ok(if decode {
            let pattern = if tag == "Unit" {
                format!("{v}::Unit")
            } else {
                format!("{v}::{tag}(__item)")
            };
            let guard = if tag == "F64" {
                " if __item.is_finite()"
            } else {
                ""
            };
            let result = if tag == "Unit" { "()" } else { "__item" };
            format!(
                "match {value} {{ {pattern}{guard} => ::std::option::Option::Some({result}), _ => ::std::option::Option::None }}"
            )
        } else {
            match tag {
                "Unit" => format!("{{ let _ = {value}; {v}::Unit }}"),
                "Str" => format!("{v}::Str(::std::string::ToString::to_string({value}))"),
                _ => format!("{v}::{tag}(*({value}))"),
            }
        });
    }
    match ty {
        Type::List(inner) => {
            let item = surface_value_code(inner, "__item", decode, program, origin, visiting)?;
            Ok(if decode {
                format!(
                    "match {value} {{ {v}::List(__items) => __items.into_iter().map(|__item| {item}).collect::<::std::option::Option<::std::vec::Vec<_>>>(), _ => ::std::option::Option::None }}"
                )
            } else {
                format!("{v}::List(({value}).iter().map(|__item| {item}).collect())")
            })
        }
        Type::Option(inner) => {
            let item = surface_value_code(
                inner,
                if decode { "*__item" } else { "__item" },
                decode,
                program,
                origin,
                visiting,
            )?;
            Ok(if decode {
                format!(
                    "match {value} {{ {v}::Option(::std::option::Option::None) => ::std::option::Option::Some(::std::option::Option::None), {v}::Option(::std::option::Option::Some(__item)) => ({item}).map(::std::option::Option::Some), _ => ::std::option::Option::None }}"
                )
            } else {
                format!(
                    "{v}::Option(({value}).as_ref().map(|__item| ::std::boxed::Box::new({item})))"
                )
            })
        }
        Type::Named(name) => {
            let declaration = program
                .struct_declarations()
                .iter()
                .find(|item| &item.name == name)
                .filter(|item| !item.fields.is_empty())
                .ok_or_else(|| refused(program, origin, "an opaque extern widget value"))?;
            if visiting.contains(name) {
                return Err(refused(program, origin, "a recursive extern widget value"));
            }
            visiting.push(name.clone());
            let mut fields = Vec::new();
            for (index, field) in declaration.fields.iter().enumerate() {
                let field_value = if decode {
                    format!("__field_{index}")
                } else {
                    format!("&({value}).{}", field.name)
                };
                let item =
                    surface_value_code(&field.ty, &field_value, decode, program, origin, visiting)?;
                fields.push(if decode {
                    format!("{}: ({item})?", field.name)
                } else {
                    format!("(::std::string::String::from({:?}), {item})", field.name)
                });
            }
            visiting.pop();
            if decode {
                let extract = declaration.fields.iter().enumerate().map(|(index, field)| format!(
                    "let (__name, __field_{index}) = __fields.next()?; if __name != {:?} {{ return ::std::option::Option::None; }}", field.name
                )).collect::<Vec<_>>().join(" ");
                Ok(format!(
                    "(|| {{ let {v}::Record {{ name: __name, fields: __fields }} = {value} else {{ return ::std::option::Option::None; }}; if __name != {name:?} || __fields.len() != {} {{ return ::std::option::Option::None; }} let mut __fields = __fields.into_iter(); {extract} ::std::option::Option::Some({} {{ {} }}) }})()",
                    declaration.fields.len(),
                    declaration.rust_path,
                    fields.join(", ")
                ))
            } else {
                Ok(format!(
                    "{v}::Record {{ name: ::std::string::String::from({name:?}), fields: ::std::vec![{}] }}",
                    fields.join(", ")
                ))
            }
        }
        _ => Err(refused(program, origin, "a non-data extern widget value")),
    }
}

fn markdown_surface(
    id: ViewId,
    identity: Option<&ResolvedViewIdentity>,
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
) -> Result<String, Error> {
    let markdown = program.resolved_markdown(id)?;
    let origin = markdown.origin;
    for font in [
        &markdown.style.font,
        &markdown.style.inline_code_font,
        &markdown.style.code_block_font,
    ] {
        refuse_when(
            program,
            origin,
            matches!(font, Some(ResolvedTextFont::Named(_))),
            "a named markdown font",
        )?;
    }
    refuse_when(
        program,
        origin,
        matches!(
            markdown.style.inline_code_background,
            Some(ResolvedContainerBackground::Linear { .. })
        ),
        "a markdown gradient",
    )?;
    let content = resolved_markdown_content(markdown, env, program)?;
    let settings = markdown_settings_code(markdown, program, env)?;
    let mut arguments = vec![format!(
        "({}).surface_value(__markdown_settings, self.__theme().palette())",
        content.code
    )];
    let (name, output) = if let Some(viewer) = &markdown.viewer {
        let function = program
            .try_extern_function(viewer.function)
            .ok_or_else(|| {
                program.invariant_at_origin(origin, "markdown viewer extern is invalid")
            })?;
        if function.kind != ExternKind::MarkdownViewer
            || function.params.len() != viewer.arguments.len()
            || function.output != viewer.output
            || function.borrowed != viewer.borrowed
            || viewer.borrowed.len() != viewer.arguments.len()
        {
            return Err(program.invariant_at_origin(origin, "markdown viewer contract diverged"));
        }
        for (expression, (_, ty)) in viewer.arguments.iter().zip(&function.params) {
            let value = resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?;
            let encoded =
                surface_value_code(ty, "__surface_arg", false, program, origin, &mut Vec::new())?;
            arguments.push(format!("{{ let __surface_arg = &({value}); {encoded} }}"));
        }
        (function.name.as_str(), &viewer.output)
    } else {
        ("ice.markdown", &Type::Str)
    };
    let decoded = surface_value_code(output, "__sent", true, program, origin, &mut Vec::new())?;
    let callback = snapshot_callback(
        &markdown.link,
        "__value",
        &["__value"],
        env,
        program,
        message,
    )?;
    let handler = handler_code(
        &format!("{WIRE}::SurfaceValue"),
        message,
        &callback,
        &format!("move |__sent| ({decoded}).map(&__route)"),
    );
    Ok(format!(
        "{{ {settings} {WIRE}::Node::Surface {{ key: {}, name: ::std::string::String::from({name:?}), args: ::std::vec![{}], on_event: ::std::option::Option::Some({handler}) }} }}",
        key_code(identity, "markdown", origin, scope, env, program)?,
        arguments.join(", ")
    ))
}
