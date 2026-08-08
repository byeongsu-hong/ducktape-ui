use crate::hir::{ExternFnId, HandlerId, RunSiteId, canonical_rust_type_name};
use crate::lower::*;
use crate::semantic::*;
use crate::{Error, canonical_snake};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;
use std::path::Path;

const RECONCILIATION_SCOPE_BINDING: &str = "\0__ice_reconciliation_scope";
const SOURCE_MARKER: &str = "// __ICE_SOURCE ";
const SOURCE_MARKER_END: &str = "// __ICE_SOURCE_END";
const RENDER_SOURCE_MARKER: &str = "__ICE_RENDER_SOURCE_";
const RENDER_SOURCE_MARKER_END: &str = "__";
const ASSET_PATH_MARKER: &str = "__ICE_ASSET_PATH_";
const ASSET_PATH_MARKER_END: &str = "__";
/// Fences around each per-fragment `mod` of outlined methods. The fenced
/// region is valid Rust where it stands (the monofile still compiles as-is);
/// ui-lang-build additionally splits every region into its own OUT_DIR file
/// and replaces it with an `include!`, so an edit to one source fragment
/// leaves every other fragment's spans — and therefore rustc's incremental
/// fingerprints — byte-identical.
pub const GROUP_MARKER_BEGIN: &str = "// __ICE_GROUP_BEGIN ";
pub const GROUP_MARKER_END: &str = "// __ICE_GROUP_END";

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

/// Slug of the source fragment an origin points into: the file stem plus a
/// short stable hash of the full path (stems repeat — `chat.ice` lives under
/// screens/, components/, and handlers/ in the ducktape app). Origins inside
/// the root document itself collapse to "root".
/// Group slugs for the app's own two large functions. A fragment slug is
/// either `root` or `<stem>_<8 hex digits>`, so neither of these can collide
/// with one.
const APP_UPDATE_GROUP: &str = "app_update";
const APP_VIEW_GROUP: &str = "app_view";

fn origin_fragment_slug(program: &LoweredProgram, origin: crate::hir::OriginId) -> String {
    let origin = program.origin(origin);
    let path = origin.path.clone().or_else(|| {
        program
            .source_origin(origin.line)
            .map(|(path, _)| path.to_owned())
    });
    let Some(path) = path else {
        return "root".to_owned();
    };
    let text = path.display().to_string();
    let stem: String = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "fragment".to_owned())
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("{stem}_{:08x}", fnv1a(text.as_bytes()) as u32)
}

/// FNV-1a rather than std's DefaultHasher: the latter is documented as
/// unstable across releases, and slug stability decides whether generated
/// file names (and their incremental caches) survive a toolchain bump.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

fn source_mapped_expression_origin(
    code: String,
    program: &LoweredProgram,
    origin: OriginId,
    message: &str,
    track_descendants: bool,
) -> String {
    let origin_value = program.origin(origin);
    let render_source = origin_value.path.as_ref().map_or_else(
        || {
            format!(
                "{RENDER_SOURCE_MARKER}{}_{}{RENDER_SOURCE_MARKER_END}",
                origin_value.line, origin_value.column
            )
        },
        |path| {
            format!(
                "::ui_lang_runtime::testing::Location::new({}, {}, {}, \"rendered view node\")",
                rust_string(&path.display().to_string()),
                origin_value.line,
                origin_value.column
            )
        },
    );
    let descendant_source = if track_descendants {
        "#[cfg(test)]\nlet __ice_rendered = ::ui_lang_runtime::testing::sourced(__ice_rendered, __ice_render_source_location);\n"
    } else {
        ""
    };
    format!(
        "{{\n{}\n#[cfg(test)]\nlet __ice_render_source_location = {render_source};\n#[cfg(test)]\nlet __ice_render_source = ::ui_lang_runtime::testing::push_render_source(__ice_render_source_location);\nlet __ice_rendered: __IceElement<'_, {message}> = {code};\n{descendant_source}#[cfg(test)]\ndrop(__ice_render_source);\n__ice_rendered\n{SOURCE_MARKER_END}\n}}",
        source_marker_for_origin(program, origin)
    )
}

pub(crate) fn encode_source_path(path: &str) -> String {
    path.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_source_path(encoded: &str) -> Option<String> {
    let bytes = (0..encoded.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(encoded.get(index..index + 2)?, 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    String::from_utf8(bytes).ok()
}

/// Defers a relative asset literal until [`generate`] knows the Ice source path.
///
/// The marker expands to a build-time path literal beside the `.ice` file,
/// which `include_bytes!` then embeds exactly like `font` and `icon-rgba`
/// assets already do, so neither the working directory nor the source tree
/// decides what a compiled binary loads.
pub(in crate::codegen) fn asset_path_marker(path: &str) -> String {
    format!(
        "{ASSET_PATH_MARKER}{}{ASSET_PATH_MARKER_END}",
        encode_source_path(path)
    )
}

fn resolve_asset_path_markers(line: &str, source_path: &str) -> String {
    let parent = Path::new(source_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let mut output = String::with_capacity(line.len());
    let mut remaining = line;
    while let Some(start) = remaining.find(ASSET_PATH_MARKER) {
        output.push_str(&remaining[..start]);
        let marker = &remaining[start + ASSET_PATH_MARKER.len()..];
        let Some(end) = marker.find(ASSET_PATH_MARKER_END) else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let Some(path) = decode_source_path(&marker[..end]) else {
            output.push_str(&remaining[start..start + ASSET_PATH_MARKER.len() + end]);
            remaining = &marker[end..];
            continue;
        };
        output.push_str(&rust_string(&parent.join(path).display().to_string()));
        remaining = &marker[end + ASSET_PATH_MARKER_END.len()..];
    }
    output.push_str(remaining);
    output
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
        output.push_str(&resolve_asset_path_markers(&resolved, source_path));
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

/// A scope expression that is about to be read, not consumed.
///
/// Scope bindings carry a trailing `.clone()` so they can be moved into a
/// closure or handed to a component by value. `format!` borrows through
/// `format_args!`, and `.to_owned()`/`.clone()` allocate their own copy, so in
/// those positions the trailing clone is one heap allocation per rendered node
/// per frame that nothing reads. Callers that really do move the scope must
/// keep it.
fn borrowed_scope(scope: &str) -> &str {
    scope.strip_suffix(".clone()").unwrap_or(scope)
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

enum LocalBindingTypeSource<'a> {
    Resolved(&'a LoweredProgram),
    Hir(&'a ResolvedMatchBinding),
}

fn resolved_local_binding(
    source: LocalBindingTypeSource<'_>,
    local_id: ResolvedLocalId,
    code: String,
    is_local: bool,
) -> Binding {
    let ty = match source {
        LocalBindingTypeSource::Resolved(program) => {
            program.expressions().local(local_id).ty.clone()
        }
        LocalBindingTypeSource::Hir(payload) => payload.ty.clone(),
    };
    Binding {
        code,
        ty,
        local: is_local,
        state: None,
        owner: Some(BindingOwner::Local(local_id)),
    }
}

fn resolved_match_pattern_code(
    arm: &ResolvedMatchArm,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let binding_name = arm.binding.as_ref().map(|binding| binding.name.as_str());
    Ok(match &arm.pattern {
        ResolvedMatchPattern::Some => format!(
            "::std::option::Option::Some({})",
            binding_name.ok_or_else(|| {
                program
                    .invariant_at_origin(arm.origin, "normalized some pattern has no payload local")
            })?
        ),
        ResolvedMatchPattern::None => "::std::option::Option::None".into(),
        ResolvedMatchPattern::Ok => format!(
            "::std::result::Result::Ok({})",
            binding_name.ok_or_else(|| program
                .invariant_at_origin(arm.origin, "normalized ok pattern has no payload local",))?
        ),
        ResolvedMatchPattern::Err => format!(
            "::std::result::Result::Err({})",
            binding_name.ok_or_else(|| program
                .invariant_at_origin(arm.origin, "normalized err pattern has no payload local",))?
        ),
        ResolvedMatchPattern::Enum { owner, variant } => binding_name.map_or_else(
            || format!("{}::{}", owner, pascal(variant)),
            |binding| format!("{}::{}({binding})", owner, pascal(variant)),
        ),
        ResolvedMatchPattern::Palette { contract, palette } => {
            format!(
                "{}::{}",
                canonical_rust_type_name(contract),
                pascal(palette)
            )
        }
        ResolvedMatchPattern::Wildcard => "_".into(),
    })
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
    let env = checked_state_env(program, "self");
    for derived in program.derived() {
        let value = resolved_expr_use_code(program, derived.initializer, &env, ValueMode::Owned)?;
        writeln!(out, "{}", source_marker(&derived.span)).unwrap();
        writeln!(
            out,
            "fn {}(&self) -> {} {{ {value} }}",
            derived_method(&derived.name),
            rust_type_code(program, &derived.ty),
        )
        .unwrap();
        writeln!(out, "{SOURCE_MARKER_END}").unwrap();
    }
    Ok(())
}

pub fn generate(program: &LoweredProgram, source_path: &str) -> Result<String, Error> {
    #[cfg(test)]
    {
        program.validate_view_hir()?;
        program.validate_handler_hir()?;
    }
    let extern_component_declarations = program.extern_component_declarations()?;
    let extern_component_ids = extern_component_declarations
        .iter()
        .map(|declaration| declaration.id)
        .collect::<HashSet<_>>();
    let app_name = program.app_name();
    let message = format!("__{app_name}Message");
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
    generate_keyboard_types(&mut out, program, program.subscriptions());
    generate_system_types(&mut out, program);
    generate_widget_selector_types(&mut out, program);
    generate_canvas_types(&mut out, program);
    generate_pane_types(&mut out, program)?;
    let theme = program.theme();
    let token_count = theme.contract.tokens.len();
    writeln!(
        out,
        "#[allow(dead_code)]\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub(crate) enum {} {{",
        canonical_rust_type_name(&theme.contract.name)
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

    for item in program.enum_declarations() {
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
            &item.rust_name
        )
        .unwrap();
        for variant in &item.variants {
            let name = pascal(&variant.name);
            if let Some(payload) = &variant.payload {
                writeln!(out, "{name}({}),", rust_type_code(program, payload)).unwrap();
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
            writeln!(
                out,
                "{}: {},",
                state.name,
                rust_type_code(program, &state.ty)
            )
            .unwrap();
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

    writeln!(out, "#[allow(dead_code)]\npub struct {app_name} {{").unwrap();
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
    for (pane, test_only) in document_pane_grids(program) {
        let pane_state = if pane.templates.is_empty() {
            "&'static str".into()
        } else {
            pane_type(&pane.name)
        };
        if test_only {
            writeln!(out, "#[cfg(test)]").unwrap();
        }
        writeln!(
            out,
            "pub(crate) {}: ::iced::widget::pane_grid::State<{pane_state}>,",
            pane_field(&pane.name)
        )
        .unwrap();
        if pane_split_slots(&pane.configuration)
            .iter()
            .any(Option::is_some)
        {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(
                out,
                "pub(crate) {}: ::std::collections::BTreeMap<&'static str, ::iced::widget::pane_grid::Split>,",
                pane_splits_field(&pane.name)
            )
            .unwrap();
        }
    }
    for state in program.app_states() {
        writeln!(out, "{}", source_marker(&state.span)).unwrap();
        writeln!(
            out,
            "pub(crate) {}: {},",
            state.name,
            rust_type_code(program, &state.ty)
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
        app_name,
        rust_string(app_name)
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
                .map(|param| rust_type_code(program, &param.ty))
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
                        .map(|param| rust_type_code(program, &param.ty)),
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
    for binding in program.controlled_input_bindings()? {
        writeln!(out, "{}(::std::string::String),", binding_variant(binding)).unwrap();
    }
    for binding in program.controlled_editor_bindings()? {
        writeln!(
            out,
            "{}(::iced::widget::text_editor::Action),",
            editor_variant(&binding.name)
        )
        .unwrap();
    }
    if needs_extern_noop(program) {
        writeln!(out, "__ExternNoop,").unwrap();
    }
    if has_animations(program) {
        writeln!(out, "__AnimationFrame,").unwrap();
    }
    for (pane, test_only) in document_pane_grids(program) {
        if pane.resize_leeway.is_some() {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(
                out,
                "{}(::iced::widget::pane_grid::ResizeEvent),",
                pane_resize_variant(&pane.name)
            )
            .unwrap();
        }
        if pane.draggable {
            if test_only {
                writeln!(out, "#[cfg(test)]").unwrap();
            }
            writeln!(
                out,
                "{}(::iced::widget::pane_grid::DragEvent),",
                pane_drag_variant(&pane.name)
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

    generate_extern_probes(
        &mut out,
        program,
        extern_component_declarations,
        &extern_component_ids,
    );
    generate_editor_binding_mapper(&mut out, program, &extern_component_ids);
    writeln!(
        out,
        "}}\n{lint_macro}! {{\n#[allow(unused_parens)]\nimpl {app_name} {{"
    )
    .unwrap();
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
    let presets = if program.preset_names().is_empty() {
        String::new()
    } else {
        format!(
            ".presets([{}])",
            program
                .preset_names()
                .iter()
                .enumerate()
                .map(|(index, preset)| format!(
                    "::iced::Preset::new({}, Self::__preset_{index})",
                    rust_string(preset)
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

    // rustc re-checks a macro expansion as a unit, so every item the generator
    // writes into one `__ice_generated_items_*!` invocation shares one fate: a
    // one-character edit anywhere in the app root re-type-checks all of them.
    // Measured on showcase, splitting one phase out took `type_check_crate`
    // from 0.69s to 0.05s and the rebuild from 3.41s to 2.60s. So each phase
    // closes the invocation and opens the next; `impl` blocks repeat freely,
    // and the boundary always lands between whole items.
    let phase = format!("}}\n}}\n{lint_macro}! {{\n#[allow(unused_parens)]\nimpl {app_name} {{");
    writeln!(out, "{phase}").unwrap();
    generate_theme(&mut out, program)?;
    writeln!(out, "{phase}").unwrap();
    generate_boot(&mut out, program, &message)?;
    writeln!(out, "{phase}").unwrap();
    generate_presets(&mut out, program, &message)?;
    writeln!(out, "{phase}").unwrap();
    // `__update` and `__view` answer to different sources — handlers write
    // one, the view tree the other — and are the two items large enough for
    // that to matter. Each gets its own group, hence its own file, so an edit
    // to one leaves the other's type check reusable.
    let mut update = String::new();
    generate_update(&mut update, program, &message)?;
    generate_subscription(&mut out, program, &message)?;
    writeln!(out, "{phase}").unwrap();
    let outline_guard = outline::enable_for_view();
    let mut view = String::new();
    generate_view(&mut view, program, &message)?;
    let mut outlined = outline::drain_outlined_methods();
    outlined.push((APP_UPDATE_GROUP.to_owned(), update));
    outlined.push((APP_VIEW_GROUP.to_owned(), view));
    drop(outline_guard);
    let outline_guard = outline::enable_for_test_mounts();
    generate_test_mounts(&mut out, program, &message, source_path)?;
    outlined.extend(outline::drain_outlined_methods());
    drop(outline_guard);
    writeln!(out, "}}").unwrap();
    generate_outlined_groups(&mut out, app_name, outlined);
    generate_tests(&mut out, program, &message, source_path)?;
    writeln!(out, "}}").unwrap();
    Ok(resolve_source_markers(out, program, source_path))
}

/// Emits the outlined methods grouped into one fenced `mod` per source
/// fragment. Each region is self-contained — `use super::*` re-imports the
/// include-site scope and the inherent impl re-targets the app type — so the
/// region compiles identically in place or split into its own file by
/// ui-lang-build (which swaps the region for an `include!`). Splitting is
/// what keeps one fragment's edit from shifting every other fragment's spans,
/// which rustc hashes into incremental fingerprints.
fn generate_outlined_groups(out: &mut String, app_name: &str, outlined: Vec<(String, String)>) {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (slug, item) in outlined {
        groups.entry(slug).or_default().push(item);
    }
    for (slug, items) in groups {
        writeln!(out, "{GROUP_MARKER_BEGIN}{slug}").unwrap();
        writeln!(
            out,
            "#[allow(warnings, clippy::all)]\nmod __ice_group_{slug} {{\nuse super::*;\nimpl super::{app_name} {{"
        )
        .unwrap();
        for item in items {
            writeln!(out, "{item}").unwrap();
        }
        writeln!(out, "}}\n}}\n{GROUP_MARKER_END}").unwrap();
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
mod type_code;
mod view;

use type_code::rust_type_code;

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
