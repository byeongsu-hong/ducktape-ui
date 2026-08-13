use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_table(
    table: &ResolvedTable,
    columns: &[ResolvedTableColumnTopology],
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let program = document;
    if columns.len() != table.columns.len() {
        return Err(program.invariant_at_origin(table.origin, "table HIR column length diverged"));
    }
    let rows = resolved_expr_use_code(program, table.rows, env, ValueMode::Owned)?;
    let item_name = &table.row.name;
    let row_rust = rust_type_code(program, &table.row.ty);
    let mut cell_env = ScopedBindingEnv::new(env);
    cell_env.insert(
        item_name.clone(),
        resolved_local_binding(
            LocalBindingTypeSource::Resolved(program),
            table.row.local,
            item_name.clone(),
            true,
        ),
    );
    // The per-column `move` closure must own its scope chain: interpolating a
    // shared scope local (a component's scope binding) would move it out of
    // the enclosing render and break sibling uses.
    cell_env.insert(
        RECONCILIATION_SCOPE_BINDING.into(),
        reconciliation_scope_binding("__ice_table_recon.clone()".into()),
    );
    let scope = borrowed_scope(scope);
    let table_recon = borrowed_scope(reconciliation_scope(scope, env)).to_owned();
    let mut column_codes = Vec::with_capacity(columns.len());
    for (index, (column, resolved)) in columns.iter().zip(&table.columns).enumerate() {
        let header_scope = format!("format!(\"{{}}/header({index})\", {scope})");
        let cell_scope =
            format!("format!(\"{{}}/row({{}})/col({index})\", __ice_table_scope, __row)");
        let header = render_node(column.header, document, message, env, &header_scope, slot)?;
        let cell = {
            let _derived_guard = enter_escaping_derived_reads();
            render_node(column.cell, document, message, &cell_env, &cell_scope, slot)?
        };
        let mut code = format!(
            "{{ let __ice_table_scope = ({scope}).to_owned(); let __ice_table_recon = ({table_recon}).to_owned(); let _ = (&__ice_table_scope, &__ice_table_recon); let __table_header: __IceElement<'_, {message}> = {header}; let __table_header = ::ui_lang_runtime::bounded_fill_element(__table_header, __table_row_count, false); ::iced::widget::table::column(__table_header, move |(__row, {item_name}): (usize, {row_rust})| -> __IceElement<'_, {message}> {{ let _ = &{item_name}; let __table_cell: __IceElement<'_, {message}> = {cell}; ::ui_lang_runtime::bounded_fill_element(__table_cell, __table_row_count, false) }})"
        );
        if let Some(width) = &resolved.width {
            write!(
                code,
                ".width(::ui_lang_runtime::bounded_fill_length({}, {}))",
                resolved_table_length_code(width, program, env)?,
                columns.len()
            )
            .unwrap();
        }
        if let Some(align) = resolved.align_x {
            let align = match align {
                InputAlignment::Left => "Left",
                InputAlignment::Center => "Center",
                InputAlignment::Right => "Right",
            };
            write!(code, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
        }
        if let Some(align) = resolved.align_y {
            let align = match align {
                VerticalAlignment::Top => "Top",
                VerticalAlignment::Center => "Center",
                VerticalAlignment::Bottom => "Bottom",
            };
            write!(code, ".align_y(::iced::alignment::Vertical::{align})").unwrap();
        }
        code.push_str(" }");
        column_codes.push(code);
    }
    let mut code = format!(
        "{{ let __table_rows = {rows}; let __table_row_count = __table_rows.len().saturating_add(1); ::iced::widget::table::table(::std::vec![{}], __table_rows.into_iter().enumerate())",
        column_codes.join(", ")
    );
    if let Some(width) = &table.width {
        write!(
            code,
            ".width({})",
            resolved_table_length_code(width, program, env)?
        )
        .unwrap();
    }
    for (value, method, entries) in [
        (
            table.padding,
            "padding",
            format!("{}usize.max(__table_row_count)", columns.len()),
        ),
        (table.padding_x, "padding_x", columns.len().to_string()),
        (table.padding_y, "padding_y", "__table_row_count".to_owned()),
        (
            table.separator,
            "separator",
            format!("{}usize.max(__table_row_count)", columns.len()),
        ),
        (table.separator_x, "separator_x", columns.len().to_string()),
        (
            table.separator_y,
            "separator_y",
            "__table_row_count".to_owned(),
        ),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}(::ui_lang_runtime::bounded_table_metric({}, {entries}))",
                resolved_expr_use_code(program, value, env, ValueMode::Owned)?,
            )
            .unwrap();
        }
    }
    Ok(format!("{code}.into() }}"))
}

fn resolved_table_length_code(
    length: &ResolvedTableLength,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match length {
        ResolvedTableLength::Fill => "::iced::Fill".into(),
        ResolvedTableLength::FillPortion(portion) => {
            format!("::iced::Length::FillPortion({portion})")
        }
        ResolvedTableLength::Shrink => "::iced::Shrink".into(),
        ResolvedTableLength::FixedF64(expression) => format!(
            "{} as f32",
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
        ),
        ResolvedTableLength::FixedLength(expression) => {
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
        }
    })
}

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_keyed_column(
    keyed: &ResolvedKeyedColumn,
    child: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let program = document;
    let items = resolved_expr_use_code(program, keyed.items, env, ValueMode::Borrowed)?;
    let item_name = &keyed.item.name;
    let mut child_env = ScopedBindingEnv::new(env);
    child_env.insert(
        item_name.clone(),
        resolved_local_binding(
            LocalBindingTypeSource::Resolved(program),
            keyed.item.local,
            item_name.clone(),
            false,
        ),
    );
    let key = resolved_expr_use_code(program, keyed.key, &child_env, ValueMode::Owned)?;
    // A row's reconciliation identity is its key — the same per-row identity
    // a `for` row's `__for_scope` carries. Without this binding, every row
    // inherits the enclosing scope (a component's, most often), so all rows'
    // `lazy` expressions park under ONE memo site and the lot's
    // one-revision-per-site rule keeps a single row of the whole list.
    child_env.insert(
        RECONCILIATION_SCOPE_BINDING.into(),
        reconciliation_scope_binding("__ice_key_recon.clone()".into()),
    );
    // Copy rows are free to copy; anything else iterates by reference — the
    // same borrow-aware treatment `for` rows get. The key expression and
    // every child use site project through the reference unchanged, so row
    // identity and diffing are untouched; only the up-front per-row deep
    // clone disappears.
    let element_ty = &program.expressions().local(keyed.item.local).ty;
    let iterate = if copy_expression_type(element_ty) {
        ".iter().cloned()"
    } else {
        ".iter()"
    };
    let scope = borrowed_scope(scope);
    let recon_base = borrowed_scope(reconciliation_scope(scope, env)).to_owned();
    let child_scope = format!("format!(\"{{}}/key({{}})\", {scope}, __key)");
    let child = render_node(child, document, message, &child_env, &child_scope, slot)?;
    let mut code = format!(
        "{{ let mut __children: ::std::vec::Vec<_> = ::std::vec::Vec::new(); for {item_name} in {items}{iterate} {{ let __key = {key}; let __ice_key_recon = format!(\"{{}}/key({{}})\", {recon_base}, __key); let _ = &__ice_key_recon; let __child: __IceElement<'_, {message}> = {child}; __children.push((__key, __child)); }} let __child_count = __children.len(); let __children = __children.into_iter().map(|(__key, __child)| (__key, ::ui_lang_runtime::bounded_fill_element(__child, __child_count, false))).collect::<::std::vec::Vec<_>>();"
    );
    let spacing = keyed
        .spacing
        .map(|spacing| resolved_expr_use_code(program, spacing, env, ValueMode::Owned))
        .transpose()?;
    if let Some(virtual_row) = keyed.virtual_row {
        // Same shape a virtualized `col` takes: an ordinary column keeps
        // padding, dimensions, and max-width, and only per-child layout moves
        // inside, where the rows the viewport cannot reach are never laid out
        // and so never shape their text. The keys ride along so per-row state
        // still follows its row rather than its index. Spacing goes in with
        // the rows, since the outer column has one child to put it between.
        let estimate = resolved_expr_use_code(program, virtual_row, env, ValueMode::Owned)?;
        let spacing = spacing.as_ref().map_or_else(String::new, |spacing| {
            format!(".spacing(::ui_lang_runtime::bounded_spacing({spacing}, __child_count))")
        });
        write!(
            code,
            " let __children = __children.into_iter().map(|(__key, __child)| (::ui_lang_runtime::VirtualKey::virtual_key(__key), __child)).collect::<::std::vec::Vec<_>>(); let __layout = ::iced::widget::column(::std::vec![::iced::Element::from(::ui_lang_runtime::virtual_keyed_children(__children, ({estimate}) as f32){spacing})])"
        )
        .unwrap();
    } else {
        code.push_str(" let __layout = ::iced::widget::keyed_column(__children)");
        if let Some(spacing) = &spacing {
            write!(
                code,
                ".spacing(::ui_lang_runtime::bounded_spacing({spacing}, __child_count))"
            )
            .unwrap();
        }
    }
    if let Some(padding) = resolved_keyed_padding_code(&keyed.padding, program, env)? {
        write!(code, ".padding({padding})").unwrap();
    }
    for (method, length) in [("width", &keyed.width), ("height", &keyed.height)] {
        if let Some(length) = length {
            write!(
                code,
                ".{method}({})",
                resolved_keyed_length_code(length, program, env)?
            )
            .unwrap();
        }
    }
    if let Some(max_width) = keyed.max_width {
        write!(
            code,
            ".max_width({} as f32)",
            resolved_expr_use_code(program, max_width, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    // Only reachable on the `keyed_column` path: `E197` rejects `align=`
    // beside `virtual-row=`, and a plain `Column` spells this `align_x`.
    if let Some(align) = keyed.align {
        let align = match align {
            FlexAlignment::Start => "Start",
            FlexAlignment::Center => "Center",
            FlexAlignment::End => "End",
        };
        write!(code, ".align_items(::iced::Alignment::{align})").unwrap();
    }
    Ok(format!("{code}; __layout.into() }}"))
}

fn resolved_keyed_length_code(
    length: &ResolvedKeyedLength,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match length {
        ResolvedKeyedLength::Fill => "::iced::Fill".into(),
        ResolvedKeyedLength::FillPortion(portion) => {
            format!("::iced::Length::FillPortion({portion})")
        }
        ResolvedKeyedLength::Shrink => "::iced::Shrink".into(),
        ResolvedKeyedLength::FixedF64(expression) => format!(
            "{} as f32",
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
        ),
        ResolvedKeyedLength::FixedLength(expression) => {
            resolved_expr_use_code(program, *expression, env, ValueMode::Owned)?
        }
    })
}

fn resolved_keyed_padding_code(
    padding: &ResolvedKeyedPadding,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    if padding.all.is_none()
        && padding.x.is_none()
        && padding.y.is_none()
        && padding.top.is_none()
        && padding.right.is_none()
        && padding.bottom.is_none()
        && padding.left.is_none()
    {
        return Ok(None);
    }
    let code = |value: Option<ResolvedExpressionId>| {
        value
            .map(|value| resolved_expr_use_code(program, value, env, ValueMode::Owned))
            .transpose()
    };
    let all = code(padding.all)?.unwrap_or_else(|| "0.0".into());
    let x = code(padding.x)?.unwrap_or_else(|| all.clone());
    let y = code(padding.y)?.unwrap_or_else(|| all.clone());
    let top = code(padding.top)?.unwrap_or_else(|| y.clone());
    let right = code(padding.right)?.unwrap_or_else(|| x.clone());
    let bottom = code(padding.bottom)?.unwrap_or(y);
    let left = code(padding.left)?.unwrap_or(x);
    Ok(Some(format!(
        "::ui_lang_runtime::bounded_padding({top}, {right}, {bottom}, {left})"
    )))
}
