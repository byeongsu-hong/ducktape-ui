use super::*;

pub(in crate::codegen) fn canvas_path_code(
    segments: &[CanvasPathSegment],
    env: &dyn BindingEnvironment,
    document: &Document,
) -> Result<String, Error> {
    let mut code = String::from("::iced::widget::canvas::Path::new(|__path| {");
    for segment in segments {
        match segment {
            CanvasPathSegment::Move(x, y) => write!(
                code,
                " __path.move_to({});",
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Line(x, y) => write!(
                code,
                " __path.line_to({});",
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Arc {
                x,
                y,
                radius,
                start,
                end,
            } => write!(
                code,
                " __path.arc(::iced::widget::canvas::path::Arc {{ center: {}, radius: {}, start_angle: ::iced::Radians({} as f32), end_angle: ::iced::Radians({} as f32) }});",
                canvas_point_code(x, y, env, document)?,
                clamped_f32_code(radius, "0.0", "f32::MAX", env, document)?,
                canvas_expr_code(start, env, document)?,
                canvas_expr_code(end, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::ArcTo {
                ax,
                ay,
                bx,
                by,
                radius,
            } => write!(
                code,
                " __path.arc_to({}, {}, {});",
                canvas_point_code(ax, ay, env, document)?,
                canvas_point_code(bx, by, env, document)?,
                clamped_f32_code(radius, "0.0", "f32::MAX", env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Ellipse {
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
                canvas_point_code(x, y, env, document)?,
                clamped_f32_code(radius_x, "0.0", "f32::MAX", env, document)?,
                clamped_f32_code(radius_y, "0.0", "f32::MAX", env, document)?,
                canvas_expr_code(rotation, env, document)?,
                canvas_expr_code(start, env, document)?,
                canvas_expr_code(end, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Bezier {
                control_ax,
                control_ay,
                control_bx,
                control_by,
                x,
                y,
            } => write!(
                code,
                " __path.bezier_curve_to({}, {}, {});",
                canvas_point_code(control_ax, control_ay, env, document)?,
                canvas_point_code(control_bx, control_by, env, document)?,
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Quadratic {
                control_x,
                control_y,
                x,
                y,
            } => write!(
                code,
                " __path.quadratic_curve_to({}, {});",
                canvas_point_code(control_x, control_y, env, document)?,
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Rectangle {
                x,
                y,
                width,
                height,
            } => write!(
                code,
                " __path.rectangle({}, {});",
                canvas_point_code(x, y, env, document)?,
                canvas_size_code(width, height, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::RoundedRectangle {
                x,
                y,
                width,
                height,
                radius,
            } => write!(
                code,
                " __path.rounded_rectangle({}, {}, {});",
                canvas_point_code(x, y, env, document)?,
                canvas_size_code(width, height, env, document)?,
                canvas_radius_code(radius, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Circle { x, y, radius } => write!(
                code,
                " __path.circle({}, {});",
                canvas_point_code(x, y, env, document)?,
                clamped_f32_code(radius, "0.0", "f32::MAX", env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Close => code.push_str(" __path.close();"),
        }
    }
    code.push_str(" })");
    Ok(code)
}
