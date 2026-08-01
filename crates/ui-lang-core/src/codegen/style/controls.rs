use super::*;

pub(in crate::codegen) fn append_slider_styles(
    code: &mut String,
    styles: &SliderStyleSet,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<(), Error> {
    let custom = styles
        .custom
        .as_ref()
        .map(|style| {
            custom_style_call_code(
                style,
                ExternKind::SliderStyle,
                "__theme, __status",
                env,
                document,
            )
        })
        .transpose()?;
    if styles.active.is_none() && styles.hovered.is_none() && styles.dragged.is_none() {
        if let Some(custom) = custom {
            write!(code, ".style(move |__theme, __status| {custom})").unwrap();
        }
        return Ok(());
    }
    let base =
        custom.unwrap_or_else(|| "::iced::widget::slider::default(__theme, __status)".to_owned());
    write!(
        code,
        ".style(move |__theme, __status| {{ let mut __style = {base};"
    )
    .unwrap();
    if let Some(active) = &styles.active {
        append_slider_style_fields(code, active, env, document)?;
    }
    if styles.hovered.is_some() || styles.dragged.is_some() {
        code.push_str(" match __status {");
        for (status, style) in [("Hovered", &styles.hovered), ("Dragged", &styles.dragged)] {
            if let Some(style) = style {
                write!(code, " ::iced::widget::slider::Status::{status} => {{").unwrap();
                append_slider_style_fields(code, style, env, document)?;
                code.push_str(" }");
            }
        }
        code.push_str(" _ => {}");
        code.push_str(" }");
    }
    code.push_str(" __style })");
    Ok(())
}

pub(in crate::codegen) fn append_slider_style_fields(
    code: &mut String,
    style: &SliderStyle,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<(), Error> {
    for (background, field) in [
        (&style.rail_start, "__style.rail.backgrounds.0"),
        (&style.rail_end, "__style.rail.backgrounds.1"),
        (&style.handle_color, "__style.handle.background"),
    ] {
        if let Some(background) = background {
            write!(
                code,
                " {field} = {};",
                background_code(background, env, document)?
            )
            .unwrap();
        }
    }
    for (color, field) in [
        (&style.rail_border_color, "__style.rail.border.color"),
        (&style.handle_border_color, "__style.handle.border_color"),
    ] {
        if let Some(color) = color {
            write!(code, " {field} = {}.into();", theme_color(document, color)).unwrap();
        }
    }
    append_f32_fields(
        code,
        [
            (&style.rail_width, "__style.rail.width"),
            (&style.rail_border_width, "__style.rail.border.width"),
            (&style.handle_border_width, "__style.handle.border_width"),
        ],
        env,
        document,
    )?;
    if let Some(radius) = radius_code(
        style.rail_radius.as_ref(),
        [
            style.rail_radius_top_left.as_ref(),
            style.rail_radius_top_right.as_ref(),
            style.rail_radius_bottom_right.as_ref(),
            style.rail_radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.rail.border.radius = {radius};").unwrap();
    }
    if let Some(shape) = &style.handle_shape {
        let shape = match shape {
            SliderHandleShape::Circle(radius) => format!(
                "::iced::widget::slider::HandleShape::Circle {{ radius: {} as f32 }}",
                expr_code(radius, env, document, ValueMode::Owned)?
            ),
            SliderHandleShape::Rectangle { width } => {
                let radius = radius_code(
                    style.handle_radius.as_ref(),
                    [
                        style.handle_radius_top_left.as_ref(),
                        style.handle_radius_top_right.as_ref(),
                        style.handle_radius_bottom_right.as_ref(),
                        style.handle_radius_bottom_left.as_ref(),
                    ],
                    env,
                    document,
                )?
                .unwrap_or_else(|| "::iced::border::Radius::default()".to_owned());
                format!(
                    "::iced::widget::slider::HandleShape::Rectangle {{ width: {width}, border_radius: {radius} }}"
                )
            }
        };
        write!(code, " __style.handle.shape = {shape};").unwrap();
    }
    Ok(())
}

pub(in crate::codegen) fn append_progress_options(
    code: &mut String,
    options: &ProgressOptions,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<(), Error> {
    let radius = radius_code(
        options.radius.as_ref(),
        [
            options.radius_top_left.as_ref(),
            options.radius_top_right.as_ref(),
            options.radius_bottom_right.as_ref(),
            options.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )?;
    if options.style.is_none()
        && options.custom_style.is_none()
        && options.background.is_none()
        && options.bar.is_none()
        && options.border_color.is_none()
        && options.border_width.is_none()
        && radius.is_none()
    {
        return Ok(());
    }
    if let Some(style) = &options.custom_style {
        let custom =
            custom_style_call_code(style, ExternKind::ProgressStyle, "__theme", env, document)?;
        write!(code, ".style(move |__theme| {{ let mut __style = {custom};").unwrap();
    } else {
        let preset = match options.style.unwrap_or(ProgressStyle::Primary) {
            ProgressStyle::Primary => "primary",
            ProgressStyle::Secondary => "secondary",
            ProgressStyle::Success => "success",
            ProgressStyle::Warning => "warning",
            ProgressStyle::Danger => "danger",
        };
        write!(
            code,
            ".style(move |__theme| {{ let mut __style = ::iced::widget::progress_bar::{preset}(__theme);"
        )
        .unwrap();
    }
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(bar) = &options.bar {
        write!(
            code,
            " __style.bar = {};",
            background_code(bar, env, document)?
        )
        .unwrap();
    }
    if let Some(border) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, border)
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
    if let Some(radius) = radius {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

pub(in crate::codegen) fn append_rule_options(
    code: &mut String,
    options: &RuleOptions,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<(), Error> {
    let radius = radius_code(
        options.radius.as_ref(),
        [
            options.radius_top_left.as_ref(),
            options.radius_top_right.as_ref(),
            options.radius_bottom_right.as_ref(),
            options.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )?;
    if options.style.is_none()
        && options.fill.is_none()
        && options.color.is_none()
        && radius.is_none()
        && options.snap.is_none()
    {
        return Ok(());
    }
    let preset = match options.style.unwrap_or(RuleStyle::Default) {
        RuleStyle::Default => "default",
        RuleStyle::Weak => "weak",
    };
    write!(
        code,
        ".style(move |__theme| {{ let mut __style = ::iced::widget::rule::{preset}(__theme);"
    )
    .unwrap();
    if let Some(fill) = &options.fill {
        let fill = match fill {
            RuleFill::Full => "::iced::widget::rule::FillMode::Full".to_owned(),
            RuleFill::Percent(value) => format!(
                "::iced::widget::rule::FillMode::Percent({})",
                clamped_f32_code(value, "0.0", "100.0", env, document)?
            ),
            RuleFill::Padded(value) => {
                format!("::iced::widget::rule::FillMode::Padded({value})")
            }
            RuleFill::AsymmetricPadding(first, second) => {
                format!("::iced::widget::rule::FillMode::AsymmetricPadding({first}, {second})")
            }
        };
        write!(code, " __style.fill_mode = {fill};").unwrap();
    }
    if let Some(color) = &options.color {
        write!(code, " __style.color = {};", theme_color(document, color)).unwrap();
    }
    if let Some(radius) = radius {
        write!(code, " __style.radius = {radius};").unwrap();
    }
    if let Some(snap) = &options.snap {
        write!(
            code,
            " __style.snap = {};",
            expr_code(snap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}
