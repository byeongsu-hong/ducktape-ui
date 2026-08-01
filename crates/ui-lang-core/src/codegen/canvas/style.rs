use super::*;

pub(in crate::codegen) fn canvas_paint_code(
    paint: &CanvasPaint,
    path: &str,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    let mut code = String::new();
    if let Some(fill) = &paint.fill {
        write!(
            code,
            " __frame.fill({path}, {});",
            canvas_fill_code(fill, paint.fill_rule, env, document)?
        )
        .unwrap();
    }
    if let Some(stroke) = &paint.stroke {
        write!(
            code,
            " __frame.stroke({path}, {});",
            canvas_stroke_code(stroke, env, document)?
        )
        .unwrap();
    }
    Ok(code)
}

pub(in crate::codegen) fn canvas_fill_code(
    fill: &BackgroundValue,
    rule: CanvasFillRule,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    let rule = match rule {
        CanvasFillRule::NonZero => "NonZero",
        CanvasFillRule::EvenOdd => "EvenOdd",
    };
    Ok(format!(
        "::iced::widget::canvas::Fill {{ style: {}, rule: ::iced::widget::canvas::fill::Rule::{rule} }}",
        canvas_style_code(fill, env, document)?
    ))
}

pub(in crate::codegen) fn canvas_stroke_code(
    stroke: &CanvasStroke,
    env: &dyn BindingEnvironment,
    document: &Document,
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
        .map(|value| clamped_f32_code(value, "0.0", "f32::MAX", env, document))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!(
        "::iced::widget::canvas::Stroke {{ style: {}, width: {}, line_cap: ::iced::widget::canvas::LineCap::{cap}, line_join: ::iced::widget::canvas::LineJoin::{join}, line_dash: ::iced::widget::canvas::LineDash {{ segments: &[{dash}], offset: usize::try_from({}).unwrap_or(0) }} }}",
        canvas_style_code(&stroke.style, env, document)?,
        clamped_f32_code(&stroke.width, "0.0", "f32::MAX", env, document)?,
        canvas_expr_code(&stroke.dash_offset, env, document)?
    ))
}

pub(in crate::codegen) fn canvas_style_code(
    style: &BackgroundValue,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    Ok(match style {
        BackgroundValue::Color(color) => format!(
            "::iced::widget::canvas::Style::Solid({})",
            theme_color(document, color)
        ),
        BackgroundValue::Linear { angle, stops } => {
            let mut gradient =
                String::from("::iced::widget::canvas::gradient::Linear::new(__start, __end)");
            for stop in stops {
                write!(
                    gradient,
                    ".add_stop({} as f32, {})",
                    canvas_expr_code(&stop.offset, env, document)?,
                    theme_color(document, &stop.color)
                )
                .unwrap();
            }
            format!(
                "{{ let __angle = {} as f32; let __direction = ::iced::Vector::new(__angle.cos(), __angle.sin()); let __center = ::iced::Point::new(__bounds.width / 2.0, __bounds.height / 2.0); let __extent = (__bounds.width * __direction.x.abs() + __bounds.height * __direction.y.abs()) / 2.0; let __start = ::iced::Point::new(__center.x - __direction.x * __extent, __center.y - __direction.y * __extent); let __end = ::iced::Point::new(__center.x + __direction.x * __extent, __center.y + __direction.y * __extent); ::iced::widget::canvas::Style::Gradient(::iced::widget::canvas::Gradient::Linear({gradient})) }}",
                canvas_expr_code(angle, env, document)?
            )
        }
    })
}

pub(in crate::codegen) fn canvas_radius_is_empty(radius: &CanvasRadius) -> bool {
    radius.all.is_none()
        && radius.top_left.is_none()
        && radius.top_right.is_none()
        && radius.bottom_right.is_none()
        && radius.bottom_left.is_none()
}

pub(in crate::codegen) fn canvas_radius_code(
    radius: &CanvasRadius,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    radius_code(
        radius.all.as_ref(),
        [
            radius.top_left.as_ref(),
            radius.top_right.as_ref(),
            radius.bottom_right.as_ref(),
            radius.bottom_left.as_ref(),
        ],
        env,
        document,
    )
    .map(|radius| radius.unwrap_or_else(|| "::iced::border::Radius::default()".into()))
}

pub(in crate::codegen) fn canvas_point_code(
    x: &Expr,
    y: &Expr,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    Ok(format!(
        "::iced::Point::new({} as f32, {} as f32)",
        canvas_expr_code(x, env, document)?,
        canvas_expr_code(y, env, document)?
    ))
}

pub(in crate::codegen) fn canvas_size_code(
    width: &Expr,
    height: &Expr,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    Ok(format!(
        "::iced::Size::new({}, {})",
        clamped_f32_code(width, "0.0", "f32::MAX", env, document)?,
        clamped_f32_code(height, "0.0", "f32::MAX", env, document)?
    ))
}

pub(in crate::codegen) fn canvas_expr_code(
    value: &Expr,
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    expr_code(value, env, document, ValueMode::Owned)
}
