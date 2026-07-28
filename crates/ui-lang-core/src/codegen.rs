use crate::ast::*;
use crate::check::{controlled_editor_bindings, controlled_state_bindings, expr_type};
use crate::{CheckedDocument, Error, canonical_snake};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

const RECONCILIATION_SCOPE_BINDING: &str = "\0__ice_reconciliation_scope";
const SOURCE_MARKER: &str = "// __ICE_SOURCE ";
const SOURCE_MARKER_END: &str = "// __ICE_SOURCE_END";

fn source_marker(span: &Span) -> String {
    format!("{SOURCE_MARKER}{} {}", span.line, span.column)
}

fn source_mapped_expression(code: String, span: &Span) -> String {
    format!(
        "{{\n{}\n{code}\n{SOURCE_MARKER_END}\n}}",
        source_marker(span)
    )
}

fn encode_source_path(path: &str) -> String {
    path.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn resolve_source_markers(
    generated: String,
    document: &CheckedDocument,
    source_path: &str,
) -> String {
    let mut output = String::with_capacity(generated.len());
    for line in generated.lines() {
        let resolved = line
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
        output.push_str(&resolved);
        output.push('\n');
    }
    output
}

fn reconciliation_scope<'a>(public_scope: &'a str, env: &'a HashMap<String, Binding>) -> &'a str {
    env.get(RECONCILIATION_SCOPE_BINDING)
        .map_or(public_scope, |binding| binding.code.as_str())
}

fn set_reconciliation_scope(env: &mut HashMap<String, Binding>, code: String) {
    env.insert(
        RECONCILIATION_SCOPE_BINDING.into(),
        Binding {
            code,
            ty: Type::Str,
            local: true,
            state: None,
        },
    );
}

fn match_pattern_code(pattern: &MatchPattern) -> String {
    match pattern {
        MatchPattern::Some(binding) => {
            format!("::std::option::Option::Some({binding})")
        }
        MatchPattern::None => "::std::option::Option::None".into(),
        MatchPattern::Ok(binding) => format!("::std::result::Result::Ok({binding})"),
        MatchPattern::Err(binding) => format!("::std::result::Result::Err({binding})"),
        MatchPattern::Enum {
            enum_name,
            variant,
            binding,
        } => {
            let enum_name = generated_named_rust(enum_name);
            binding.as_ref().map_or_else(
                || format!("{enum_name}::{}", pascal(variant)),
                |binding| format!("{enum_name}::{}({binding})", pascal(variant)),
            )
        }
        MatchPattern::Wildcard => "_".into(),
    }
}

fn match_pattern_binding(
    pattern: &MatchPattern,
    value_ty: &Type,
    document: &Document,
) -> Option<(String, Type)> {
    match (pattern, value_ty) {
        (MatchPattern::Some(binding), Type::Option(inner)) => {
            Some((binding.clone(), inner.as_ref().clone()))
        }
        (MatchPattern::Ok(binding), Type::Result(output, _)) => {
            Some((binding.clone(), output.as_ref().clone()))
        }
        (MatchPattern::Err(binding), Type::Result(_, error)) => {
            Some((binding.clone(), error.as_ref().clone()))
        }
        (
            MatchPattern::Enum {
                enum_name,
                variant,
                binding: Some(binding),
            },
            Type::Named(name),
        ) if enum_name == name => document
            .enums
            .iter()
            .find(|item| item.name == *enum_name)
            .and_then(|item| item.variants.iter().find(|item| item.name == *variant))
            .and_then(|variant| variant.payload.clone())
            .map(|payload| (binding.clone(), payload)),
        _ => None,
    }
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

pub(in crate::codegen) fn component_generation_lines(
    component: &Component,
) -> impl Iterator<Item = usize> + '_ {
    component
        .handlers
        .iter()
        .flat_map(|handler| &handler.statements)
        .filter_map(|statement| match statement {
            Statement::Run { mode, span, .. } if *mode != FutureMode::Every => Some(span.line),
            _ => None,
        })
}

pub(in crate::codegen) fn component_replace_lines(
    component: &Component,
) -> impl Iterator<Item = usize> + '_ {
    component
        .handlers
        .iter()
        .flat_map(|handler| &handler.statements)
        .filter_map(|statement| match statement {
            Statement::Run {
                mode: FutureMode::Replace,
                span,
                ..
            } => Some(span.line),
            _ => None,
        })
}

pub(in crate::codegen) fn handler_future(handler: &Handler) -> Option<(FutureMode, usize)> {
    handler
        .statements
        .iter()
        .find_map(|statement| match statement {
            Statement::Run {
                kind: EffectKind::Future,
                mode,
                span,
                ..
            } => Some((*mode, span.line)),
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

fn generate_derived(out: &mut String, document: &Document) -> Result<(), Error> {
    let env = state_env(document, "self");
    for derived in &document.derived {
        let value = expr_code(&derived.value, &env, document, ValueMode::Owned)?;
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

pub fn generate(document: &CheckedDocument, source_path: &str) -> Result<String, Error> {
    let message = format!("__{}Message", document.app);
    let mut out = String::new();
    writeln!(
        out,
        "const _: &str = include_str!({});",
        rust_string(source_path)
    )
    .unwrap();
    writeln!(
        out,
        "type __IceRenderer = {}; type __IceElement<'a, Message, Theme = ::iced::Theme> = ::iced::Element<'a, Message, Theme, __IceRenderer>;",
        document
            .settings
            .renderer
            .as_deref()
            .unwrap_or("::iced::Renderer")
    )
    .unwrap();
    generate_keyboard_types(&mut out, document);
    generate_system_types(&mut out, document);
    generate_widget_selector_types(&mut out, document);
    generate_canvas_types(&mut out, document);
    generate_pane_types(&mut out, document)?;
    let token_count = document
        .theme_contract
        .as_ref()
        .map_or(0, |contract| contract.tokens.len());
    if let Some(contract) = &document.theme_contract {
        writeln!(
            out,
            "#[allow(dead_code)]\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub(crate) enum {} {{",
            generated_named_rust(&contract.name)
        )
        .unwrap();
        for palette in &document.palettes {
            writeln!(out, "{},", pascal(&palette.name)).unwrap();
        }
        writeln!(out, "}}").unwrap();
    }
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
            "#[derive({derives})]\npub(crate) enum {} {{",
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

    for component in document
        .components
        .iter()
        .filter(|component| !component.states.is_empty() || !component.handlers.is_empty())
    {
        let ty = component_state_type(&component.name);
        writeln!(out, "struct {ty} {{").unwrap();
        for state in &component.states {
            writeln!(out, "{}", source_marker(&state.span)).unwrap();
            writeln!(out, "{}: {},", state.name, state.ty.rust(&document.structs)).unwrap();
            writeln!(out, "{SOURCE_MARKER_END}").unwrap();
        }
        for line in component_generation_lines(component) {
            writeln!(out, "{}: u64,", component_latest_field(line)).unwrap();
        }
        for line in component_replace_lines(component) {
            writeln!(
                out,
                "{}: ::std::option::Option<::iced::task::Handle>,",
                component_replace_field(line)
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
            writeln!(out, "{}: {},", state.name, initial_code(state, document)).unwrap();
            writeln!(out, "{SOURCE_MARKER_END}").unwrap();
        }
        for line in component_generation_lines(component) {
            writeln!(out, "{}: 0,", component_latest_field(line)).unwrap();
        }
        for line in component_replace_lines(component) {
            writeln!(
                out,
                "{}: ::std::option::Option::None,",
                component_replace_field(line)
            )
            .unwrap();
        }
        writeln!(out, "}} }}\n}}").unwrap();
    }

    writeln!(out, "pub struct {} {{", document.app).unwrap();
    writeln!(
        out,
        "pub(crate) __ice_accessibility: ::ui_lang_runtime::Bridge<{message}>,"
    )
    .unwrap();
    if !document.daemon {
        writeln!(
            out,
            "#[cfg(all(target_os = \"windows\", not(test)))]\npub(crate) __ice_accessibility_initial: ::std::option::Option<usize>,\n#[cfg(all(target_os = \"windows\", not(test)))]\npub(crate) __ice_accessibility_pending: ::std::vec::Vec<{message}>,"
        )
        .unwrap();
    }
    for (node, test_only) in document_pane_grids(document) {
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
    for component in document
        .components
        .iter()
        .filter(|component| !component.states.is_empty() || !component.handlers.is_empty())
    {
        let field = component_state_field(&component.name);
        let ty = component_state_type(&component.name);
        match component.lifetime {
            ComponentLifetime::Retained => writeln!(
                out,
                "pub(crate) {field}: ::std::collections::HashMap<::std::string::String, {ty}>,"
            )
            .unwrap(),
            ComponentLifetime::Mounted => writeln!(
                out,
                "pub(crate) {field}: ::ui_lang_runtime::MountedComponentState<{ty}>,"
            )
            .unwrap(),
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

    writeln!(out, "#[derive(Clone)]\nenum {message} {{").unwrap();
    writeln!(
        out,
        "__AccessibilitySnapshot(::std::boxed::Box<::ui_lang_runtime::Snapshot<{message}>>),\n__AccessibilityAction(::ui_lang_runtime::ActionRequest),\n__AccessibilityWindow(::iced::window::Id, ::iced::window::Event),\n#[cfg(all(target_os = \"windows\", not(test)))]\n__AccessibilityNativeWindow(::ui_lang_runtime::NativeWindow),\n__AccessibilityFocusNext,\n__AccessibilityFocusPrevious,"
    )
    .unwrap();
    for handler in &document.handlers {
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
    for component in &document.components {
        for line in component_generation_lines(component) {
            writeln!(
                out,
                "{}(::std::string::String, u64, ::std::boxed::Box<{message}>),",
                component_latest_variant(&component.name, line)
            )
            .unwrap();
        }
        for handler in &component.handlers {
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
    if has_animations(document) {
        writeln!(out, "__AnimationFrame,").unwrap();
    }
    for (node, test_only) in document_pane_grids(document) {
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
    generate_derived(&mut out, document)?;
    generate_named_windows(&mut out, document, source_path);
    let subscription = ".subscription(Self::__subscription)";
    let default_font = document
        .fonts
        .iter()
        .find(|font| font.default)
        .map_or_else(String::new, |font| {
            format!(".default_font({})", font_decl_code(font))
        });
    let title = document
        .settings
        .title
        .as_ref()
        .map_or("", |_| ".title(Self::__title)");
    let settings = app_settings_code(&document.settings);
    let fonts = font_assets_code(&document.settings, source_path);
    let window = if document.daemon {
        String::new()
    } else {
        window_settings_code(document.settings.window.as_ref(), source_path)
    };
    let executor = document
        .settings
        .executor
        .as_ref()
        .map_or_else(String::new, |executor| format!(".executor::<{executor}>()"));
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
    let scale_factor = document
        .settings
        .scale_factor
        .as_ref()
        .map_or("", |_| ".scale_factor(Self::__scale_factor)");
    let style = if document.settings.background.is_some() || document.settings.text_color.is_some()
    {
        ".style(Self::__style)"
    } else {
        ""
    };
    let root = if document.daemon {
        "::iced::daemon(Self::__boot, Self::__update, Self::__view)"
    } else {
        "::iced::application(Self::__boot, Self::__update, Self::__view)"
    };
    let program = if document.daemon {
        "::iced::Daemon"
    } else {
        "::iced::Application"
    };
    writeln!(
        out,
        "fn __program() -> {program}<impl ::iced::Program<State = Self, Message = {message}, Theme = ::iced::Theme>> {{"
    )
    .unwrap();
    writeln!(out, "{root}{title}{subscription}.theme(Self::__theme){style}{settings}{default_font}{fonts}{window}{scale_factor}{executor}{presets}").unwrap();
    writeln!(
        out,
        "}}\npub fn run() -> ::iced::Result {{\nSelf::__program().run()\n}}"
    )
    .unwrap();

    generate_theme(&mut out, document)?;
    generate_boot(&mut out, document, &message)?;
    generate_presets(&mut out, document, &message)?;
    generate_update(&mut out, document, &message)?;
    generate_subscription(&mut out, document, &message)?;
    generate_view(&mut out, document, &message)?;
    generate_test_mounts(&mut out, document, &message, source_path)?;
    writeln!(out, "}}").unwrap();
    generate_tests(&mut out, document, &message, source_path)?;
    Ok(resolve_source_markers(out, document, source_path))
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
