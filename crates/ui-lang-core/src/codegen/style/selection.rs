use super::*;

pub(in crate::codegen) fn pick_list_handle_code(
    handle: &PickListHandle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match handle {
        PickListHandle::Arrow { size } => {
            let size = size.as_ref().map_or_else(
                || Ok("::std::option::Option::None".to_owned()),
                |value| {
                    Ok::<_, Error>(format!(
                        "::std::option::Option::Some({}.into())",
                        clamped_f32_code(value, "f32::EPSILON", "f32::MAX", env, document)?
                    ))
                },
            )?;
            format!("::iced::widget::pick_list::Handle::Arrow {{ size: {size} }}")
        }
        PickListHandle::Static(icon) => format!(
            "::iced::widget::pick_list::Handle::Static({})",
            pick_list_icon_code(icon, env, document)?
        ),
        PickListHandle::Dynamic { closed, open } => format!(
            "::iced::widget::pick_list::Handle::Dynamic {{ closed: {}, open: {} }}",
            pick_list_icon_code(closed, env, document)?,
            pick_list_icon_code(open, env, document)?
        ),
        PickListHandle::None => "::iced::widget::pick_list::Handle::None".to_owned(),
    })
}

pub(in crate::codegen) fn pick_list_icon_code(
    icon: &PickListIcon,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let font = icon.font.as_ref().map_or_else(
        || Ok("::iced::Font::DEFAULT".to_owned()),
        |font| font_preset_code(font, document),
    )?;
    let size = icon.size.as_ref().map_or_else(
        || Ok("::std::option::Option::None".to_owned()),
        |value| {
            Ok::<_, Error>(format!(
                "::std::option::Option::Some({}.into())",
                clamped_f32_code(value, "f32::EPSILON", "f32::MAX", env, document)?
            ))
        },
    )?;
    let line_height = icon.line_height.as_ref().map_or_else(
        || Ok("::iced::widget::text::LineHeight::default()".to_owned()),
        |value| {
            Ok::<_, Error>(format!(
                "::iced::widget::text::LineHeight::Relative({})",
                clamped_f32_code(value, "f32::EPSILON", "f32::MAX", env, document)?
            ))
        },
    )?;
    let shaping = icon.shaping.map_or_else(
        || "::iced::widget::text::Shaping::default()".to_owned(),
        |shaping| {
            format!(
                "::iced::widget::text::Shaping::{}",
                text_shaping_code(shaping)
            )
        },
    );
    Ok(format!(
        "::iced::widget::pick_list::Icon {{ font: {font}, code_point: {:?}, size: {size}, line_height: {line_height}, shaping: {shaping} }}",
        icon.code_point
    ))
}

pub(in crate::codegen) fn pick_list_style_code(
    options: &PickListOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = options
        .custom_style
        .as_ref()
        .map(|style| {
            custom_style_call_code(
                style,
                ExternKind::PickListStyle,
                "__theme, __status",
                env,
                document,
            )
        })
        .transpose()?;
    let overrides = [
        ("Active", &options.style.active),
        ("Hovered", &options.style.hovered),
        ("Opened { is_hovered: false }", &options.style.opened),
        ("Opened { is_hovered: true }", &options.style.opened_hovered),
    ];
    let mut code = String::new();
    if overrides.iter().any(|(_, style)| style.is_some()) {
        let base = custom
            .unwrap_or_else(|| "::iced::widget::pick_list::default(__theme, __status)".to_owned());
        write!(
            code,
            ".style(move |__theme, __status| {{ let mut __style = {base};"
        )
        .unwrap();
        if let Some(active) = &options.style.active {
            append_pick_list_status_style(&mut code, active, env, document)?;
        }
        let overrides = [
            ("Hovered", None, options.style.hovered.as_ref()),
            (
                "Opened { is_hovered: false }",
                None,
                options.style.opened.as_ref(),
            ),
            (
                "Opened { is_hovered: true }",
                options.style.opened.as_ref(),
                options.style.opened_hovered.as_ref(),
            ),
        ];
        if overrides
            .iter()
            .any(|(_, inherited, style)| inherited.is_some() || style.is_some())
        {
            code.push_str(" match __status {");
            for (status, inherited, style) in overrides {
                if inherited.is_none() && style.is_none() {
                    continue;
                }
                write!(code, " ::iced::widget::pick_list::Status::{status} => {{").unwrap();
                if let Some(inherited) = inherited {
                    append_pick_list_status_style(&mut code, inherited, env, document)?;
                }
                if let Some(style) = style {
                    append_pick_list_status_style(&mut code, style, env, document)?;
                }
                code.push_str(" }");
            }
            code.push_str(" _ => {} }");
        }
        code.push_str(" __style })");
    } else if let Some(custom) = custom {
        write!(code, ".style(move |__theme, __status| {custom})").unwrap();
    }
    code.push_str(&menu_style_code(
        options.menu_style.as_deref(),
        options.custom_menu_style.as_ref(),
        env,
        document,
    )?);
    Ok(code)
}

fn append_pick_list_status_style(
    code: &mut String,
    style: &PickListStatusStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    append_select_surface_overrides(code, &style.options, env, document, false)?;
    if let Some(color) = &style.placeholder_color {
        write!(
            code,
            " __style.placeholder_color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(color) = &style.handle_color {
        write!(
            code,
            " __style.handle_color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn menu_style_code(
    style: Option<&MenuStyleOptions>,
    custom: Option<&ExternCall>,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = custom
        .map(|style| custom_style_call_code(style, ExternKind::MenuStyle, "__theme", env, document))
        .transpose()?;
    let Some(style) = style else {
        return Ok(custom
            .map(|custom| format!(".menu_style(move |__theme| {custom})"))
            .unwrap_or_default());
    };
    let base = custom.unwrap_or_else(|| "::iced::overlay::menu::default(__theme)".to_owned());
    let mut code = String::new();
    write!(
        code,
        ".menu_style(move |__theme| {{ let mut __style = {base};"
    )
    .unwrap();
    append_select_surface_overrides(&mut code, &style.options, env, document, true)?;
    if let Some(color) = &style.selected_text_color {
        write!(
            code,
            " __style.selected_text_color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(background) = &style.selected_background {
        write!(
            code,
            " __style.selected_background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(code)
}

pub(in crate::codegen) fn append_select_surface_overrides(
    code: &mut String,
    options: &ContainerStyleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
    shadow: bool,
) -> Result<(), Error> {
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(color) = &options.text_color {
        write!(
            code,
            " __style.text_color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(color) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(width) = &options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = radius_code(
        options.radius.as_ref(),
        [
            options.radius_top_left.as_ref(),
            options.radius_top_right.as_ref(),
            options.radius_bottom_right.as_ref(),
            options.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if shadow {
        if let Some(color) = &options.shadow_color {
            write!(
                code,
                " __style.shadow.color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        append_f32_fields(
            code,
            [
                (&options.shadow_x, "__style.shadow.offset.x"),
                (&options.shadow_y, "__style.shadow.offset.y"),
                (&options.shadow_blur, "__style.shadow.blur_radius"),
            ],
            env,
            document,
        )?;
    }
    Ok(())
}

pub(in crate::codegen) fn text_input_icon_code(
    icon: &TextInputIcon,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let font = icon.font.as_ref().map_or_else(
        || Ok("::iced::Font::DEFAULT".to_owned()),
        |font| font_preset_code(font, document),
    )?;
    let size = icon.size.as_ref().map_or_else(
        || Ok("::std::option::Option::None".to_owned()),
        |value| {
            Ok::<_, Error>(format!(
                "::std::option::Option::Some({}.into())",
                clamped_f32_code(value, "f32::EPSILON", "f32::MAX", env, document)?
            ))
        },
    )?;
    let spacing = icon.spacing.as_ref().map_or_else(
        || Ok("0.0".to_owned()),
        |value| expr_code(value, env, document, ValueMode::Owned),
    )?;
    let side = match icon.side {
        IconSide::Left => "Left",
        IconSide::Right => "Right",
    };
    Ok(format!(
        "::iced::widget::text_input::Icon {{ font: {font}, code_point: {:?}, size: {size}, spacing: {spacing} as f32, side: ::iced::widget::text_input::Side::{side} }}",
        icon.code_point
    ))
}

pub(in crate::codegen) fn text_input_style_code(
    styles: &TextInputStyleSet,
    custom: Option<&ExternCall>,
    utilities: Option<&ResolvedStyle>,
    env: &HashMap<String, Binding>,
    document: &Document,
    method: &str,
    widget: &str,
) -> Result<String, Error> {
    let custom_kind = if widget == "text_editor" {
        ExternKind::EditorStyle
    } else {
        ExternKind::InputStyle
    };
    let custom = custom
        .map(|style| custom_style_call_code(style, custom_kind, "__theme, __status", env, document))
        .transpose()?;
    let has_utilities = utilities.is_some_and(|style| {
        style.background.is_some()
            || style.border_color.is_some()
            || style.border_width != 0
            || style.radius != 0
            || style.focus_border_color.is_some()
    });
    let overrides = [
        ("Active", &styles.active),
        ("Hovered", &styles.hovered),
        ("Focused { is_hovered: false }", &styles.focused),
        ("Focused { is_hovered: true }", &styles.focused_hovered),
        ("Disabled", &styles.disabled),
    ];
    let has_overrides = overrides.iter().any(|(_, style)| style.is_some());
    if !has_overrides && !has_utilities {
        return Ok(custom
            .map(|custom| format!(".{method}(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base =
        custom.unwrap_or_else(|| format!("::iced::widget::{widget}::default(__theme, __status)"));
    let mut code = format!(".{method}(move |__theme, __status| {{ let mut __style = {base};");
    if let Some(style) = utilities.filter(|_| has_utilities) {
        if let Some(background) = &style.background {
            write!(
                code,
                " __style.background = {}.into();",
                resolved_theme_color(background)
            )
            .unwrap();
        }
        if let Some(border) = &style.border_color {
            write!(
                code,
                " __style.border.color = {};",
                resolved_theme_color(border)
            )
            .unwrap();
        }
        if style.border_width != 0 {
            write!(code, " __style.border.width = {}.0;", style.border_width).unwrap();
        }
        if style.radius != 0 {
            write!(code, " __style.border.radius = {}.0.into();", style.radius).unwrap();
        }
        if let Some(focus) = &style.focus_border_color {
            write!(
                code,
                " if matches!(__status, ::iced::widget::text_input::Status::Focused {{ .. }}) {{ __style.border.color = {}; }}",
                resolved_theme_color(focus)
            )
            .unwrap();
        }
    }
    if has_overrides {
        if let Some(active) = &styles.active {
            append_text_input_style_overrides(&mut code, active, env, document)?;
        }
        let overrides = [
            ("Hovered", None, styles.hovered.as_ref()),
            (
                "Focused { is_hovered: false }",
                None,
                styles.focused.as_ref(),
            ),
            (
                "Focused { is_hovered: true }",
                styles.focused.as_ref(),
                styles.focused_hovered.as_ref(),
            ),
            ("Disabled", None, styles.disabled.as_ref()),
        ];
        if overrides
            .iter()
            .any(|(_, inherited, style)| inherited.is_some() || style.is_some())
        {
            code.push_str(" match __status {");
            for (status, inherited, style) in overrides {
                if inherited.is_none() && style.is_none() {
                    continue;
                }
                write!(code, " ::iced::widget::{widget}::Status::{status} => {{").unwrap();
                if let Some(inherited) = inherited {
                    append_text_input_style_overrides(&mut code, inherited, env, document)?;
                }
                if let Some(style) = style {
                    append_text_input_style_overrides(&mut code, style, env, document)?;
                }
                code.push_str(" }");
            }
            code.push_str(" _ => {} }");
        }
    }
    code.push_str(" __style })");
    Ok(code)
}

pub(in crate::codegen) fn append_text_input_style_overrides(
    code: &mut String,
    style: &TextInputStatusStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    if let Some(background) = &style.options.background {
        write!(
            code,
            " __style.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(color) = &style.options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(width) = &style.options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = radius_code(
        style.options.radius.as_ref(),
        [
            style.options.radius_top_left.as_ref(),
            style.options.radius_top_right.as_ref(),
            style.options.radius_bottom_right.as_ref(),
            style.options.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    for (color, field) in [
        (&style.icon_color, "__style.icon"),
        (&style.placeholder_color, "__style.placeholder"),
        (&style.value_color, "__style.value"),
        (&style.selection_color, "__style.selection"),
    ] {
        if let Some(color) = color {
            write!(code, " {field} = {};", theme_color(document, color)).unwrap();
        }
    }
    Ok(())
}
