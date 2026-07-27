use super::*;

#[derive(Clone)]
pub(in crate::codegen) struct Binding {
    pub(in crate::codegen) code: String,
    pub(in crate::codegen) ty: Type,
    pub(in crate::codegen) local: bool,
    pub(in crate::codegen) state: Option<StateBinding>,
}

#[derive(Clone)]
pub(in crate::codegen) enum StateBinding {
    App(String),
    Component {
        component: String,
        name: String,
        scope: String,
    },
}

#[derive(Clone)]
pub(in crate::codegen) struct SlotContext {
    pub(in crate::codegen) entries: Vec<SlotContent>,
    pub(in crate::codegen) parent: Option<Box<SlotContext>>,
}

#[derive(Clone)]
pub(in crate::codegen) struct SlotContent {
    pub(in crate::codegen) name: String,
    pub(in crate::codegen) node: ViewNode,
    pub(in crate::codegen) env: HashMap<String, Binding>,
}

#[derive(Clone, Copy)]
pub(in crate::codegen) enum ValueMode {
    Owned,
    Borrowed,
}

const COMPONENT_CONTEXT_PREFIX: &str = "\0component:";
const COMPONENT_OUTPUT_PREFIX: &str = "\0component-output:";

pub(in crate::codegen) fn component_context_key(component: &str) -> String {
    format!("{COMPONENT_CONTEXT_PREFIX}{component}")
}

pub(in crate::codegen) fn component_context(
    env: &HashMap<String, Binding>,
) -> Option<(&str, &Binding)> {
    env.iter().find_map(|(name, binding)| {
        name.strip_prefix(COMPONENT_CONTEXT_PREFIX)
            .map(|component| (component, binding))
    })
}

pub(in crate::codegen) fn component_output_key(component: &str) -> String {
    format!("{COMPONENT_OUTPUT_PREFIX}{component}")
}

pub(in crate::codegen) fn component_output(env: &HashMap<String, Binding>) -> Option<&Binding> {
    env.iter()
        .find_map(|(name, binding)| name.starts_with(COMPONENT_OUTPUT_PREFIX).then_some(binding))
}

pub(in crate::codegen) fn component_state_field(component: &str) -> String {
    if canonical_component(component) {
        format!("__ice_component_{}", component.to_ascii_lowercase())
    } else {
        format!("__ice_component_0{}", rust_identifier_hex(component))
    }
}

pub(in crate::codegen) fn component_state_type(component: &str) -> String {
    if simple_component(component)
        && !component.starts_with("Pane")
        && !component.starts_with("EventFilter")
    {
        format!("__Ice{component}State")
    } else {
        format!("__Ice0C{}State", rust_identifier_hex(component))
    }
}

pub(in crate::codegen) fn component_scope_binding(component: &str, line: usize) -> String {
    format!("{}_scope_{line}", component_state_field(component))
}

pub(in crate::codegen) fn rust_identifier_hex(value: &str) -> String {
    value.bytes().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical_component(value: &str) -> bool {
    value.as_bytes().split_first().is_some_and(|(first, rest)| {
        first.is_ascii_uppercase()
            && rest
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}

fn simple_component(value: &str) -> bool {
    value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn canonical_component_variant(value: &str) -> bool {
    canonical_component(value)
        && !["Bind", "Edit", "Pane"]
            .iter()
            .any(|prefix| value.starts_with(prefix))
}

pub(in crate::codegen) fn handler_variant(handler: &str) -> String {
    if canonical_snake(handler) {
        pascal(handler)
    } else {
        format!("__0H{}", rust_identifier_hex(handler))
    }
}

pub(in crate::codegen) fn component_handler_variant(component: &str, handler: &str) -> String {
    if canonical_component_variant(component) && canonical_snake(handler) {
        format!("__{component}Handle{}", pascal(handler))
    } else {
        format!(
            "__0C{}H{}",
            rust_identifier_hex(component),
            rust_identifier_hex(handler)
        )
    }
}

pub(in crate::codegen) fn component_binding_variant(component: &str, state: &str) -> String {
    if canonical_component_variant(component) && canonical_snake(state) {
        format!("__{component}Bind{}", pascal(state))
    } else {
        format!(
            "__0C{}B{}",
            rust_identifier_hex(component),
            rust_identifier_hex(state)
        )
    }
}

pub(in crate::codegen) fn component_latest_field(line: usize) -> String {
    format!("__ice_latest_{line}")
}

pub(in crate::codegen) fn component_latest_variant(component: &str, line: usize) -> String {
    if simple_component(component)
        && !component.starts_with("Bind")
        && !component.starts_with("Edit")
    {
        format!("__{component}Latest{line}")
    } else {
        format!("__0C{}L{line}", rust_identifier_hex(component))
    }
}

pub(in crate::codegen) fn derived_method(name: &str) -> String {
    format!("__ice_derived_{name}")
}

pub(in crate::codegen) fn state_env(document: &Document, name: &str) -> HashMap<String, Binding> {
    let mut env = document
        .states
        .iter()
        .map(|state| {
            (
                state.name.clone(),
                Binding {
                    code: format!("{name}.{}", state.name),
                    ty: state.ty.clone(),
                    local: false,
                    state: Some(StateBinding::App(state.name.clone())),
                },
            )
        })
        .collect::<HashMap<_, _>>();
    env.extend(document.derived.iter().map(|derived| {
        (
            derived.name.clone(),
            Binding {
                code: format!("Self::{}({name})", derived_method(&derived.name)),
                ty: derived.ty.clone(),
                local: true,
                state: None,
            },
        )
    }));
    env
}

pub(in crate::codegen) fn env_types(env: &HashMap<String, Binding>) -> HashMap<String, Type> {
    env.iter()
        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
        .collect()
}

pub(in crate::codegen) fn pixel_value_code(
    value: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let code = expr_code(value, env, document, ValueMode::Owned)?;
    Ok(
        if expr_type(value, &env_types(env), document, &Span::line(1))? == Type::Pixels {
            code
        } else {
            format!("({code}) as f32")
        },
    )
}

pub(in crate::codegen) fn pixel_scalar_code(
    value: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let code = expr_code(value, env, document, ValueMode::Owned)?;
    Ok(
        if expr_type(value, &env_types(env), document, &Span::line(1))? == Type::Pixels {
            format!("({code}).0")
        } else {
            format!("({code}) as f32")
        },
    )
}

pub(in crate::codegen) fn radius_value_code(
    value: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let code = expr_code(value, env, document, ValueMode::Owned)?;
    Ok(
        if expr_type(value, &env_types(env), document, &Span::line(1))? == Type::Radius {
            code
        } else {
            format!("::iced::border::Radius::from(({code}) as f32)")
        },
    )
}

pub(in crate::codegen) fn radians_value_code(
    value: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let code = expr_code(value, env, document, ValueMode::Owned)?;
    Ok(
        if expr_type(value, &env_types(env), document, &Span::line(1))? == Type::Radians {
            code
        } else {
            format!("::iced::Radians(({code}) as f32)")
        },
    )
}

pub(in crate::codegen) fn native_field_type(ty: &Type, field: &str) -> Option<Type> {
    match ty {
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
        _ => None,
    }
}

pub(in crate::codegen) fn native_field_projection(
    ty: &Type,
    field: &str,
    code: &str,
) -> Option<(String, Type)> {
    let projected = match (ty, field) {
        (Type::TestTarget, field) => {
            let ty = match field {
                "kind" | "value" => Type::Str,
                "visible" => Type::Bool,
                "x" | "y" | "width" | "height" | "left" | "top" | "right" | "bottom"
                | "center_x" | "center_y" | "visible_x" | "visible_y" | "visible_width"
                | "visible_height" | "content_x" | "content_y" | "content_width"
                | "content_height" | "scroll_x" | "scroll_y" | "translation_x"
                | "translation_y" | "text_size" => Type::F64,
                "background" => Type::Background,
                "border" => Type::Border,
                "shadow" => Type::Shadow,
                "text_color" => Type::Color,
                "font" => Type::Font,
                "line_height" => Type::TextLineHeight,
                _ => return None,
            };
            (format!("({code}).{field}()"), ty)
        }
        (Type::Key, "kind") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::Key::Named(_) => \"named\", ::iced::keyboard::Key::Character(_) => \"character\", ::iced::keyboard::Key::Unidentified => \"unidentified\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::Key, "named") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::Key::Named(value) => ::std::option::Option::Some(::std::format!(\"{{value:?}}\")), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Str)),
        ),
        (Type::Key, "character") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::Key::Character(value) => ::std::option::Option::Some(value.to_string()), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Str)),
        ),
        (Type::PhysicalKey, "kind") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::key::Physical::Code(_) => \"code\", ::iced::keyboard::key::Physical::Unidentified(_) => \"native\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::PhysicalKey, "code") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::key::Physical::Code(value) => ::std::option::Option::Some(::std::format!(\"{{value:?}}\")), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Str)),
        ),
        (Type::PhysicalKey, "native_platform") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Unidentified) => ::std::option::Option::Some(\"unidentified\".to_owned()), ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Android(_)) => ::std::option::Option::Some(\"android\".to_owned()), ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::MacOS(_)) => ::std::option::Option::Some(\"macos\".to_owned()), ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Windows(_)) => ::std::option::Option::Some(\"windows\".to_owned()), ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Xkb(_)) => ::std::option::Option::Some(\"xkb\".to_owned()), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Str)),
        ),
        (Type::PhysicalKey, "native_code") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::Android(value) | ::iced::keyboard::key::NativeCode::Xkb(value)) => ::std::option::Option::Some(i64::from(*value)), ::iced::keyboard::key::Physical::Unidentified(::iced::keyboard::key::NativeCode::MacOS(value) | ::iced::keyboard::key::NativeCode::Windows(value)) => ::std::option::Option::Some(i64::from(*value)), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::I64)),
        ),
        (Type::KeyLocation, "name") => (
            format!(
                "match &({code}) {{ ::iced::keyboard::Location::Standard => \"standard\", ::iced::keyboard::Location::Left => \"left\", ::iced::keyboard::Location::Right => \"right\", ::iced::keyboard::Location::Numpad => \"numpad\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::KeyModifiers, field) => {
            let method = match field {
                "shift" => "shift",
                "control" => "control",
                "alt" => "alt",
                "logo" => "logo",
                "command" => "command",
                "jump" => "jump",
                "macos_command" => "macos_command",
                _ => return None,
            };
            (format!("({code}).{method}()"), Type::Bool)
        }
        (Type::Pixels | Type::Degrees | Type::Radians, "value") => {
            (format!("({code}).0 as f64"), Type::F64)
        }
        (Type::Padding, "top" | "right" | "bottom" | "left") => {
            (format!("({code}).{field} as f64"), Type::F64)
        }
        (Type::Padding, "x" | "y") => (format!("({code}).{field}() as f64"), Type::F64),
        (Type::Radians, "display") => (format!("::std::format!(\"{{}}\", {code})"), Type::Str),
        (Type::Point, "x" | "y")
        | (Type::Vector, "x" | "y")
        | (Type::Size, "width" | "height")
        | (Type::Rectangle, "x" | "y" | "width" | "height") => {
            (format!("({code}).{field} as f64"), Type::F64)
        }
        (Type::PointU32, "x" | "y") | (Type::RectangleU32, "x" | "y" | "width" | "height") => {
            (format!("({code}).{field} as i64"), Type::I64)
        }
        (Type::SizeU32, "width" | "height") => (format!("({code}).{field} as i64"), Type::I64),
        (Type::ImageAllocation, "handle") => (format!("({code}).handle().clone()"), Type::Image),
        (Type::ImageAllocation, "size") => (format!("({code}).size()"), Type::SizeU32),
        (Type::ImageError, "kind") => (
            format!(
                "match &({code}) {{ ::iced::widget::image::Error::Invalid(_) => \"invalid\", ::iced::widget::image::Error::Inaccessible(_) => \"inaccessible\", ::iced::widget::image::Error::Unsupported => \"unsupported\", ::iced::widget::image::Error::Empty => \"empty\", ::iced::widget::image::Error::OutOfMemory => \"out-of-memory\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::ImageError, "message") => {
            (format!("::std::format!(\"{{}}\", &({code}))"), Type::Str)
        }
        (Type::Rotation, "radians") => (format!("({code}).radians()"), Type::Radians),
        (Type::Rotation, "degrees") => (format!("({code}).degrees()"), Type::Degrees),
        (Type::Rotation, "kind") => (
            format!(
                "match ({code}) {{ ::iced::Rotation::Floating(_) => \"floating\", ::iced::Rotation::Solid(_) => \"solid\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::ContentFit, "kind") => (
            format!(
                "match ({code}) {{ ::iced::ContentFit::Contain => \"contain\", ::iced::ContentFit::Cover => \"cover\", ::iced::ContentFit::Fill => \"fill\", ::iced::ContentFit::None => \"none\", ::iced::ContentFit::ScaleDown => \"scale-down\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::ContentFit, "display") => (format!("::std::format!(\"{{}}\", ({code}))"), Type::Str),
        (Type::Color, "r" | "g" | "b" | "a") => (format!("({code}).{field} as f64"), Type::F64),
        (Type::Color, "luminance") => (format!("({code}).relative_luminance() as f64"), Type::F64),
        (Type::Color, "rgba8") => (
            format!(
                "({code}).into_rgba8().into_iter().map(i64::from).collect::<::std::vec::Vec<_>>()"
            ),
            Type::List(Box::new(Type::I64)),
        ),
        (Type::Color, "linear") => (
            format!(
                "({code}).into_linear().into_iter().map(f64::from).collect::<::std::vec::Vec<_>>()"
            ),
            Type::List(Box::new(Type::F64)),
        ),
        (Type::Color, "display") => (format!("::std::format!(\"{{}}\", ({code}))"), Type::Str),
        (Type::Background, "kind") => (
            format!(
                "match ({code}) {{ ::iced::Background::Color(_) => \"color\", ::iced::Background::Gradient(_) => \"gradient\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::Background, "color") => (
            format!(
                "match ({code}) {{ ::iced::Background::Color(__value) => ::std::option::Option::Some(__value), ::iced::Background::Gradient(_) => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Color)),
        ),
        (Type::Background, "gradient") => (
            format!(
                "match ({code}) {{ ::iced::Background::Gradient(__value) => ::std::option::Option::Some(__value), ::iced::Background::Color(_) => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Gradient)),
        ),
        (Type::Gradient, "kind") => (
            format!("match ({code}) {{ ::iced::Gradient::Linear(_) => \"linear\" }}.to_owned()"),
            Type::Str,
        ),
        (Type::Gradient, "linear") => (
            format!("match ({code}) {{ ::iced::Gradient::Linear(__value) => __value }}"),
            Type::LinearGradient,
        ),
        (Type::LinearGradient, "angle") => (format!("({code}).angle"), Type::Radians),
        (Type::LinearGradient, "stops") => (
            format!(
                "({code}).stops.into_iter().collect::<::std::vec::Vec<::std::option::Option<::iced::gradient::ColorStop>>>()"
            ),
            Type::List(Box::new(Type::Option(Box::new(Type::ColorStop)))),
        ),
        (Type::ColorStop, "offset") => (format!("({code}).offset as f64"), Type::F64),
        (Type::ColorStop, "color") => (format!("({code}).color"), Type::Color),
        (Type::Font, "family") => (format!("({code}).family"), Type::FontFamily),
        (Type::Font, "weight") => (format!("({code}).weight"), Type::FontWeight),
        (Type::Font, "stretch") => (format!("({code}).stretch"), Type::FontStretch),
        (Type::Font, "style") => (format!("({code}).style"), Type::FontStyle),
        (Type::FontFamily, "kind") => (
            format!(
                "match ({code}) {{ ::iced::font::Family::Name(_) => \"named\", ::iced::font::Family::Serif => \"serif\", ::iced::font::Family::SansSerif => \"sans-serif\", ::iced::font::Family::Cursive => \"cursive\", ::iced::font::Family::Fantasy => \"fantasy\", ::iced::font::Family::Monospace => \"monospace\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::FontFamily, "name") => (
            format!(
                "match ({code}) {{ ::iced::font::Family::Name(__value) => ::std::option::Option::Some(__value.to_owned()), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Str)),
        ),
        (Type::FontWeight, "kind") => (
            format!(
                "match ({code}) {{ ::iced::font::Weight::Thin => \"thin\", ::iced::font::Weight::ExtraLight => \"extra-light\", ::iced::font::Weight::Light => \"light\", ::iced::font::Weight::Normal => \"normal\", ::iced::font::Weight::Medium => \"medium\", ::iced::font::Weight::Semibold => \"semibold\", ::iced::font::Weight::Bold => \"bold\", ::iced::font::Weight::ExtraBold => \"extra-bold\", ::iced::font::Weight::Black => \"black\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::FontStretch, "kind") => (
            format!(
                "match ({code}) {{ ::iced::font::Stretch::UltraCondensed => \"ultra-condensed\", ::iced::font::Stretch::ExtraCondensed => \"extra-condensed\", ::iced::font::Stretch::Condensed => \"condensed\", ::iced::font::Stretch::SemiCondensed => \"semi-condensed\", ::iced::font::Stretch::Normal => \"normal\", ::iced::font::Stretch::SemiExpanded => \"semi-expanded\", ::iced::font::Stretch::Expanded => \"expanded\", ::iced::font::Stretch::ExtraExpanded => \"extra-expanded\", ::iced::font::Stretch::UltraExpanded => \"ultra-expanded\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::FontStyle, "kind") => (
            format!(
                "match ({code}) {{ ::iced::font::Style::Normal => \"normal\", ::iced::font::Style::Italic => \"italic\", ::iced::font::Style::Oblique => \"oblique\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::ThemeMode, "kind") => (
            format!(
                "match ({code}) {{ ::iced::theme::Mode::None => \"none\", ::iced::theme::Mode::Light => \"light\", ::iced::theme::Mode::Dark => \"dark\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::TextAlignment, "kind") => (
            format!(
                "match ({code}) {{ ::iced::widget::text::Alignment::Default => \"default\", ::iced::widget::text::Alignment::Left => \"left\", ::iced::widget::text::Alignment::Center => \"center\", ::iced::widget::text::Alignment::Right => \"right\", ::iced::widget::text::Alignment::Justified => \"justified\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::TextShaping, "kind") => (
            format!(
                "match ({code}) {{ ::iced::widget::text::Shaping::Auto => \"auto\", ::iced::widget::text::Shaping::Basic => \"basic\", ::iced::widget::text::Shaping::Advanced => \"advanced\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::TextWrapping, "kind") => (
            format!(
                "match ({code}) {{ ::iced::widget::text::Wrapping::None => \"none\", ::iced::widget::text::Wrapping::Word => \"word\", ::iced::widget::text::Wrapping::Glyph => \"glyph\", ::iced::widget::text::Wrapping::WordOrGlyph => \"word-or-glyph\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::TextLineHeight, "kind") => (
            format!(
                "match ({code}) {{ ::iced::widget::text::LineHeight::Relative(_) => \"relative\", ::iced::widget::text::LineHeight::Absolute(_) => \"absolute\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::TextLineHeight, "relative") => (
            format!(
                "match ({code}) {{ ::iced::widget::text::LineHeight::Relative(__value) => ::std::option::Option::Some(__value as f64), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::F64)),
        ),
        (Type::TextLineHeight, "absolute") => (
            format!(
                "match ({code}) {{ ::iced::widget::text::LineHeight::Absolute(__value) => ::std::option::Option::Some(__value), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Pixels)),
        ),
        (Type::MouseInteraction, "kind") => (
            format!(
                "match ({code}) {{ ::iced::mouse::Interaction::None => \"none\", ::iced::mouse::Interaction::Hidden => \"hidden\", ::iced::mouse::Interaction::Idle => \"idle\", ::iced::mouse::Interaction::ContextMenu => \"context-menu\", ::iced::mouse::Interaction::Help => \"help\", ::iced::mouse::Interaction::Pointer => \"pointer\", ::iced::mouse::Interaction::Progress => \"progress\", ::iced::mouse::Interaction::Wait => \"wait\", ::iced::mouse::Interaction::Cell => \"cell\", ::iced::mouse::Interaction::Crosshair => \"crosshair\", ::iced::mouse::Interaction::Text => \"text\", ::iced::mouse::Interaction::Alias => \"alias\", ::iced::mouse::Interaction::Copy => \"copy\", ::iced::mouse::Interaction::Move => \"move\", ::iced::mouse::Interaction::NoDrop => \"no-drop\", ::iced::mouse::Interaction::NotAllowed => \"not-allowed\", ::iced::mouse::Interaction::Grab => \"grab\", ::iced::mouse::Interaction::Grabbing => \"grabbing\", ::iced::mouse::Interaction::ResizingHorizontally => \"resize-horizontal\", ::iced::mouse::Interaction::ResizingVertically => \"resize-vertical\", ::iced::mouse::Interaction::ResizingDiagonallyUp => \"resize-diagonal-up\", ::iced::mouse::Interaction::ResizingDiagonallyDown => \"resize-diagonal-down\", ::iced::mouse::Interaction::ResizingColumn => \"resize-column\", ::iced::mouse::Interaction::ResizingRow => \"resize-row\", ::iced::mouse::Interaction::AllScroll => \"all-scroll\", ::iced::mouse::Interaction::ZoomIn => \"zoom-in\", ::iced::mouse::Interaction::ZoomOut => \"zoom-out\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::ScrollDelta, "kind") => (
            format!(
                "match ({code}) {{ ::iced::mouse::ScrollDelta::Lines {{ .. }} => \"lines\", ::iced::mouse::ScrollDelta::Pixels {{ .. }} => \"pixels\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::ScrollDelta, "x") => (
            format!(
                "match ({code}) {{ ::iced::mouse::ScrollDelta::Lines {{ x, .. }} | ::iced::mouse::ScrollDelta::Pixels {{ x, .. }} => x as f64 }}"
            ),
            Type::F64,
        ),
        (Type::ScrollDelta, "y") => (
            format!(
                "match ({code}) {{ ::iced::mouse::ScrollDelta::Lines {{ y, .. }} | ::iced::mouse::ScrollDelta::Pixels {{ y, .. }} => y as f64 }}"
            ),
            Type::F64,
        ),
        (Type::EventStatus, "kind") => (
            format!(
                "match ({code}) {{ ::iced::event::Status::Ignored => \"ignored\", ::iced::event::Status::Captured => \"captured\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::RedrawRequest, "kind") => (
            format!(
                "match ({code}) {{ ::iced::window::RedrawRequest::NextFrame => \"next-frame\", ::iced::window::RedrawRequest::At(_) => \"at\", ::iced::window::RedrawRequest::Wait => \"wait\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::RedrawRequest, "instant") => (
            format!(
                "match ({code}) {{ ::iced::window::RedrawRequest::At(__value) => ::std::option::Option::Some(__value), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Instant)),
        ),
        (Type::WindowId, "display") => (format!("({code}).to_string()"), Type::Str),
        (Type::WindowScreenshot, "rgba") => (format!("({code}).rgba.to_vec()"), Type::Bytes),
        (Type::WindowScreenshot, "size") => (format!("({code}).size"), Type::SizeU32),
        (Type::WindowScreenshot, "scale_factor") => {
            (format!("({code}).scale_factor as f64"), Type::F64)
        }
        (Type::WindowScreenshot, "debug") => {
            (format!("::std::format!(\"{{:?}}\", &({code}))"), Type::Str)
        }
        (Type::WindowPosition, "kind") => (
            format!(
                "match ({code}) {{ ::iced::window::Position::Default => \"default\", ::iced::window::Position::Centered => \"centered\", ::iced::window::Position::Specific(_) => \"specific\", ::iced::window::Position::SpecificWith(_) => \"specific-with\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::WindowPosition, "point") => (
            format!(
                "match ({code}) {{ ::iced::window::Position::Specific(__value) => ::std::option::Option::Some(__value), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::Point)),
        ),
        (Type::WindowDirection, "kind") => (
            format!(
                "match ({code}) {{ ::iced::window::Direction::North => \"north\", ::iced::window::Direction::South => \"south\", ::iced::window::Direction::East => \"east\", ::iced::window::Direction::West => \"west\", ::iced::window::Direction::NorthEast => \"north-east\", ::iced::window::Direction::NorthWest => \"north-west\", ::iced::window::Direction::SouthEast => \"south-east\", ::iced::window::Direction::SouthWest => \"south-west\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::WindowLevel, "kind") => (
            format!(
                "match ({code}) {{ ::iced::window::Level::Normal => \"normal\", ::iced::window::Level::AlwaysOnBottom => \"always-on-bottom\", ::iced::window::Level::AlwaysOnTop => \"always-on-top\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::WindowMode, "kind") => (
            format!(
                "match ({code}) {{ ::iced::window::Mode::Windowed => \"windowed\", ::iced::window::Mode::Fullscreen => \"fullscreen\", ::iced::window::Mode::Hidden => \"hidden\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::WindowAttention, "kind") => (
            format!(
                "match ({code}) {{ ::iced::window::UserAttention::Critical => \"critical\", ::iced::window::UserAttention::Informational => \"informational\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::Length, "fill_factor") => (format!("({code}).fill_factor() as i64"), Type::I64),
        (Type::Length, "is_fill") => (format!("({code}).is_fill()"), Type::Bool),
        (Type::Length, "kind") => (
            format!(
                "match ({code}) {{ ::iced::Length::Fill => \"fill\", ::iced::Length::FillPortion(_) => \"fill-portion\", ::iced::Length::Shrink => \"shrink\", ::iced::Length::Fixed(_) => \"fixed\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::Length, "portion") => (
            format!(
                "match ({code}) {{ ::iced::Length::FillPortion(__value) => ::std::option::Option::Some(__value as i64), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::I64)),
        ),
        (Type::Length, "fixed") => (
            format!(
                "match ({code}) {{ ::iced::Length::Fixed(__value) => ::std::option::Option::Some(__value as f64), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::F64)),
        ),
        (Type::Alignment, "kind") => (
            format!(
                "match ({code}) {{ ::iced::Alignment::Start => \"start\", ::iced::Alignment::Center => \"center\", ::iced::Alignment::End => \"end\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::HorizontalAlignment, "kind") => (
            format!(
                "match ({code}) {{ ::iced::alignment::Horizontal::Left => \"left\", ::iced::alignment::Horizontal::Center => \"center\", ::iced::alignment::Horizontal::Right => \"right\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::VerticalAlignment, "kind") => (
            format!(
                "match ({code}) {{ ::iced::alignment::Vertical::Top => \"top\", ::iced::alignment::Vertical::Center => \"center\", ::iced::alignment::Vertical::Bottom => \"bottom\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::Border, "color") => (format!("({code}).color"), Type::Color),
        (Type::Border, "width") => (format!("({code}).width as f64"), Type::F64),
        (Type::Border, "radius") => (format!("({code}).radius"), Type::Radius),
        (Type::Radius, "top_left" | "top_right" | "bottom_right" | "bottom_left") => {
            (format!("({code}).{field} as f64"), Type::F64)
        }
        (Type::Radius, "values") => (
            format!(
                "::std::convert::Into::<[f32; 4]>::into({code}).into_iter().map(f64::from).collect::<::std::vec::Vec<_>>()"
            ),
            Type::List(Box::new(Type::F64)),
        ),
        (Type::Shadow, "color") => (format!("({code}).color"), Type::Color),
        (Type::Shadow, "offset") => (format!("({code}).offset"), Type::Vector),
        (Type::Shadow, "blur") => (format!("({code}).blur_radius as f64"), Type::F64),
        (Type::Point | Type::Vector | Type::Size, "values") => (
            format!(
                "::std::convert::Into::<[f32; 2]>::into({code}).into_iter().map(f64::from).collect::<::std::vec::Vec<_>>()"
            ),
            Type::List(Box::new(Type::F64)),
        ),
        (Type::Point, "display") => (format!("::std::format!(\"{{}}\", {code})"), Type::Str),
        (Type::Rectangle, "center") => (format!("({code}).center()"), Type::Point),
        (Type::Rectangle, "center_x") => (format!("({code}).center_x() as f64"), Type::F64),
        (Type::Rectangle, "center_y") => (format!("({code}).center_y() as f64"), Type::F64),
        (Type::Rectangle, "position") => (format!("({code}).position()"), Type::Point),
        (Type::Rectangle, "size") => (format!("({code}).size()"), Type::Size),
        (Type::Rectangle, "area") => (format!("({code}).area() as f64"), Type::F64),
        (Type::Transformation, "scale_factor") => {
            (format!("({code}).scale_factor() as f64"), Type::F64)
        }
        (Type::Transformation, "translation") => (format!("({code}).translation()"), Type::Vector),
        (Type::Transformation, "matrix") => (
            format!(
                "::std::convert::Into::<[f32; 16]>::into({code}).into_iter().map(f64::from).collect::<::std::vec::Vec<_>>()"
            ),
            Type::List(Box::new(Type::F64)),
        ),
        (Type::MouseButton, "kind") => (
            format!(
                "match &({code}) {{ ::iced::mouse::Button::Left => \"left\", ::iced::mouse::Button::Right => \"right\", ::iced::mouse::Button::Middle => \"middle\", ::iced::mouse::Button::Back => \"back\", ::iced::mouse::Button::Forward => \"forward\", ::iced::mouse::Button::Other(_) => \"other\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::MouseButton, "number") => (
            format!(
                "match &({code}) {{ ::iced::mouse::Button::Other(value) => ::std::option::Option::Some(i64::from(*value)), _ => ::std::option::Option::None }}"
            ),
            Type::Option(Box::new(Type::I64)),
        ),
        (Type::MouseCursor, "kind") => (
            format!(
                "match &({code}) {{ ::iced::mouse::Cursor::Available(_) => \"available\", ::iced::mouse::Cursor::Levitating(_) => \"levitating\", ::iced::mouse::Cursor::Unavailable => \"unavailable\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::MouseCursor, "position") => (
            format!("({code}).position()"),
            Type::Option(Box::new(Type::Point)),
        ),
        (Type::MouseCursor, "levitating") => (format!("({code}).is_levitating()"), Type::Bool),
        (Type::MouseClick, "kind") => (
            format!(
                "match ({code}).kind() {{ ::iced::advanced::mouse::click::Kind::Single => \"single\", ::iced::advanced::mouse::click::Kind::Double => \"double\", ::iced::advanced::mouse::click::Kind::Triple => \"triple\" }}.to_owned()"
            ),
            Type::Str,
        ),
        (Type::MouseClick, "position") => (format!("({code}).position()"), Type::Point),
        (Type::TouchFinger, "id") => (format!("({code}).0.to_string()"), Type::Str),
        _ => return None,
    };
    Some(projected)
}
