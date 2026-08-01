use crate::ast::*;
pub(crate) use crate::check::CheckedExprUseId;
use crate::check::{
    CheckedComponentArgumentSource, CheckedFacts, CheckedSubscription, CheckedSubscriptionExprRole,
    CheckedSubscriptionSource, CheckedValueRef,
};
use crate::hir::Origin;
pub(crate) use crate::hir::{
    AppStateId, ComponentCallId, ComponentEventId, ComponentId, ComponentParamId, ComponentSlotId,
    ComponentStateId, DeclarationIndex, ExternFnId, ExternRef, HandlerId, NamedTypeId, OriginArena,
    OriginId, PaletteId, SubscriptionId,
};
use crate::{CheckedDocument, Error};
use std::collections::HashMap;
use std::path::Path;

mod style;

pub(crate) use style::*;

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct ComponentParamContract {
    id: ComponentParamId,
    name: String,
    ty: Type,
    capability: ParamCapability,
    default: Option<CheckedExprUseId>,
    origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParamCapability {
    Read,
    Bind,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct ComponentEventContract {
    id: ComponentEventId,
    name: String,
    payloads: Vec<Type>,
    origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct ComponentSlotContract {
    id: ComponentSlotId,
    name: String,
    optional: bool,
    origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ComponentStateContract {
    pub(crate) id: ComponentStateId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) initializer: ResolvedInitializer,
    pub(crate) span: Span,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct AppStateContract {
    pub(crate) id: AppStateId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) initializer: ResolvedInitializer,
    pub(crate) span: Span,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct DerivedContract {
    pub(crate) id: crate::hir::DerivedId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) initializer: CheckedExprUseId,
    pub(crate) span: Span,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedType {
    Value(Type),
    List(Box<ResolvedType>),
    Option(Box<ResolvedType>),
    Result(Box<ResolvedType>, Box<ResolvedType>),
    Combo(Box<ResolvedType>),
    Animation(Box<ResolvedType>),
    Named(NamedTypeId),
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct ResolvedExternContract {
    pub(crate) id: ExternFnId,
    pub(crate) name: String,
    pub(crate) rust_path: String,
    pub(crate) params: Vec<ResolvedType>,
    pub(crate) output: Type,
    pub(crate) error: Option<Type>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedSubscriptionSource {
    Every {
        milliseconds: u64,
    },
    Repeat {
        function: ResolvedExternContract,
        milliseconds: u64,
    },
    Run {
        function: ResolvedExternContract,
        arguments: Vec<CheckedExprUseId>,
    },
    Recipe {
        function: ResolvedExternContract,
        arguments: Vec<CheckedExprUseId>,
    },
    Events {
        identity: CheckedExprUseId,
        filter: ResolvedExternContract,
    },
    Event {
        raw: bool,
    },
    Extern {
        function: ResolvedExternContract,
        arguments: Vec<CheckedExprUseId>,
    },
    InputMethod(InputMethodEvent),
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
    SystemTheme,
    Touch(TouchEvent),
    Window(WindowEvent),
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct ResolvedSubscriptionRoute {
    pub(crate) handler: HandlerId,
    pub(crate) handler_name: String,
    pub(crate) payloads: Vec<u32>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct ResolvedSubscription {
    pub(crate) id: SubscriptionId,
    pub(crate) source: ResolvedSubscriptionSource,
    pub(crate) source_payloads: Vec<Type>,
    pub(crate) delivered_payloads: Vec<Type>,
    pub(crate) filter: Option<ResolvedExternContract>,
    pub(crate) context: Option<CheckedExprUseId>,
    pub(crate) condition: Option<CheckedExprUseId>,
    pub(crate) window_id: bool,
    pub(crate) status: Option<EventStatus>,
    pub(crate) route: ResolvedSubscriptionRoute,
    pub(crate) span: Span,
    pub(crate) origin: OriginId,
}

fn subscription_source_matches(
    checked: &CheckedSubscriptionSource,
    raw: &SubscriptionSource,
) -> bool {
    match (checked, raw) {
        (
            CheckedSubscriptionSource::Every { milliseconds },
            SubscriptionSource::Every {
                milliseconds: raw_milliseconds,
            },
        ) => milliseconds == raw_milliseconds,
        (
            CheckedSubscriptionSource::Repeat {
                function,
                milliseconds,
            },
            SubscriptionSource::Repeat {
                function: raw_function,
                milliseconds: raw_milliseconds,
            },
        ) => function.name == *raw_function && milliseconds == raw_milliseconds,
        (
            CheckedSubscriptionSource::Run {
                function,
                arguments,
            },
            SubscriptionSource::Run {
                function: raw_function,
                args,
            },
        )
        | (
            CheckedSubscriptionSource::Recipe {
                function,
                arguments,
            },
            SubscriptionSource::Recipe {
                function: raw_function,
                args,
            },
        )
        | (
            CheckedSubscriptionSource::Extern {
                function,
                arguments,
            },
            SubscriptionSource::Extern {
                function: raw_function,
                args,
            },
        ) => function.name == *raw_function && arguments.len() == args.len(),
        (
            CheckedSubscriptionSource::Events { filter, .. },
            SubscriptionSource::Events {
                filter: raw_filter, ..
            },
        ) => filter.name == *raw_filter,
        (
            CheckedSubscriptionSource::Event { raw: checked_raw },
            SubscriptionSource::Event { raw },
        ) => checked_raw == raw,
        (
            CheckedSubscriptionSource::InputMethod(event),
            SubscriptionSource::InputMethod(raw_event),
        ) => event == raw_event,
        (CheckedSubscriptionSource::Keyboard(event), SubscriptionSource::Keyboard(raw_event)) => {
            event == raw_event
        }
        (CheckedSubscriptionSource::Mouse(event), SubscriptionSource::Mouse(raw_event)) => {
            event == raw_event
        }
        (CheckedSubscriptionSource::SystemTheme, SubscriptionSource::SystemTheme) => true,
        (CheckedSubscriptionSource::Touch(event), SubscriptionSource::Touch(raw_event)) => {
            event == raw_event
        }
        (CheckedSubscriptionSource::Window(event), SubscriptionSource::Window(raw_event)) => {
            event == raw_event
        }
        _ => false,
    }
}

fn extern_subscription_payload(function: &crate::hir::ExternDeclaration) -> Type {
    function.error.as_ref().map_or_else(
        || function.output.clone(),
        |error| Type::Result(Box::new(function.output.clone()), Box::new(error.clone())),
    )
}

fn resolved_native_subscription_payloads(
    source: &CheckedSubscriptionSource,
    window_id: bool,
) -> Option<Vec<Type>> {
    let mut payloads = match source {
        CheckedSubscriptionSource::Every { .. } => vec![Type::Instant],
        CheckedSubscriptionSource::Event { .. } => vec![Type::Event],
        CheckedSubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened | InputMethodEvent::Closed => Vec::new(),
            InputMethodEvent::Preedit => vec![
                Type::Str,
                Type::Option(Box::new(Type::I64)),
                Type::Option(Box::new(Type::I64)),
            ],
            InputMethodEvent::Commit => vec![Type::Str],
        },
        CheckedSubscriptionSource::Keyboard(KeyboardEvent::Press) => vec![Type::KeyPress],
        CheckedSubscriptionSource::Keyboard(KeyboardEvent::Release) => vec![Type::KeyRelease],
        CheckedSubscriptionSource::Keyboard(KeyboardEvent::Modifiers) => vec![Type::KeyModifiers],
        CheckedSubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered | MouseEvent::Left => Vec::new(),
            MouseEvent::Moved => vec![Type::F64, Type::F64],
            MouseEvent::Pressed | MouseEvent::Released => vec![Type::MouseButton],
            MouseEvent::Wheel => vec![Type::F64, Type::F64, Type::Bool],
        },
        CheckedSubscriptionSource::SystemTheme => vec![Type::Str],
        CheckedSubscriptionSource::Touch(_) => {
            vec![Type::TouchFinger, Type::F64, Type::F64]
        }
        CheckedSubscriptionSource::Window(event) => match event {
            WindowEvent::Frame
            | WindowEvent::Closed
            | WindowEvent::CloseRequested
            | WindowEvent::Focused
            | WindowEvent::Unfocused
            | WindowEvent::FilesHoveredLeft => Vec::new(),
            WindowEvent::Opened => vec![
                Type::Option(Box::new(Type::F64)),
                Type::Option(Box::new(Type::F64)),
                Type::F64,
                Type::F64,
            ],
            WindowEvent::Moved | WindowEvent::Resized => vec![Type::F64, Type::F64],
            WindowEvent::Rescaled => vec![Type::F64],
            WindowEvent::FileHovered | WindowEvent::FileDropped => vec![Type::Str],
        },
        _ => return None,
    };
    if window_id {
        if !matches!(
            source,
            CheckedSubscriptionSource::Event { .. } | CheckedSubscriptionSource::Window(_)
        ) {
            return None;
        }
        payloads.insert(0, Type::WindowId);
    }
    Some(payloads)
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedInitializer {
    pub(crate) expression: CheckedExprUseId,
    pub(crate) animation: Option<ResolvedAnimation>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedAnimation {
    pub(crate) easing: Option<ResolvedAnimationEasing>,
    pub(crate) duration: Option<AnimationDuration>,
    pub(crate) delay_ms: Option<u64>,
    pub(crate) repeat: Option<u32>,
    pub(crate) repeat_forever: bool,
    pub(crate) auto_reverse: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedAnimationEasing {
    Builtin(String),
    Custom(ExternFnId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComponentStorage {
    Stateless,
    Retained,
    Mounted,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct ComponentContract {
    pub(crate) id: ComponentId,
    pub(crate) name: String,
    params: Vec<ComponentParamContract>,
    pub(crate) output: Type,
    events: Vec<ComponentEventContract>,
    slots: Vec<ComponentSlotContract>,
    pub(crate) states: Vec<ComponentStateContract>,
    pub(crate) handlers: Vec<Handler>,
    pub(crate) root: ViewNode,
    pub(crate) storage: ComponentStorage,
    pub(crate) origin: OriginId,
}

#[derive(Debug)]
struct ComponentIndex {
    params_by_name: HashMap<String, usize>,
    events_by_name: HashMap<String, usize>,
    slots_by_name: HashMap<String, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArgumentScope {
    Caller,
    Definition,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum WritableStateRef {
    App { id: AppStateId, name: String },
    ComponentParam { id: ComponentParamId, name: String },
    ComponentState { id: ComponentStateId, name: String },
}

impl WritableStateRef {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::App { name, .. }
            | Self::ComponentParam { name, .. }
            | Self::ComponentState { name, .. } => name,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedArgument {
    pub(crate) param: ComponentParamId,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) expression: CheckedExprUseId,
    scope: ArgumentScope,
    pub(crate) writable: Option<WritableStateRef>,
    origin: OriginId,
}

impl ResolvedArgument {
    pub(crate) fn uses_definition_scope(&self) -> bool {
        self.scope == ArgumentScope::Definition
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum ResolvedEventRoute {
    Direct {
        event: ComponentEventId,
        name: String,
        payloads: Vec<Type>,
        route: Route,
        origin: OriginId,
    },
    Forward {
        event: ComponentEventId,
        name: String,
        payloads: Vec<Type>,
        outer_component: ComponentId,
        outer_event: ComponentEventId,
        origin: OriginId,
    },
}

impl ResolvedEventRoute {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Direct { name, .. } | Self::Forward { name, .. } => name,
        }
    }

    pub(crate) fn payloads(&self) -> &[Type] {
        match self {
            Self::Direct { payloads, .. } | Self::Forward { payloads, .. } => payloads,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedSlot {
    slot: ComponentSlotId,
    pub(crate) name: String,
    pub(crate) optional: bool,
    pub(crate) content: Option<ViewNode>,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum ComponentScope {
    Explicit {
        id: Id,
        origin: OriginId,
    },
    Implicit {
        component: ComponentId,
        call_site: usize,
        origin: OriginId,
    },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum ComponentOutputRoute {
    None,
    Direct {
        output: Type,
        route: Route,
        origin: OriginId,
    },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ComponentCall {
    id: ComponentCallId,
    pub(crate) component: ComponentId,
    pub(crate) origin: OriginId,
    pub(crate) arguments: Vec<ResolvedArgument>,
    pub(crate) events: Vec<ResolvedEventRoute>,
    pub(crate) slots: Vec<ResolvedSlot>,
    pub(crate) output: ComponentOutputRoute,
    pub(crate) scope: ComponentScope,
    pub(crate) storage: ComponentStorage,
    pub(crate) binding_site: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CallSite {
    line: usize,
    column: usize,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct LoweredProgram {
    document: Document,
    daemon: bool,
    facts: CheckedFacts,
    declarations: DeclarationIndex,
    subscriptions: Vec<ResolvedSubscription>,
    named_type_rust_paths: HashMap<NamedTypeId, String>,
    app_states: Vec<AppStateContract>,
    derived: Vec<DerivedContract>,
    components: Vec<ComponentContract>,
    calls: Vec<ComponentCall>,
    calls_by_site: HashMap<CallSite, ComponentCallId>,
    styles: StyleProgram,
    origins: OriginArena,
}

impl LoweredProgram {
    pub(crate) fn document(&self) -> &Document {
        &self.document
    }

    pub(crate) fn daemon(&self) -> bool {
        self.daemon
    }

    #[allow(dead_code)]
    pub(crate) fn checked_facts(&self) -> &CheckedFacts {
        &self.facts
    }

    pub(crate) fn subscriptions(&self) -> &[ResolvedSubscription] {
        &self.subscriptions
    }

    pub(crate) fn named_type_rust_path(&self, id: NamedTypeId) -> Option<&str> {
        self.named_type_rust_paths.get(&id).map(String::as_str)
    }

    pub(crate) fn declarations(&self) -> &DeclarationIndex {
        &self.declarations
    }

    pub(crate) fn app_states(&self) -> &[AppStateContract] {
        &self.app_states
    }

    pub(crate) fn derived(&self) -> &[DerivedContract] {
        &self.derived
    }

    pub(crate) fn components(&self) -> &[ComponentContract] {
        &self.components
    }

    pub(crate) fn component(&self, id: ComponentId) -> &ComponentContract {
        &self.components[id.0 as usize]
    }

    pub(crate) fn component_slot_name(&self, id: ComponentSlotId) -> Result<&str, Error> {
        self.components
            .get(id.component.0 as usize)
            .and_then(|component| component.slots.get(id.index as usize))
            .map(|slot| slot.name.as_str())
            .ok_or_else(|| {
                Error::new(
                    "E196",
                    &Span::line(1),
                    "checked slot expression references an invalid slot ID",
                )
            })
    }

    pub(crate) fn component_call(&self, span: &Span) -> Result<&ComponentCall, Error> {
        let site = CallSite {
            line: span.line,
            column: span.column,
        };
        let id = self.calls_by_site.get(&site).ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "component call reached code generation without a lowered call",
            )
        })?;
        self.calls.get(id.0 as usize).ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "component call references an invalid lowered call ID",
            )
        })
    }

    pub(crate) fn checked_view(&self, span: &Span) -> Result<&crate::check::CheckedView, Error> {
        let id = self.declarations.view_id(span).ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "view reached code generation without a shared view ID",
            )
        })?;
        let view = self.facts.view(id);
        if view.id != id {
            return Err(Error::new(
                "E196",
                span,
                "view reached code generation with a mismatched checked view ID",
            ));
        }
        Ok(view)
    }

    pub(crate) fn validate_checked_view(&self, node: &ViewNode) -> Result<(), Error> {
        let span = node.span();
        let view = self.checked_view(span)?;
        let id = view.id;
        if view.kind != crate::hir::view_kind(node) {
            return Err(Error::new(
                "E196",
                span,
                "raw view kind diverged from its checked topology",
            ));
        }
        let children = crate::hir::view_children(node)
            .into_iter()
            .map(|child| {
                self.declarations.view_id(child.span()).ok_or_else(|| {
                    Error::new(
                        "E196",
                        child.span(),
                        "raw view child has no shared checked ID",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if children != view.children {
            return Err(Error::new(
                "E196",
                span,
                "raw view children diverged from its checked topology",
            ));
        }
        for child in &children {
            if self.facts.view(*child).parent != Some(id) {
                return Err(Error::new(
                    "E196",
                    span,
                    "checked view child has a mismatched parent",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn style_use(&self, span: &Span) -> Result<&ResolvedStyleUse, Error> {
        self.styles.style_use(span)
    }

    pub(crate) fn theme(&self) -> &ResolvedThemeProgram {
        &self.styles.theme
    }

    pub(crate) fn nested_theme(&self, span: &Span) -> Result<&ResolvedNestedTheme, Error> {
        self.styles.nested_theme(span)
    }

    pub(crate) fn extern_function(&self, id: ExternFnId) -> &crate::hir::ExternDeclaration {
        self.declarations.extern_decl(id)
    }

    #[allow(dead_code)]
    pub(crate) fn origin(&self, id: OriginId) -> &Origin {
        self.origins.get(id)
    }

    pub(crate) fn source_origin(&self, merged_line: usize) -> Option<(&Path, usize)> {
        self.origins.source_origin(merged_line)
    }
}

pub(crate) fn lower(checked: CheckedDocument) -> Result<LoweredProgram, Error> {
    Lowerer::new(checked).lower()
}

struct Lowerer {
    document: Document,
    facts: CheckedFacts,
    declarations: DeclarationIndex,
    origins: OriginArena,
    components: Vec<ComponentContract>,
    component_indexes: Vec<ComponentIndex>,
    component_ids: HashMap<String, ComponentId>,
    calls: Vec<ComponentCall>,
    calls_by_site: HashMap<CallSite, ComponentCallId>,
    styles: StyleProgramBuilder,
}

impl Lowerer {
    fn new(checked: CheckedDocument) -> Self {
        let CheckedDocument {
            document,
            facts,
            declarations,
            origins,
            ..
        } = checked;
        let component_ids = declarations.component_ids();
        Self {
            document,
            facts,
            declarations,
            origins,
            components: Vec::new(),
            component_indexes: Vec::new(),
            component_ids,
            calls: Vec::new(),
            calls_by_site: HashMap::new(),
            styles: StyleProgramBuilder::default(),
        }
    }

    fn lower(mut self) -> Result<LoweredProgram, Error> {
        if self.document.daemon != self.declarations.daemon() {
            return Err(self.invariant_at(
                &Span::line(1),
                "checked program kind changed before HIR lowering",
            ));
        }
        self.lower_style_program()?;
        let subscriptions = self.lower_subscriptions()?;
        let named_type_rust_paths = self.declarations.named_type_rust_paths();
        let app_states = self.lower_app_states()?;
        let derived = self.lower_derived()?;
        self.index_components()?;
        let component_roots = self
            .components
            .iter()
            .map(|component| (component.id, component.root.clone()))
            .collect::<Vec<_>>();
        for (component, root) in component_roots {
            self.lower_view(&root, Some(component))?;
        }
        let app_view = self.document.view.clone();
        self.lower_view(&app_view, None)?;
        let mounts = self
            .document
            .tests
            .iter()
            .filter_map(|test| test.mount.clone())
            .collect::<Vec<_>>();
        for mount in mounts {
            self.lower_view(&mount, None)?;
        }
        let styles = self.styles.finish().ok_or_else(|| {
            Error::new(
                "E196",
                &Span::line(1),
                "style lowering completed without a normalized theme program",
            )
        })?;
        let daemon = self.declarations.daemon();
        Ok(LoweredProgram {
            document: self.document,
            daemon,
            facts: self.facts,
            declarations: self.declarations,
            subscriptions,
            named_type_rust_paths,
            app_states,
            derived,
            components: self.components,
            calls: self.calls,
            calls_by_site: self.calls_by_site,
            styles,
            origins: self.origins,
        })
    }

    fn lower_subscriptions(&self) -> Result<Vec<ResolvedSubscription>, Error> {
        if self.facts.subscriptions().len() != self.document.subscriptions.len()
            || self.facts.subscriptions().len() != self.declarations.subscription_count()
        {
            return Err(self.invariant_at(
                &Span::line(1),
                "checked subscription topology changed before HIR lowering",
            ));
        }
        self.facts
            .subscriptions()
            .iter()
            .zip(&self.document.subscriptions)
            .enumerate()
            .map(|(index, (subscription, source))| {
                self.lower_subscription(index, subscription, source)
            })
            .collect()
    }

    fn lower_subscription(
        &self,
        index: usize,
        subscription: &CheckedSubscription,
        raw: &Subscription,
    ) -> Result<ResolvedSubscription, Error> {
        let span = &subscription.span;
        let declaration = self.declarations.try_subscription(index).ok_or_else(|| {
            self.invariant_at(
                &raw.span,
                "checked subscription has no declaration identity",
            )
        })?;
        if subscription.id != declaration.id
            || subscription.origin != declaration.origin
            || raw.span != subscription.span
            || raw.window_id != subscription.window_id
            || raw.status != subscription.status
            || raw.condition.is_some() != subscription.condition.is_some()
            || raw.context.is_some() != subscription.context.is_some()
            || raw.filter.as_deref()
                != subscription
                    .filter
                    .as_ref()
                    .map(|filter| filter.name.as_str())
            || raw.route.handler != subscription.route.handler_name
            || raw.route.args.len() != subscription.route.payloads.len()
            || raw
                .route
                .args
                .iter()
                .any(|argument| !matches!(argument, RouteArg::Payload))
            || !subscription_source_matches(&subscription.source, &raw.source)
        {
            return Err(Error::new(
                "E196",
                &raw.span,
                "checked subscription topology changed before HIR lowering",
            ));
        }
        let (source_payloads, delivered_payloads, filter) =
            self.validate_subscription_contract(subscription)?;
        let source = match &subscription.source {
            CheckedSubscriptionSource::Every { milliseconds } => {
                ResolvedSubscriptionSource::Every {
                    milliseconds: *milliseconds,
                }
            }
            CheckedSubscriptionSource::Repeat {
                function,
                milliseconds,
            } => ResolvedSubscriptionSource::Repeat {
                function: self.resolve_subscription_extern(function, ExternKind::Future, span)?,
                milliseconds: *milliseconds,
            },
            CheckedSubscriptionSource::Run {
                function,
                arguments,
            } => ResolvedSubscriptionSource::Run {
                function: self.resolve_subscription_extern(function, ExternKind::Stream, span)?,
                arguments: arguments.clone(),
            },
            CheckedSubscriptionSource::Recipe {
                function,
                arguments,
            } => ResolvedSubscriptionSource::Recipe {
                function: self.resolve_subscription_extern(function, ExternKind::Recipe, span)?,
                arguments: arguments.clone(),
            },
            CheckedSubscriptionSource::Events { identity, filter } => {
                ResolvedSubscriptionSource::Events {
                    identity: *identity,
                    filter: self.resolve_subscription_extern(
                        filter,
                        ExternKind::EventFilter,
                        span,
                    )?,
                }
            }
            CheckedSubscriptionSource::Event { raw } => {
                ResolvedSubscriptionSource::Event { raw: *raw }
            }
            CheckedSubscriptionSource::Extern {
                function,
                arguments,
            } => ResolvedSubscriptionSource::Extern {
                function: self.resolve_subscription_extern(
                    function,
                    ExternKind::Subscription,
                    span,
                )?,
                arguments: arguments.clone(),
            },
            CheckedSubscriptionSource::InputMethod(event) => {
                ResolvedSubscriptionSource::InputMethod(*event)
            }
            CheckedSubscriptionSource::Keyboard(event) => {
                ResolvedSubscriptionSource::Keyboard(*event)
            }
            CheckedSubscriptionSource::Mouse(event) => ResolvedSubscriptionSource::Mouse(*event),
            CheckedSubscriptionSource::SystemTheme => ResolvedSubscriptionSource::SystemTheme,
            CheckedSubscriptionSource::Touch(event) => ResolvedSubscriptionSource::Touch(*event),
            CheckedSubscriptionSource::Window(event) => ResolvedSubscriptionSource::Window(*event),
        };
        let handler = self
            .declarations
            .checked_handler(subscription.route.handler, span)?;
        if handler.name != subscription.route.handler_name {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription route has a mismatched handler identity",
            ));
        }
        Ok(ResolvedSubscription {
            id: subscription.id,
            source,
            source_payloads,
            delivered_payloads,
            filter,
            context: subscription.context,
            condition: subscription.condition,
            window_id: subscription.window_id,
            status: subscription.status,
            route: ResolvedSubscriptionRoute {
                handler: subscription.route.handler,
                handler_name: handler.name.clone(),
                payloads: subscription.route.payloads.clone(),
            },
            span: subscription.span.clone(),
            origin: subscription.origin,
        })
    }

    fn validate_subscription_contract(
        &self,
        subscription: &CheckedSubscription,
    ) -> Result<(Vec<Type>, Vec<Type>, Option<ResolvedExternContract>), Error> {
        let span = &subscription.span;
        if let Some(condition) = subscription.condition {
            self.facts.validate_subscription_expression_use(
                condition,
                subscription.id,
                CheckedSubscriptionExprRole::Condition,
                Some(&Type::Bool),
                &self.declarations,
                span,
            )?;
        }
        let source_payloads = match &subscription.source {
            CheckedSubscriptionSource::Repeat { function, .. } => {
                let function =
                    self.checked_subscription_extern(function, ExternKind::Future, span)?;
                if !function.params.is_empty() {
                    return Err(Error::new(
                        "E196",
                        span,
                        "checked repeat subscription has a mismatched arity",
                    ));
                }
                vec![extern_subscription_payload(function)]
            }
            CheckedSubscriptionSource::Run {
                function,
                arguments,
            } => {
                let function =
                    self.checked_subscription_extern(function, ExternKind::Stream, span)?;
                self.validate_subscription_arguments(subscription.id, arguments, function, span)?;
                vec![extern_subscription_payload(function)]
            }
            CheckedSubscriptionSource::Recipe {
                function,
                arguments,
            } => {
                let function =
                    self.checked_subscription_extern(function, ExternKind::Recipe, span)?;
                self.validate_subscription_arguments(subscription.id, arguments, function, span)?;
                vec![function.output.clone()]
            }
            CheckedSubscriptionSource::Events { identity, filter } => {
                self.facts.validate_subscription_expression_use(
                    *identity,
                    subscription.id,
                    CheckedSubscriptionExprRole::EventIdentity,
                    None,
                    &self.declarations,
                    span,
                )?;
                let function =
                    self.checked_subscription_extern(filter, ExternKind::EventFilter, span)?;
                if !function.params.is_empty() {
                    return Err(Error::new(
                        "E196",
                        span,
                        "checked event-filter subscription has a mismatched arity",
                    ));
                }
                vec![function.output.clone()]
            }
            CheckedSubscriptionSource::Extern {
                function,
                arguments,
            } => {
                let function =
                    self.checked_subscription_extern(function, ExternKind::Subscription, span)?;
                self.validate_subscription_arguments(subscription.id, arguments, function, span)?;
                vec![function.output.clone()]
            }
            source => resolved_native_subscription_payloads(source, subscription.window_id)
                .ok_or_else(|| {
                    Error::new(
                        "E196",
                        span,
                        "checked subscription source has no intrinsic payload contract",
                    )
                })?,
        };
        if source_payloads != subscription.source_payloads {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription source has a mismatched payload contract",
            ));
        }

        let filter = subscription
            .filter
            .as_ref()
            .map(|reference| {
                let function =
                    self.checked_subscription_extern(reference, ExternKind::Sync, span)?;
                if function
                    .params
                    .iter()
                    .map(|(_, ty)| ty)
                    .ne(&source_payloads)
                {
                    return Err(Error::new(
                        "E196",
                        span,
                        "checked subscription filter has mismatched parameter types",
                    ));
                }
                if !matches!(function.output, Type::Option(_)) {
                    return Err(Error::new(
                        "E196",
                        span,
                        "checked subscription filter has a non-optional output",
                    ));
                }
                self.resolve_subscription_extern(reference, ExternKind::Sync, span)
            })
            .transpose()?;
        let mut delivered_payloads = if let Some(filter) = &filter {
            let Type::Option(output) = &filter.output else {
                return Err(Error::new(
                    "E196",
                    span,
                    "resolved subscription filter lost its optional output contract",
                ));
            };
            vec![(**output).clone()]
        } else {
            source_payloads.clone()
        };
        if let Some(context) = subscription.context {
            delivered_payloads.insert(
                0,
                self.facts.validate_subscription_expression_use(
                    context,
                    subscription.id,
                    CheckedSubscriptionExprRole::Context,
                    None,
                    &self.declarations,
                    span,
                )?,
            );
        }
        if delivered_payloads != subscription.delivered_payloads {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription transforms have a mismatched delivered payload contract",
            ));
        }
        let handler = self
            .declarations
            .checked_handler(subscription.route.handler, span)?;
        let routed = subscription
            .route
            .payloads
            .iter()
            .map(|index| {
                delivered_payloads
                    .get(*index as usize)
                    .cloned()
                    .ok_or_else(|| {
                        Error::new(
                            "E196",
                            span,
                            "checked subscription route references an invalid payload index",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if handler.name != subscription.route.handler_name || routed != handler.payloads {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription route has a mismatched handler contract",
            ));
        }
        Ok((source_payloads, delivered_payloads, filter))
    }

    fn validate_subscription_arguments(
        &self,
        subscription: SubscriptionId,
        arguments: &[CheckedExprUseId],
        function: &crate::hir::ExternDeclaration,
        span: &Span,
    ) -> Result<(), Error> {
        if arguments.len() != function.params.len() {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription extern arguments have a mismatched arity",
            ));
        }
        for (index, (argument, (_, expected))) in arguments.iter().zip(&function.params).enumerate()
        {
            self.facts.validate_subscription_expression_use(
                *argument,
                subscription,
                CheckedSubscriptionExprRole::SourceArgument(index as u32),
                Some(expected),
                &self.declarations,
                span,
            )?;
        }
        Ok(())
    }

    fn checked_subscription_extern(
        &self,
        reference: &ExternRef,
        kind: ExternKind,
        span: &Span,
    ) -> Result<&crate::hir::ExternDeclaration, Error> {
        let declaration = self.declarations.checked_extern_decl(reference.id, span)?;
        if declaration.name != reference.name || declaration.kind != kind {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription extern reference has a mismatched declaration contract",
            ));
        }
        Ok(declaration)
    }

    fn resolve_subscription_extern(
        &self,
        reference: &ExternRef,
        kind: ExternKind,
        span: &Span,
    ) -> Result<ResolvedExternContract, Error> {
        let declaration = self.declarations.checked_extern_decl(reference.id, span)?;
        if declaration.name != reference.name || declaration.kind != kind {
            return Err(Error::new(
                "E196",
                span,
                "checked subscription extern reference has a mismatched declaration contract",
            ));
        }
        Ok(ResolvedExternContract {
            id: declaration.declaration.id,
            name: declaration.name.clone(),
            rust_path: declaration.rust_path.clone(),
            params: declaration
                .params
                .iter()
                .map(|(_, ty)| self.resolve_type(ty, span))
                .collect::<Result<Vec<_>, _>>()?,
            output: declaration.output.clone(),
            error: declaration.error.clone(),
        })
    }

    fn resolve_type(&self, ty: &Type, span: &Span) -> Result<ResolvedType, Error> {
        Ok(match ty {
            Type::List(inner) => ResolvedType::List(Box::new(self.resolve_type(inner, span)?)),
            Type::Option(inner) => ResolvedType::Option(Box::new(self.resolve_type(inner, span)?)),
            Type::Result(output, error) => ResolvedType::Result(
                Box::new(self.resolve_type(output, span)?),
                Box::new(self.resolve_type(error, span)?),
            ),
            Type::Combo(inner) => ResolvedType::Combo(Box::new(self.resolve_type(inner, span)?)),
            Type::Animation(inner) => {
                ResolvedType::Animation(Box::new(self.resolve_type(inner, span)?))
            }
            Type::Named(name) => {
                ResolvedType::Named(self.declarations.named_type_id(name).ok_or_else(|| {
                    Error::new(
                        "E196",
                        span,
                        format!("checked type references unknown declaration `{name}`"),
                    )
                })?)
            }
            Type::Unknown => {
                return Err(Error::new(
                    "E196",
                    span,
                    "checked subscription type remained unknown",
                ));
            }
            ty => ResolvedType::Value(ty.clone()),
        })
    }

    fn invariant_at(&self, span: &Span, message: impl Into<String>) -> Error {
        let message = message.into();
        if let Some((path, line)) = self.origins.source_origin(span.line) {
            Error::new(
                "E196",
                &Span {
                    line,
                    column: span.column,
                },
                message,
            )
            .at_path(path.display().to_string())
        } else {
            Error::new("E196", span, message)
        }
    }

    fn lower_app_states(&self) -> Result<Vec<AppStateContract>, Error> {
        self.document
            .states
            .iter()
            .enumerate()
            .map(|(index, state)| {
                let declaration = self.declarations.app_state(index);
                Ok(AppStateContract {
                    id: declaration.id,
                    name: state.name.clone(),
                    ty: state.ty.clone(),
                    initializer: self
                        .resolve_initializer(CheckedValueRef::AppState(declaration.id), state)?,
                    span: state.span.clone(),
                    origin: declaration.origin,
                })
            })
            .collect()
    }

    fn lower_derived(&self) -> Result<Vec<DerivedContract>, Error> {
        self.document
            .derived
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let declaration = self.declarations.derived(index);
                let checked = self
                    .facts
                    .value_by_ref(CheckedValueRef::Derived(declaration.id));
                Ok(DerivedContract {
                    id: declaration.id,
                    name: value.name.clone(),
                    ty: value.ty.clone(),
                    initializer: checked.initializer.ok_or_else(|| {
                        self.invariant(&value.span, "derived value has no checked initializer")
                    })?,
                    span: value.span.clone(),
                    origin: declaration.origin,
                })
            })
            .collect()
    }

    fn resolve_initializer(
        &self,
        value_ref: CheckedValueRef,
        state: &State,
    ) -> Result<ResolvedInitializer, Error> {
        let checked = self.facts.value_by_ref(value_ref);
        let expression = checked.initializer.ok_or_else(|| {
            self.invariant(&state.span, "state has no checked initializer expression")
        })?;
        let animation = state
            .animation
            .as_ref()
            .map(|options| self.resolve_animation(options, &state.span))
            .transpose()?;
        Ok(ResolvedInitializer {
            expression,
            animation,
        })
    }

    fn resolve_animation(
        &self,
        options: &AnimationOptions,
        span: &Span,
    ) -> Result<ResolvedAnimation, Error> {
        let easing = options
            .easing
            .as_deref()
            .map(|easing| {
                if ANIMATION_EASINGS.contains(&easing) {
                    Ok(ResolvedAnimationEasing::Builtin(easing.to_owned()))
                } else {
                    self.declarations
                        .extern_decl_by_name(easing)
                        .map(|declaration| {
                            ResolvedAnimationEasing::Custom(declaration.declaration.id)
                        })
                        .ok_or_else(|| {
                            self.invariant(span, "animation easing has no extern declaration")
                        })
                }
            })
            .transpose()?;
        Ok(ResolvedAnimation {
            easing,
            duration: options.duration,
            delay_ms: options.delay_ms,
            repeat: options.repeat,
            repeat_forever: options.repeat_forever,
            auto_reverse: options.auto_reverse,
        })
    }

    fn index_components(&mut self) -> Result<(), Error> {
        let source_components = self.document.components.clone();
        for (index, component) in source_components.into_iter().enumerate() {
            let declaration = self.declarations.component(index);
            let id = declaration.id;
            let origin = declaration.origin;
            let mut params = Vec::with_capacity(component.params.len());
            for (index, param) in component.params.iter().enumerate() {
                let declaration = self.declarations.component_param(id, index);
                params.push(ComponentParamContract {
                    id: declaration.id,
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                    capability: if param.bind {
                        ParamCapability::Bind
                    } else {
                        ParamCapability::Read
                    },
                    default: self
                        .facts
                        .value_by_ref(CheckedValueRef::ComponentParam(declaration.id))
                        .initializer,
                    origin: declaration.origin,
                });
            }
            let events: Vec<ComponentEventContract> = component
                .events
                .iter()
                .enumerate()
                .map(|(index, event)| ComponentEventContract {
                    id: ComponentEventId {
                        component: id,
                        index: index as u32,
                    },
                    name: event.name.clone(),
                    payloads: event.payloads.clone(),
                    origin: self.push_origin(&event.span, Some(origin)),
                })
                .collect();
            let slots: Vec<ComponentSlotContract> = declared_slots(&component.root)
                .into_iter()
                .enumerate()
                .map(|(index, (name, optional, _span))| ComponentSlotContract {
                    id: self.declarations.component_slot(id, index).id,
                    name,
                    optional,
                    origin: self.declarations.component_slot(id, index).origin,
                })
                .collect();
            let states = component
                .states
                .iter()
                .enumerate()
                .map(|(index, state)| {
                    let declaration = self.declarations.component_state(id, index);
                    Ok(ComponentStateContract {
                        id: declaration.id,
                        name: state.name.clone(),
                        ty: state.ty.clone(),
                        initializer: self.resolve_initializer(
                            CheckedValueRef::ComponentState(declaration.id),
                            state,
                        )?,
                        span: state.span.clone(),
                        origin: declaration.origin,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            let stateful = !states.is_empty() || !component.handlers.is_empty();
            let storage = match (stateful, component.lifetime) {
                (false, _) => ComponentStorage::Stateless,
                (true, ComponentLifetime::Retained) => ComponentStorage::Retained,
                (true, ComponentLifetime::Mounted) => ComponentStorage::Mounted,
            };
            let params_by_name = params
                .iter()
                .enumerate()
                .map(|(index, param)| (param.name.clone(), index))
                .collect();
            let events_by_name = events
                .iter()
                .enumerate()
                .map(|(index, event)| (event.name.clone(), index))
                .collect();
            let slots_by_name = slots
                .iter()
                .enumerate()
                .map(|(index, slot)| (slot.name.clone(), index))
                .collect();
            self.components.push(ComponentContract {
                id,
                name: component.name,
                params,
                output: component.output,
                events,
                slots,
                states,
                handlers: component.handlers,
                root: component.root,
                storage,
                origin,
            });
            self.component_indexes.push(ComponentIndex {
                params_by_name,
                events_by_name,
                slots_by_name,
            });
        }
        Ok(())
    }

    fn lower_view(
        &mut self,
        node: &ViewNode,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        self.lower_view_style(node)?;
        match node {
            ViewNode::Component {
                name,
                args,
                id,
                slots,
                events,
                route,
                span,
            } => {
                self.lower_component_call(
                    name,
                    args,
                    id,
                    slots,
                    events,
                    route,
                    span,
                    outer_component,
                )?;
                for slot in slots {
                    self.lower_view(&slot.content, outer_component)?;
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    self.lower_view(child, outer_component)?;
                }
            }
            ViewNode::Match { arms, .. } => {
                for child in arms.iter().flat_map(|arm| &arm.children) {
                    self.lower_view(child, outer_component)?;
                }
            }
            ViewNode::Button {
                content: Some(content),
                ..
            }
            | ViewNode::MouseArea { content, .. }
            | ViewNode::ResizeHandle { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => {
                self.lower_view(content, outer_component)?;
            }
            ViewNode::Tooltip { content, tip, .. } => {
                self.lower_view(content, outer_component)?;
                self.lower_view(tip, outer_component)?;
            }
            ViewNode::Overlay { content, layer, .. } => {
                self.lower_view(content, outer_component)?;
                self.lower_view(layer, outer_component)?;
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for child in panes
                    .iter()
                    .flat_map(PaneView::nodes)
                    .chain(templates.iter().flat_map(|template| template.pane.nodes()))
                {
                    self.lower_view(child, outer_component)?;
                }
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    self.lower_view(&column.header, outer_component)?;
                    self.lower_view(&column.cell, outer_component)?;
                }
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    self.lower_view(narrow, outer_component)?;
                    self.lower_view(wide, outer_component)?;
                }
                ResponsiveContent::Size { content, .. } => {
                    self.lower_view(content, outer_component)?;
                }
            },
            _ => {}
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_component_call(
        &mut self,
        name: &str,
        supplied_args: &[ComponentArg],
        id: &Option<Id>,
        supplied_slots: &[ComponentSlot],
        supplied_events: &[ComponentEventRoute],
        route: &Option<Route>,
        span: &Span,
        outer_component: Option<ComponentId>,
    ) -> Result<(), Error> {
        let component_id =
            self.component_ids.get(name).copied().ok_or_else(|| {
                self.invariant(span, format!("unknown checked component `{name}`"))
            })?;
        let component_index = component_id.0 as usize;
        let view_id = self
            .declarations
            .view_id(span)
            .ok_or_else(|| self.invariant(span, "component call has no shared view ID"))?;
        let call_id = self
            .declarations
            .component_call_id(view_id)
            .ok_or_else(|| self.invariant(span, "component call has no shared call ID"))?;
        let supplied_args = {
            let contract = &self.components[component_index];
            let index = &self.component_indexes[component_index];
            let mut ordered = vec![None; contract.params.len()];
            for supplied in supplied_args {
                let position = index.params_by_name.get(&supplied.name).ok_or_else(|| {
                    self.invariant(span, format!("unknown checked prop `{}`", supplied.name))
                })?;
                if ordered[*position].replace(supplied).is_some() {
                    return Err(
                        self.invariant(span, format!("duplicate checked prop `{}`", supplied.name))
                    );
                }
            }
            ordered
        };
        let supplied_events = {
            let contract = &self.components[component_index];
            let index = &self.component_indexes[component_index];
            let mut ordered = vec![None; contract.events.len()];
            for supplied in supplied_events {
                let position = index.events_by_name.get(&supplied.name).ok_or_else(|| {
                    self.invariant(
                        &supplied.span,
                        format!("unknown checked event `{}`", supplied.name),
                    )
                })?;
                if ordered[*position].replace(supplied).is_some() {
                    return Err(self.invariant(
                        &supplied.span,
                        format!("duplicate checked event `{}`", supplied.name),
                    ));
                }
            }
            ordered
        };
        let supplied_slots = {
            let contract = &self.components[component_index];
            let index = &self.component_indexes[component_index];
            let mut ordered = vec![None; contract.slots.len()];
            for supplied in supplied_slots {
                let position = index.slots_by_name.get(&supplied.name).ok_or_else(|| {
                    self.invariant(
                        &supplied.span,
                        format!("unknown checked slot `{}`", supplied.name),
                    )
                })?;
                if ordered[*position].replace(supplied).is_some() {
                    return Err(self.invariant(
                        &supplied.span,
                        format!("duplicate checked slot `{}`", supplied.name),
                    ));
                }
            }
            ordered
        };
        // Calls need the compact semantic shape below, never the component body,
        // states, or handlers. ComponentContract intentionally is not Clone so a
        // future call-site change cannot accidentally restore whole-contract copies.
        let (params, events, slots, output_ty, storage) = {
            let contract = &self.components[component_index];
            (
                contract.params.clone(),
                contract.events.clone(),
                contract.slots.clone(),
                contract.output.clone(),
                contract.storage,
            )
        };
        let origin = self.declarations.view(view_id).origin;
        let mut arguments = Vec::with_capacity(params.len());
        for (param, supplied) in params.iter().zip(supplied_args) {
            let source = self
                .facts
                .component_argument_source(call_id, param.id)
                .ok_or_else(|| self.invariant(span, "component argument has no checked source"))?;
            if matches!(source, CheckedComponentArgumentSource::Supplied(_)) != supplied.is_some() {
                return Err(self.invariant(
                    span,
                    format!(
                        "raw prop `{}` supplied/default topology diverged from checking",
                        param.name
                    ),
                ));
            }
            let (expression, scope) = match source {
                CheckedComponentArgumentSource::Supplied(expression) => {
                    (expression, ArgumentScope::Caller)
                }
                CheckedComponentArgumentSource::Default(expression) => {
                    (expression, ArgumentScope::Definition)
                }
            };
            let writable = if param.capability == ParamCapability::Bind {
                if supplied.is_none() {
                    return Err(self.invariant(span, "bind argument resolved to a default"));
                }
                Some(self.resolve_writable(expression, outer_component, span)?)
            } else {
                None
            };
            let argument_origin = self.push_origin(span, Some(origin));
            arguments.push(ResolvedArgument {
                param: param.id,
                name: param.name.clone(),
                ty: param.ty.clone(),
                expression,
                scope,
                writable,
                origin: argument_origin,
            });
        }

        let mut resolved_events = Vec::with_capacity(events.len());
        for (event, supplied) in events.iter().zip(supplied_events) {
            let supplied = supplied.ok_or_else(|| {
                self.invariant(span, format!("event `{}` has no checked route", event.name))
            })?;
            let event_origin = self.push_origin(&supplied.span, Some(origin));
            if let Some(route) = &supplied.route {
                resolved_events.push(ResolvedEventRoute::Direct {
                    event: event.id,
                    name: event.name.clone(),
                    payloads: event.payloads.clone(),
                    route: route.clone(),
                    origin: event_origin,
                });
            } else {
                let outer = outer_component.ok_or_else(|| {
                    self.invariant(&supplied.span, "forwarded event has no outer component")
                })?;
                let outer_index = outer.0 as usize;
                let outer_event = self.component_indexes[outer_index]
                    .events_by_name
                    .get(&event.name)
                    .and_then(|position| self.components[outer_index].events.get(*position))
                    .ok_or_else(|| {
                        self.invariant(
                            &supplied.span,
                            format!("forwarded event `{}` has no outer declaration", event.name),
                        )
                    })?
                    .id;
                resolved_events.push(ResolvedEventRoute::Forward {
                    event: event.id,
                    name: event.name.clone(),
                    payloads: event.payloads.clone(),
                    outer_component: outer,
                    outer_event,
                    origin: event_origin,
                });
            }
        }

        let mut resolved_slots = Vec::with_capacity(slots.len());
        for (declared, supplied) in slots.iter().zip(supplied_slots) {
            if supplied.is_none() && !declared.optional {
                return Err(self.invariant(
                    span,
                    format!("required slot `{}` has no checked content", declared.name),
                ));
            }
            resolved_slots.push(ResolvedSlot {
                slot: declared.id,
                name: declared.name.clone(),
                optional: declared.optional,
                content: supplied.map(|slot| (*slot.content).clone()),
                origin: supplied.map_or(declared.origin, |slot| {
                    self.push_origin(&slot.span, Some(origin))
                }),
            });
        }

        let output = match (&output_ty, route) {
            (Type::Unit, None) => ComponentOutputRoute::None,
            (output, Some(route)) => ComponentOutputRoute::Direct {
                output: output.clone(),
                route: route.clone(),
                origin,
            },
            _ => {
                return Err(
                    self.invariant(span, "component output route was not resolved by checking")
                );
            }
        };
        let scope = id.as_ref().map_or_else(
            || ComponentScope::Implicit {
                component: component_id,
                call_site: span.line,
                origin,
            },
            |id| ComponentScope::Explicit {
                id: id.clone(),
                origin,
            },
        );
        if call_id.0 as usize != self.calls.len() {
            return Err(self.invariant(span, "component call arena order diverged"));
        }
        let site = CallSite {
            line: span.line,
            column: span.column,
        };
        if self.calls_by_site.insert(site, call_id).is_some() {
            return Err(self.invariant(span, "component call source identity is not unique"));
        }
        self.calls.push(ComponentCall {
            id: call_id,
            component: component_id,
            origin,
            arguments,
            events: resolved_events,
            slots: resolved_slots,
            output,
            scope,
            storage,
            binding_site: span.line,
        });
        Ok(())
    }

    fn resolve_writable(
        &self,
        expression: CheckedExprUseId,
        outer_component: Option<ComponentId>,
        span: &Span,
    ) -> Result<WritableStateRef, Error> {
        let expression = self.facts.expression_use(expression);
        let checked = self.facts.expression(expression.root);
        let crate::check::CheckedExprKind::Path {
            root: crate::check::CheckedPathRoot::Value(value),
            projections,
        } = &checked.kind
        else {
            return Err(self.invariant(span, "bind argument is not a direct path"));
        };
        if !projections.is_empty() {
            return Err(self.invariant(span, "bind argument is not a direct state path"));
        }
        let resolved = match value {
            CheckedValueRef::AppState(id) => WritableStateRef::App {
                id: *id,
                name: self.facts.value_by_ref(*value).name.clone(),
            },
            CheckedValueRef::ComponentParam(id)
                if outer_component == Some(id.component)
                    && self.components[id.component.0 as usize].params[id.index as usize]
                        .capability
                        == ParamCapability::Bind =>
            {
                WritableStateRef::ComponentParam {
                    id: *id,
                    name: self.facts.value_by_ref(*value).name.clone(),
                }
            }
            CheckedValueRef::ComponentState(id) if outer_component == Some(id.component) => {
                WritableStateRef::ComponentState {
                    id: *id,
                    name: self.facts.value_by_ref(*value).name.clone(),
                }
            }
            _ => return Err(self.invariant(span, "bind argument root is not writable here")),
        };
        Ok(resolved)
    }

    fn push_origin(&mut self, span: &Span, parent: Option<OriginId>) -> OriginId {
        self.origins.push(span, parent)
    }

    fn invariant(&self, span: &Span, message: impl Into<String>) -> Error {
        Error::new(
            "E196",
            span,
            format!("lowering invariant failed: {}", message.into()),
        )
    }
}

fn declared_slots(node: &ViewNode) -> Vec<(String, bool, Span)> {
    fn collect(node: &ViewNode, output: &mut Vec<(String, bool, Span)>) {
        match node {
            ViewNode::Slot {
                name,
                optional,
                span,
            } => output.push((name.clone(), *optional, span.clone())),
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Match { arms, .. } => {
                for child in arms.iter().flat_map(|arm| &arm.children) {
                    collect(child, output);
                }
            }
            ViewNode::Button {
                content: Some(content),
                ..
            }
            | ViewNode::MouseArea { content, .. }
            | ViewNode::ResizeHandle { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, output);
                collect(layer, output);
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for child in panes
                    .iter()
                    .flat_map(PaneView::nodes)
                    .chain(templates.iter().flat_map(|template| template.pane.nodes()))
                {
                    collect(child, output);
                }
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, output);
                }
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output);
                    collect(wide, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, output),
            },
            _ => {}
        }
    }
    let mut output = Vec::new();
    collect(node, &mut output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyze, analyze_file};
    use std::fmt::Write as _;
    use std::fs;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    const THEME: &str = "theme contract AppTheme\n  bg\n  fg\n  primary\n  danger\npalette app for AppTheme\n  bg #000000\n  fg #ffffff\n  primary #333333\n  danger #ff0000\n";

    #[test]
    fn lowers_component_calls_into_ordered_complete_contracts() {
        let source = format!(
            "app Demo\n{THEME}state\n  draft = \"Draft\"\n  checked = false\non changed(value)\n  draft = value\non toggled(value)\n  checked = value\ncomponent Field(bind value:str, label:str=\"Name\")\n  emits\n    change(str)\n  lifetime mounted\n  state\n    local = \"\"\n  col\n    text label\n    slot Leading?\n    slot Body\ncomponent Shell(bind value:str)\n  emits\n    change(str)\n  state\n    scratch = \"\"\n  Field value<->value\n    Leading:\n      text \"L\"\n    Body:\n      text \"B\"\n    forward\n      change\ncomponent Choice() -> bool\n  checkbox \"Choice\" checked=false -> emit(_)\nview\n  col\n    Field value<->draft #field\n      Body:\n        text \"Body\"\n      events\n        change -> changed _\n    Choice -> toggled _\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        assert_eq!(program.components.len(), 3);
        let field = &program.components[0];
        assert_eq!(field.name, "Field");
        assert_eq!(field.params.len(), 2);
        assert_eq!(field.slots.len(), 2);
        assert_eq!(field.storage, ComponentStorage::Mounted);
        assert_eq!(program.components[1].storage, ComponentStorage::Retained);

        let nested = program
            .calls
            .iter()
            .find(|call| matches!(call.events[0], ResolvedEventRoute::Forward { .. }))
            .unwrap();
        assert_eq!(nested.arguments[0].name, "value");
        assert!(matches!(
            nested.arguments[0].writable,
            Some(WritableStateRef::ComponentParam { .. })
        ));
        assert!(nested.arguments[1].uses_definition_scope());
        assert_eq!(
            nested
                .slots
                .iter()
                .filter(|slot| slot.content.is_some())
                .count(),
            2
        );
        assert!(matches!(nested.scope, ComponentScope::Implicit { .. }));

        let root = program
            .calls
            .iter()
            .find(|call| matches!(call.scope, ComponentScope::Explicit { .. }))
            .unwrap();
        assert!(matches!(
            root.arguments[0].writable,
            Some(WritableStateRef::App { .. })
        ));
        assert!(matches!(root.events[0], ResolvedEventRoute::Direct { .. }));
        assert!(matches!(root.scope, ComponentScope::Explicit { .. }));
        assert!(
            root.slots
                .iter()
                .any(|slot| { slot.name == "Leading" && slot.optional && slot.content.is_none() })
        );
        assert!(
            root.slots
                .iter()
                .any(|slot| slot.name == "Body" && slot.content.is_some())
        );
        let output = program
            .calls
            .iter()
            .find(|call| matches!(call.output, ComponentOutputRoute::Direct { .. }))
            .unwrap();
        assert!(matches!(output.output, ComponentOutputRoute::Direct { .. }));
    }

    #[test]
    fn bind_writability_comes_from_the_checked_expression_root() {
        let source = format!(
            "app BindFacts\n{THEME}state\n  draft = \"Draft\"\ncomponent Field(bind value:str)\n  text value\nview\n  Field value<->draft\n"
        );
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Component { args, .. } = &mut checked.document.view else {
            panic!("fixture root must be a component call");
        };
        args[0].value = Expr::Bool(false);

        let program = lower(checked).unwrap();
        let argument = &program.calls[0].arguments[0];
        assert!(matches!(
            &argument.writable,
            Some(WritableStateRef::App { name, .. }) if name == "draft"
        ));
        let root = program.checked_facts().expression(
            program
                .checked_facts()
                .expression_use(argument.expression)
                .root,
        );
        assert!(matches!(
            root.kind,
            crate::check::CheckedExprKind::Path {
                root: crate::check::CheckedPathRoot::Value(CheckedValueRef::AppState(_)),
                ref projections,
            } if projections.is_empty()
        ));
    }

    #[test]
    fn keeps_parented_source_origins() {
        let source = format!(
            "app Demo\n{THEME}component Card(title:str=\"Default\")\n  text title\nview\n  Card\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let call = &program.calls[0];
        let call_origin = program.origin(call.origin);
        assert_eq!(call_origin.line, source.lines().count());
        assert_eq!(call_origin.column, 1);
        let component = &program.components[0];
        assert_eq!(
            program.origin(component.origin).line,
            source.lines().count() - 3
        );
        assert_eq!(
            program.origin(component.params[0].origin).parent,
            Some(component.origin)
        );
    }

    #[test]
    fn snapshots_the_complete_initializer_hir_slice() {
        let source = format!(
            "app Initializers\nextern crate::backend\n  sync elastic(value:f64) -> f64\n{THEME}state\n  progress:animation[f64] = 0.0\n    easing elastic\n    duration 120ms\n    delay 5ms\n    repeat 2\n    auto-reverse true\nderived\n  total = 1.0 + 2.0\ncomponent Meter(label:str=\"ready\")\n  state\n    open = false\n  text label\nview\n  Meter\n"
        );
        let program = lower(analyze(&source).unwrap()).unwrap();
        let facts = program.checked_facts();
        let app = &program.app_states[0];
        let derived = &program.derived[0];
        let component = &program.components[0];
        let default = component.params[0].default.unwrap();
        let component_state = &component.states[0];
        let app_use = facts.expression_use(app.initializer.expression);
        let derived_use = facts.expression_use(derived.initializer);
        let default_use = facts.expression_use(default);
        let component_state_use = facts.expression_use(component_state.initializer.expression);

        let snapshot = format!(
            "app {:?} {} {:?} use={:?} {:?} line={} animation={:?}\n\
             derived {:?} {} {:?} use={:?} {:?} line={}\n\
             default {:?} {} {:?} use={:?} {:?} line={}\n\
             component-state {:?} {} {:?} use={:?} {:?} line={} animation={:?}\n",
            app.id,
            app.name,
            app.ty,
            app.initializer.expression,
            app_use.coercion,
            program.origin(app.origin).line,
            app.initializer.animation,
            derived.id,
            derived.name,
            derived.ty,
            derived.initializer,
            derived_use.coercion,
            program.origin(derived.origin).line,
            component.params[0].id,
            component.params[0].name,
            component.params[0].ty,
            default,
            default_use.coercion,
            program.origin(component.params[0].origin).line,
            component_state.id,
            component_state.name,
            component_state.ty,
            component_state.initializer.expression,
            component_state_use.coercion,
            program.origin(component_state.origin).line,
            component_state.initializer.animation,
        );
        assert_eq!(
            snapshot,
            "app AppStateId(0) progress Animation(F64) use=CheckedExprUseId(0) ValueToAnimation { value: F64 } line=15 animation=Some(ResolvedAnimation { easing: Some(Custom(ExternFnId(0))), duration: Some(Milliseconds(120)), delay_ms: Some(5), repeat: Some(2), repeat_forever: false, auto_reverse: true })\n\
             derived DerivedId(0) total F64 use=CheckedExprUseId(1) None line=22\n\
             default ComponentParamId { component: ComponentId(0), index: 0 } label Str use=CheckedExprUseId(2) None line=23\n\
             component-state ComponentStateId { component: ComponentId(0), index: 0 } open Bool use=CheckedExprUseId(3) None line=25 animation=None\n"
        );

        let generated = crate::codegen::generate(&program, "initializers.ice").unwrap();
        assert!(generated.contains(
            "::iced::Animation::new((0.0) as f32).easing(::iced::animation::Easing::Custom(|__value: f32| crate::backend::elastic(__value as f64) as f32)).duration(::std::time::Duration::from_millis(120)).delay(::std::time::Duration::from_millis(5)).repeat(2).auto_reverse()"
        ));
        assert!(generated.contains("fn __ice_derived_total(&self) -> f64 { (1.0 + 2.0) }"));
        assert!(generated.contains("(\"ready\").to_string()"));
        assert!(generated.contains("open: false"));
    }

    #[test]
    fn rejects_a_checked_component_call_that_cannot_be_resolved() {
        let source = format!("app Demo\n{THEME}component Card()\n  text \"Card\"\nview\n  Card\n");
        let mut checked = analyze(&source).unwrap();
        let ViewNode::Component { name, .. } = &mut checked.document.view else {
            panic!("fixture root must be a component call");
        };
        *name = "Missing".into();

        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert_eq!(error.line, source.lines().count());
        assert!(
            error
                .message
                .contains("unknown checked component `Missing`")
        );
    }

    #[test]
    fn lowering_large_component_call_surface_stays_linear() {
        let mut source = format!(
            "app Demo\n{THEME}component Badge(label:str=\"Badge\")\n  text label\nview\n  col\n"
        );
        for _ in 0..10_000 {
            source.push_str("    Badge\n");
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.calls.len(), 10_000);
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "10k component calls lowered in {elapsed:?}"
        );
    }

    #[test]
    #[ignore = "large generated-source performance contract"]
    fn lowering_wide_component_calls_scales_with_call_surface_not_contract_body() {
        const CALLS: usize = 2_000;
        const DEFAULT_PARAMS: usize = 32;
        const EVENTS: usize = 32;
        const SLOTS: usize = 32;
        const STATES: usize = 128;
        const HANDLERS: usize = 128;
        const BODY_NODES: usize = 256;

        let mut source = format!(
            "app Demo\n{THEME}state\n  draft = \"Draft\"\non changed\n  draft = \"Changed\"\ncomponent Wide(bind value:str"
        );
        for index in 0..DEFAULT_PARAMS {
            write!(source, ", prop_{index}:str=\"value-{index}\"").unwrap();
        }
        source.push_str(")\n  emits\n");
        for index in 0..EVENTS {
            writeln!(source, "    event_{index}").unwrap();
        }
        source.push_str("  lifetime mounted\n  state\n");
        for index in 0..STATES {
            writeln!(source, "    local_{index} = \"state-{index}\"").unwrap();
        }
        for index in 0..HANDLERS {
            writeln!(
                source,
                "  on reset_{index}\n    local_{index} = \"reset-{index}\""
            )
            .unwrap();
        }
        source.push_str("  col\n    text value\n");
        for index in 0..BODY_NODES {
            writeln!(source, "    text \"body-{index}\"").unwrap();
        }
        for index in 0..SLOTS {
            writeln!(source, "    slot Slot{index}?").unwrap();
        }
        source.push_str("view\n  col\n");
        for _ in 0..CALLS {
            source.push_str("    Wide value<->draft\n      events\n");
            for index in 0..EVENTS {
                writeln!(source, "        event_{index} -> changed").unwrap();
            }
        }

        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let elapsed = started.elapsed();
        assert_eq!(program.calls.len(), CALLS);
        assert_eq!(
            program
                .calls
                .iter()
                .map(|call| call.arguments.len() + call.events.len() + call.slots.len())
                .sum::<usize>(),
            CALLS * (1 + DEFAULT_PARAMS + EVENTS + SLOTS)
        );
        let ViewNode::Layout { children, .. } = &program.components[0].root else {
            panic!("wide component root must remain a layout");
        };
        assert_eq!(children.len(), BODY_NODES + SLOTS + 1);
        assert_eq!(program.components[0].states.len(), STATES);
        assert_eq!(program.components[0].handlers.len(), HANDLERS);
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "2k calls to a wide component lowered in {elapsed:?}"
        );
    }

    #[test]
    fn resolves_namespaced_import_calls_and_their_physical_origins() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-lowered-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("parts.ice");
        fs::write(
            &root,
            format!("app Demo\nuse \"parts.ice\" as ui\n{THEME}view\n  ui::Outer\n"),
        )
        .unwrap();
        fs::write(
            &imported,
            "component Inner()\n  text \"Inner\"\ncomponent Outer()\n  state\n    value = \"\"\n  Inner\n",
        )
        .unwrap();

        let program = lower(analyze_file(&root).unwrap()).unwrap();
        let inner = program
            .components
            .iter()
            .find(|component| component.name == "ui::Inner")
            .unwrap();
        let outer = program
            .components
            .iter()
            .find(|component| component.name == "ui::Outer")
            .unwrap();
        let nested = program
            .calls
            .iter()
            .find(|call| call.component == inner.id)
            .unwrap();
        let root_call = program
            .calls
            .iter()
            .find(|call| call.component == outer.id)
            .unwrap();
        let nested_origin = program.origin(nested.origin);
        let root_origin = program.origin(root_call.origin);
        assert_eq!(nested_origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(nested_origin.line, 6);
        assert_eq!(root_origin.path.as_deref(), Some(root.as_path()));
        assert_eq!(
            root_origin.line,
            fs::read_to_string(&root).unwrap().lines().count()
        );

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn normalizes_recipe_inheritance_precedence_and_every_utility_variant() {
        let source = r#"app Styles
recipe action for button
  px-16px py-11px bg-surface/75 hover:bg-primary pressed:bg-danger disabled:bg-border disabled:text-fg disabled:opacity-25 border border-border rounded-9px text-12.5px leading-snug font-semibold
recipe emphasized for button extends action
  bg-primary hover:bg-danger
recipe destructive for button extends emphasized
  pressed:bg-primary disabled:bg-surface text-fg
recipe field for input
  w-full border border-border focus:border-primary rounded-md
theme contract AppTheme
  bg
  fg
  primary
  danger
  surface
  border
palette app for AppTheme
  bg #101010
  fg #f0f0f0
  primary #336699
  danger #cc0000
  surface #202020
  border #404040
state
  value = ""
on pressed
view
  col
    button "Delete" @destructive bg-danger -> pressed
    input "Name" <-> value @field
"#;
        let program = lower(analyze(source).unwrap()).unwrap();
        assert_eq!(program.styles.recipes.len(), 4);
        assert!(matches!(
            program.theme().active_palette,
            ResolvedPaletteSelection::Static(PaletteId(0))
        ));
        let destructive = &program.styles.recipes[2];
        assert_eq!(destructive.base, Some(RecipeId(1)));
        assert_eq!(destructive.declared_utilities.len(), 3);
        assert!(matches!(
            destructive.style.background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 2, .. }),
                opacity: None,
            })
        ));
        assert!(matches!(
            destructive.style.hover_background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 3, .. }),
                ..
            })
        ));
        assert!(matches!(
            destructive.style.pressed_background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 2, .. }),
                ..
            })
        ));
        assert_eq!(destructive.style.disabled_opacity, Some(0.25));
        assert_eq!(destructive.style.padding, [11, 16, 11, 16]);
        assert_eq!(destructive.style.radius, 9);
        assert_eq!(destructive.style.text_size, Some(12.5));
        assert_eq!(destructive.style.text_line_height, Some(1.35));
        assert_eq!(
            destructive.style.font_weight,
            Some(ResolvedStyleFontWeight::Semibold)
        );

        let ViewNode::Layout { children, .. } = &program.document.view else {
            panic!("fixture view must be a layout");
        };
        let button = program.style_use(children[0].span()).unwrap();
        assert_eq!(button.recipes, [RecipeId(2)]);
        assert_eq!(button.utilities.len(), 1);
        assert!(matches!(
            button.style.background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 3, .. }),
                ..
            })
        ));
        let input = program.style_use(children[1].span()).unwrap();
        assert!(matches!(
            input.style.focus_border_color,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 2, .. }),
                ..
            })
        ));
    }

    #[test]
    fn preserves_maximum_exact_padding_without_sentinel_values() {
        let source = format!(
            r#"app Padding
recipe all for box
  p-65535px
recipe axes for box
  px-65535px py-65534px
{THEME}view
  col
    box @all
      text "all"
    box @axes
      text "axes"
"#
        );
        let program = lower(analyze(&source).unwrap()).unwrap();

        assert_eq!(program.styles.recipes[0].style.padding, [u16::MAX; 4]);
        assert_eq!(
            program.styles.recipes[1].style.padding,
            [u16::MAX - 1, u16::MAX, u16::MAX - 1, u16::MAX]
        );
    }

    #[test]
    fn resolves_theme_contract_palettes_and_native_factories() {
        let source = r#"extern crate::backend
  theme native_theme(dark:bool)
app Themes
  theme native_theme(dark)
  palette active_palette
theme contract Ducktape
  bg
  fg
  primary
  danger
  surface
palette light for Ducktape
  bg #ffffff
  fg #111111
  primary #3366ff
  danger #cc3344
  surface #f4f4f480
palette dark for Ducktape
  bg #111111
  fg #ffffff
  primary #88aaff
  danger #ff6677
  surface #222222
state
  dark = false
  active_palette:palette[Ducktape] = Ducktape.light
view
  theme native_theme(!dark) fg=fg bg=linear(1.57, surface@0.0, bg@1.0)
    text "Theme" @text-surface/60
"#;
        let program = lower(analyze(source).unwrap()).unwrap();
        let theme = program.theme();
        assert_eq!(theme.contract.name, "Ducktape");
        assert_eq!(
            theme
                .contract
                .tokens
                .iter()
                .map(|token| token.name.as_str())
                .collect::<Vec<_>>(),
            ["bg", "fg", "primary", "danger", "surface"]
        );
        assert_eq!(theme.palettes.len(), 2);
        assert_eq!(theme.palettes[0].colors[4].rgba, [244, 244, 244, 128]);
        assert!(matches!(
            theme.active_palette,
            ResolvedPaletteSelection::Dynamic(Expr::Path(_))
        ));
        let ResolvedAppThemeSelection::Factory(factory) = &theme.app_theme else {
            panic!("app theme factory must be resolved");
        };
        assert_eq!(
            program.extern_function(factory.function).name,
            "native_theme"
        );
        let nested = program
            .nested_theme(&program.document.view.span().clone())
            .unwrap();
        let ResolvedThemePreset::Factory(factory) = &nested.preset else {
            panic!("nested theme factory must be resolved");
        };
        assert_eq!(
            program.extern_function(factory.function).name,
            "native_theme"
        );
        assert!(matches!(
            nested.text,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 1, .. }),
                ..
            })
        ));
        let ResolvedBackground::Linear { stops, .. } = &nested.background.as_ref().unwrap() else {
            panic!("nested gradient must be normalized");
        };
        assert_eq!(stops.len(), 2);
        let ViewNode::Theme { content, .. } = &program.document.view else {
            panic!("fixture root must be a theme");
        };
        let style = program.style_use(content.span()).unwrap();
        assert!(matches!(
            style.style.text_color,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(ThemeTokenId { index: 4, .. }),
                opacity: Some(60),
            })
        ));
    }

    #[test]
    fn rejects_checked_style_and_palette_states_that_cannot_be_normalized() {
        let source = format!(
            "app Demo\nrecipe label for text\n  text-fg\n{THEME}view\n  text \"ok\" @label\n"
        );
        let mut checked = analyze(&source).unwrap();
        checked.document.recipes[0].base = Some("missing".into());
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error
                .message
                .contains("unknown checked recipe base `missing`")
        );

        let mut checked = analyze(&source).unwrap();
        checked.document.palettes[0].colors.remove("fg");
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("missing checked token `fg`"));

        let mut checked = analyze(&source).unwrap();
        checked.document.recipes[0].utilities[0] = "rounded-nope".into();
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("invalid checked radius utility"));

        let inheritance = format!(
            "app Demo\nrecipe base for text\n  text-fg\nrecipe child for text extends base\n  font-bold\n{THEME}view\n  text \"ok\" @child\n"
        );
        let mut checked = analyze(&inheritance).unwrap();
        checked.document.recipes[1].target = StyleRecipeTarget::Container;
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(
            error.message.contains(
                "recipe `child` targets `box` but its checked base `base` targets `text`"
            )
        );

        let mut checked = analyze(&inheritance).unwrap();
        checked.document.recipes[0].base = Some("child".into());
        let error = lower(checked).unwrap_err();
        assert_eq!(error.code, "E196");
        assert!(error.message.contains("checked recipe cycle includes"));
    }

    #[test]
    fn classifies_static_app_and_nested_builtin_theme_choices() {
        let source = r#"app StaticThemes
  theme "dark"
  palette AppTheme.second
theme contract AppTheme
  bg
  fg
  primary
  danger
palette first for AppTheme
  bg #000000
  fg #ffffff
  primary #336699
  danger #cc0000
palette second for AppTheme
  bg #ffffff
  fg #000000
  primary #6688cc
  danger #dd3344
view
  theme light
    text "built in"
"#;
        let program = lower(analyze(source).unwrap()).unwrap();
        assert!(matches!(
            program.theme().active_palette,
            ResolvedPaletteSelection::Static(PaletteId(1))
        ));
        assert!(matches!(
            &program.theme().app_theme,
            ResolvedAppThemeSelection::BuiltIn(name) if name == "dark"
        ));
        let nested = program.nested_theme(program.document.view.span()).unwrap();
        assert!(matches!(
            &nested.preset,
            ResolvedThemePreset::BuiltIn(name) if name == "light"
        ));

        let explicit_default = source.replace("theme \"dark\"", "theme \"default\"");
        let program = lower(analyze(&explicit_default).unwrap()).unwrap();
        assert!(matches!(
            program.theme().app_theme,
            ResolvedAppThemeSelection::Default
        ));
    }

    #[test]
    fn resolves_namespaced_recipe_origins_without_losing_the_physical_file() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "ui-lang-style-origins-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let root = directory.join("app.ice");
        let imported = directory.join("styles.ice");
        fs::write(
            &root,
            format!(
                "app Demo\n  theme ui::native_theme(dark)\nuse \"styles.ice\" as ui\n{THEME}state\n  dark = false\nview\n  theme ui::native_theme(!dark)\n    text \"Imported\" @ui::emphasis\n"
            ),
        )
        .unwrap();
        fs::write(
            &imported,
            "extern crate::backend\n  theme native_theme(dark:bool)\nrecipe label for text\n  text-fg\nrecipe emphasis for text extends label\n  font-bold\ncomponent Decorated()\n  box @bg-primary\n    text \"decorated\"\n",
        )
        .unwrap();

        let program = lower(analyze_file(&root).unwrap()).unwrap();
        let recipe = program
            .styles
            .recipes
            .iter()
            .find(|recipe| recipe.name == "ui::emphasis")
            .unwrap();
        let origin = program.origin(recipe.origin);
        assert_eq!(origin.path.as_deref(), Some(imported.as_path()));
        assert_eq!(origin.line, 5);
        let ResolvedAppThemeSelection::Factory(factory) = &program.theme().app_theme else {
            panic!("namespaced app theme factory must be resolved");
        };
        assert_eq!(
            program.extern_function(factory.function).name,
            "ui::native_theme"
        );
        let contract_origin = program.origin(program.theme().contract.origin);
        assert_eq!(contract_origin.path.as_deref(), Some(root.as_path()));
        let ViewNode::Theme { content, span, .. } = &program.document.view else {
            panic!("fixture root must be a nested theme");
        };
        let nested = program.nested_theme(span).unwrap();
        let ResolvedThemePreset::Factory(factory) = &nested.preset else {
            panic!("namespaced nested theme factory must be resolved");
        };
        assert_eq!(
            program.extern_function(factory.function).name,
            "ui::native_theme"
        );
        assert_eq!(
            program.origin(nested.origin).path.as_deref(),
            Some(root.as_path())
        );
        let style = program.style_use(content.span()).unwrap();
        assert_eq!(style.recipes, [recipe.id]);
        let imported_style = program
            .styles
            .style_uses
            .iter()
            .find(|style| {
                style.style.background
                    == Some(ResolvedThemeColor {
                        base: ResolvedThemeColorBase::Token(program.theme().native_tokens.primary),
                        opacity: None,
                    })
            })
            .expect("imported component style must be lowered");
        assert_eq!(
            program.origin(imported_style.origin).path.as_deref(),
            Some(imported.as_path())
        );
        assert_eq!(program.origin(imported_style.origin).line, 8);
        assert_eq!(
            imported_style.style.background,
            Some(ResolvedThemeColor {
                base: ResolvedThemeColorBase::Token(program.theme().native_tokens.primary),
                opacity: None,
            })
        );

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[ignore = "explicit large style lowering performance contract"]
    fn performance_contract_lowering_many_deep_recipes_and_uses_has_constant_per_use_recipe_work() {
        const TOKENS: usize = 128;
        const RECIPES: usize = 256;
        const USES: usize = 10_000;
        let mut source = String::from(
            "app StylePerf\ntheme contract PerfTheme\n  bg\n  fg\n  primary\n  danger\n",
        );
        for index in 0..TOKENS {
            writeln!(source, "  token_{index}").unwrap();
        }
        source.push_str(
            "palette app for PerfTheme\n  bg #000000\n  fg #ffffff\n  primary #336699\n  danger #cc0000\n",
        );
        for index in 0..TOKENS {
            writeln!(source, "  token_{index} #{:06x}", index + 1).unwrap();
        }
        source.push_str("recipe recipe_0 for text\n  text-token_0\n");
        for index in 1..RECIPES {
            writeln!(
                source,
                "recipe recipe_{index} for text extends recipe_{}\n  text-token_{}",
                index - 1,
                index % TOKENS
            )
            .unwrap();
        }
        source.push_str("view\n  col\n");
        for index in 0..USES {
            writeln!(source, "    text \"row-{index}\" @recipe_{}", RECIPES - 1).unwrap();
        }
        let checked = analyze(&source).unwrap();
        let started = Instant::now();
        let program = lower(checked).unwrap();
        let elapsed = started.elapsed();
        eprintln!("normalized {TOKENS} tokens, {RECIPES} recipes, and {USES} uses in {elapsed:?}");
        assert_eq!(program.theme().contract.tokens.len(), TOKENS + 4);
        assert_eq!(program.styles.recipes.len(), RECIPES);
        assert_eq!(program.styles.style_uses.len(), USES + 1);
        assert_eq!(
            program
                .styles
                .style_uses
                .iter()
                .map(|style| style.utilities.len())
                .sum::<usize>(),
            0,
            "recipe uses retain IDs and fixed-size styles, never inherited utility copies"
        );
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "128 tokens, 256 recipes, and 10k recipe uses lowered in {elapsed:?}"
        );
    }
}
