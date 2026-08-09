use super::*;

pub(in crate::codegen) struct Binding {
    pub(in crate::codegen) code: String,
    pub(in crate::codegen) ty: Type,
    pub(in crate::codegen) local: bool,
    pub(in crate::codegen) state: Option<StateBinding>,
    pub(in crate::codegen) owner: Option<BindingOwner>,
}

impl Clone for Binding {
    fn clone(&self) -> Self {
        #[cfg(test)]
        record_binding_clone();
        Self {
            code: self.code.clone(),
            ty: self.ty.clone(),
            local: self.local,
            state: self.state.clone(),
            owner: self.owner,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::codegen) enum BindingOwner {
    Value(ResolvedValueRef),
    Local(ResolvedLocalId),
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
    pub(in crate::codegen) slot: ComponentSlotId,
    pub(in crate::codegen) name: String,
    pub(in crate::codegen) view: ViewId,
    pub(in crate::codegen) env: HashMap<String, Binding>,
    /// The recorder that stood at the call site, so reads the content makes
    /// once it renders inside the callee still reach it.
    pub(in crate::codegen) recorder: Option<std::rc::Rc<RecordingSink>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::codegen) enum ValueMode {
    Owned,
    Borrowed,
    /// A borrow consumed while building the current Rust expression or loop.
    /// Unlike `Borrowed`, it cannot be retained by the returned element.
    TransientBorrowed,
}

const COMPONENT_CONTEXT_PREFIX: &str = "\0component:";
const COMPONENT_CONTEXT_INDEX: &str = "\0component-context";
const COMPONENT_OUTPUT_PREFIX: &str = "\0component-output:";
const COMPONENT_EVENT_PREFIX: &str = "\0component-event:";
const DERIVED_TRANSIENT_PREFIX: &str = "\0derived-transient:";
pub(in crate::codegen) const DERIVED_SNAPSHOT_BINDING: &str = "\0derived-snapshot";

pub(in crate::codegen) fn derived_transient_key(name: &str) -> String {
    format!("{DERIVED_TRANSIENT_PREFIX}{name}")
}

pub(in crate::codegen) fn is_derived_transient_key(name: &str) -> bool {
    name.starts_with(DERIVED_TRANSIENT_PREFIX)
}

pub(in crate::codegen) fn component_context_key(component: &str) -> String {
    format!("{COMPONENT_CONTEXT_PREFIX}{component}")
}

/// The reserved key that marks a component argument binding as self-backed:
/// its use-site expression resolved only `self`-rooted bindings, plus the
/// scope-local identifiers listed (comma-joined) in the marker's `code`.
/// [`RecordingEnv`] reads it to decide whether a nested component use can
/// outline. [`RecordingEnv`]: super::RecordingEnv
pub(in crate::codegen) fn self_backed_param_key(name: &str) -> String {
    format!("\0self-backed:{name}")
}

pub(in crate::codegen) fn is_self_backed_param_key(name: &str) -> bool {
    name.starts_with("\0self-backed:")
}

/// The reserved key that carries an outlined-method parameter type for a
/// callback binding (`impl Fn(payloads) -> Message + Clone + 'static`),
/// inserted beside the callback itself so a nested use can take the callback
/// as a typed parameter instead of hard-capturing its caller-built closure.
/// The reserved key that marks a component argument as a VALUE parameter of
/// the enclosing outlined method: its expression referenced render-site
/// locals (loop items, a daemon window id), so the baked binding code is the
/// method parameter's identifier rather than the original expression. A
/// nested use reading such a prop treats it like a local value of its own
/// render site and can parameterize it onward.
pub(in crate::codegen) fn value_param_key(name: &str) -> String {
    format!("\0value-param:{name}")
}

pub(in crate::codegen) fn is_value_param_key(name: &str) -> bool {
    name.starts_with("\0value-param:")
}

pub(in crate::codegen) fn callback_sig_key(callback_key: &str) -> String {
    format!("\0cb-sig:{callback_key}")
}

pub(in crate::codegen) fn is_callback_sig_key(name: &str) -> bool {
    name.starts_with("\0cb-sig:")
}

pub(in crate::codegen) fn is_component_context_index_key(name: &str) -> bool {
    name == COMPONENT_CONTEXT_INDEX
}

pub(in crate::codegen) fn is_component_context_key(name: &str) -> bool {
    name.starts_with(COMPONENT_CONTEXT_PREFIX)
}

pub(in crate::codegen) fn is_component_callback_key(name: &str) -> bool {
    name.starts_with(COMPONENT_OUTPUT_PREFIX) || name.starts_with(COMPONENT_EVENT_PREFIX)
}

pub(in crate::codegen) fn component_context(
    env: &dyn BindingEnvironment,
) -> Option<(&str, &Binding)> {
    let component = env.get(COMPONENT_CONTEXT_INDEX)?.code.as_str();
    env.get(&component_context_key(component))
        .map(|binding| (component, binding))
}

pub(in crate::codegen) fn insert_component_context(
    env: &mut HashMap<String, Binding>,
    component: &str,
    binding: Binding,
) {
    env.insert(
        COMPONENT_CONTEXT_INDEX.into(),
        Binding {
            code: component.to_owned(),
            ty: Type::Unit,
            local: true,
            state: None,
            owner: None,
        },
    );
    env.insert(component_context_key(component), binding);
}

pub(in crate::codegen) fn insert_scoped_component_context(
    env: &mut ScopedBindingEnv<'_>,
    component: &str,
    binding: Binding,
) {
    env.insert(
        COMPONENT_CONTEXT_INDEX.into(),
        Binding {
            code: component.to_owned(),
            ty: Type::Unit,
            local: true,
            state: None,
            owner: None,
        },
    );
    env.insert(component_context_key(component), binding);
}

pub(in crate::codegen) fn component_output_key(component: &str) -> String {
    format!("{COMPONENT_OUTPUT_PREFIX}{component}")
}

pub(in crate::codegen) fn component_output(env: &dyn BindingEnvironment) -> Option<&Binding> {
    env.binding_with_prefix(COMPONENT_OUTPUT_PREFIX)
}

pub(in crate::codegen) fn component_event_key(component: &str, event: &str) -> String {
    format!("{COMPONENT_EVENT_PREFIX}{component}:{event}")
}

pub(in crate::codegen) fn component_event<'a>(
    env: &'a dyn BindingEnvironment,
    component: &str,
    event: &str,
) -> Option<&'a Binding> {
    env.get(&component_event_key(component, event))
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

pub(in crate::codegen) fn run_lane_generation_field(lane: usize) -> String {
    format!("__ice_run_lane_{lane}_generation")
}

pub(in crate::codegen) fn run_lane_handle_field(lane: usize) -> String {
    format!("__ice_run_lane_{lane}_handle")
}

pub(in crate::codegen) fn run_lane_variant(lane: usize) -> String {
    format!("__RequestLane{lane}")
}

pub(in crate::codegen) fn derived_method(name: &str) -> String {
    format!("__ice_derived_{name}")
}

pub(in crate::codegen) fn derived_snapshot_method(name: &str) -> String {
    format!("__ice_snapshot_{name}")
}

pub(in crate::codegen) fn checked_state_env(
    program: &LoweredProgram,
    name: &str,
) -> HashMap<String, Binding> {
    let mut env = program
        .app_states()
        .iter()
        .map(|state| {
            (
                state.name.clone(),
                Binding {
                    code: format!("{name}.{}", state.name),
                    ty: state.ty.clone(),
                    local: false,
                    state: Some(StateBinding::App(state.name.clone())),
                    owner: Some(BindingOwner::Value(ResolvedValueRef::AppState(state.id))),
                },
            )
        })
        .collect::<HashMap<_, _>>();
    env.extend(program.derived().iter().map(|derived| {
        (
            derived.name.clone(),
            Binding {
                code: format!("{name}.{}()", derived_method(&derived.name)),
                ty: derived.ty.clone(),
                local: true,
                state: None,
                owner: Some(BindingOwner::Value(ResolvedValueRef::Derived(derived.id))),
            },
        )
    }));
    env
}

pub(in crate::codegen) fn checked_view_state_env(
    program: &LoweredProgram,
    name: &str,
    snapshot: &str,
    cached: &HashSet<crate::hir::DerivedId>,
) -> HashMap<String, Binding> {
    let mut env = checked_state_env(program, name);
    if cached.is_empty() {
        return env;
    }
    for derived in program
        .derived()
        .iter()
        .filter(|derived| cached.contains(&derived.id))
    {
        let accessor = format!(
            "{name}.{}({snapshot})",
            derived_snapshot_method(&derived.name)
        );
        env.insert(
            derived_transient_key(&derived.name),
            Binding {
                code: format!("(*{accessor})"),
                ty: derived.ty.clone(),
                local: true,
                state: None,
                owner: Some(BindingOwner::Value(ResolvedValueRef::Derived(derived.id))),
            },
        );
    }
    env.insert(
        DERIVED_SNAPSHOT_BINDING.into(),
        Binding {
            code: snapshot.into(),
            ty: Type::Unit,
            local: true,
            state: None,
            owner: None,
        },
    );
    env
}

pub(in crate::codegen) fn native_field_projection(
    ty: &Type,
    field: &str,
    code: &str,
) -> Option<(String, Type)> {
    let projected = match (ty, field) {
        (Type::TestTarget, field) => {
            let ty = match field {
                "kind"
                | "value"
                | "accessibility_role"
                | "accessibility_name"
                | "accessibility_description"
                | "accessibility_value" => Type::Str,
                "visible"
                | "pixel_aligned"
                | "focused"
                | "accessibility_checked"
                | "accessibility_disabled"
                | "accessibility_supports_activate"
                | "accessibility_supports_focus" => Type::Bool,
                "surface_count" | "text_count" | "image_count" => Type::I64,
                "x" | "y" | "width" | "height" | "left" | "top" | "right" | "bottom"
                | "center_x" | "center_y" | "visible_x" | "visible_y" | "visible_width"
                | "visible_height" | "content_x" | "content_y" | "content_width"
                | "content_height" | "scroll_x" | "scroll_y" | "translation_x"
                | "translation_y" | "text_size" | "text_x" | "text_y" | "text_width"
                | "text_height" | "text_baseline" | "image_x" | "image_y" | "image_width"
                | "image_height" => Type::F64,
                "background" => Type::Background,
                "border" => Type::Border,
                "shadow" => Type::Shadow,
                "text_color" => Type::Color,
                "font" => Type::Font,
                "line_height" => Type::TextLineHeight,
                _ => return None,
            };
            let method = if field == "accessibility_role" {
                "accessibility_role_name"
            } else {
                field
            };
            let projected = if matches!(field, "surface_count" | "text_count" | "image_count") {
                format!(
                    "::std::primitive::i64::try_from(({code}).{method}()).expect(\"test target primitive count exceeds i64\")"
                )
            } else {
                format!("({code}).{method}()")
            };
            (projected, ty)
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
                "match {code} {{ ::iced::widget::text::LineHeight::Relative(_) => \"relative\", ::iced::widget::text::LineHeight::Absolute(_) => \"absolute\" }}.to_owned()"
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
