use super::*;
use crate::check::CheckedExprUseId;
use crate::hir::OriginId;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn render_pane_grid(
    pane_grid: &ResolvedPaneGrid,
    panes: &[PaneView],
    templates: &[PaneTemplate],
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let program = document.hir();
    if panes.len() != pane_grid.panes.len() || templates.len() != pane_grid.templates.len() {
        return Err(program.invariant_at_origin(
            pane_grid.origin,
            "pane grid render topology diverged from normalized HIR",
        ));
    }
    let id = Id {
        name: pane_grid.name.clone(),
        key: None,
    };
    let pane_grid_scope = id_code(&id, scope, env, document)?;
    let pane_type = (!pane_grid.templates.is_empty()).then(|| pane_type(&pane_grid.name));
    let mut arms = panes
        .iter()
        .zip(&pane_grid.panes)
        .map(|(pane, resolved)| {
            let mut pane_env = ScopedBindingEnv::new(env);
            if let Some(binding) = &resolved.maximized {
                pane_env.insert(
                    binding.name.clone(),
                    checked_local_binding(program, binding.local, "__pane_maximized".into(), true),
                );
            }
            let pane_scope = format!("format!(\"{{}}/{}\", {pane_grid_scope})", resolved.name);
            let pattern = pane_type.as_ref().map_or_else(
                || rust_string(&resolved.name),
                |pane_type| format!("{pane_type}::__Static({})", rust_string(&resolved.name)),
            );
            Ok(format!(
                "{} => {}",
                pattern,
                render_pane_content(
                    pane,
                    resolved,
                    document,
                    message,
                    &pane_env,
                    &pane_scope,
                    slot,
                )?
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    for (template, resolved) in templates.iter().zip(&pane_grid.templates) {
        let item = &resolved.item.name;
        let mut template_env = ScopedBindingEnv::new(env);
        template_env.insert(
            item.clone(),
            checked_local_binding(program, resolved.item.local, format!("(*{item})"), false),
        );
        if let Some(binding) = &resolved.pane.maximized {
            template_env.insert(
                binding.name.clone(),
                checked_local_binding(program, binding.local, "__pane_maximized".into(), true),
            );
        }
        let key = checked_expr_use_code(program, resolved.key, &template_env, ValueMode::Owned)?;
        let pane_scope = format!(
            "format!(\"{{}}/{}({{}})\", {pane_grid_scope}, __pane_key)",
            item
        );
        let content = render_pane_content(
            &template.pane,
            &resolved.pane,
            document,
            message,
            &template_env,
            &pane_scope,
            slot,
        )?;
        let items = pane_items_code(resolved.items, env, program, resolved.origin)?;
        arms.push(format!(
            "{}::{}(__pane_key) => match {items}.iter().find(|{}| {key} == (*__pane_key).clone()) {{ ::std::option::Option::Some({}) => {content}, ::std::option::Option::None => ::iced::widget::pane_grid::Content::new(::iced::widget::text(::std::format!({}, __pane_key))), }}",
            pane_type.as_deref().ok_or_else(|| program.invariant_at_origin(resolved.origin, "dynamic pane template has no normalized pane type"))?,
            pane_template_variant(item),
            item,
            item,
            rust_string(&format!("Missing pane `{}({{}})`", item)),
        ));
    }
    let arms = arms.join(", ");
    let field = pane_field(&pane_grid.name);
    let pane_value = if pane_type.is_some() {
        "__pane_name"
    } else {
        "*__pane_name"
    };
    let mut code = format!(
        "::iced::widget::pane_grid(&self.{field}, move |_, __pane_name, __pane_maximized| match {pane_value} {{ {arms}, _ => ::core::unreachable!() }})"
    );
    for (length, method) in [(&pane_grid.width, "width"), (&pane_grid.height, "height")] {
        if let Some(length) = length {
            write!(
                code,
                ".{method}({})",
                resolved_pane_length_code(length, program, env)?
            )
            .unwrap();
        }
    }
    for (value, method) in [
        (pane_grid.spacing, "spacing"),
        (pane_grid.min_size, "min_size"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}(::ui_lang_runtime::bounded_table_metric({}, self.{field}.len()))",
                checked_expr_use_code(program, value, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(leeway) = pane_grid.resize_leeway {
        write!(
            code,
            ".on_resize(::ui_lang_runtime::bounded_table_metric({}, self.{field}.len()), {message}::{})",
            checked_expr_use_code(program, leeway, env, ValueMode::Owned)?,
            pane_resize_variant(&pane_grid.name)
        )
        .unwrap();
    }
    if pane_grid.draggable {
        write!(
            code,
            ".on_drag({message}::{})",
            pane_drag_variant(&pane_grid.name)
        )
        .unwrap();
    }
    if let Some(route) = &pane_grid.click {
        if pane_type.is_some() {
            let route =
                resolved_interaction_route_code(route, &["__pane_name"], env, program, message)?;
            write!(
                code,
                ".on_click(move |__pane| {{ let __pane_name = self.{field}.get(__pane).map(|__pane| __pane.__name()).unwrap_or_default(); {route} }})"
            )
            .unwrap();
        } else {
            let route = resolved_interaction_route_code(
                route,
                &["__pane_name.to_owned()"],
                env,
                program,
                message,
            )?;
            write!(
                code,
                ".on_click(move |__pane| {{ let __pane_name = self.{field}.get(__pane).copied().unwrap_or(\"\"); {route} }})"
            )
            .unwrap();
        }
    }
    append_pane_grid_style(&mut code, pane_grid, env, document)?;
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
    pane_grid: &ResolvedPaneGrid,
    env: &dyn BindingEnvironment,
    document: &RenderDocument<'_>,
) -> Result<(), Error> {
    let program = document.hir();
    let style = &pane_grid.style;
    let has_radius = resolved_pane_radius_present(&style.region_radius);
    let has_typed = style.region_background.is_some()
        || style.region_border.is_some()
        || style.region_border_width.is_some()
        || has_radius
        || style.hovered_split.is_some()
        || style.hovered_split_width.is_some()
        || style.picked_split.is_some()
        || style.picked_split_width.is_some();
    let custom = pane_grid
        .custom_style
        .as_ref()
        .map(|style| resolved_pane_custom_style_code(style, env, program))
        .transpose()?;
    if !has_typed && custom.is_none() {
        return Ok(());
    }
    if !has_typed {
        write!(
            code,
            ".style(move |__theme| {})",
            custom.ok_or_else(
                || program.invariant_at_origin(pane_grid.origin, "pane style base is absent")
            )?
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
            resolved_pane_background_code(background, program, env)?
        )
        .unwrap();
    }
    if let Some(border) = &style.region_border {
        write!(
            code,
            " __style.hovered_region.border.color = {};",
            resolved_theme_color(border)
        )
        .unwrap();
    }
    if let Some(width) = &style.region_border_width {
        write!(
            code,
            " __style.hovered_region.border.width = {} as f32;",
            checked_expr_use_code(program, *width, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if has_radius {
        let radius =
            resolved_pane_radius_code(&style.region_radius, program, env)?.ok_or_else(|| {
                program.invariant_at_origin(pane_grid.origin, "pane radius presence diverged")
            })?;
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
                resolved_theme_color(color)
            )
            .unwrap();
        }
        if let Some(width) = width {
            write!(
                code,
                " __style.{field}.width = {} as f32;",
                checked_expr_use_code(program, *width, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    code.push_str(" __style })");
    Ok(())
}

pub(in crate::codegen) fn render_pane_content(
    pane: &PaneView,
    resolved: &ResolvedPaneView,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let program = document.hir();
    if pane.title.is_some() != resolved.title.is_some() {
        return Err(program.invariant_at_origin(
            resolved.origin,
            "pane title render topology diverged from normalized HIR",
        ));
    }
    let body = render_node(&pane.content, document, message, env, scope, slot)?;
    let mut declarations = format!("let __pane_content: __IceElement<'_, {message}> = {body};");
    let mut content = String::from("::iced::widget::pane_grid::Content::new(__pane_content)");
    if let Some(style) =
        resolved_pane_surface_style_value(&resolved.utility_style, &resolved.surface, env, program)?
    {
        write!(content, ".style(move |_| {style})").unwrap();
    }
    if let (Some(title), Some(resolved_title)) = (&pane.title, &resolved.title) {
        if title.controls.is_some() != resolved_title.has_controls
            || title.compact_controls.is_some() != resolved_title.has_compact_controls
        {
            return Err(program.invariant_at_origin(
                resolved_title.origin,
                "pane controls render topology diverged from normalized HIR",
            ));
        }
        let title_content = render_node(&title.content, document, message, env, scope, slot)?;
        write!(
            declarations,
            " let __pane_title: __IceElement<'_, {message}> = {title_content};"
        )
        .unwrap();
        let mut title_bar = String::from("::iced::widget::pane_grid::TitleBar::new(__pane_title)");
        if let Some(padding) = resolved_pane_padding_code(&resolved_title.padding, program, env)? {
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
        if resolved_title.always_show_controls {
            title_bar.push_str(".always_show_controls()");
        }
        if let Some(style) = resolved_pane_surface_style_value(
            &resolved_title.utility_style,
            &resolved_title.surface,
            env,
            program,
        )? {
            write!(title_bar, ".style(move |_| {style})").unwrap();
        }
        write!(content, ".title_bar({title_bar})").unwrap();
    }
    Ok(format!("{{ {declarations} {content} }}"))
}

fn pane_items_code(
    items: ResolvedPaneItems,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
    origin: OriginId,
) -> Result<String, Error> {
    let owner = match items {
        ResolvedPaneItems::Value(value) => BindingOwner::Value(value),
        ResolvedPaneItems::Local(local) => BindingOwner::Local(local),
    };
    let mut code = None;
    env.visit(&mut |_, binding| {
        if binding.owner == Some(owner) {
            code = Some(binding.code.clone());
        }
    });
    code.ok_or_else(|| {
        program.invariant_at_origin(origin, "pane items binding is absent from emission scope")
    })
}

fn resolved_pane_length_code(
    length: &ResolvedPaneLength,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match length {
        ResolvedPaneLength::Fill => "::iced::Fill".into(),
        ResolvedPaneLength::FillPortion(portion) => {
            format!("::iced::Length::FillPortion({portion})")
        }
        ResolvedPaneLength::Shrink => "::iced::Shrink".into(),
        ResolvedPaneLength::FixedF64(expression) => format!(
            "{} as f32",
            checked_expr_use_code(program, *expression, env, ValueMode::Owned)?
        ),
        ResolvedPaneLength::FixedLength(expression) => {
            checked_expr_use_code(program, *expression, env, ValueMode::Owned)?
        }
    })
}

fn resolved_pane_custom_style_code(
    style: &ResolvedPaneCustomStyle,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let arguments = style
        .arguments
        .iter()
        .map(|argument| checked_expr_use_code(program, *argument, env, ValueMode::Owned))
        .collect::<Result<Vec<_>, _>>()?;
    let suffix = arguments
        .into_iter()
        .map(|argument| format!(", {argument}"))
        .collect::<String>();
    Ok(format!(
        "{}(__theme{suffix})",
        program.extern_function(style.function).rust_path
    ))
}

fn resolved_pane_background_code(
    background: &ResolvedPaneBackground,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    Ok(match background {
        ResolvedPaneBackground::Color(color) => {
            format!("::iced::Background::Color({})", resolved_theme_color(color))
        }
        ResolvedPaneBackground::Linear { angle, stops } => {
            let mut code = format!(
                "::iced::Background::from(::iced::gradient::Linear::new({} as f32)",
                checked_expr_use_code(program, *angle, env, ValueMode::Owned)?
            );
            for stop in stops {
                write!(
                    code,
                    ".add_stop({} as f32, {})",
                    checked_expr_use_code(program, stop.offset, env, ValueMode::Owned)?,
                    resolved_theme_color(&stop.color)
                )
                .unwrap();
            }
            code.push(')');
            code
        }
    })
}

fn resolved_pane_radius_present(radius: &ResolvedPaneRadius) -> bool {
    radius.all.is_some()
        || radius.top_left.is_some()
        || radius.top_right.is_some()
        || radius.bottom_right.is_some()
        || radius.bottom_left.is_some()
}

fn resolved_pane_radius_code(
    radius: &ResolvedPaneRadius,
    program: &LoweredProgram,
    env: &dyn BindingEnvironment,
) -> Result<Option<String>, Error> {
    if !resolved_pane_radius_present(radius) {
        return Ok(None);
    }
    let value = |expression: Option<CheckedExprUseId>| {
        expression
            .map(|expression| {
                checked_expr_use_code(program, expression, env, ValueMode::Owned)
                    .map(|code| format!("(({code}) as f32).max(0.0).min(f32::MAX)"))
            })
            .transpose()
    };
    let all = value(radius.all)?.unwrap_or_else(|| "0.0".into());
    let top_left = value(radius.top_left)?.unwrap_or_else(|| all.clone());
    let top_right = value(radius.top_right)?.unwrap_or_else(|| all.clone());
    let bottom_right = value(radius.bottom_right)?.unwrap_or_else(|| all.clone());
    let bottom_left = value(radius.bottom_left)?.unwrap_or(all);
    Ok(Some(format!(
        "::iced::border::Radius {{ top_left: {top_left}, top_right: {top_right}, bottom_right: {bottom_right}, bottom_left: {bottom_left} }}"
    )))
}

fn resolved_pane_padding_code(
    padding: &ResolvedPanePadding,
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
    let value = |expression: Option<CheckedExprUseId>| {
        expression
            .map(|expression| checked_expr_use_code(program, expression, env, ValueMode::Owned))
            .transpose()
    };
    let all = value(padding.all)?.unwrap_or_else(|| "0.0".into());
    let x = value(padding.x)?.unwrap_or_else(|| all.clone());
    let y = value(padding.y)?.unwrap_or_else(|| all.clone());
    let top = value(padding.top)?.unwrap_or_else(|| y.clone());
    let right = value(padding.right)?.unwrap_or_else(|| x.clone());
    let bottom = value(padding.bottom)?.unwrap_or(y);
    let left = value(padding.left)?.unwrap_or(x);
    Ok(Some(format!(
        "::ui_lang_runtime::bounded_padding({top}, {right}, {bottom}, {left})"
    )))
}

fn resolved_pane_surface_style_value(
    utilities: &ResolvedStyle,
    surface: &ResolvedPaneSurface,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<Option<String>, Error> {
    let has_typed = surface.background.is_some()
        || surface.text_color.is_some()
        || surface.border_color.is_some()
        || surface.border_width.is_some()
        || resolved_pane_radius_present(&surface.radius)
        || surface.shadow_color.is_some()
        || surface.shadow_x.is_some()
        || surface.shadow_y.is_some()
        || surface.shadow_blur.is_some()
        || surface.pixel_snap.is_some();
    let utility = container_style_value(utilities);
    if !has_typed {
        return Ok(utility);
    }
    let base = utility.unwrap_or_else(|| "::iced::widget::container::Style::default()".into());
    let mut code = format!("{{ let mut __style = {base};");
    if let Some(background) = &surface.background {
        write!(
            code,
            " __style.background = ::std::option::Option::Some({});",
            resolved_pane_background_code(background, program, env)?
        )
        .unwrap();
    }
    if let Some(color) = &surface.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    if let Some(color) = &surface.border_color {
        write!(
            code,
            " __style.border.color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    if let Some(width) = surface.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            checked_expr_use_code(program, width, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = resolved_pane_radius_code(&surface.radius, program, env)? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if let Some(color) = &surface.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            resolved_theme_color(color)
        )
        .unwrap();
    }
    for (expression, field) in [
        (surface.shadow_x, "offset.x"),
        (surface.shadow_y, "offset.y"),
        (surface.shadow_blur, "blur_radius"),
    ] {
        if let Some(expression) = expression {
            write!(
                code,
                " __style.shadow.{field} = {} as f32;",
                checked_expr_use_code(program, expression, env, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(snap) = surface.pixel_snap {
        write!(
            code,
            " __style.snap = {};",
            checked_expr_use_code(program, snap, env, ValueMode::Owned)?
        )
        .unwrap();
    }
    code.push_str(" __style }");
    Ok(Some(code))
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
