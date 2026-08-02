use super::*;

pub(in crate::codegen) fn canvas_commands_code(
    commands: &[ResolvedCanvasCommand],
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let mut code = String::new();
    for command in commands {
        match command {
            ResolvedCanvasCommand::Rectangle {
                x,
                y,
                width,
                height,
                radius,
                paint,
                ..
            } => {
                let point = canvas_point_code(*x, *y, env, program)?;
                let size = canvas_size_code(*width, *height, env, program)?;
                if canvas_radius_is_empty(radius) {
                    if let Some(fill) = &paint.fill {
                        write!(
                            code,
                            " __frame.fill_rectangle({point}, {size}, {});",
                            canvas_fill_code(fill, paint.fill_rule, env, program)?
                        )
                        .unwrap();
                    }
                    if let Some(stroke) = &paint.stroke {
                        write!(
                            code,
                            " __frame.stroke_rectangle({point}, {size}, {});",
                            canvas_stroke_code(stroke, env, program)?
                        )
                        .unwrap();
                    }
                } else {
                    let radius = canvas_radius_code(radius, env, program)?;
                    write!(
                        code,
                        " {{ let __path = ::iced::widget::canvas::Path::rounded_rectangle({point}, {size}, {radius}); {} }}",
                        canvas_paint_code(paint, "&__path", env, program)?
                    )
                    .unwrap();
                }
            }
            ResolvedCanvasCommand::Circle {
                x,
                y,
                radius,
                paint,
                ..
            } => {
                let point = canvas_point_code(*x, *y, env, program)?;
                let radius = canvas_clamped_f32_code(*radius, "0.0", "f32::MAX", env, program)?;
                write!(
                    code,
                    " {{ let __path = ::iced::widget::canvas::Path::circle({point}, {radius}); {} }}",
                    canvas_paint_code(paint, "&__path", env, program)?
                )
                .unwrap();
            }
            ResolvedCanvasCommand::Line {
                x1,
                y1,
                x2,
                y2,
                stroke,
                ..
            } => {
                let from = canvas_point_code(*x1, *y1, env, program)?;
                let to = canvas_point_code(*x2, *y2, env, program)?;
                write!(
                    code,
                    " {{ let __path = ::iced::widget::canvas::Path::line({from}, {to}); __frame.stroke(&__path, {}); }}",
                    canvas_stroke_code(stroke, env, program)?
                )
                .unwrap();
            }
            ResolvedCanvasCommand::Text {
                value,
                x,
                y,
                max_width,
                color,
                size,
                line_height,
                font,
                align_x,
                align_y,
                shaping,
                value_type,
                ..
            } => {
                let value = resolved_expr_use_code(program, *value, env, ValueMode::Owned)?;
                let content = if *value_type == Type::Str {
                    value
                } else {
                    format!("::std::format!(\"{{}}\", {value})")
                };
                let position = canvas_point_code(*x, *y, env, program)?;
                let max_width = max_width
                    .as_ref()
                    .map(|value| canvas_clamped_f32_code(*value, "0.0", "f32::MAX", env, program))
                    .transpose()?
                    .unwrap_or_else(|| "f32::INFINITY".into());
                let color = resolved_theme_color(color);
                let size = size
                    .as_ref()
                    .map(|value| {
                        canvas_clamped_f32_code(*value, "f32::EPSILON", "f32::MAX", env, program)
                    })
                    .transpose()?
                    .unwrap_or_else(|| "16.0".into());
                let line_height = match line_height {
                    Some(ResolvedCanvasLineHeight::Relative(value)) => format!(
                        "::iced::widget::text::LineHeight::Relative({})",
                        canvas_clamped_f32_code(*value, "f32::EPSILON", "f32::MAX", env, program)?
                    ),
                    Some(ResolvedCanvasLineHeight::Absolute(value)) => format!(
                        "::iced::widget::text::LineHeight::Absolute(::iced::Pixels({}))",
                        canvas_clamped_f32_code(*value, "f32::EPSILON", "f32::MAX", env, program)?
                    ),
                    None => "::iced::widget::text::LineHeight::default()".into(),
                };
                let font = font
                    .as_ref()
                    .map(|font| match font {
                        ResolvedCanvasFont::Default => "::iced::Font::DEFAULT".into(),
                        ResolvedCanvasFont::Monospace => "::iced::Font::MONOSPACE".into(),
                        ResolvedCanvasFont::Custom(font) => resolved_default_font_code(font),
                    })
                    .unwrap_or_else(|| "::iced::Font::DEFAULT".into());
                let align_x = align_x.map_or("Default", |value| text_alignment_code(value));
                let align_y = match align_y {
                    None | Some(VerticalAlignment::Top) => "Top",
                    Some(VerticalAlignment::Center) => "Center",
                    Some(VerticalAlignment::Bottom) => "Bottom",
                };
                let shaping = shaping.map_or("Auto", text_shaping_code);
                write!(
                    code,
                    " __frame.fill_text(::iced::widget::canvas::Text {{ content: {content}, position: {position}, max_width: {max_width}, color: {color}, size: ::iced::Pixels({size}), line_height: {line_height}, font: {font}, align_x: ::iced::widget::text::Alignment::{align_x}, align_y: ::iced::alignment::Vertical::{align_y}, shaping: ::iced::widget::text::Shaping::{shaping} }});"
                )
                .unwrap();
            }
            ResolvedCanvasCommand::Image {
                source,
                x,
                y,
                width,
                height,
                filter,
                rotation,
                opacity,
                snap,
                radius,
                source_type,
                ..
            } => {
                let source = resolved_expr_use_code(program, *source, env, ValueMode::Owned)?;
                let handle = if *source_type == Type::Str {
                    format!("::iced::widget::image::Handle::from_path({source})")
                } else {
                    source
                };
                let filter = match filter {
                    ImageFilter::Linear => "Linear",
                    ImageFilter::Nearest => "Nearest",
                };
                write!(
                    code,
                    " __frame.draw_image(::iced::Rectangle::new({}, {}), ::iced::widget::canvas::Image {{ handle: {handle}, filter_method: ::iced::widget::image::FilterMethod::{filter}, rotation: ::iced::Radians({} as f32), border_radius: {}, opacity: {}, snap: {} }});",
                    canvas_point_code(*x, *y, env, program)?,
                    canvas_size_code(*width, *height, env, program)?,
                    canvas_expr_code(*rotation, env, program)?,
                    canvas_radius_code(radius, env, program)?,
                    canvas_clamped_f32_code(*opacity, "0.0", "1.0", env, program)?,
                    canvas_expr_code(*snap, env, program)?
                )
                .unwrap();
            }
            ResolvedCanvasCommand::Svg {
                source,
                memory,
                x,
                y,
                width,
                height,
                color,
                rotation,
                opacity,
                source_type,
                ..
            } => {
                let source = resolved_expr_use_code(program, *source, env, ValueMode::Owned)?;
                let handle = if *memory && *source_type == Type::Bytes {
                    format!("::iced::advanced::svg::Handle::from_memory({source})")
                } else if *memory {
                    format!("::iced::advanced::svg::Handle::from_memory(({source}).into_bytes())")
                } else {
                    format!("::iced::advanced::svg::Handle::from_path({source})")
                };
                let color = color.as_ref().map_or_else(
                    || "::std::option::Option::None".into(),
                    |color| {
                        format!(
                            "::std::option::Option::Some({})",
                            resolved_theme_color(color)
                        )
                    },
                );
                write!(
                    code,
                    " __frame.draw_svg(::iced::Rectangle::new({}, {}), ::iced::advanced::svg::Svg {{ handle: {handle}, color: {color}, rotation: ::iced::Radians({} as f32), opacity: {} }});",
                    canvas_point_code(*x, *y, env, program)?,
                    canvas_size_code(*width, *height, env, program)?,
                    canvas_expr_code(*rotation, env, program)?,
                    canvas_clamped_f32_code(*opacity, "0.0", "1.0", env, program)?
                )
                .unwrap();
            }
            ResolvedCanvasCommand::Path {
                segments, paint, ..
            } => {
                let path = canvas_path_code(segments, env, program)?;
                write!(
                    code,
                    " {{ let __path = {path}; {} }}",
                    canvas_paint_code(paint, "&__path", env, program)?
                )
                .unwrap();
            }
            ResolvedCanvasCommand::Group {
                transform,
                commands,
                ..
            } => {
                let inner = canvas_commands_code(commands, env, program)?;
                let mut body = String::new();
                if transform.x.is_some() || transform.y.is_some() {
                    let x = transform
                        .x
                        .as_ref()
                        .map(|value| canvas_expr_code(*value, env, program))
                        .transpose()?
                        .unwrap_or_else(|| "0.0".into());
                    let y = transform
                        .y
                        .as_ref()
                        .map(|value| canvas_expr_code(*value, env, program))
                        .transpose()?
                        .unwrap_or_else(|| "0.0".into());
                    write!(
                        body,
                        " __frame.translate(::iced::Vector::new({x} as f32, {y} as f32));"
                    )
                    .unwrap();
                }
                if let Some(value) = &transform.rotate {
                    write!(
                        body,
                        " __frame.rotate({} as f32);",
                        canvas_expr_code(*value, env, program)?
                    )
                    .unwrap();
                }
                if let Some(value) = &transform.scale {
                    write!(
                        body,
                        " __frame.scale({});",
                        canvas_clamped_f32_code(*value, "f32::EPSILON", "f32::MAX", env, program)?
                    )
                    .unwrap();
                }
                if transform.scale_x.is_some() || transform.scale_y.is_some() {
                    let x = transform
                        .scale_x
                        .as_ref()
                        .map(|value| {
                            canvas_clamped_f32_code(
                                *value,
                                "f32::EPSILON",
                                "f32::MAX",
                                env,
                                program,
                            )
                        })
                        .transpose()?
                        .unwrap_or_else(|| "1.0".into());
                    let y = transform
                        .scale_y
                        .as_ref()
                        .map(|value| {
                            canvas_clamped_f32_code(
                                *value,
                                "f32::EPSILON",
                                "f32::MAX",
                                env,
                                program,
                            )
                        })
                        .transpose()?
                        .unwrap_or_else(|| "1.0".into());
                    write!(
                        body,
                        " __frame.scale_nonuniform(::iced::Vector::new({x}, {y}));"
                    )
                    .unwrap();
                }
                if let Some([x, y, width, height]) = &transform.clip {
                    let point = canvas_point_code(*x, *y, env, program)?;
                    let size = canvas_size_code(*width, *height, env, program)?;
                    write!(
                        body,
                        " __frame.with_clip(::iced::Rectangle {{ x: {point}.x, y: {point}.y, width: {size}.width, height: {size}.height }}, |__frame| {{ {inner} }});"
                    )
                    .unwrap();
                } else {
                    body.push_str(&inner);
                }
                write!(code, " __frame.with_save(|__frame| {{ {body} }});").unwrap();
            }
            ResolvedCanvasCommand::If {
                condition,
                commands,
                ..
            } => {
                let condition = resolved_expr_use_code(program, *condition, env, ValueMode::Owned)?;
                write!(
                    code,
                    " if {condition} {{ {} }}",
                    canvas_commands_code(commands, env, program)?
                )
                .unwrap();
            }
            ResolvedCanvasCommand::For {
                item,
                items,
                commands,
                ..
            } => {
                let items = resolved_expr_use_code(program, *items, env, ValueMode::Borrowed)?;
                let mut child_env = ScopedBindingEnv::new(env);
                child_env.insert(
                    item.name.clone(),
                    Binding {
                        code: item.name.clone(),
                        ty: item.ty.clone(),
                        local: false,
                        state: None,
                        owner: Some(BindingOwner::Local(item.local)),
                    },
                );
                write!(
                    code,
                    " for {} in {items}.iter() {{ {} }}",
                    item.name,
                    canvas_commands_code(commands, &child_env, program)?
                )
                .unwrap();
            }
        }
    }
    Ok(code)
}
