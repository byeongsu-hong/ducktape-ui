use super::*;

pub(in crate::check) fn infer_view(
    node: &ViewNode,
    env: &HashMap<String, Type>,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<(), Error> {
    if infer_layout_group(node, env, document, signatures, ids)? {
        return Ok(());
    }
    if infer_content_group(node, env, document, signatures, ids)? {
        return Ok(());
    }
    if infer_controls_group(node, env, document, signatures, ids)? {
        return Ok(());
    }
    if infer_documents_group(node, env, document, signatures, ids)? {
        return Ok(());
    }
    if infer_components_group(node, env, document, signatures, ids)? {
        return Ok(());
    }
    if infer_media_group(node, env, document, signatures, ids)? {
        return Ok(());
    }
    if infer_structure_group(node, env, document, signatures, ids)? {
        return Ok(());
    }
    unreachable!("every view node belongs to an inference group")
}

pub(in crate::check) fn lazy_hashable(ty: &Type) -> bool {
    match ty {
        Type::Bool
        | Type::I64
        | Type::Str
        | Type::Bytes
        | Type::Instant
        | Type::WindowId
        | Type::WidgetId
        | Type::Key
        | Type::PhysicalKey
        | Type::KeyModifiers
        | Type::MouseButton
        | Type::TouchFinger
        | Type::ContentFit
        | Type::Font
        | Type::FontFamily
        | Type::FontWeight
        | Type::FontStretch
        | Type::FontStyle
        | Type::TextAlignment
        | Type::TextShaping
        | Type::TextWrapping
        | Type::TextLineHeight
        | Type::Alignment
        | Type::HorizontalAlignment
        | Type::VerticalAlignment
        | Type::Named(_) => true,
        Type::List(inner) | Type::Option(inner) => lazy_hashable(inner),
        Type::Result(output, error) => lazy_hashable(output) && lazy_hashable(error),
        Type::F64
        | Type::Combo(_)
        | Type::Animation(_)
        | Type::Markdown
        | Type::Editor
        | Type::Event
        | Type::EventStatus
        | Type::ThemeMode
        | Type::KeyLocation
        | Type::KeyPress
        | Type::KeyRelease
        | Type::Pixels
        | Type::Padding
        | Type::Degrees
        | Type::Radians
        | Type::Rotation
        | Type::Color
        | Type::Background
        | Type::Gradient
        | Type::LinearGradient
        | Type::ColorStop
        | Type::Length
        | Type::Border
        | Type::Radius
        | Type::Shadow
        | Type::Point
        | Type::PointU32
        | Type::Vector
        | Type::Size
        | Type::Rectangle
        | Type::RectangleU32
        | Type::Transformation
        | Type::MouseInteraction
        | Type::ScrollDelta
        | Type::MouseCursor
        | Type::MouseClick
        | Type::SystemInfo
        | Type::WindowScreenshot
        | Type::WindowPosition
        | Type::RedrawRequest
        | Type::WindowDirection
        | Type::WindowLevel
        | Type::WindowMode
        | Type::WindowAttention
        | Type::WidgetTarget
        | Type::TestTarget
        | Type::TaskHandle
        | Type::Image
        | Type::ImageAllocation
        | Type::ImageMemory
        | Type::ImageError
        | Type::DebugSpan
        | Type::SizeU32
        | Type::Unit
        | Type::Unknown => false,
    }
}

mod components;
mod content;
mod controls;
mod documents;
mod layout;
mod media;
mod structure;

pub(super) use components::*;
pub(super) use content::*;
pub(super) use controls::*;
pub(super) use documents::*;
pub(super) use layout::*;
pub(super) use media::*;
pub(super) use structure::*;
