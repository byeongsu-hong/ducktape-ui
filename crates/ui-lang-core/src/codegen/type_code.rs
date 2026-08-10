use super::*;

pub(super) fn rust_type_code(program: &LoweredProgram, ty: &Type) -> String {
    rust_type_code_with_named(ty, &|name| {
        program.struct_rust_path_by_name(name).map(str::to_owned)
    })
}

fn rust_type_code_with_named(
    ty: &Type,
    named_rust_path: &impl Fn(&str) -> Option<String>,
) -> String {
    match ty {
        Type::Bool => "bool".into(),
        Type::I64 => "i64".into(),
        Type::F64 => "f64".into(),
        Type::Str => "::std::string::String".into(),
        Type::Bytes => "::std::vec::Vec<u8>".into(),
        Type::Image => "::iced::widget::image::Handle".into(),
        Type::ImageAllocation => "::iced::widget::image::Allocation".into(),
        Type::ImageMemory => "::std::sync::Weak<::iced::advanced::image::Memory>".into(),
        Type::ImageError => "::iced::widget::image::Error".into(),
        Type::DebugSpan => "::iced::debug::Span".into(),
        Type::List(inner) => format!(
            "::std::vec::Vec<{}>",
            rust_type_code_with_named(inner, named_rust_path)
        ),
        Type::Option(inner) => format!(
            "::std::option::Option<{}>",
            rust_type_code_with_named(inner, named_rust_path)
        ),
        Type::Result(output, error) => format!(
            "::std::result::Result<{}, {}>",
            rust_type_code_with_named(output, named_rust_path),
            rust_type_code_with_named(error, named_rust_path)
        ),
        Type::Combo(inner) => format!(
            "::iced::widget::combo_box::State<{}>",
            rust_type_code_with_named(inner, named_rust_path)
        ),
        Type::Animation(inner) => match inner.as_ref() {
            Type::F64 => "::iced::Animation<f32>".into(),
            inner => format!(
                "::iced::Animation<{}>",
                rust_type_code_with_named(inner, named_rust_path)
            ),
        },
        Type::Markdown => "::iced::widget::markdown::Content".into(),
        Type::Editor => "::iced::widget::text_editor::Content".into(),
        Type::Event => "::iced::Event".into(),
        Type::EventStatus => "::iced::event::Status".into(),
        Type::Key => "::iced::keyboard::Key".into(),
        Type::PhysicalKey => "::iced::keyboard::key::Physical".into(),
        Type::KeyLocation => "::iced::keyboard::Location".into(),
        Type::KeyPress => "__IceKeyPress".into(),
        Type::KeyRelease => "__IceKeyRelease".into(),
        Type::KeyModifiers => "::iced::keyboard::Modifiers".into(),
        Type::Pixels => "::iced::Pixels".into(),
        Type::Padding => "::iced::Padding".into(),
        Type::Degrees => "::iced::Degrees".into(),
        Type::Radians => "::iced::Radians".into(),
        Type::Rotation => "::iced::Rotation".into(),
        Type::ContentFit => "::iced::ContentFit".into(),
        Type::Color => "::iced::Color".into(),
        Type::Background => "::iced::Background".into(),
        Type::Gradient => "::iced::Gradient".into(),
        Type::LinearGradient => "::iced::gradient::Linear".into(),
        Type::ColorStop => "::iced::gradient::ColorStop".into(),
        Type::Font => "::iced::Font".into(),
        Type::FontFamily => "::iced::font::Family".into(),
        Type::FontWeight => "::iced::font::Weight".into(),
        Type::FontStretch => "::iced::font::Stretch".into(),
        Type::FontStyle => "::iced::font::Style".into(),
        Type::ThemeMode => "::iced::theme::Mode".into(),
        Type::TextAlignment => "::iced::widget::text::Alignment".into(),
        Type::TextShaping => "::iced::widget::text::Shaping".into(),
        Type::TextWrapping => "::iced::widget::text::Wrapping".into(),
        Type::TextLineHeight => "::iced::widget::text::LineHeight".into(),
        Type::Length => "::iced::Length".into(),
        Type::Alignment => "::iced::Alignment".into(),
        Type::HorizontalAlignment => "::iced::alignment::Horizontal".into(),
        Type::VerticalAlignment => "::iced::alignment::Vertical".into(),
        Type::Border => "::iced::Border".into(),
        Type::Radius => "::iced::border::Radius".into(),
        Type::Shadow => "::iced::Shadow".into(),
        Type::Point => "::iced::Point".into(),
        Type::PointU32 => "::iced::Point<u32>".into(),
        Type::Vector => "::iced::Vector".into(),
        Type::Size => "::iced::Size".into(),
        Type::SizeU32 => "::iced::Size<u32>".into(),
        Type::Rectangle => "::iced::Rectangle".into(),
        Type::RectangleU32 => "::iced::Rectangle<u32>".into(),
        Type::Transformation => "::iced::Transformation".into(),
        Type::MouseInteraction => "::iced::mouse::Interaction".into(),
        Type::ScrollDelta => "::iced::mouse::ScrollDelta".into(),
        Type::MouseButton => "::iced::mouse::Button".into(),
        Type::MouseCursor => "::iced::mouse::Cursor".into(),
        Type::MouseClick => "::iced::advanced::mouse::Click".into(),
        Type::TouchFinger => "::iced::touch::Finger".into(),
        Type::SystemInfo => "__IceSystemInfo".into(),
        Type::Instant => "::iced::time::Instant".into(),
        Type::WindowId => "::iced::window::Id".into(),
        Type::WindowScreenshot => "::iced::window::Screenshot".into(),
        Type::WindowPosition => "::iced::window::Position".into(),
        Type::RedrawRequest => "::iced::window::RedrawRequest".into(),
        Type::WindowDirection => "::iced::window::Direction".into(),
        Type::WindowLevel => "::iced::window::Level".into(),
        Type::WindowMode => "::iced::window::Mode".into(),
        Type::WindowAttention => "::iced::window::UserAttention".into(),
        Type::Secret => "::ui_lang_runtime::Secret".into(),
        Type::WidgetId => "::iced::widget::Id".into(),
        Type::WidgetTarget => "__IceWidgetTarget".into(),
        Type::TestTarget => "::ui_lang_runtime::testing::Target".into(),
        Type::TaskHandle => "::iced::task::Handle".into(),
        Type::Palette(contract) => canonical_rust_type_name(contract),
        Type::Named(name) => {
            named_rust_path(name).unwrap_or_else(|| canonical_rust_type_name(name))
        }
        Type::Unit => "()".into(),
        Type::Unknown => "_".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_uses_the_public_runtime_target() {
        assert_eq!(
            rust_type_code_with_named(&Type::TestTarget, &|_| None),
            "::ui_lang_runtime::testing::Target"
        );
    }

    #[test]
    fn nested_and_named_types_preserve_backend_mapping() {
        let named = |name: &str| (name == "external").then(|| "crate::External".to_owned());
        assert_eq!(
            rust_type_code_with_named(
                &Type::Result(
                    Box::new(Type::List(Box::new(Type::Named("external".into())))),
                    Box::new(Type::Option(Box::new(Type::Named("generated-type".into())))),
                ),
                &named,
            ),
            "::std::result::Result<::std::vec::Vec<crate::External>, ::std::option::Option<__IceType067656e6572617465642d74797065>>"
        );
        assert_eq!(
            rust_type_code_with_named(&Type::Animation(Box::new(Type::F64)), &named),
            "::iced::Animation<f32>"
        );
    }
}
