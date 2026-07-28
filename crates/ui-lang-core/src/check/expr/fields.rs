use super::*;

pub(in crate::check) fn contains_task_handle(ty: &Type) -> bool {
    match ty {
        Type::TaskHandle => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) => contains_task_handle(inner),
        Type::Result(output, error) => contains_task_handle(output) || contains_task_handle(error),
        _ => false,
    }
}

pub(in crate::check) fn contains_mouse_click(ty: &Type) -> bool {
    match ty {
        Type::MouseClick => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) => contains_mouse_click(inner),
        Type::Result(output, error) => contains_mouse_click(output) || contains_mouse_click(error),
        _ => false,
    }
}

pub(in crate::check) fn contains_window_screenshot(ty: &Type) -> bool {
    match ty {
        Type::WindowScreenshot => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) => {
            contains_window_screenshot(inner)
        }
        Type::Result(output, error) => {
            contains_window_screenshot(output) || contains_window_screenshot(error)
        }
        _ => false,
    }
}

pub(in crate::check) fn contains_target(ty: &Type) -> bool {
    match ty {
        Type::TestTarget | Type::WidgetTarget => true,
        Type::List(inner) | Type::Option(inner) | Type::Combo(inner) | Type::Animation(inner) => {
            contains_target(inner)
        }
        Type::Result(output, error) => contains_target(output) || contains_target(error),
        _ => false,
    }
}

pub(in crate::check) fn field_type(
    ty: &Type,
    field: &str,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    if let Type::Option(inner) = ty
        && **inner == Type::WidgetTarget
    {
        return Ok(Type::Option(Box::new(field_type(
            inner, field, document, span,
        )?)));
    }
    let found = match ty {
        Type::Named(name) => {
            let item = document
                .structs
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| {
                    Error::new("E151", span, format!("unknown extern struct `{name}`"))
                })?;
            return item
                .fields
                .iter()
                .find(|(name, _)| name == field)
                .map(|(_, ty)| ty.clone())
                .ok_or_else(|| {
                    Error::new(
                        "E151",
                        span,
                        format!("struct `{name}` has no field `{field}`"),
                    )
                });
        }
        Type::KeyPress => match field {
            "key" | "modified_key" => Some(Type::Key),
            "physical_key" => Some(Type::PhysicalKey),
            "location" => Some(Type::KeyLocation),
            "modifiers" => Some(Type::KeyModifiers),
            "text" => Some(Type::Option(Box::new(Type::Str))),
            "repeat" => Some(Type::Bool),
            _ => None,
        },
        Type::KeyRelease => match field {
            "key" | "modified_key" => Some(Type::Key),
            "physical_key" => Some(Type::PhysicalKey),
            "location" => Some(Type::KeyLocation),
            "modifiers" => Some(Type::KeyModifiers),
            _ => None,
        },
        Type::Key => match field {
            "kind" => Some(Type::Str),
            "named" | "character" => Some(Type::Option(Box::new(Type::Str))),
            _ => None,
        },
        Type::PhysicalKey => match field {
            "kind" => Some(Type::Str),
            "code" | "native_platform" => Some(Type::Option(Box::new(Type::Str))),
            "native_code" => Some(Type::Option(Box::new(Type::I64))),
            _ => None,
        },
        Type::KeyLocation => match field {
            "name" => Some(Type::Str),
            _ => None,
        },
        Type::KeyModifiers => match field {
            "shift" | "control" | "alt" | "logo" | "command" | "jump" | "macos_command" => {
                Some(Type::Bool)
            }
            _ => None,
        },
        Type::Pixels => match field {
            "value" => Some(Type::F64),
            _ => None,
        },
        Type::Padding => match field {
            "top" | "right" | "bottom" | "left" | "x" | "y" => Some(Type::F64),
            _ => None,
        },
        Type::Degrees => match field {
            "value" => Some(Type::F64),
            _ => None,
        },
        Type::Radians => match field {
            "value" => Some(Type::F64),
            "display" => Some(Type::Str),
            _ => None,
        },
        Type::Rotation => match field {
            "radians" => Some(Type::Radians),
            "degrees" => Some(Type::Degrees),
            "kind" => Some(Type::Str),
            _ => None,
        },
        Type::ContentFit => match field {
            "kind" | "display" => Some(Type::Str),
            _ => None,
        },
        Type::Color => match field {
            "r" | "g" | "b" | "a" | "luminance" => Some(Type::F64),
            "rgba8" => Some(Type::List(Box::new(Type::I64))),
            "linear" => Some(Type::List(Box::new(Type::F64))),
            "display" => Some(Type::Str),
            _ => None,
        },
        Type::Background => match field {
            "kind" => Some(Type::Str),
            "color" => Some(Type::Option(Box::new(Type::Color))),
            "gradient" => Some(Type::Option(Box::new(Type::Gradient))),
            _ => None,
        },
        Type::Gradient => match field {
            "kind" => Some(Type::Str),
            "linear" => Some(Type::LinearGradient),
            _ => None,
        },
        Type::LinearGradient => match field {
            "angle" => Some(Type::Radians),
            "stops" => Some(Type::List(Box::new(Type::Option(Box::new(
                Type::ColorStop,
            ))))),
            _ => None,
        },
        Type::ColorStop => match field {
            "offset" => Some(Type::F64),
            "color" => Some(Type::Color),
            _ => None,
        },
        Type::Font => match field {
            "family" => Some(Type::FontFamily),
            "weight" => Some(Type::FontWeight),
            "stretch" => Some(Type::FontStretch),
            "style" => Some(Type::FontStyle),
            _ => None,
        },
        Type::FontFamily => match field {
            "kind" => Some(Type::Str),
            "name" => Some(Type::Option(Box::new(Type::Str))),
            _ => None,
        },
        Type::FontWeight | Type::FontStretch | Type::FontStyle => match field {
            "kind" => Some(Type::Str),
            _ => None,
        },
        Type::ThemeMode => match field {
            "kind" => Some(Type::Str),
            _ => None,
        },
        Type::TextAlignment | Type::TextShaping | Type::TextWrapping => match field {
            "kind" => Some(Type::Str),
            _ => None,
        },
        Type::TextLineHeight => match field {
            "kind" => Some(Type::Str),
            "relative" => Some(Type::Option(Box::new(Type::F64))),
            "absolute" => Some(Type::Option(Box::new(Type::Pixels))),
            _ => None,
        },
        Type::MouseInteraction => match field {
            "kind" => Some(Type::Str),
            _ => None,
        },
        Type::ScrollDelta => match field {
            "kind" => Some(Type::Str),
            "x" | "y" => Some(Type::F64),
            _ => None,
        },
        Type::EventStatus => match field {
            "kind" => Some(Type::Str),
            _ => None,
        },
        Type::RedrawRequest => match field {
            "kind" => Some(Type::Str),
            "instant" => Some(Type::Option(Box::new(Type::Instant))),
            _ => None,
        },
        Type::WindowId => match field {
            "display" => Some(Type::Str),
            _ => None,
        },
        Type::WindowScreenshot => match field {
            "rgba" => Some(Type::Bytes),
            "size" => Some(Type::SizeU32),
            "scale_factor" => Some(Type::F64),
            "debug" => Some(Type::Str),
            _ => None,
        },
        Type::WindowPosition => match field {
            "kind" => Some(Type::Str),
            "point" => Some(Type::Option(Box::new(Type::Point))),
            _ => None,
        },
        Type::WindowDirection | Type::WindowLevel | Type::WindowMode | Type::WindowAttention => {
            match field {
                "kind" => Some(Type::Str),
                _ => None,
            }
        }
        Type::Length => match field {
            "fill_factor" => Some(Type::I64),
            "is_fill" => Some(Type::Bool),
            "kind" => Some(Type::Str),
            "portion" => Some(Type::Option(Box::new(Type::I64))),
            "fixed" => Some(Type::Option(Box::new(Type::F64))),
            _ => None,
        },
        Type::Alignment | Type::HorizontalAlignment | Type::VerticalAlignment => match field {
            "kind" => Some(Type::Str),
            _ => None,
        },
        Type::Border => match field {
            "color" => Some(Type::Color),
            "width" => Some(Type::F64),
            "radius" => Some(Type::Radius),
            _ => None,
        },
        Type::Radius => match field {
            "top_left" | "top_right" | "bottom_right" | "bottom_left" => Some(Type::F64),
            "values" => Some(Type::List(Box::new(Type::F64))),
            _ => None,
        },
        Type::Shadow => match field {
            "color" => Some(Type::Color),
            "offset" => Some(Type::Vector),
            "blur" => Some(Type::F64),
            _ => None,
        },
        Type::Point => match field {
            "x" | "y" => Some(Type::F64),
            "values" => Some(Type::List(Box::new(Type::F64))),
            "display" => Some(Type::Str),
            _ => None,
        },
        Type::PointU32 => match field {
            "x" | "y" => Some(Type::I64),
            _ => None,
        },
        Type::Vector => match field {
            "x" | "y" => Some(Type::F64),
            "values" => Some(Type::List(Box::new(Type::F64))),
            _ => None,
        },
        Type::Size => match field {
            "width" | "height" => Some(Type::F64),
            "values" => Some(Type::List(Box::new(Type::F64))),
            _ => None,
        },
        Type::SizeU32 => match field {
            "width" | "height" => Some(Type::I64),
            _ => None,
        },
        Type::Rectangle => match field {
            "x" | "y" | "width" | "height" => Some(Type::F64),
            "center" | "position" => Some(Type::Point),
            "center_x" | "center_y" | "area" => Some(Type::F64),
            "size" => Some(Type::Size),
            _ => None,
        },
        Type::RectangleU32 => match field {
            "x" | "y" | "width" | "height" => Some(Type::I64),
            _ => None,
        },
        Type::Transformation => match field {
            "scale_factor" => Some(Type::F64),
            "translation" => Some(Type::Vector),
            "matrix" => Some(Type::List(Box::new(Type::F64))),
            _ => None,
        },
        Type::MouseButton => match field {
            "kind" => Some(Type::Str),
            "number" => Some(Type::Option(Box::new(Type::I64))),
            _ => None,
        },
        Type::MouseCursor => match field {
            "kind" => Some(Type::Str),
            "position" => Some(Type::Option(Box::new(Type::Point))),
            "levitating" => Some(Type::Bool),
            _ => None,
        },
        Type::MouseClick => match field {
            "kind" => Some(Type::Str),
            "position" => Some(Type::Point),
            _ => None,
        },
        Type::TouchFinger => match field {
            "id" => Some(Type::Str),
            _ => None,
        },
        Type::SystemInfo => match field {
            "system_name" | "system_kernel" | "system_version" | "system_short_version" => {
                Some(Type::Option(Box::new(Type::Str)))
            }
            "cpu_brand" | "graphics_backend" | "graphics_adapter" => Some(Type::Str),
            "cpu_cores" | "memory_used" => Some(Type::Option(Box::new(Type::I64))),
            "memory_total" => Some(Type::I64),
            _ => None,
        },
        Type::ImageAllocation => match field {
            "handle" => Some(Type::Image),
            "size" => Some(Type::SizeU32),
            _ => None,
        },
        Type::ImageError => match field {
            "kind" | "message" => Some(Type::Str),
            _ => None,
        },
        Type::WidgetTarget => match field {
            "kind" => Some(Type::Str),
            "id" => Some(Type::Option(Box::new(Type::WidgetId))),
            "x" | "y" | "width" | "height" => Some(Type::F64),
            "visible_x" | "visible_y" | "visible_width" | "visible_height" | "content_x"
            | "content_y" | "content_width" | "content_height" | "translation_x"
            | "translation_y" => Some(Type::Option(Box::new(Type::F64))),
            "content" => Some(Type::Option(Box::new(Type::Str))),
            _ => None,
        },
        Type::TestTarget => match field {
            "kind"
            | "value"
            | "accessibility_role"
            | "accessibility_name"
            | "accessibility_description"
            | "accessibility_value" => Some(Type::Str),
            "visible"
            | "pixel_aligned"
            | "focused"
            | "accessibility_checked"
            | "accessibility_disabled"
            | "accessibility_supports_activate"
            | "accessibility_supports_focus" => Some(Type::Bool),
            "surface_count" | "text_count" | "image_count" => Some(Type::I64),
            "x" | "y" | "width" | "height" | "left" | "top" | "right" | "bottom" | "center_x"
            | "center_y" | "visible_x" | "visible_y" | "visible_width" | "visible_height"
            | "content_x" | "content_y" | "content_width" | "content_height" | "scroll_x"
            | "scroll_y" | "translation_x" | "translation_y" | "text_size" | "text_x"
            | "text_y" | "text_width" | "text_height" | "text_baseline" | "image_x" | "image_y"
            | "image_width" | "image_height" => Some(Type::F64),
            "background" => Some(Type::Background),
            "border" => Some(Type::Border),
            "shadow" => Some(Type::Shadow),
            "text_color" => Some(Type::Color),
            "font" => Some(Type::Font),
            "line_height" => Some(Type::TextLineHeight),
            _ => None,
        },
        _ => None,
    };
    found.ok_or_else(|| {
        Error::new(
            "E151",
            span,
            format!("type `{}` has no field `{field}`", ty.display()),
        )
    })
}
