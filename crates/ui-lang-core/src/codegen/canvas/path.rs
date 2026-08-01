use super::*;

pub(in crate::codegen) fn canvas_path_code(
    segments: &[ResolvedCanvasPathSegment],
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let mut code = String::from("::iced::widget::canvas::Path::new(|__path| {");
    for segment in segments {
        match segment {
            ResolvedCanvasPathSegment::Move(x, y) => write!(
                code,
                " __path.move_to({});",
                canvas_point_code(*x, *y, env, program)?
            )
            .unwrap(),
            ResolvedCanvasPathSegment::Line(x, y) => write!(
                code,
                " __path.line_to({});",
                canvas_point_code(*x, *y, env, program)?
            )
            .unwrap(),
            ResolvedCanvasPathSegment::Arc {
                x,
                y,
                radius,
                start,
                end,
            } => write!(
                code,
                " __path.arc(::iced::widget::canvas::path::Arc {{ center: {}, radius: {}, start_angle: ::iced::Radians({} as f32), end_angle: ::iced::Radians({} as f32) }});",
                canvas_point_code(*x, *y, env, program)?,
                canvas_clamped_f32_code(*radius, "0.0", "f32::MAX", env, program)?,
                canvas_expr_code(*start, env, program)?,
                canvas_expr_code(*end, env, program)?
            )
            .unwrap(),
            ResolvedCanvasPathSegment::ArcTo {
                ax,
                ay,
                bx,
                by,
                radius,
            } => write!(
                code,
                " __path.arc_to({}, {}, {});",
                canvas_point_code(*ax, *ay, env, program)?,
                canvas_point_code(*bx, *by, env, program)?,
                canvas_clamped_f32_code(*radius, "0.0", "f32::MAX", env, program)?
            )
            .unwrap(),
            ResolvedCanvasPathSegment::Ellipse {
                x,
                y,
                radius_x,
                radius_y,
                rotation,
                start,
                end,
            } => write!(
                code,
                " __path.ellipse(::iced::widget::canvas::path::arc::Elliptical {{ center: {}, radii: ::iced::Vector::new({}, {}), rotation: ::iced::Radians({} as f32), start_angle: ::iced::Radians({} as f32), end_angle: ::iced::Radians({} as f32) }});",
                canvas_point_code(*x, *y, env, program)?,
                canvas_clamped_f32_code(*radius_x, "0.0", "f32::MAX", env, program)?,
                canvas_clamped_f32_code(*radius_y, "0.0", "f32::MAX", env, program)?,
                canvas_expr_code(*rotation, env, program)?,
                canvas_expr_code(*start, env, program)?,
                canvas_expr_code(*end, env, program)?
            )
            .unwrap(),
            ResolvedCanvasPathSegment::Bezier {
                control_ax,
                control_ay,
                control_bx,
                control_by,
                x,
                y,
            } => write!(
                code,
                " __path.bezier_curve_to({}, {}, {});",
                canvas_point_code(*control_ax, *control_ay, env, program)?,
                canvas_point_code(*control_bx, *control_by, env, program)?,
                canvas_point_code(*x, *y, env, program)?
            )
            .unwrap(),
            ResolvedCanvasPathSegment::Quadratic {
                control_x,
                control_y,
                x,
                y,
            } => write!(
                code,
                " __path.quadratic_curve_to({}, {});",
                canvas_point_code(*control_x, *control_y, env, program)?,
                canvas_point_code(*x, *y, env, program)?
            )
            .unwrap(),
            ResolvedCanvasPathSegment::Rectangle {
                x,
                y,
                width,
                height,
            } => write!(
                code,
                " __path.rectangle({}, {});",
                canvas_point_code(*x, *y, env, program)?,
                canvas_size_code(*width, *height, env, program)?
            )
            .unwrap(),
            ResolvedCanvasPathSegment::RoundedRectangle {
                x,
                y,
                width,
                height,
                radius,
            } => write!(
                code,
                " __path.rounded_rectangle({}, {}, {});",
                canvas_point_code(*x, *y, env, program)?,
                canvas_size_code(*width, *height, env, program)?,
                canvas_radius_code(radius, env, program)?
            )
            .unwrap(),
            ResolvedCanvasPathSegment::Circle { x, y, radius } => write!(
                code,
                " __path.circle({}, {});",
                canvas_point_code(*x, *y, env, program)?,
                canvas_clamped_f32_code(*radius, "0.0", "f32::MAX", env, program)?
            )
            .unwrap(),
            ResolvedCanvasPathSegment::Close => code.push_str(" __path.close();"),
        }
    }
    code.push_str(" })");
    Ok(code)
}
