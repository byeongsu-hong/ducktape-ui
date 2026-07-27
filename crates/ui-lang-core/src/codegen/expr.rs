use super::*;
use crate::unqualified_name;

pub(in crate::codegen) fn expr_list_code(
    values: &[Expr],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(values
        .iter()
        .map(|value| expr_code(value, env, document, ValueMode::Owned))
        .collect::<Result<Vec<_>, _>>()?
        .join(", "))
}

pub(in crate::codegen) fn expr_args_suffix_code(
    values: &[Expr],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let args = expr_list_code(values, env, document)?;
    Ok(if args.is_empty() {
        args
    } else {
        format!(", {args}")
    })
}

pub(in crate::codegen) fn expr_code(
    expr: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
    mode: ValueMode,
) -> Result<String, Error> {
    Ok(match expr {
        Expr::Bool(value) => value.to_string(),
        Expr::I64(value) => value.to_string(),
        Expr::F64(value) => rust_f64(*value),
        Expr::Str(value) => match mode {
            ValueMode::Owned => format!("{}.to_owned()", rust_string(value)),
            ValueMode::Borrowed => rust_string(value),
        },
        Expr::Bytes(values) => format!(
            "::std::vec![{}]",
            values
                .iter()
                .map(|value| format!("0x{value:02x}u8"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::EmptyList => "::std::vec::Vec::new()".into(),
        Expr::List(values) => format!("::std::vec![{}]", expr_list_code(values, env, document)?),
        Expr::None => "::std::option::Option::None".into(),
        Expr::Path(path) => {
            if let [enum_name, variant_name] = path.as_slice()
                && document.enums.iter().any(|item| {
                    item.name == *enum_name
                        && item.variants.iter().any(|variant| {
                            variant.name == *variant_name && variant.payload.is_none()
                        })
                })
            {
                return Ok(format!(
                    "{}::{}",
                    generated_named_rust(enum_name),
                    pascal(variant_name)
                ));
            }
            let binding = env.get(&path[0]).ok_or_else(|| {
                Error::new(
                    "E150",
                    &Span::line(1),
                    format!("unknown value `{}`", path[0]),
                )
            })?;
            let mut code = binding.code.clone();
            let mut ty = binding.ty.clone();
            let mut owned_projection = false;
            for field in &path[1..] {
                if let Some((projection, projected_ty)) = native_field_projection(&ty, field, &code)
                {
                    code = projection;
                    ty = projected_ty;
                    owned_projection = true;
                    continue;
                }
                if let Type::Option(inner) = &ty
                    && **inner == Type::WidgetTarget
                {
                    code = format!("({code}).as_ref().map(|value| value.{field}.clone())");
                    ty = Type::Option(Box::new(
                        widget_target_field_type(field).unwrap_or(Type::Unknown),
                    ));
                    owned_projection = true;
                    continue;
                }
                write!(code, ".{field}").unwrap();
                if let Type::Named(name) = &ty {
                    ty = document
                        .structs
                        .iter()
                        .find(|item| item.name == *name)
                        .and_then(|item| item.fields.iter().find(|(name, _)| name == field))
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or(Type::Unknown);
                } else if ty == Type::WidgetTarget {
                    ty = widget_target_field_type(field).unwrap_or(Type::Unknown);
                } else if let Some(field_ty) = native_field_type(&ty, field) {
                    ty = field_ty;
                }
            }
            let clone_unnecessary = matches!(
                ty,
                Type::Bool
                    | Type::I64
                    | Type::F64
                    | Type::PhysicalKey
                    | Type::KeyLocation
                    | Type::KeyModifiers
                    | Type::Pixels
                    | Type::Padding
                    | Type::Degrees
                    | Type::Radians
                    | Type::Rotation
                    | Type::ContentFit
                    | Type::Color
                    | Type::Background
                    | Type::Gradient
                    | Type::LinearGradient
                    | Type::ColorStop
                    | Type::Font
                    | Type::FontFamily
                    | Type::FontWeight
                    | Type::FontStretch
                    | Type::FontStyle
                    | Type::ThemeMode
                    | Type::TextAlignment
                    | Type::TextShaping
                    | Type::TextWrapping
                    | Type::TextLineHeight
                    | Type::MouseInteraction
                    | Type::ScrollDelta
                    | Type::EventStatus
                    | Type::Length
                    | Type::Alignment
                    | Type::HorizontalAlignment
                    | Type::VerticalAlignment
                    | Type::Border
                    | Type::Radius
                    | Type::Shadow
                    | Type::Point
                    | Type::PointU32
                    | Type::Vector
                    | Type::Size
                    | Type::SizeU32
                    | Type::Rectangle
                    | Type::RectangleU32
                    | Type::Transformation
                    | Type::MouseButton
                    | Type::MouseCursor
                    | Type::MouseClick
                    | Type::TouchFinger
                    | Type::WindowId
                    | Type::WindowPosition
                    | Type::RedrawRequest
                    | Type::WindowDirection
                    | Type::WindowLevel
                    | Type::WindowMode
                    | Type::WindowAttention
                    | Type::Unit
            ) || (binding.local && path.len() == 1)
                || owned_projection;
            if matches!(mode, ValueMode::Owned) && !clone_unnecessary {
                code.push_str(".clone()");
            }
            code
        }
        Expr::Call { name, args } => {
            if let Some((enum_name, variant_name)) = name.split_once('.')
                && document.enums.iter().any(|item| {
                    item.name == enum_name
                        && item.variants.iter().any(|variant| {
                            variant.name == variant_name && variant.payload.is_some()
                        })
                })
            {
                return Ok(format!(
                    "{}::{}({})",
                    generated_named_rust(enum_name),
                    pascal(variant_name),
                    expr_code(&args[0], env, document, ValueMode::Owned)?
                ));
            }
            if let Some(function) = find_extern_function(document, name, ExternKind::Sync) {
                let args = expr_list_code(args, env, document)?;
                return Ok(format!("{}({args})", function.rust_path));
            }
            let name = unqualified_name(name);
            match name {
            "color.default" => "::iced::Color::default()".into(),
            "color.black" => "::iced::Color::BLACK".into(),
            "color.white" => "::iced::Color::WHITE".into(),
            "color.transparent" => "::iced::Color::TRANSPARENT".into(),
            "color.rgb" => format!(
                "::iced::Color::from_rgb({}, {}, {})",
                unit_f32_code(&args[0], env, document)?,
                unit_f32_code(&args[1], env, document)?,
                unit_f32_code(&args[2], env, document)?
            ),
            "color.rgba" => format!(
                "::iced::Color::from_rgba({}, {}, {}, {})",
                unit_f32_code(&args[0], env, document)?,
                unit_f32_code(&args[1], env, document)?,
                unit_f32_code(&args[2], env, document)?,
                unit_f32_code(&args[3], env, document)?
            ),
            "color.rgb8" => {
                let [Expr::I64(r), Expr::I64(g), Expr::I64(b)] = args.as_slice() else {
                    unreachable!("checker requires literal u8 channels")
                };
                format!("::iced::Color::from_rgb8({r}u8, {g}u8, {b}u8)")
            }
            "color.rgba8" => {
                let [Expr::I64(r), Expr::I64(g), Expr::I64(b), alpha] = args.as_slice() else {
                    unreachable!("checker requires literal u8 channels")
                };
                format!(
                    "::iced::Color::from_rgba8({r}u8, {g}u8, {b}u8, {})",
                    unit_f32_code(alpha, env, document)?
                )
            }
            "color.try_rgb8" | "color.try_rgba8" => {
                let red = expr_code(&args[0], env, document, ValueMode::Owned)?;
                let green = expr_code(&args[1], env, document, ValueMode::Owned)?;
                let blue = expr_code(&args[2], env, document, ValueMode::Owned)?;
                let constructor = if name == "color.try_rgb8" {
                    "::iced::Color::from_rgb8(__red, __green, __blue)"
                } else {
                    "::iced::Color::from_rgba8(__red, __green, __blue, __alpha as f32)"
                };
                if name == "color.try_rgb8" {
                    format!(
                        "match (<u8>::try_from({red}), <u8>::try_from({green}), <u8>::try_from({blue})) {{ (::std::result::Result::Ok(__red), ::std::result::Result::Ok(__green), ::std::result::Result::Ok(__blue)) => ::std::option::Option::Some({constructor}), _ => ::std::option::Option::None }}"
                    )
                } else {
                    let alpha = expr_code(&args[3], env, document, ValueMode::Owned)?;
                    format!(
                        "{{ let __alpha = {alpha}; match (<u8>::try_from({red}), <u8>::try_from({green}), <u8>::try_from({blue})) {{ (::std::result::Result::Ok(__red), ::std::result::Result::Ok(__green), ::std::result::Result::Ok(__blue)) if (0.0..=1.0).contains(&__alpha) => ::std::option::Option::Some({constructor}), _ => ::std::option::Option::None }} }}"
                    )
                }
            }
            "color.linear_rgba" => format!(
                "::iced::Color::from_linear_rgba({}, {}, {}, {})",
                unit_f32_code(&args[0], env, document)?,
                unit_f32_code(&args[1], env, document)?,
                unit_f32_code(&args[2], env, document)?,
                unit_f32_code(&args[3], env, document)?
            ),
            "color.from3" => format!(
                "::iced::Color::from([{}, {}, {}])",
                unit_f32_code(&args[0], env, document)?,
                unit_f32_code(&args[1], env, document)?,
                unit_f32_code(&args[2], env, document)?
            ),
            "color.from4" => format!(
                "::iced::Color::from([{}, {}, {}, {}])",
                unit_f32_code(&args[0], env, document)?,
                unit_f32_code(&args[1], env, document)?,
                unit_f32_code(&args[2], env, document)?,
                unit_f32_code(&args[3], env, document)?
            ),
            "color.parse" => format!(
                "({}).parse::<::iced::Color>().ok()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "color.inverse" => format!(
                "({}).inverse()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "color.invert" => format!(
                "({}).inverse()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "color.scale_alpha" => format!(
                "({}).scale_alpha(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "color.luminance" => format!(
                "({}).relative_luminance() as f64",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "color.contrast" => format!(
                "({}).relative_contrast({}) as f64",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "color.readable" => format!(
                "({}).is_readable_on({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "color_stop.default" => "::iced::gradient::ColorStop::default()".into(),
            "color_stop" => format!(
                "::iced::gradient::ColorStop {{ offset: ({}) as f32, color: {} }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "linear" => format!(
                "::iced::gradient::Linear::new({})",
                radians_value_code(&args[0], env, document)?
            ),
            "linear.add_stop" => format!(
                "::ui_lang_runtime::add_gradient_stops({}, [::iced::gradient::ColorStop {{ offset: ({}) as f32, color: {} }}])",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "linear.add_stops" => format!(
                "::ui_lang_runtime::add_gradient_stops({}, {})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "linear.scale_alpha" => format!(
                "({}).scale_alpha(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "gradient.linear" => format!(
                "::iced::Gradient::Linear({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "gradient.from_linear" => format!(
                "::iced::Gradient::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "gradient.scale_alpha" => format!(
                "({}).scale_alpha(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "background.color" => format!(
                "::iced::Background::Color({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "background.gradient" => format!(
                "::iced::Background::Gradient({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "background.from_color" => format!(
                "::iced::Background::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "background.from_gradient" => format!(
                "::iced::Background::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "background.from_linear" => format!(
                "::iced::Background::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "background.scale_alpha" => format!(
                "({}).scale_alpha(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "font.default" => "::iced::Font::default()".into(),
            "font.sans" => "::iced::Font::DEFAULT".into(),
            "font.monospace" => "::iced::Font::MONOSPACE".into(),
            "font.with_name" => {
                let Expr::Str(name) = &args[0] else {
                    unreachable!("checker requires a font name literal")
                };
                format!("::iced::Font::with_name({})", rust_string(name))
            }
            "font.new" => format!(
                "::iced::Font {{ family: {}, weight: {}, stretch: {}, style: {} }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?,
                expr_code(&args[3], env, document, ValueMode::Owned)?
            ),
            "family.default" => "::iced::font::Family::default()".into(),
            "family.named" => {
                let Expr::Str(name) = &args[0] else {
                    unreachable!("checker requires a family name literal")
                };
                format!("::iced::font::Family::Name({})", rust_string(name))
            }
            "family.serif"
            | "family.sans_serif"
            | "family.cursive"
            | "family.fantasy"
            | "family.monospace" => format!(
                "::iced::font::Family::{}",
                pascal(name.strip_prefix("family.").expect("checked prefix"))
            ),
            "weight.default" => "::iced::font::Weight::default()".into(),
            "weight.thin"
            | "weight.extra_light"
            | "weight.light"
            | "weight.normal"
            | "weight.medium"
            | "weight.semibold"
            | "weight.bold"
            | "weight.extra_bold"
            | "weight.black" => format!(
                "::iced::font::Weight::{}",
                pascal(name.strip_prefix("weight.").expect("checked prefix"))
            ),
            "stretch.default" => "::iced::font::Stretch::default()".into(),
            "stretch.ultra_condensed"
            | "stretch.extra_condensed"
            | "stretch.condensed"
            | "stretch.semi_condensed"
            | "stretch.normal"
            | "stretch.semi_expanded"
            | "stretch.expanded"
            | "stretch.extra_expanded"
            | "stretch.ultra_expanded" => format!(
                "::iced::font::Stretch::{}",
                pascal(name.strip_prefix("stretch.").expect("checked prefix"))
            ),
            "font_style.default" => "::iced::font::Style::default()".into(),
            "font_style.normal" | "font_style.italic" | "font_style.oblique" => format!(
                "::iced::font::Style::{}",
                pascal(name.strip_prefix("font_style.").expect("checked prefix"))
            ),
            "theme_mode.default" => "::iced::theme::Mode::default()".into(),
            "theme_mode.none" => "::iced::theme::Mode::None".into(),
            "theme_mode.light" => "::iced::theme::Mode::Light".into(),
            "theme_mode.dark" => "::iced::theme::Mode::Dark".into(),
            "text_alignment.default" => "::iced::widget::text::Alignment::default()".into(),
            "text_alignment.left"
            | "text_alignment.center"
            | "text_alignment.right"
            | "text_alignment.justified" => format!(
                "::iced::widget::text::Alignment::{}",
                pascal(
                    name.strip_prefix("text_alignment.")
                        .expect("checked prefix")
                )
            ),
            "text_alignment.from_horizontal" | "text_alignment.from_alignment" => format!(
                "::iced::widget::text::Alignment::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "horizontal.from_text_alignment" => format!(
                "::iced::alignment::Horizontal::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "text_shaping.default" => "::iced::widget::text::Shaping::default()".into(),
            "text_shaping.auto" | "text_shaping.basic" | "text_shaping.advanced" => format!(
                "::iced::widget::text::Shaping::{}",
                pascal(
                    name.strip_prefix("text_shaping.")
                        .expect("checked prefix")
                )
            ),
            "text_wrapping.default" => "::iced::widget::text::Wrapping::default()".into(),
            "text_wrapping.none"
            | "text_wrapping.word"
            | "text_wrapping.glyph"
            | "text_wrapping.word_or_glyph" => format!(
                "::iced::widget::text::Wrapping::{}",
                pascal(
                    name.strip_prefix("text_wrapping.")
                        .expect("checked prefix")
                )
            ),
            "line_height.default" => "::iced::widget::text::LineHeight::default()".into(),
            "line_height.relative" => format!(
                "::iced::widget::text::LineHeight::Relative(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "line_height.absolute" => format!(
                "::iced::widget::text::LineHeight::Absolute({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "line_height.from_f64" => format!(
                "::iced::widget::text::LineHeight::from(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "line_height.from_pixels" => format!(
                "::iced::widget::text::LineHeight::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "line_height.to_absolute" => format!(
                "({}).to_absolute({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "screenshot.new" => format!(
                "::iced::window::Screenshot::new({}, {}, ({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "screenshot.crop" => format!(
                "::ui_lang_runtime::crop_screenshot(&({}), {}).ok()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "screenshot.crop_error" => format!(
                "match ::ui_lang_runtime::crop_screenshot(&({}), {}) {{ ::std::result::Result::Ok(_) => ::std::option::Option::None, ::std::result::Result::Err(::iced::window::screenshot::CropError::Zero) => ::std::option::Option::Some(\"zero\".to_owned()), ::std::result::Result::Err(::iced::window::screenshot::CropError::OutOfBounds) => ::std::option::Option::Some(\"out-of-bounds\".to_owned()) }}",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "screenshot.crop_error_message" => format!(
                "::ui_lang_runtime::crop_screenshot(&({}), {}).err().map(|error| error.to_string())",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "screenshot.as_bytes" => format!(
                "::std::convert::AsRef::<[u8]>::as_ref(&({})).to_vec()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "screenshot.into_bytes" => format!(
                "({}).rgba.to_vec()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "interaction.default" => "::iced::mouse::Interaction::default()".into(),
            "interaction.none"
            | "interaction.hidden"
            | "interaction.idle"
            | "interaction.context_menu"
            | "interaction.help"
            | "interaction.pointer"
            | "interaction.progress"
            | "interaction.wait"
            | "interaction.cell"
            | "interaction.crosshair"
            | "interaction.text"
            | "interaction.alias"
            | "interaction.copy"
            | "interaction.move"
            | "interaction.no_drop"
            | "interaction.not_allowed"
            | "interaction.grab"
            | "interaction.grabbing"
            | "interaction.resize_horizontal"
            | "interaction.resize_vertical"
            | "interaction.resize_diagonal_up"
            | "interaction.resize_diagonal_down"
            | "interaction.resize_column"
            | "interaction.resize_row"
            | "interaction.all_scroll"
            | "interaction.zoom_in"
            | "interaction.zoom_out" => format!(
                "::iced::mouse::Interaction::{}",
                first_class_mouse_interaction_code(name)
            ),
            "scroll.lines" | "scroll.pixels" => format!(
                "::iced::mouse::ScrollDelta::{} {{ x: ({}) as f32, y: ({}) as f32 }}",
                if name == "scroll.lines" {
                    "Lines"
                } else {
                    "Pixels"
                },
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "event_status.ignored" => "::iced::event::Status::Ignored".into(),
            "event_status.captured" => "::iced::event::Status::Captured".into(),
            "event_status.merge" => format!(
                "({}).merge({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "redraw_request.next_frame" => "::iced::window::RedrawRequest::NextFrame".into(),
            "redraw_request.wait" => "::iced::window::RedrawRequest::Wait".into(),
            "redraw_request.at" => format!(
                "::iced::window::RedrawRequest::At({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "window_id.unique" => "::iced::window::Id::unique()".into(),
            "window_direction.north"
            | "window_direction.south"
            | "window_direction.east"
            | "window_direction.west"
            | "window_direction.north_east"
            | "window_direction.north_west"
            | "window_direction.south_east"
            | "window_direction.south_west" => format!(
                "::iced::window::Direction::{}",
                pascal(
                    name.strip_prefix("window_direction.")
                        .expect("checked prefix")
                )
            ),
            "window_level.default" => "::iced::window::Level::default()".into(),
            "window_level.normal"
            | "window_level.always_on_bottom"
            | "window_level.always_on_top" => format!(
                "::iced::window::Level::{}",
                pascal(
                    name.strip_prefix("window_level.")
                        .expect("checked prefix")
                )
            ),
            "window_mode.windowed" | "window_mode.fullscreen" | "window_mode.hidden" => {
                format!(
                    "::iced::window::Mode::{}",
                    pascal(
                        name.strip_prefix("window_mode.")
                            .expect("checked prefix")
                    )
                )
            }
            "window_attention.critical" | "window_attention.informational" => format!(
                "::iced::window::UserAttention::{}",
                pascal(
                    name.strip_prefix("window_attention.")
                        .expect("checked prefix")
                )
            ),
            "window_position.default" => "::iced::window::Position::default()".into(),
            "window_position.centered" => "::iced::window::Position::Centered".into(),
            "window_position.specific" => format!(
                "::iced::window::Position::Specific({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "length.fill" => "::iced::Length::Fill".into(),
            "length.shrink" => "::iced::Length::Shrink".into(),
            "length.fill_portion" => {
                let Expr::I64(value) = &args[0] else {
                    unreachable!("checker requires a u16 literal")
                };
                format!("::iced::Length::FillPortion({value}u16)")
            }
            "length.try_fill_portion" => format!(
                "<u16>::try_from({}).ok().map(::iced::Length::FillPortion)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "length.fixed" => format!(
                "::iced::Length::Fixed(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "length.from_f64" => format!(
                "::iced::Length::from(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "length.from_pixels" => format!(
                "::iced::Length::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "length.from_u32" => {
                let Expr::I64(value) = &args[0] else {
                    unreachable!("checker requires a u32 literal")
                };
                format!("::iced::Length::from({value}u32)")
            }
            "length.try_from_u32" => format!(
                "<u32>::try_from({}).ok().map(::iced::Length::from)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "length.fluid" => format!(
                "({}).fluid()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "length.enclose" => format!(
                "({}).enclose({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "alignment.start" => "::iced::Alignment::Start".into(),
            "alignment.center" => "::iced::Alignment::Center".into(),
            "alignment.end" => "::iced::Alignment::End".into(),
            "horizontal.left" => "::iced::alignment::Horizontal::Left".into(),
            "horizontal.center" => "::iced::alignment::Horizontal::Center".into(),
            "horizontal.right" => "::iced::alignment::Horizontal::Right".into(),
            "vertical.top" => "::iced::alignment::Vertical::Top".into(),
            "vertical.center" => "::iced::alignment::Vertical::Center".into(),
            "vertical.bottom" => "::iced::alignment::Vertical::Bottom".into(),
            "alignment.from_horizontal" | "alignment.from_vertical" => format!(
                "::iced::Alignment::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "horizontal.from_alignment" => format!(
                "::iced::alignment::Horizontal::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "vertical.from_alignment" => format!(
                "::iced::alignment::Vertical::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "border.default" => "::iced::Border::default()".into(),
            "border.new" => format!(
                "::iced::Border {{ color: {}, width: {}, radius: {} }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                pixel_scalar_code(&args[1], env, document)?,
                radius_value_code(&args[2], env, document)?
            ),
            "border.color" => format!(
                "::iced::border::color({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "border.width" => format!(
                "::iced::border::width({})",
                pixel_value_code(&args[0], env, document)?
            ),
            "border.rounded" => format!(
                "::iced::border::rounded({})",
                radius_value_code(&args[0], env, document)?
            ),
            "border.with_color" => format!(
                "({}).color({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "border.with_width" => format!(
                "({}).width({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                pixel_value_code(&args[1], env, document)?
            ),
            "border.with_radius" => format!(
                "({}).rounded({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                radius_value_code(&args[1], env, document)?
            ),
            "radius" => format!(
                "::iced::border::radius({})",
                pixel_value_code(&args[0], env, document)?
            ),
            "radius.new" => format!(
                "::iced::border::Radius::new({})",
                pixel_value_code(&args[0], env, document)?
            ),
            "radius.default" => "::iced::border::Radius::default()".into(),
            "radius.top_left"
            | "radius.top_right"
            | "radius.bottom_right"
            | "radius.bottom_left"
            | "radius.top"
            | "radius.bottom"
            | "radius.left"
            | "radius.right" => {
                let function = name.strip_prefix("radius.").expect("checked prefix");
                format!(
                    "::iced::border::{function}({})",
                    pixel_value_code(&args[0], env, document)?
                )
            }
            "radius.with_top_left"
            | "radius.with_top_right"
            | "radius.with_bottom_right"
            | "radius.with_bottom_left"
            | "radius.with_top"
            | "radius.with_bottom"
            | "radius.with_left"
            | "radius.with_right" => {
                let method = name.strip_prefix("radius.with_").expect("checked prefix");
                format!(
                    "({}).{method}({})",
                    expr_code(&args[0], env, document, ValueMode::Owned)?,
                    pixel_value_code(&args[1], env, document)?
                )
            }
            "radius.from_f64" => format!(
                "::iced::border::Radius::from(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "radius.from_u8" | "radius.from_u32" => {
                let Expr::I64(value) = &args[0] else {
                    unreachable!("checker requires a radius integer literal")
                };
                let ty = name.strip_prefix("radius.from_").expect("checked prefix");
                format!("::iced::border::Radius::from({value}{ty})")
            }
            "radius.from_i32" => format!(
                "::iced::border::Radius::from(({}) as i32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "radius.try_from_u8" | "radius.try_from_u32" | "radius.try_from_i32" => {
                let ty = name
                    .strip_prefix("radius.try_from_")
                    .expect("checked prefix");
                format!(
                    "<{ty}>::try_from(({}) as i64).ok().map(::iced::border::Radius::from)",
                    expr_code(&args[0], env, document, ValueMode::Owned)?
                )
            }
            "shadow.default" => "::iced::Shadow::default()".into(),
            "shadow.new" => format!(
                "::iced::Shadow {{ color: {}, offset: {}, blur_radius: ({}) as f32 }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "fit.default" => "::iced::ContentFit::default()".into(),
            "fit.contain" => "::iced::ContentFit::Contain".into(),
            "fit.cover" => "::iced::ContentFit::Cover".into(),
            "fit.fill" => "::iced::ContentFit::Fill".into(),
            "fit.none" => "::iced::ContentFit::None".into(),
            "fit.scale_down" => "::iced::ContentFit::ScaleDown".into(),
            "fit.apply" => format!(
                "({}).fit({}, {})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "rotation.default" => "::iced::Rotation::default()".into(),
            "rotation.floating" => format!(
                "::iced::Rotation::Floating({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rotation.solid" => format!(
                "::iced::Rotation::Solid({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rotation.from" => format!(
                "::iced::Rotation::from({} as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rotation.with_radians" => format!(
                "{{ let mut __rotation = {}; *__rotation.radians_mut() = {}; __rotation }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rotation.apply" => format!(
                "({}).apply({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "debug.active" => format!(
                "({}).is_some()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "debug.time_with" => format!(
                "::iced::debug::time_with({}, || {})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, mode)?
            ),
            "image.downgrade" => format!(
                "({}).downgrade()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "image.upgrade" => format!(
                "::iced::widget::image::Allocation::upgrade(&({}))",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "animation.value" => {
                let animation = expr_code(&args[0], env, document, ValueMode::Borrowed)?;
                if expr_type(&args[0], &env_types(env), document, &Span::line(1))?
                    == Type::Animation(Box::new(Type::F64))
                {
                    format!("({animation}).value() as f64")
                } else {
                    format!("({animation}).value()")
                }
            }
            "animation.animating" => format!(
                "({}).is_animating({})",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?,
                animation_at_code(args, 1, env, document)?
            ),
            "animation.interpolate" => {
                let animation = expr_code(&args[0], env, document, ValueMode::Borrowed)?;
                let start = expr_code(&args[1], env, document, ValueMode::Owned)?;
                let end = expr_code(&args[2], env, document, ValueMode::Owned)?;
                let at = animation_at_code(args, 3, env, document)?;
                if expr_type(&args[1], &env_types(env), document, &Span::line(1))? == Type::F64 {
                    format!(
                        "({animation}).interpolate(({start}) as f32, ({end}) as f32, {at}) as f64"
                    )
                } else {
                    let start = if matches!(args[1], Expr::None) {
                        "::std::option::Option::<f32>::None".into()
                    } else {
                        format!("({start}).map(|__value| __value as f32)")
                    };
                    let end = if matches!(args[2], Expr::None) {
                        "::std::option::Option::<f32>::None".into()
                    } else {
                        format!("({end}).map(|__value| __value as f32)")
                    };
                    format!(
                        "({animation}).interpolate({start}, {end}, {at}).map(|__value| __value as f64)"
                    )
                }
            }
            "animation.remaining" => format!(
                "::ui_lang_runtime::animation_remaining_millis(&({}), {})",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?,
                animation_at_code(args, 1, env, document)?
            ),
            "animation.project" => {
                let animation = expr_code(&args[0], env, document, ValueMode::Borrowed)?;
                let Type::Animation(inner) =
                    expr_type(&args[0], &env_types(env), document, &Span::line(1))?
                else {
                    unreachable!("checker requires animation")
                };
                let Expr::Path(binding) = &args[1] else {
                    unreachable!("checker requires projection binding")
                };
                let mut projection_env = env.clone();
                projection_env.insert(
                    binding[0].clone(),
                    Binding {
                        code: if *inner == Type::F64 {
                            "(__value as f64)".into()
                        } else {
                            "__value".into()
                        },
                        ty: *inner,
                        local: true,
                        state: None,
                    },
                );
                let projection = expr_code(&args[2], &projection_env, document, ValueMode::Owned)?;
                let output = expr_type(
                    &args[2],
                    &env_types(&projection_env),
                    document,
                    &Span::line(1),
                )?;
                let at = animation_at_code(args, 3, env, document)?;
                if output == Type::F64 {
                    format!(
                        "({animation}).interpolate_with(|__value| ({projection}) as f32, {at}) as f64"
                    )
                } else {
                    format!(
                        "({animation}).interpolate_with(|__value| ({projection}).map(|__value| __value as f32), {at}).map(|__value| __value as f64)"
                    )
                }
            }
            "pixels" => format!(
                "::iced::Pixels(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "pixels.zero" => "::iced::Pixels::ZERO".into(),
            "pixels.from_u32" => {
                let Expr::I64(value) = &args[0] else {
                    unreachable!("checker requires a pixels u32 literal")
                };
                format!("::iced::Pixels::from({value}u32)")
            }
            "pixels.try_from_u32" => format!(
                "<u32>::try_from({}).ok().map(::iced::Pixels::from)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "padding" => format!(
                "::iced::Padding {{ top: ({}) as f32, right: ({}) as f32, bottom: ({}) as f32, left: ({}) as f32 }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?,
                expr_code(&args[3], env, document, ValueMode::Owned)?
            ),
            "padding.zero" => "::iced::Padding::ZERO".into(),
            "padding.all"
            | "padding.top"
            | "padding.right"
            | "padding.bottom"
            | "padding.left"
            | "padding.horizontal"
            | "padding.vertical" => {
                let function = name.strip_prefix("padding.").expect("checked prefix");
                format!(
                    "::iced::padding::{function}({})",
                    pixel_value_code(&args[0], env, document)?
                )
            }
            "padding.axes" => format!(
                "::iced::Padding::from([({}) as f32, ({}) as f32])",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "padding.from_pixels" => format!(
                "::iced::Padding::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "padding.with_top"
            | "padding.with_right"
            | "padding.with_bottom"
            | "padding.with_left"
            | "padding.with_horizontal"
            | "padding.with_vertical" => {
                let method = name.strip_prefix("padding.with_").expect("checked prefix");
                format!(
                    "({}).{method}({})",
                    expr_code(&args[0], env, document, ValueMode::Owned)?,
                    pixel_value_code(&args[1], env, document)?
                )
            }
            "padding.fit" => format!(
                "({}).fit({}, {})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "degrees" => format!(
                "::iced::Degrees(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "radians" => format!(
                "::iced::Radians(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "degrees.range_start" => "*::iced::Degrees::RANGE.start()".into(),
            "degrees.range_end" => "*::iced::Degrees::RANGE.end()".into(),
            "radians.range_start" => "*::iced::Radians::RANGE.start()".into(),
            "radians.range_end" => "*::iced::Radians::RANGE.end()".into(),
            "radians.pi" => "::iced::Radians::PI".into(),
            "degrees.in_range" => format!(
                "::iced::Degrees::RANGE.contains(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "radians.in_range" => format!(
                "::iced::Radians::RANGE.contains(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "radians.from_degrees" => format!(
                "::iced::Radians::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "radians.distance_start" | "radians.distance_end" => format!(
                "({}).to_distance(&({})).{}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                if name == "radians.distance_start" { 0 } else { 1 }
            ),
            "point" => format!(
                "::iced::Point::new(({}) as f32, ({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "vector" => format!(
                "::iced::Vector::new(({}) as f32, ({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "size" => format!(
                "::iced::Size::new(({}) as f32, ({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle" => format!(
                "::iced::Rectangle {{ x: ({}) as f32, y: ({}) as f32, width: ({}) as f32, height: ({}) as f32 }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?,
                expr_code(&args[3], env, document, ValueMode::Owned)?
            ),
            "point.origin" => "::iced::Point::ORIGIN".into(),
            "point.distance" => format!(
                "({}).distance({}) as f64",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "point.snap" => format!(
                "({}).snap()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "vector.zero" => "::iced::Vector::ZERO".into(),
            "size.zero" => "::iced::Size::ZERO".into(),
            "size.unit" => "::iced::Size::UNIT".into(),
            "size.infinite" => "::iced::Size::INFINITE".into(),
            "size.min" => format!(
                "({}).min({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "size.max" => format!(
                "({}).max({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "size.expand" => format!(
                "({}).expand({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "size.rotate" => format!(
                "({}).rotate({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                radians_value_code(&args[1], env, document)?
            ),
            "size.ratio" => format!(
                "({}).ratio(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "size.from_vector" => format!(
                "::iced::Size::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "vector.from_size" => format!(
                "::iced::Vector::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "size.from_padding" => format!(
                "::iced::Size::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "size.from_u32" => {
                let (Expr::I64(width), Expr::I64(height)) = (&args[0], &args[1]) else {
                    unreachable!("checker requires size dimensions as integer literals")
                };
                format!("::iced::Size::from(({width}u32, {height}u32))")
            }
            "size.try_from_u32" => format!(
                "match (<u32>::try_from({}), <u32>::try_from({})) {{ (::std::result::Result::Ok(width), ::std::result::Result::Ok(height)) => ::std::option::Option::Some(::iced::Size::from((width, height))), _ => ::std::option::Option::None }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.zero" => "::iced::Rectangle::default()".into(),
            "rectangle.infinite" => "::iced::Rectangle::INFINITE".into(),
            "rectangle.with_size" => format!(
                "::iced::Rectangle::with_size({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rectangle.with_radius" => format!(
                "::iced::Rectangle::with_radius(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rectangle.with_vertices" => format!(
                "::iced::Rectangle::with_vertices({}, {}, {}).0",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "rectangle.vertices_rotation" => format!(
                "::iced::Rectangle::with_vertices({}, {}, {}).1.0 as f64",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "rectangle.vertices_angle" => format!(
                "::iced::Rectangle::with_vertices({}, {}, {}).1",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "rectangle.contains" => format!(
                "({}).contains({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.distance" => format!(
                "({}).distance({}) as f64",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.offset" => format!(
                "({}).offset(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.is_within" => format!(
                "({}).is_within(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.intersection" => format!(
                "({}).intersection(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.intersects" => format!(
                "({}).intersects(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.union" => format!(
                "({}).union(&({}))",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.snap" => format!(
                "({}).snap()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rectangle.expand" | "rectangle.shrink" => format!(
                "({}).{}(::iced::Padding {{ top: ({}) as f32, right: ({}) as f32, bottom: ({}) as f32, left: ({}) as f32 }})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                if name == "rectangle.expand" {
                    "expand"
                } else {
                    "shrink"
                },
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?,
                expr_code(&args[3], env, document, ValueMode::Owned)?,
                expr_code(&args[4], env, document, ValueMode::Owned)?
            ),
            "rectangle.expand_padding" | "rectangle.shrink_padding" => format!(
                "({}).{}({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                if name == "rectangle.expand_padding" {
                    "expand"
                } else {
                    "shrink"
                },
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.rotate" => format!(
                "({}).rotate({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                radians_value_code(&args[1], env, document)?
            ),
            "rectangle.zoom" => format!(
                "({}).zoom(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "rectangle.anchor" => {
                let (Expr::Str(horizontal), Expr::Str(vertical)) = (&args[2], &args[3]) else {
                    unreachable!("checker requires literal rectangle alignments")
                };
                let horizontal = match horizontal.as_str() {
                    "left" => "Left",
                    "center" => "Center",
                    "right" => "Right",
                    _ => unreachable!("checker validates horizontal alignment"),
                };
                let vertical = match vertical.as_str() {
                    "top" => "Top",
                    "center" => "Center",
                    "bottom" => "Bottom",
                    _ => unreachable!("checker validates vertical alignment"),
                };
                format!(
                    "({}).anchor({}, ::iced::alignment::Horizontal::{horizontal}, ::iced::alignment::Vertical::{vertical})",
                    expr_code(&args[0], env, document, ValueMode::Owned)?,
                    expr_code(&args[1], env, document, ValueMode::Owned)?
                )
            }
            "rectangle.from_u32" => format!(
                "::iced::Rectangle::from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "transform.identity" => "::iced::Transformation::IDENTITY".into(),
            "transform.orthographic" => {
                let (Expr::I64(width), Expr::I64(height)) = (&args[0], &args[1]) else {
                    unreachable!("checker requires orthographic dimension literals")
                };
                format!("::iced::Transformation::orthographic({width}u32, {height}u32)")
            }
            "transform.try_orthographic" => format!(
                "match (<u32>::try_from({}), <u32>::try_from({})) {{ (::std::result::Result::Ok(width), ::std::result::Result::Ok(height)) => ::std::option::Option::Some(::iced::Transformation::orthographic(width, height)), _ => ::std::option::Option::None }}",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "transform.translate" => format!(
                "::iced::Transformation::translate(({}) as f32, ({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "transform.scale" => format!(
                "::iced::Transformation::scale(({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "transform.inverse" => format!(
                "({}).inverse()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "transform.compose" => format!(
                "({}) * ({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "transform.point"
            | "transform.vector"
            | "transform.size"
            | "transform.rectangle"
            | "transform.cursor"
            | "transform.click" => format!(
                "({}) * ({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "mouse.button" => {
                let Expr::Str(value) = &args[0] else {
                    unreachable!("checker requires a mouse button literal")
                };
                let variant = match value.as_str() {
                    "left" => "Left",
                    "right" => "Right",
                    "middle" => "Middle",
                    "back" => "Back",
                    "forward" => "Forward",
                    _ => unreachable!("checker validates mouse buttons"),
                };
                format!("::iced::mouse::Button::{variant}")
            }
            "mouse.other_button" => {
                let Expr::I64(value) = &args[0] else {
                    unreachable!("checker requires a mouse button literal")
                };
                format!("::iced::mouse::Button::Other({value}u16)")
            }
            "mouse.try_other_button" => format!(
                "<u16>::try_from({}).ok().map(::iced::mouse::Button::Other)",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor" => format!(
                "::iced::mouse::Cursor::Available({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.levitating" => format!(
                "::iced::mouse::Cursor::Levitating({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.unavailable" => "::iced::mouse::Cursor::Unavailable".into(),
            "mouse.cursor_position" => format!(
                "({}).position()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_over" => format!(
                "({}).position_over({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_in" => format!(
                "({}).position_in({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_from" => format!(
                "({}).position_from({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_is_over" => format!(
                "({}).is_over({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_is_levitating" => format!(
                "({}).is_levitating()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_levitate" => format!(
                "({}).levitate()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_land" => format!(
                "({}).land()",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "mouse.cursor_translate" => format!(
                "({}) + ::iced::Vector::new(({}) as f32, ({}) as f32)",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "mouse.click" => format!(
                "::iced::advanced::mouse::Click::new({}, {}, {})",
                expr_code(&args[0], env, document, ValueMode::Owned)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "touch.finger" => {
                let Expr::Str(value) = &args[0] else {
                    unreachable!("checker requires a touch finger literal")
                };
                let value = value
                    .parse::<u64>()
                    .expect("checker validates touch finger literals");
                format!("::iced::touch::Finger({value}u64)")
            }
            "touch.try_finger" => format!(
                "({}).parse::<u64>().ok().map(::iced::touch::Finger)",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "key.named" => {
                let Expr::Str(variant) = &args[0] else {
                    unreachable!("checker requires a named key variant")
                };
                format!(
                    "::iced::keyboard::Key::Named(::iced::keyboard::key::Named::{variant})"
                )
            }
            "key.character" => format!(
                "::iced::keyboard::Key::Character(({}).into())",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "key.unidentified" => "::iced::keyboard::Key::Unidentified".into(),
            "key.code" => {
                let Expr::Str(variant) = &args[0] else {
                    unreachable!("checker requires a physical key variant")
                };
                format!(
                    "::iced::keyboard::key::Physical::Code(::iced::keyboard::key::Code::{variant})"
                )
            }
            "key.native_unidentified" => "::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Unidentified)".into(),
            "key.command_modifiers" => "::iced::keyboard::Modifiers::COMMAND".into(),
            "key.native" => {
                let (Expr::Str(platform), Expr::I64(value)) = (&args[0], &args[1]) else {
                    unreachable!("checker requires a literal native key")
                };
                let (variant, ty) = match platform.as_str() {
                    "android" => ("Android", "u32"),
                    "macos" => ("MacOS", "u16"),
                    "windows" => ("Windows", "u16"),
                    "xkb" => ("Xkb", "u32"),
                    _ => unreachable!("checker validates native key platforms"),
                };
                format!(
                    "::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::{variant}({value}{ty}))"
                )
            }
            "key.try_native" => {
                let Expr::Str(platform) = &args[0] else {
                    unreachable!("checker requires a native key platform")
                };
                let (variant, ty) = match platform.as_str() {
                    "android" => ("Android", "u32"),
                    "macos" => ("MacOS", "u16"),
                    "windows" => ("Windows", "u16"),
                    "xkb" => ("Xkb", "u32"),
                    _ => unreachable!("checker validates native key platforms"),
                };
                format!(
                    "<{ty}>::try_from({}).ok().map(|value| ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::{variant}(value)))",
                    expr_code(&args[1], env, document, ValueMode::Owned)?
                )
            }
            "key.location" => {
                let Expr::Str(value) = &args[0] else {
                    unreachable!("checker requires a key location literal")
                };
                let variant = match value.as_str() {
                    "standard" => "Standard",
                    "left" => "Left",
                    "right" => "Right",
                    "numpad" => "Numpad",
                    _ => unreachable!("checker validates key locations"),
                };
                format!("::iced::keyboard::Location::{variant}")
            }
            "key.modifiers" => {
                let values = ["SHIFT", "CTRL", "ALT", "LOGO"]
                    .into_iter()
                    .zip(args)
                    .map(|(flag, value)| {
                        Ok(format!(
                            "if {} {{ ::iced::keyboard::Modifiers::{flag} }} else {{ ::iced::keyboard::Modifiers::empty() }}",
                            expr_code(value, env, document, ValueMode::Owned)?
                        ))
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                format!("({})", values.join(" | "))
            }
            "key.latin" => format!(
                "({}).to_latin({}).map(|value| value.to_string())",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?,
                expr_code(&args[1], env, document, ValueMode::Owned)?
            ),
            "len" => format!(
                "({}).len() as i64",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "empty" => format!(
                "({}).is_empty()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "trim" => format!(
                "({}).trim().to_owned()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "some" => format!(
                "::std::option::Option::Some({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "ok" => format!(
                "::std::result::Result::Ok({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "err" => format!(
                "::std::result::Result::Err({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "markdown" => format!(
                "::iced::widget::markdown::Content::parse(&{})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "markdown_images" => format!(
                "({}).images().iter().cloned().collect::<::std::vec::Vec<_>>()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "editor" => format!(
                "::iced::widget::text_editor::Content::with_text(&{})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "editor_text" => format!(
                "({}).text()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "encoded" => format!(
                "::iced::widget::image::Handle::from_bytes({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rgba" => format!(
                "::iced::widget::image::Handle::from_rgba({}, {}, {})",
                u32_code(&args[0], env, document)?,
                u32_code(&args[1], env, document)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "aborted" => format!(
                "({}).as_ref().is_some_and(::iced::task::Handle::is_aborted)",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            _ => {
                let function = find_extern_function(document, name, ExternKind::Sync)
                    .expect("checker accepts only declared sync calls");
                let args = expr_list_code(args, env, document)?;
                format!("{}({args})", function.rust_path)
            }
        }
        }
        Expr::Unary { op, value } => format!(
            "({}{})",
            match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            },
            expr_code(value, env, document, ValueMode::Owned)?
        ),
        Expr::Binary { left, op, right } => {
            let mode = if matches!(
                op,
                BinaryOp::Eq
                    | BinaryOp::NotEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq
            ) {
                ValueMode::Borrowed
            } else {
                ValueMode::Owned
            };
            let types = env_types(env);
            let left_ty = expr_type(left, &types, document, &Span::line(1))?;
            let right_ty = expr_type(right, &types, document, &Span::line(1))?;
            let left = expr_code(left, env, document, mode)?;
            let right = expr_code(right, env, document, mode)?;
            let left = if left_ty == Type::F64 && right_ty == Type::Radians && *op == BinaryOp::Mul
            {
                format!("({left}) as f32")
            } else {
                left
            };
            let right = if right_ty == Type::F64
                && matches!(
                    left_ty,
                    Type::Pixels
                        | Type::Degrees
                        | Type::Radians
                        | Type::Radius
                        | Type::Vector
                        | Type::Size
                        | Type::Rectangle
                ) {
                format!("({right}) as f32")
            } else {
                right
            };
            format!(
                "({} {} {})",
                left,
                match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Rem => "%",
                    BinaryOp::Eq => "==",
                    BinaryOp::NotEq => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::LtEq => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::GtEq => ">=",
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
                },
                right
            )
        }
    })
}

pub(in crate::codegen) fn clamped_f32_code(
    expr: &Expr,
    min: &str,
    max: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let code = expr_code(expr, env, document, ValueMode::Owned)?;
    Ok(format!("(({code}) as f32).max({min}).min({max})"))
}

fn unit_f32_code(
    expr: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    clamped_f32_code(expr, "0.0", "1.0", env, document)
}

mod binding;
mod children;
mod discovery;
mod routes;

pub(super) use binding::*;
pub(super) use children::*;
pub(super) use discovery::*;
pub(super) use routes::*;

pub(in crate::codegen) fn animation_at_code(
    args: &[Expr],
    index: usize,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    args.get(index).map_or_else(
        || Ok("::iced::time::Instant::now()".into()),
        |at| expr_code(at, env, document, ValueMode::Owned),
    )
}
