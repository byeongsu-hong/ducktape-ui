use crate::ast::*;
use crate::check::{
    CheckedLocalId, CheckedMatchPattern, CheckedValueRef, CheckedViewFlow,
    controlled_editor_bindings, controlled_state_bindings, expr_type,
};
use crate::hir::{HandlerId, RunSiteId};
use crate::lower::*;
use crate::{Error, canonical_snake};
use std::collections::HashMap;
use std::fmt::Write;
use std::ops::Deref;
use std::path::Path;

const RECONCILIATION_SCOPE_BINDING: &str = "\0__ice_reconciliation_scope";
const SOURCE_MARKER: &str = "// __ICE_SOURCE ";
const SOURCE_MARKER_END: &str = "// __ICE_SOURCE_END";
const RENDER_SOURCE_MARKER: &str = "__ICE_RENDER_SOURCE_";
const RENDER_SOURCE_MARKER_END: &str = "__";

fn source_marker(span: &Span) -> String {
    format!("{SOURCE_MARKER}{} {}", span.line, span.column)
}

fn source_marker_origin(program: &LoweredProgram, origin: crate::hir::OriginId) -> String {
    let origin = program.origin(origin);
    match &origin.path {
        Some(path) => format!(
            "{SOURCE_MARKER}{} {} {}",
            origin.line,
            origin.column,
            encode_source_path(&path.display().to_string())
        ),
        None => format!("{SOURCE_MARKER}{} {}", origin.line, origin.column),
    }
}

fn source_marker_for_origin(program: &LoweredProgram, origin: crate::hir::OriginId) -> String {
    source_marker_origin(program, origin)
}

fn source_mapped_expression(
    code: String,
    span: &Span,
    message: &str,
    track_descendants: bool,
) -> String {
    let render_source = format!(
        "{RENDER_SOURCE_MARKER}{}_{}{RENDER_SOURCE_MARKER_END}",
        span.line, span.column
    );
    let descendant_source = if track_descendants {
        "#[cfg(test)]\nlet __ice_rendered = ::ui_lang_runtime::testing::sourced(__ice_rendered, __ice_render_source_location);\n"
    } else {
        ""
    };
    format!(
        "{{\n{}\n#[cfg(test)]\nlet __ice_render_source_location = {render_source};\n#[cfg(test)]\nlet __ice_render_source = ::ui_lang_runtime::testing::push_render_source(__ice_render_source_location);\nlet __ice_rendered: __IceElement<'_, {message}> = {code};\n{descendant_source}#[cfg(test)]\ndrop(__ice_render_source);\n__ice_rendered\n{SOURCE_MARKER_END}\n}}",
        source_marker(span)
    )
}

pub(crate) fn encode_source_path(path: &str) -> String {
    path.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn resolve_source_markers(
    generated: String,
    document: &LoweredProgram,
    source_path: &str,
) -> String {
    let mut output = String::with_capacity(generated.len());
    for line in generated.lines() {
        let resolved_marker = line
            .strip_prefix(SOURCE_MARKER)
            .and_then(|location| location.split_once(' '))
            .and_then(|(line, column)| {
                Some((line.parse::<usize>().ok()?, column.parse::<usize>().ok()?))
            })
            .map(|(merged_line, column)| {
                let (path, line) = document.source_origin(merged_line).map_or_else(
                    || (source_path.to_owned(), merged_line),
                    |(path, line)| (path.display().to_string(), line),
                );
                format!(
                    "{SOURCE_MARKER}{line} {column} {}",
                    encode_source_path(&path)
                )
            })
            .unwrap_or_else(|| line.to_owned());
        let resolved = resolve_render_source_markers(&resolved_marker, document, source_path);
        output.push_str(&resolved);
        output.push('\n');
    }
    output
}

fn resolve_render_source_markers(
    line: &str,
    document: &LoweredProgram,
    source_path: &str,
) -> String {
    let mut output = String::with_capacity(line.len());
    let mut remaining = line;
    while let Some(start) = remaining.find(RENDER_SOURCE_MARKER) {
        output.push_str(&remaining[..start]);
        let marker = &remaining[start + RENDER_SOURCE_MARKER.len()..];
        let Some(end) = marker.find(RENDER_SOURCE_MARKER_END) else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let Some((line, column)) = marker[..end].split_once('_').and_then(|(line, column)| {
            Some((line.parse::<usize>().ok()?, column.parse::<usize>().ok()?))
        }) else {
            output.push_str(&remaining[start..start + RENDER_SOURCE_MARKER.len() + end]);
            remaining = &marker[end..];
            continue;
        };
        let (path, line) = document.source_origin(line).map_or_else(
            || (source_path.to_owned(), line),
            |(path, line)| (path.display().to_string(), line),
        );
        write!(
            output,
            "::ui_lang_runtime::testing::Location::new({}, {line}, {column}, \"rendered view node\")",
            rust_string(&path)
        )
        .unwrap();
        remaining = &marker[end + RENDER_SOURCE_MARKER_END.len()..];
    }
    output.push_str(remaining);
    output
}

fn reconciliation_scope<'a>(public_scope: &'a str, env: &'a dyn BindingEnvironment) -> &'a str {
    env.get(RECONCILIATION_SCOPE_BINDING)
        .map_or(public_scope, |binding| binding.code.as_str())
}

fn reconciliation_scope_binding(code: String) -> Binding {
    Binding {
        code,
        ty: Type::Str,
        local: true,
        state: None,
        owner: None,
    }
}

fn component_state_scopes(env: &dyn BindingEnvironment) -> Vec<String> {
    let mut scopes = Vec::new();
    env.visit(&mut |_, binding| {
        if let Some(StateBinding::Component { scope, .. }) = &binding.state {
            scopes.push(scope.clone());
        }
    });
    scopes
}

fn set_reconciliation_scope(env: &mut HashMap<String, Binding>, code: String) {
    env.insert(
        RECONCILIATION_SCOPE_BINDING.into(),
        reconciliation_scope_binding(code),
    );
}

fn checked_local_binding(
    program: &LoweredProgram,
    local_id: CheckedLocalId,
    code: String,
    is_local: bool,
) -> Binding {
    let local = program.checked_facts().local(local_id);
    Binding {
        code,
        ty: local.ty.clone(),
        local: is_local,
        state: None,
        owner: Some(BindingOwner::Local(local_id)),
    }
}

fn checked_match_pattern_code(
    program: &LoweredProgram,
    pattern: &CheckedMatchPattern,
    binding: Option<CheckedLocalId>,
    value_ty: &Type,
    span: &Span,
) -> Result<String, Error> {
    let binding_name = binding.map(|binding| {
        program
            .checked_facts()
            .local(binding)
            .name
            .as_str()
            .to_owned()
    });
    Ok(match pattern {
        CheckedMatchPattern::Some => format!(
            "::std::option::Option::Some({})",
            binding_name.ok_or_else(|| Error::new(
                "E196",
                span,
                "checked some pattern has no payload local",
            ))?
        ),
        CheckedMatchPattern::None => "::std::option::Option::None".into(),
        CheckedMatchPattern::Ok => format!(
            "::std::result::Result::Ok({})",
            binding_name.ok_or_else(|| Error::new(
                "E196",
                span,
                "checked ok pattern has no payload local",
            ))?
        ),
        CheckedMatchPattern::Err => format!(
            "::std::result::Result::Err({})",
            binding_name.ok_or_else(|| Error::new(
                "E196",
                span,
                "checked err pattern has no payload local",
            ))?
        ),
        CheckedMatchPattern::Enum(id) => {
            let owner = program
                .declarations()
                .try_enum_decl(id.owner)
                .ok_or_else(|| {
                    Error::new(
                        "E196",
                        span,
                        "checked match pattern references an invalid enum ID",
                    )
                })?;
            let variant = program
                .declarations()
                .try_enum_variant_decl(*id)
                .ok_or_else(|| {
                    Error::new(
                        "E196",
                        span,
                        "checked match pattern references an invalid enum variant ID",
                    )
                })?;
            binding_name.map_or_else(
                || format!("{}::{}", owner.rust_name, pascal(&variant.name)),
                |binding| format!("{}::{}({binding})", owner.rust_name, pascal(&variant.name)),
            )
        }
        CheckedMatchPattern::Palette(id) => {
            let Type::Palette(contract) = value_ty else {
                return Err(Error::new(
                    "E196",
                    span,
                    "checked palette pattern has a non-palette value type",
                ));
            };
            let palette = program.declarations().palette_name(*id).ok_or_else(|| {
                Error::new(
                    "E196",
                    span,
                    "checked palette pattern references an invalid palette ID",
                )
            })?;
            format!("{}::{}", generated_named_rust(contract), pascal(palette))
        }
        CheckedMatchPattern::Wildcard => "_".into(),
    })
}

pub(in crate::codegen) fn find_extern_function<'a>(
    document: &'a Document,
    name: &str,
    kind: ExternKind,
) -> Option<&'a ExternFn> {
    document
        .functions
        .iter()
        .find(|item| item.name == name && item.kind == kind)
}

pub(in crate::codegen) fn component_run_sites(
    program: &LoweredProgram,
    handlers: &[HandlerId],
) -> Vec<(RunSiteId, FutureMode)> {
    handlers
        .iter()
        .flat_map(|handler| &program.handler(*handler).statements)
        .filter_map(|statement| match &statement.kind {
            ResolvedStatementKind::Run(run) => run.site.map(|site| (site, run.mode)),
            _ => None,
        })
        .collect()
}

pub(in crate::codegen) fn handler_future(
    handler: &ResolvedHandler,
) -> Option<(FutureMode, Option<RunSiteId>)> {
    handler
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            ResolvedStatementKind::Run(run) if run.kind == EffectKind::Future => {
                Some((run.mode, run.site))
            }
            _ => None,
        })
}

pub(in crate::codegen) fn event_filter_type(name: &str) -> String {
    if canonical_snake(name) {
        format!("__IceEventFilter{}", pascal(name))
    } else {
        format!("__Ice0E{}", rust_identifier_hex(name))
    }
}

fn generate_derived(out: &mut String, program: &LoweredProgram) -> Result<(), Error> {
    let document = program.document();
    let env = checked_state_env(program, "self");
    for derived in program.derived() {
        let value = checked_expr_use_code(program, derived.initializer, &env, ValueMode::Owned)?;
        writeln!(out, "{}", source_marker(&derived.span)).unwrap();
        writeln!(
            out,
            "fn {}(&self) -> {} {{ {value} }}",
            derived_method(&derived.name),
            derived.ty.rust(&document.structs),
        )
        .unwrap();
        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
    }
    Ok(())
}

pub fn generate(program: &LoweredProgram, source_path: &str) -> Result<String, Error> {
    program.validate_handler_hir()?;
    let document = program.document();
    let message = format!("__{}Message", document.app);
    let lint_macro = format!("__ice_generated_items_{}", encode_source_path(source_path));
    let mut out = String::new();
    // Attributes on `include!` do not reach the included items, while a module
    // wrapper would change their visibility and name-resolution scope. Expand
    // the items in place and attach the lint boundary to each one instead.
    writeln!(
        out,
        "macro_rules! {lint_macro} {{ ($($item:item)*) => {{ $(#[allow(warnings, clippy::all)] $item)* }}; }}\n{lint_macro}! {{"
    )
    .unwrap();
    writeln!(
        out,
        "const _: &str = include_str!({});",
        rust_string(source_path)
    )
    .unwrap();
    match &program.settings().renderer {
        ResolvedRendererSelection::Default => writeln!(
            out,
            "type __IceRenderer = ::iced::Renderer; type __IceElement<'a, Message, Theme = ::iced::Theme> = ::iced::Element<'a, Message, Theme, __IceRenderer>;"
        )
        .unwrap(),
        ResolvedRendererSelection::Custom { path, origin } => writeln!(
            out,
            "{}\ntype __IceRenderer = {path}; type __IceElement<'a, Message, Theme = ::iced::Theme> = ::iced::Element<'a, Message, Theme, __IceRenderer>;\n{SOURCE_MARKER_END}",
            source_marker_for_origin(program, *origin)
        )
        .unwrap(),
    }
    generate_keyboard_types(&mut out, document);
    generate_system_types(&mut out, document);
    generate_widget_selector_types(&mut out, document);
    generate_canvas_types(&mut out, document);
    generate_pane_types(&mut out, program)?;
    let theme = program.theme();
    let token_count = theme.contract.tokens.len();
    writeln!(
        out,
        "#[allow(dead_code)]\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub(crate) enum {} {{",
        generated_named_rust(&theme.contract.name)
    )
    .unwrap();
    for palette in &theme.palettes {
        writeln!(out, "{},", pascal(&palette.name)).unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(
        out,
        "#[derive(Clone, Copy)]\nstruct __IcePalette {{ name: &'static str, colors: [::iced::Color; {token_count}] }}"
    )
    .unwrap();

    for item in &document.enums {
        let derives = if item
            .variants
            .iter()
            .all(|variant| variant.payload.is_none())
        {
            "Debug, Clone, Copy, PartialEq, Eq"
        } else {
            "Clone"
        };
        writeln!(
            out,
            "#[allow(dead_code)]\n#[derive({derives})]\npub(crate) enum {} {{",
            generated_named_rust(&item.name)
        )
        .unwrap();
        for variant in &item.variants {
            let name = pascal(&variant.name);
            if let Some(payload) = &variant.payload {
                writeln!(out, "{name}({}),", payload.rust(&document.structs)).unwrap();
            } else {
                writeln!(out, "{name},").unwrap();
            }
        }
        writeln!(out, "}}").unwrap();
    }

    for component in program
        .components()
        .iter()
        .filter(|component| component.storage != ComponentStorage::Stateless)
    {
        let ty = component_state_type(&component.name);
        writeln!(out, "#[allow(dead_code)]\npub(crate) struct {ty} {{").unwrap();
        for state in &component.states {
            writeln!(out, "{}", source_marker(&state.span)).unwrap();
            writeln!(out, "{}: {},", state.name, state.ty.rust(&document.structs)).unwrap();
            writeln!(out, "{SOURCE_MARKER_END}").unwrap();
        }
        for (site, _) in component_run_sites(program, &component.handlers) {
            writeln!(out, "{}: u64,", component_latest_field(site.0 as usize)).unwrap();
        }
        for (site, _mode) in component_run_sites(program, &component.handlers)
            .into_iter()
            .filter(|(_, mode)| *mode == FutureMode::Replace)
        {
            writeln!(
                out,
                "{}: ::std::option::Option<::iced::task::Handle>,",
                component_replace_field(site.0 as usize)
            )
            .unwrap();
        }
        writeln!(
            out,
            "}}\nimpl ::std::default::Default for {ty} {{\nfn default() -> Self {{ Self {{"
        )
        .unwrap();
        for state in &component.states {
            writeln!(out, "{}", source_marker(&state.span)).unwrap();
            writeln!(
                out,
                "{}: {},",
                state.name,
                resolved_initializer_code(&state.initializer, program)?
            )
            .unwrap();
            writeln!(out, "{SOURCE_MARKER_END}").unwrap();
        }
        for (site, _) in component_run_sites(program, &component.handlers) {
            writeln!(out, "{}: 0,", component_latest_field(site.0 as usize)).unwrap();
        }
        for (site, _mode) in component_run_sites(program, &component.handlers)
            .into_iter()
            .filter(|(_, mode)| *mode == FutureMode::Replace)
        {
            writeln!(
                out,
                "{}: ::std::option::Option::None,",
                component_replace_field(site.0 as usize)
            )
            .unwrap();
        }
        writeln!(out, "}} }}\n}}").unwrap();
    }

    writeln!(out, "#[allow(dead_code)]\npub struct {} {{", document.app).unwrap();
    writeln!(
        out,
        "pub(crate) __ice_accessibility: ::ui_lang_runtime::Bridge<{message}>,"
    )
    .unwrap();
    if program.settings().kind == ProgramKind::Application {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\npub(crate) __ice_accessibility_initial: ::std::option::Option<usize>,\n#[cfg(all(target_os = \"windows\", not(test)))]\npub(crate) __ice_accessibility_pending: ::std::vec::Vec<{message}>,"
        )
        .unwrap();
    }
    for (node, test_only) in document_pane_grids(program) {
        let ViewNode::PaneGrid {
            name,
            configuration,
            templates,
            ..
        } = node
        else {
            unreachable!()
        };
        let pane_state = if templates.is_empty() {
            "&'static str".into()
        } else {
            pane_type(name)
        };
        if test_only {
            writeln!(out, "#[cfg(test)]").unwrap();
        }
        writeln!(
            out,
            "pub(crate) {}: ::iced::widget::pane_grid::State<{pane_state}>,",
            pane_field(name)
        )
        .unwrap();
        if pane_split_slots(configuration).iter().any(Option::is_some) {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(
                out,
                "pub(crate) {}: ::std::collections::BTreeMap<&'static str, ::iced::widget::pane_grid::Split>,",
                pane_splits_field(name)
            )
            .unwrap();
        }
    }
    for state in &document.states {
        writeln!(out, "{}", source_marker(&state.span)).unwrap();
        writeln!(
            out,
            "pub(crate) {}: {},",
            state.name,
            state.ty.rust(&document.structs)
        )
        .unwrap();
        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
    }
    for component in program
        .components()
        .iter()
        .filter(|component| component.storage != ComponentStorage::Stateless)
    {
        let field = component_state_field(&component.name);
        let ty = component_state_type(&component.name);
        match component.storage {
            ComponentStorage::Retained => writeln!(
                out,
                "pub(crate) {field}: ::std::collections::HashMap<::std::string::String, {ty}>,"
            )
            .unwrap(),
            ComponentStorage::Mounted => writeln!(
                out,
                "pub(crate) {field}: ::ui_lang_runtime::MountedComponentState<{ty}>,"
            )
            .unwrap(),
            ComponentStorage::Stateless => unreachable!(),
        }
    }
    writeln!(out, "}}").unwrap();
    writeln!(
        out,
        "impl ::std::fmt::Debug for {} {{ fn fmt(&self, __formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {{ __formatter.write_str({}) }} }}",
        document.app,
        rust_string(&document.app)
    )
    .unwrap();

    writeln!(out, "#[derive(Clone)]\npub(crate) enum {message} {{").unwrap();
    writeln!(
        out,
        "__AccessibilitySnapshot(::std::boxed::Box<::ui_lang_runtime::Snapshot<{message}>>),\n__AccessibilityAction(::ui_lang_runtime::ActionRequest),\n__AccessibilityWindow(::iced::window::Id, ::iced::window::Event),\n#[cfg(all(target_os = \"windows\", not(test)))]\n__AccessibilityNativeWindow(::ui_lang_runtime::NativeWindow),\n__AccessibilityFocusNext,\n__AccessibilityFocusPrevious,"
    )
    .unwrap();
    for handler in program.app_handlers() {
        if handler.name == "mount" {
            continue;
        }
        let variant = handler_variant(&handler.name);
        if handler.params.is_empty() {
            writeln!(out, "{variant},").unwrap();
        } else {
            let fields = handler
                .params
                .iter()
                .map(|param| param.ty.rust(&document.structs))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(out, "{variant}({fields}),").unwrap();
        }
    }
    for component in program.components() {
        for (site, _) in component_run_sites(program, &component.handlers) {
            writeln!(
                out,
                "{}(::std::string::String, u64, ::std::boxed::Box<{message}>),",
                component_latest_variant(&component.name, site.0 as usize)
            )
            .unwrap();
        }
        for handler in component.handlers.iter().map(|id| program.handler(*id)) {
            let variant = component_handler_variant(&component.name, &handler.name);
            let fields = ::std::iter::once("::std::string::String".to_owned())
                .chain(
                    handler
                        .params
                        .iter()
                        .map(|param| param.ty.rust(&document.structs)),
                )
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(out, "{variant}({fields}),").unwrap();
        }
        for state in component
            .states
            .iter()
            .filter(|state| state.ty == Type::Str)
        {
            writeln!(
                out,
                "{}(::std::string::String, ::std::string::String),",
                component_binding_variant(&component.name, &state.name)
            )
            .unwrap();
        }
    }
    for binding in controlled_state_bindings(document, false)
        .expect("checker validates controlled input bindings")
    {
        writeln!(out, "{}(::std::string::String),", binding_variant(&binding)).unwrap();
    }
    for binding in controlled_state_bindings(document, true)
        .expect("checker validates controlled editor bindings")
    {
        writeln!(
            out,
            "{}(::iced::widget::text_editor::Action),",
            editor_variant(&binding)
        )
        .unwrap();
    }
    if needs_extern_noop(document) {
        writeln!(out, "__ExternNoop,").unwrap();
    }
    if has_animations(program) {
        writeln!(out, "__AnimationFrame,").unwrap();
    }
    for (node, test_only) in document_pane_grids(program) {
        let ViewNode::PaneGrid { name, options, .. } = node else {
            unreachable!()
        };
        if options.resize_leeway.is_some() {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(
                out,
                "{}(::iced::widget::pane_grid::ResizeEvent),",
                pane_resize_variant(name)
            )
            .unwrap();
        }
        if options.draggable {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(
                out,
                "{}(::iced::widget::pane_grid::DragEvent),",
                pane_drag_variant(name)
            )
            .unwrap();
        }
    }
    writeln!(out, "}}").unwrap();
    writeln!(
        out,
        "impl ::std::fmt::Debug for {message} {{ fn fmt(&self, __formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {{ __formatter.write_str({}) }} }}",
        rust_string(&message)
    )
    .unwrap();

    generate_extern_probes(&mut out, document);
    generate_editor_binding_mapper(&mut out, document);
    writeln!(out, "#[allow(unused_parens)]\nimpl {} {{", document.app).unwrap();
    let app_settings = program.settings();
    if let Some(font) = &app_settings.default_font {
        writeln!(out, "{}", source_marker_for_origin(program, font.origin)).unwrap();
        writeln!(
            out,
            "#[must_use]\npub fn default_font() -> ::iced::Font {{ {} }}\n{SOURCE_MARKER_END}",
            resolved_default_font_code(font)
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "#[must_use]\npub fn default_font() -> ::iced::Font {{ ::iced::Font::DEFAULT }}"
        )
        .unwrap();
    }
    generate_derived(&mut out, program)?;
    generate_named_windows(&mut out, program, app_settings, source_path);
    let subscription = ".subscription(Self::__subscription)";
    let default_font = if app_settings.default_font.is_some() {
        ".default_font(Self::default_font())"
    } else {
        ""
    };
    let title = app_settings
        .title
        .as_ref()
        .map_or("", |_| ".title(Self::__title)");
    let settings = app_settings_code(program, app_settings);
    let fonts = font_assets_code(program, app_settings, source_path);
    let window = if app_settings.kind == ProgramKind::Daemon {
        String::new()
    } else {
        window_settings_code(program, &app_settings.primary_window, source_path)
    };
    let executor = match &app_settings.executor {
        ResolvedExecutorSelection::Default => String::new(),
        ResolvedExecutorSelection::Custom { path, origin } => format!(
            "\n{}\n.executor::<{path}>()\n{SOURCE_MARKER_END}\n",
            source_marker_for_origin(program, *origin)
        ),
    };
    let presets = if document.presets.is_empty() {
        String::new()
    } else {
        format!(
            ".presets([{}])",
            document
                .presets
                .iter()
                .enumerate()
                .map(|(index, preset)| format!(
                    "::iced::Preset::new({}, Self::__preset_{index})",
                    rust_string(&preset.name)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let scale_factor = app_settings
        .scale_factor
        .as_ref()
        .map_or("", |_| ".scale_factor(Self::__scale_factor)");
    let style = if app_settings.background.is_some() || app_settings.text_color.is_some() {
        ".style(Self::__style)"
    } else {
        ""
    };
    let root = if app_settings.kind == ProgramKind::Daemon {
        "::iced::daemon(Self::__boot, Self::__update, Self::__view)"
    } else {
        "::iced::application(Self::__boot, Self::__update, Self::__view)"
    };
    let program_kind = if app_settings.kind == ProgramKind::Daemon {
        "::iced::Daemon"
    } else {
        "::iced::Application"
    };
    writeln!(
        out,
        "fn __program() -> {program_kind}<impl ::iced::Program<State = Self, Message = {message}, Theme = ::iced::Theme>> {{"
    )
    .unwrap();
    writeln!(out, "{root}{title}{subscription}.theme(Self::__theme){style}{settings}{default_font}{fonts}{window}{scale_factor}{executor}{presets}").unwrap();
    writeln!(
        out,
        "}}\n#[allow(dead_code)]\npub fn run() -> ::iced::Result {{\nSelf::__program().run()\n}}"
    )
    .unwrap();

    generate_theme(&mut out, program)?;
    generate_boot(&mut out, program, &message)?;
    generate_presets(&mut out, program, &message)?;
    generate_update(&mut out, program, &message)?;
    generate_subscription(
        &mut out,
        document,
        app_settings,
        has_animations(program),
        &message,
    )?;
    generate_view(&mut out, program, &message)?;
    generate_test_mounts(&mut out, program, &message, source_path)?;
    writeln!(out, "}}").unwrap();
    generate_tests(&mut out, program, &message, source_path)?;
    writeln!(out, "}}").unwrap();
    Ok(resolve_source_markers(out, program, source_path))
}

pub(in crate::codegen) struct RenderDocument<'a> {
    program: &'a LoweredProgram,
}

impl<'a> RenderDocument<'a> {
    pub(in crate::codegen) fn new(program: &'a LoweredProgram) -> Self {
        Self { program }
    }

    pub(in crate::codegen) fn program(&self) -> &'a LoweredProgram {
        self.program
    }
}

impl Deref for RenderDocument<'_> {
    type Target = Document;

    fn deref(&self) -> &Self::Target {
        self.program.document()
    }
}

mod application;
mod canvas;
mod expr;
mod probes;
mod runtime;
mod settings;
mod statement;
mod style;
mod subscription;
mod testing;
mod view;

use application::*;
use canvas::*;
use expr::*;
use probes::*;
use runtime::*;
use settings::*;
use statement::*;
use style::*;
use subscription::*;
use testing::*;
use view::*;

#[cfg(test)]
#[path = "codegen/tests.rs"]
mod tests;
