use super::*;

pub(in crate::codegen) fn canvas_paint_code(
    paint: &ResolvedCanvasPaint,
    path: &str,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let mut code = String::new();
    if let Some(fill) = &paint.fill {
        write!(
            code,
            " __frame.fill({path}, {});",
            canvas_fill_code(fill, paint.fill_rule, env, program)?
        )
        .unwrap();
    }
    if let Some(stroke) = &paint.stroke {
        write!(
            code,
            " __frame.stroke({path}, {});",
            canvas_stroke_code(stroke, env, program)?
        )
        .unwrap();
    }
    Ok(code)
}

pub(in crate::codegen) fn canvas_fill_code(
    fill: &ResolvedCanvasBackground,
    rule: CanvasFillRule,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let rule = match rule {
        CanvasFillRule::NonZero => "NonZero",
        CanvasFillRule::EvenOdd => "EvenOdd",
    };
    Ok(format!(
        "::iced::widget::canvas::Fill {{ style: {}, rule: ::iced::widget::canvas::fill::Rule::{rule} }}",
        canvas_style_code(fill, env, program)?
    ))
}

pub(in crate::codegen) fn canvas_stroke_code(
    stroke: &ResolvedCanvasStroke,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let cap = match stroke.cap {
        CanvasLineCap::Butt => "Butt",
        CanvasLineCap::Square => "Square",
        CanvasLineCap::Round => "Round",
    };
    let join = match stroke.join {
        CanvasLineJoin::Miter => "Miter",
        CanvasLineJoin::Round => "Round",
        CanvasLineJoin::Bevel => "Bevel",
    };
    let dash = stroke
        .dash
        .iter()
        .map(|value| clamped_f32_code(*value, "0.0", "f32::MAX", program, env))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!(
        "::iced::widget::canvas::Stroke {{ style: {}, width: {}, line_cap: ::iced::widget::canvas::LineCap::{cap}, line_join: ::iced::widget::canvas::LineJoin::{join}, line_dash: ::iced::widget::canvas::LineDash {{ segments: &[{dash}], offset: usize::try_from({}).unwrap_or(0) }} }}",
        canvas_style_code(&stroke.style, env, program)?,
        clamped_f32_code(stroke.width, "0.0", "f32::MAX", program, env)?,
        canvas_expr_code(stroke.dash_offset, env, program)?
    ))
}

pub(in crate::codegen) fn canvas_style_code(
    style: &ResolvedCanvasBackground,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    Ok(match style {
        ResolvedCanvasBackground::Color(color) => format!(
            "::iced::widget::canvas::Style::Solid({})",
            resolved_theme_color(color)
        ),
        ResolvedCanvasBackground::Linear { angle, stops } => {
            let mut gradient =
                String::from("::iced::widget::canvas::gradient::Linear::new(__start, __end)");
            for stop in stops {
                write!(
                    gradient,
                    ".add_stop({} as f32, {})",
                    canvas_expr_code(stop.offset, env, program)?,
                    resolved_theme_color(&stop.color)
                )
                .unwrap();
            }
            format!(
                "{{ let __angle = {} as f32; let __direction = ::iced::Vector::new(__angle.cos(), __angle.sin()); let __center = ::iced::Point::new(__bounds.width / 2.0, __bounds.height / 2.0); let __extent = (__bounds.width * __direction.x.abs() + __bounds.height * __direction.y.abs()) / 2.0; let __start = ::iced::Point::new(__center.x - __direction.x * __extent, __center.y - __direction.y * __extent); let __end = ::iced::Point::new(__center.x + __direction.x * __extent, __center.y + __direction.y * __extent); ::iced::widget::canvas::Style::Gradient(::iced::widget::canvas::Gradient::Linear({gradient})) }}",
                canvas_expr_code(*angle, env, program)?
            )
        }
    })
}

pub(in crate::codegen) fn canvas_radius_is_empty(radius: &ResolvedCanvasRadius) -> bool {
    radius.all.is_none()
        && radius.top_left.is_none()
        && radius.top_right.is_none()
        && radius.bottom_right.is_none()
        && radius.bottom_left.is_none()
}

pub(in crate::codegen) fn canvas_radius_code(
    radius: &ResolvedCanvasRadius,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let all = radius
        .all
        .map(|value| canvas_expr_code(value, env, program))
        .transpose()?;
    let corner = |value: Option<ResolvedExpressionId>| {
        value
            .map(|value| canvas_expr_code(value, env, program))
            .transpose()
    };
    let top_left = corner(radius.top_left)?
        .or_else(|| all.clone())
        .unwrap_or_else(|| "0.0".into());
    let top_right = corner(radius.top_right)?
        .or_else(|| all.clone())
        .unwrap_or_else(|| "0.0".into());
    let bottom_right = corner(radius.bottom_right)?
        .or_else(|| all.clone())
        .unwrap_or_else(|| "0.0".into());
    let bottom_left = corner(radius.bottom_left)?
        .or(all)
        .unwrap_or_else(|| "0.0".into());
    Ok(format!(
        "::iced::border::Radius {{ top_left: ({top_left}) as f32, top_right: ({top_right}) as f32, bottom_right: ({bottom_right}) as f32, bottom_left: ({bottom_left}) as f32 }}"
    ))
}

pub(in crate::codegen) fn canvas_point_code(
    x: ResolvedExpressionId,
    y: ResolvedExpressionId,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    Ok(format!(
        "::iced::Point::new({} as f32, {} as f32)",
        canvas_expr_code(x, env, program)?,
        canvas_expr_code(y, env, program)?
    ))
}

pub(in crate::codegen) fn canvas_size_code(
    width: ResolvedExpressionId,
    height: ResolvedExpressionId,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    Ok(format!(
        "::iced::Size::new({}, {})",
        clamped_f32_code(width, "0.0", "f32::MAX", program, env)?,
        clamped_f32_code(height, "0.0", "f32::MAX", program, env)?
    ))
}

pub(in crate::codegen) fn canvas_expr_code(
    value: ResolvedExpressionId,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    resolved_expr_use_code(program, value, env, ValueMode::Owned)
}
