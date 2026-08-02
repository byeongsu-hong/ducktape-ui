use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_pane_grid(
    name: &str,
    options: &PaneGridOptions,
    panes: &[PaneView],
    templates: &[PaneTemplate],
    span: &Span,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let CheckedViewFlow::PaneGrid {
        static_maximized,
        templates: checked_templates,
    } = &document.program().checked_view(span)?.flow
    else {
        return Err(Error::new("E196", span, "pane grid has no checked flow"));
    };
    if panes.len() != static_maximized.len() || templates.len() != checked_templates.len() {
        return Err(Error::new(
            "E196",
            span,
            "pane grid checked local arena length diverged",
        ));
    }
    let id = Id {
        name: name.to_owned(),
        key: None,
    };
    let pane_grid_scope = id_code(&id, scope, env, document)?;
    let pane_type = (!templates.is_empty()).then(|| pane_type(name));
    let mut arms = panes
        .iter()
        .zip(static_maximized)
        .map(|(pane, maximized)| {
            let mut pane_env = ScopedBindingEnv::new(env);
            if let Some(local) = maximized {
                let binding = document
                    .program()
                    .checked_facts()
                    .local(*local)
                    .name
                    .clone();
                pane_env.insert(
                    binding,
                    checked_local_binding(
                        LocalBindingTypeSource::Checked(document.program()),
                        *local,
                        "__pane_maximized".into(),
                        true,
                    ),
                );
            }
            let pane_scope = format!("format!(\"{{}}/{}\", {pane_grid_scope})", pane.name);
            let pattern = pane_type.as_ref().map_or_else(
                || rust_string(&pane.name),
                |pane_type| format!("{pane_type}::__Static({})", rust_string(&pane.name)),
            );
            Ok(format!(
                "{} => {}",
                pattern,
                render_pane_content(pane, document, message, &pane_env, &pane_scope, slot)?
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    for (template, checked) in templates.iter().zip(checked_templates) {
        let item = document
            .program()
            .checked_facts()
            .local(checked.item)
            .name
            .clone();
        let mut template_env = ScopedBindingEnv::new(env);
        template_env.insert(
            item.clone(),
            checked_local_binding(
                LocalBindingTypeSource::Checked(document.program()),
                checked.item,
                format!("(*{item})"),
                false,
            ),
        );
        if let Some(local) = checked.maximized {
            let binding = document.program().checked_facts().local(local).name.clone();
            template_env.insert(
                binding,
                checked_local_binding(
                    LocalBindingTypeSource::Checked(document.program()),
                    local,
                    "__pane_maximized".into(),
                    true,
                ),
            );
        }
        let key = checked_expr_use_code(
            document.program(),
            checked.key,
            &template_env,
            ValueMode::Owned,
        )?;
        let pane_scope = format!(
            "format!(\"{{}}/{}({{}})\", {pane_grid_scope}, __pane_key)",
            item
        );
        let content = render_pane_content(
            &template.pane,
            document,
            message,
            &template_env,
            &pane_scope,
            slot,
        )?;
        let items = &env
            .get(&template.items)
            .expect("checker validates dynamic pane state")
            .code;
        arms.push(format!(
            "{}::{}(__pane_key) => match {items}.iter().find(|{}| {key} == (*__pane_key).clone()) {{ ::std::option::Option::Some({}) => {content}, ::std::option::Option::None => ::iced::widget::pane_grid::Content::new(::iced::widget::text(::std::format!({}, __pane_key))), }}",
            pane_type.as_deref().expect("dynamic pane type"),
            pane_template_variant(&template.item),
            item,
            item,
            rust_string(&format!("Missing pane `{}({{}})`", item)),
        ));
    }
    let arms = arms.join(", ");
    let field = pane_field(name);
    let pane_value = if pane_type.is_some() {
        "__pane_name"
    } else {
        "*__pane_name"
    };
    let mut code = format!(
        "::iced::widget::pane_grid(&self.{field}, move |_, __pane_name, __pane_maximized| match {pane_value} {{ {arms}, _ => ::core::unreachable!() }})"
    );
    for (length, method) in [(&options.width, "width"), (&options.height, "height")] {
        if let Some(length) = length {
            write!(code, ".{method}({})", length_code(length, env, document)?).unwrap();
        }
    }
    for (value, method) in [
        (&options.spacing, "spacing"),
        (&options.min_size, "min_size"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}(::ui_lang_runtime::bounded_table_metric({}, self.{field}.len()))",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(leeway) = &options.resize_leeway {
        write!(
            code,
            ".on_resize(::ui_lang_runtime::bounded_table_metric({}, self.{field}.len()), {message}::{})",
            expr_code(leeway, env, document, ValueMode::Owned)?,
            pane_resize_variant(name)
        )
        .unwrap();
    }
    if options.draggable {
        write!(code, ".on_drag({message}::{})", pane_drag_variant(name)).unwrap();
    }
    if let Some(route) = &options.click {
        if pane_type.is_some() {
            let route = route_code(route, "__pane_name", env, document, message)?;
            write!(
                code,
                ".on_click(move |__pane| {{ let __pane_name = self.{field}.get(__pane).map(|__pane| __pane.__name()).unwrap_or_default(); {route} }})"
            )
            .unwrap();
        } else {
            let route = route_code(route, "__pane_name.to_owned()", env, document, message)?;
            write!(
                code,
                ".on_click(move |__pane| {{ let __pane_name = self.{field}.get(__pane).copied().unwrap_or(\"\"); {route} }})"
            )
            .unwrap();
        }
    }
    append_pane_grid_style(
        &mut code,
        &options.style,
        options.custom_style.as_ref(),
        env,
        document,
    )?;
    identify_rendered(
        format!("{code}.into()"),
        Some(&id),
        message,
        env,
        document,
        scope,
    )
}

pub(in crate::codegen) fn append_pane_grid_style(
    code: &mut String,
    style: &PaneGridStyle,
    custom: Option<&ExternCall>,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = style.region_radius.is_some()
        || style.region_radius_top_left.is_some()
        || style.region_radius_top_right.is_some()
        || style.region_radius_bottom_right.is_some()
        || style.region_radius_bottom_left.is_some();
    let has_typed = style.region_background.is_some()
        || style.region_border.is_some()
        || style.region_border_width.is_some()
        || has_radius
        || style.hovered_split.is_some()
        || style.hovered_split_width.is_some()
        || style.picked_split.is_some()
        || style.picked_split_width.is_some();
    let custom = custom
        .map(|style| {
            custom_style_call_code(style, ExternKind::PaneGridStyle, "__theme", env, document)
        })
        .transpose()?;
    if !has_typed && custom.is_none() {
        return Ok(());
    }
    if !has_typed {
        write!(
            code,
            ".style(move |__theme| {})",
            custom.expect("custom style is present")
        )
        .unwrap();
        return Ok(());
    }
    let base = custom.unwrap_or_else(|| "::iced::widget::pane_grid::default(__theme)".into());
    code.push_str(".style(move |__theme| { let mut __style = ");
    code.push_str(&base);
    code.push(';');
    if let Some(background) = &style.region_background {
        write!(
            code,
            " __style.hovered_region.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(border) = &style.region_border {
        write!(
            code,
            " __style.hovered_region.border.color = {};",
            theme_color(document, border)
        )
        .unwrap();
    }
    if let Some(width) = &style.region_border_width {
        write!(
            code,
            " __style.hovered_region.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if has_radius {
        let radius = radius_code(
            style.region_radius.as_ref(),
            [
                style.region_radius_top_left.as_ref(),
                style.region_radius_top_right.as_ref(),
                style.region_radius_bottom_right.as_ref(),
                style.region_radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("panes region radius options were present");
        write!(code, " __style.hovered_region.border.radius = {radius};").unwrap();
    }
    for (color, width, field) in [
        (
            &style.hovered_split,
            &style.hovered_split_width,
            "hovered_split",
        ),
        (
            &style.picked_split,
            &style.picked_split_width,
            "picked_split",
        ),
    ] {
        if let Some(color) = color {
            write!(
                code,
                " __style.{field}.color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(width) = width {
            write!(
                code,
                " __style.{field}.width = {} as f32;",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    code.push_str(" __style })");
    Ok(())
}

pub(in crate::codegen) fn render_pane_content(
    pane: &PaneView,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let body = render_node(&pane.content, document, message, env, scope, slot)?;
    let mut declarations = format!("let __pane_content: __IceElement<'_, {message}> = {body};");
    let mut content = String::from("::iced::widget::pane_grid::Content::new(__pane_content)");
    if let Some(style) = container_surface_style_value(
        &document.program().style_use(&pane.span)?.style,
        &pane.style,
        None,
        env,
        document,
    )? {
        write!(content, ".style(move |_| {style})").unwrap();
    }
    if let Some(title) = &pane.title {
        let title_content = render_node(&title.content, document, message, env, scope, slot)?;
        write!(
            declarations,
            " let __pane_title: __IceElement<'_, {message}> = {title_content};"
        )
        .unwrap();
        let mut title_bar = String::from("::iced::widget::pane_grid::TitleBar::new(__pane_title)");
        if let Some(padding) = typed_padding_code(&title.padding, env, document)? {
            write!(title_bar, ".padding({padding})").unwrap();
        }
        if let Some(controls) = &title.controls {
            let controls = render_node(controls, document, message, env, scope, slot)?;
            write!(
                declarations,
                " let __pane_controls: __IceElement<'_, {message}> = {controls};"
            )
            .unwrap();
            if let Some(compact) = &title.compact_controls {
                let compact = render_node(compact, document, message, env, scope, slot)?;
                write!(
                    declarations,
                    " let __pane_compact_controls: __IceElement<'_, {message}> = {compact};"
                )
                .unwrap();
                title_bar.push_str(".controls(::iced::widget::pane_grid::Controls::dynamic(__pane_controls, __pane_compact_controls))");
            } else {
                title_bar.push_str(
                    ".controls(::iced::widget::pane_grid::Controls::new(__pane_controls))",
                );
            }
        }
        if title.always_show_controls {
            title_bar.push_str(".always_show_controls()");
        }
        if let Some(style) = container_surface_style_value(
            &document.program().style_use(&title.span)?.style,
            &title.style,
            None,
            env,
            document,
        )? {
            write!(title_bar, ".style(move |_| {style})").unwrap();
        }
        write!(content, ".title_bar({title_bar})").unwrap();
    }
    Ok(format!("{{ {declarations} {content} }}"))
}

pub(in crate::codegen) fn render_rich_span(
    item: &RichSpan,
    document: &RenderDocument<'_>,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let style = &document.program().style_use(&item.span)?.style;
    let value = expr_code(&item.value, env, document, ValueMode::Owned)?;
    let mut code = format!("::iced::widget::span({value})");
    if let Some(size) = &item.options.size {
        write!(
            code,
            ".size({})",
            clamped_f32_code(size, "f32::EPSILON", "f32::MAX", env, document)?
        )
        .unwrap();
    } else if let Some(size) = style.text_size {
        write!(code, ".size({size})").unwrap();
    }
    if let Some(line_height) = &item.options.line_height {
        let line_height = text_line_height_code(line_height, env, document)?;
        write!(code, ".line_height({line_height})").unwrap();
    } else if let Some(line_height) = style.text_line_height {
        write!(
            code,
            ".line_height(::iced::widget::text::LineHeight::Relative({line_height}))"
        )
        .unwrap();
    }
    if let Some(font) = styled_font_code(item.options.font.as_ref(), style, document)? {
        write!(code, ".font({font})").unwrap();
    }
    if let Some(color) = &item.options.color {
        write!(code, ".color({})", theme_color(document, color)).unwrap();
    } else if let Some(color) = &style.text_color {
        write!(code, ".color({})", resolved_theme_color(color)).unwrap();
    }
    if let Some(link) = &item.options.link {
        write!(
            code,
            ".link({})",
            expr_code(link, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(background) = &item.options.background {
        write!(
            code,
            ".background({})",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    let has_border = item.options.border.is_some()
        || item.options.border_width.is_some()
        || item.options.radius.is_some()
        || item.options.radius_top_left.is_some()
        || item.options.radius_top_right.is_some()
        || item.options.radius_bottom_right.is_some()
        || item.options.radius_bottom_left.is_some();
    if has_border {
        let color = item
            .options
            .border
            .as_ref()
            .map(|color| theme_color(document, color))
            .unwrap_or_else(|| "::iced::Color::TRANSPARENT".into());
        let width = item.options.border_width.as_ref().map_or_else(
            || Ok("0.0".to_owned()),
            |width| expr_code(width, env, document, ValueMode::Owned),
        )?;
        let radius = radius_code(
            item.options.radius.as_ref(),
            [
                item.options.radius_top_left.as_ref(),
                item.options.radius_top_right.as_ref(),
                item.options.radius_bottom_right.as_ref(),
                item.options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .unwrap_or_else(|| "::iced::border::Radius::default()".into());
        write!(
            code,
            ".border(::iced::Border {{ color: {color}, width: {width} as f32, radius: {radius} }})"
        )
        .unwrap();
    }
    if let Some(padding) = typed_padding_code(&item.options.padding, env, document)? {
        write!(code, ".padding({padding})").unwrap();
    }
    if let Some(underline) = &item.options.underline {
        write!(
            code,
            ".underline({})",
            expr_code(underline, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(strikethrough) = &item.options.strikethrough {
        write!(
            code,
            ".strikethrough({})",
            expr_code(strikethrough, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    Ok(code)
}
