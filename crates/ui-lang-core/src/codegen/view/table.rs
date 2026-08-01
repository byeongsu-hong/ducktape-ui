use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_table(
    options: &TableOptions,
    columns: &[TableColumn],
    span: &Span,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let CheckedViewFlow::Table { rows, item } = &document.program().checked_view(span)?.flow else {
        return Err(Error::new("E196", span, "table has no checked flow"));
    };
    let rows = checked_expr_use_code(document.program(), *rows, env, ValueMode::Owned)?;
    let checked_item = document.program().checked_facts().local(*item);
    let item_name = &checked_item.name;
    let row_type = checked_item.ty.clone();
    let row_rust = row_type.rust(&document.structs);
    let mut cell_env = ScopedBindingEnv::new(env);
    cell_env.insert(
        item_name.clone(),
        checked_local_binding(document.program(), *item, item_name.clone(), true),
    );
    let mut column_codes = Vec::with_capacity(columns.len());
    for (index, column) in columns.iter().enumerate() {
        let header_scope = format!("format!(\"{{}}/header({index})\", {scope})");
        let cell_scope = format!("format!(\"{{}}/row({{}})/col({index})\", {scope}, __row)");
        let header = render_node(&column.header, document, message, env, &header_scope, slot)?;
        let cell = render_node(
            &column.cell,
            document,
            message,
            &cell_env,
            &cell_scope,
            slot,
        )?;
        let mut code = format!(
            "{{ let __table_header: __IceElement<'_, {message}> = {header}; let __table_header = ::ui_lang_runtime::bounded_fill_element(__table_header, __table_row_count, false); ::iced::widget::table::column(__table_header, move |(__row, {item_name}): (usize, {row_rust})| -> __IceElement<'_, {message}> {{ let _ = &{item_name}; let __table_cell: __IceElement<'_, {message}> = {cell}; ::ui_lang_runtime::bounded_fill_element(__table_cell, __table_row_count, false) }})"
        );
        if let Some(width) = &column.width {
            write!(
                code,
                ".width(::ui_lang_runtime::bounded_fill_length({}, {}))",
                length_code(width, env, document)?,
                columns.len()
            )
            .unwrap();
        }
        if let Some(align) = column.align_x {
            let align = match align {
                InputAlignment::Left => "Left",
                InputAlignment::Center => "Center",
                InputAlignment::Right => "Right",
            };
            write!(code, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
        }
        if let Some(align) = column.align_y {
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
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    for (value, method, entries) in [
        (
            &options.padding,
            "padding",
            format!("{}usize.max(__table_row_count)", columns.len()),
        ),
        (&options.padding_x, "padding_x", columns.len().to_string()),
        (
            &options.padding_y,
            "padding_y",
            "__table_row_count".to_owned(),
        ),
        (
            &options.separator,
            "separator",
            format!("{}usize.max(__table_row_count)", columns.len()),
        ),
        (
            &options.separator_x,
            "separator_x",
            columns.len().to_string(),
        ),
        (
            &options.separator_y,
            "separator_y",
            "__table_row_count".to_owned(),
        ),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}(::ui_lang_runtime::bounded_table_metric({}, {entries}))",
                expr_code(value, env, document, ValueMode::Owned)?,
            )
            .unwrap();
        }
    }
    Ok(format!("{code}.into() }}"))
}

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_keyed_column(
    options: &LayoutOptions,
    child: &ViewNode,
    span: &Span,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let CheckedViewFlow::Keyed { items, key, item } = &document.program().checked_view(span)?.flow
    else {
        return Err(Error::new("E196", span, "keyed column has no checked flow"));
    };
    let items = checked_expr_use_code(document.program(), *items, env, ValueMode::Borrowed)?;
    let checked_item = document.program().checked_facts().local(*item);
    let item_name = &checked_item.name;
    let mut child_env = ScopedBindingEnv::new(env);
    child_env.insert(
        item_name.clone(),
        checked_local_binding(document.program(), *item, item_name.clone(), false),
    );
    let key = checked_expr_use_code(document.program(), *key, &child_env, ValueMode::Owned)?;
    let child_scope = format!("format!(\"{{}}/key({{}})\", {scope}, __key)");
    let child = render_node(child, document, message, &child_env, &child_scope, slot)?;
    let mut code = format!(
        "{{ let mut __children: ::std::vec::Vec<_> = ::std::vec::Vec::new(); for {item_name} in {items}.iter() {{ let __key = {key}; let __child: __IceElement<'_, {message}> = {child}; __children.push((__key, __child)); }} let __child_count = __children.len(); let __children = __children.into_iter().map(|(__key, __child)| (__key, ::ui_lang_runtime::bounded_fill_element(__child, __child_count, false))).collect::<::std::vec::Vec<_>>(); let __layout = ::iced::widget::keyed_column(__children)"
    );
    if let Some(spacing) = &options.spacing {
        write!(
            code,
            ".spacing(::ui_lang_runtime::bounded_spacing({}, __child_count))",
            expr_code(spacing, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(padding) = typed_padding_code(&options.padding, env, document)? {
        write!(code, ".padding({padding})").unwrap();
    }
    append_dimensions(&mut code, [&options.width, &options.height], env, document)?;
    if let Some(max_width) = &options.max_width {
        write!(
            code,
            ".max_width({} as f32)",
            expr_code(max_width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(align) = options.align {
        let align = match align {
            FlexAlignment::Start => "Start",
            FlexAlignment::Center => "Center",
            FlexAlignment::End => "End",
        };
        write!(code, ".align_items(::iced::Alignment::{align})").unwrap();
    }
    Ok(format!("{code}; __layout.into() }}"))
}
