use super::super::layout::{
    render_flex_children, resolved_flex_content_alignment_name, resolved_flex_direction_name,
    resolved_flex_item_alignment_name,
};
use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    layout: &ResolvedLayout,
    flex: &ResolvedFlexLayout,
    identity: Option<&ResolvedViewIdentity>,
    children: &[ViewId],
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let style = &layout.utility_style;
    let mut allowed = style.clone();
    allowed.clip = false;
    allowed.max_width = None;
    refuse_box_utilities(&allowed, program, layout.origin)?;
    let key = key_code(identity, "layout", layout.origin, scope, env, program)?;
    let child_scope = rendered_child_scope(identity, scope)?;
    let mut body = String::from("{ let mut __items = ::std::vec::Vec::new();");
    render_flex_children(
        &mut body,
        children,
        program,
        message,
        env,
        &child_scope,
        slot,
        flex.min_cell,
    )?;
    let number = |value: CheckedExprUseId| {
        resolved_expr_use_code(program, value, env, ValueMode::Owned)
            .map(|value| format!("({value}) as f32"))
    };
    let spacing = flex
        .spacing
        .map(number)
        .transpose()?
        .or_else(|| style.gap.map(|value| format!("{value}.0")));
    let mut row_gap = spacing.clone();
    let mut column_gap = spacing;
    if let Some(value) = flex.wrap_spacing {
        let gap = Some(number(value)?);
        match flex.direction {
            ResolvedFlexDirection::Row | ResolvedFlexDirection::RowReverse => row_gap = gap,
            ResolvedFlexDirection::Column | ResolvedFlexDirection::ColumnReverse => {
                column_gap = gap
            }
        }
    }
    if let Some(value) = flex.row_gap {
        row_gap = Some(number(value)?);
    }
    if let Some(value) = flex.column_gap {
        column_gap = Some(number(value)?);
    }
    let width = dimension_code(
        flex.width.as_ref(),
        style.width_fill,
        program,
        env,
        layout.origin,
    )?;
    let height = dimension_code(
        flex.height.as_ref(),
        style.height_fill,
        program,
        env,
        layout.origin,
    )?;
    let max_width = option_code(
        flex.max_width
            .map(number)
            .transpose()?
            .or_else(|| style.max_width.map(|value| format!("{value}.0"))),
    );
    let max_height = option_code(flex.max_height.map(number).transpose()?);
    let padding = edges_code(&flex.padding, style.padding, program, env)?;
    let direction = resolved_flex_direction_name(flex.direction);
    let wrap = match flex.wrap {
        ResolvedFlexWrap::NoWrap => "NoWrap",
        ResolvedFlexWrap::Wrap => "Wrap",
        ResolvedFlexWrap::WrapReverse => "WrapReverse",
    };
    let justify = option_code(flex.justify_content.map(|value| {
        format!(
            "{WIRE}::FlexContentAlignment::{}",
            resolved_flex_content_alignment_name(value)
        )
    }));
    let content = option_code(flex.align_content.map(|value| {
        format!(
            "{WIRE}::FlexContentAlignment::{}",
            resolved_flex_content_alignment_name(value)
        )
    }));
    let items_align = option_code(
        flex.align_items
            .map(|value| {
                format!(
                    "{WIRE}::FlexItemAlignment::{}",
                    resolved_flex_item_alignment_name(value)
                )
            })
            .or_else(|| {
                style
                    .items_center
                    .then(|| format!("{WIRE}::FlexItemAlignment::Center"))
            }),
    );
    let clip = flex
        .clip
        .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| style.clip.to_string());
    let surface_width = dimension_code(None, style.width_fill, program, env, layout.origin)?;
    let surface_height = dimension_code(None, style.height_fill, program, env, layout.origin)?;
    let surface_max_width = option_code(style.max_width.map(|value| format!("{value}.0")));
    let (background, border) = layout_surface_code(style);
    write!(body, " let (items, children) = __items.into_iter().unzip(); {WIRE}::Node::Flex {{ key: {key}, items, children, background: {background}, border: {border}, layout: {WIRE}::FlexLayout {{ direction: {WIRE}::FlexDirection::{direction}, wrap: {WIRE}::FlexWrap::{wrap}, justify: {justify}, items: {items_align}, content: {content}, row_gap: {}, column_gap: {}, padding: {padding}, width: {width}, height: {height}, max_width: {max_width}, max_height: {max_height}, clip: ({clip}), surface_width: {surface_width}, surface_height: {surface_height}, surface_max_width: {surface_max_width} }} }} }}", option_code(row_gap), option_code(column_gap)).unwrap();
    Ok(body)
}

pub(in crate::codegen::view) fn item_code(
    child: &str,
    options: Option<&ResolvedContainerFlexItem>,
    min_cell: Option<ResolvedExpressionId>,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    if let Some(min_cell) = min_cell {
        let pixels = clamped_f32_code(min_cell, "f32::EPSILON", "f32::MAX", program, env)?;
        return Ok(format!(
            "({WIRE}::FlexItem {{ grow: Some(1.0), shrink: 0.0, basis: {WIRE}::FlexBasis::Fixed({pixels}), ..Default::default() }}, {child})"
        ));
    }
    let Some(options) = options else {
        return Ok(format!("({WIRE}::FlexItem::default(), {child})"));
    };
    let number = |value: CheckedExprUseId| {
        resolved_expr_use_code(program, value, env, ValueMode::Owned)
            .map(|value| format!("({value}) as f32"))
    };
    let order = options
        .order
        .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "0".into());
    let grow = option_code(options.grow.map(number).transpose()?);
    let shrink = options
        .shrink
        .map(number)
        .transpose()?
        .unwrap_or_else(|| "1.0".into());
    let basis = match &options.basis {
        None | Some(ResolvedContainerFlexBasis::Auto) => format!("{WIRE}::FlexBasis::Auto"),
        Some(ResolvedContainerFlexBasis::Content) => format!("{WIRE}::FlexBasis::Content"),
        Some(ResolvedContainerFlexBasis::Fixed(value)) => {
            format!("{WIRE}::FlexBasis::Fixed({})", number(*value)?)
        }
        Some(ResolvedContainerFlexBasis::Percent(value)) => {
            format!("{WIRE}::FlexBasis::Percent(({}) / 100.0)", number(*value)?)
        }
    };
    let align = option_code(options.align_self.map(|value| {
        format!(
            "{WIRE}::FlexItemAlignment::{}",
            resolved_flex_item_alignment_name(value)
        )
    }));
    let margins = if let Some(margins) = &options.margins {
        let margin = |value: &ResolvedContainerFlexMargin| -> Result<String, Error> {
            Ok(match value {
                ResolvedContainerFlexMargin::Zero => format!("{WIRE}::FlexMargin::Zero"),
                ResolvedContainerFlexMargin::Auto => format!("{WIRE}::FlexMargin::Auto"),
                ResolvedContainerFlexMargin::Fixed(value) => {
                    format!("{WIRE}::FlexMargin::Fixed({})", number(*value)?)
                }
                ResolvedContainerFlexMargin::Percent(value) => {
                    format!("{WIRE}::FlexMargin::Percent(({}) / 100.0)", number(*value)?)
                }
            })
        };
        format!(
            "{WIRE}::FlexMargins {{ top: {}, right: {}, bottom: {}, left: {} }}",
            margin(&margins.top)?,
            margin(&margins.right)?,
            margin(&margins.bottom)?,
            margin(&margins.left)?
        )
    } else {
        format!("{WIRE}::FlexMargins::default()")
    };
    Ok(format!(
        "({WIRE}::FlexItem {{ order: {order}, grow: {grow}, shrink: {shrink}, basis: {basis}, align: {align}, margins: {margins} }}, {child})"
    ))
}
