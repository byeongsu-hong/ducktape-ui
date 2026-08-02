use super::*;

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
