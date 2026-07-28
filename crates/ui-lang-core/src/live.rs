use crate::{
    BinaryOp, CheckedDocument, ComponentLifetime, Expr, ExternFn, ExternStruct, Layout, Route,
    RouteArg, Span, State, Type, UiEnum, UnaryOp, ViewNode,
};
use sha2::{Digest, Sha256};
use std::fmt;
pub use ui_lang_live_protocol::{
    LIVE_PROTOCOL_VERSION, LiveBinaryOp, LiveEvent, LiveExpression, LiveExternFunctionAbi,
    LiveExternStructAbi, LiveNamedTypeAbi, LivePlan, LiveProgramAbi, LiveProgramContract,
    LiveProgramMode, LiveReloadDecision, LiveRestartReason, LiveRoute, LiveStateChange,
    LiveStateId, LiveStateSchema, LiveStateSlot, LiveStateStorage, LiveUnaryOp, LiveValue,
    LiveView, evaluate_live_reload,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveLoweringError {
    pub node: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl fmt::Display for LiveLoweringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: live reload cannot lower {}: {}",
            self.line, self.column, self.node, self.message
        )
    }
}

impl std::error::Error for LiveLoweringError {}

/// Lowers one checked source graph into the renderer-independent live plan.
///
/// The first vertical slice deliberately rejects unsupported nodes and options
/// instead of rendering them with different semantics. Callers can then keep
/// the last-known-good plan or use the restart fallback.
pub fn live_plan(document: &CheckedDocument, revision: u64) -> Result<LivePlan, LiveLoweringError> {
    Ok(LivePlan {
        revision,
        contract: live_program_contract(document),
        view: Some(lower_view(&document.view, document, "root")?),
    })
}

fn lower_view(
    node: &ViewNode,
    document: &CheckedDocument,
    path: &str,
) -> Result<LiveView, LiveLoweringError> {
    match node {
        ViewNode::Layout {
            kind,
            options,
            id,
            styles,
            children,
            span,
        } => {
            if id.is_some() || !styles.is_empty() || !is_default(options.as_ref()) {
                return Err(lowering_error(
                    node,
                    span,
                    "layout IDs, styles, and options are not live yet",
                ));
            }
            let children = children
                .iter()
                .enumerate()
                .map(|(index, child)| lower_view(child, document, &format!("{path}/{index}")))
                .collect::<Result<Vec<_>, _>>()?;
            match kind {
                Layout::Column => Ok(LiveView::Column {
                    key: live_key(document, path, "column"),
                    children,
                }),
                Layout::Row => Ok(LiveView::Row {
                    key: live_key(document, path, "row"),
                    children,
                }),
                _ => Err(lowering_error(
                    node,
                    span,
                    "only row and column layouts are live in this revision",
                )),
            }
        }
        ViewNode::Text {
            value,
            id,
            options,
            styles,
            span,
        } => {
            if id.is_some() || !styles.is_empty() || !is_default(options) {
                return Err(lowering_error(
                    node,
                    span,
                    "text IDs, styles, and options are not live yet",
                ));
            }
            Ok(LiveView::Text {
                key: live_key(document, path, "text"),
                value: lower_expression(value, node, span, document)?,
            })
        }
        ViewNode::Button {
            label,
            content,
            id,
            disabled,
            options,
            styles,
            route,
            span,
        } => {
            if id.is_some() || !styles.is_empty() || !is_default(options) {
                return Err(lowering_error(
                    node,
                    span,
                    "button IDs, styles, and options are not live yet",
                ));
            }
            let (Some(label), None) = (label, content) else {
                return Err(lowering_error(
                    node,
                    span,
                    "only label buttons are live in this revision",
                ));
            };
            Ok(LiveView::Button {
                key: live_key(document, path, "button"),
                label: label.clone(),
                disabled: disabled
                    .as_ref()
                    .map(|expression| lower_expression(expression, node, span, document))
                    .transpose()?,
                route: lower_route(route, node, span, document)?,
            })
        }
        ViewNode::If {
            condition,
            children,
            span,
        } => {
            if path == "root" {
                return Err(lowering_error(
                    node,
                    span,
                    "if must be a child of a live row or column",
                ));
            }
            Ok(LiveView::If {
                condition: lower_expression(condition, node, span, document)?,
                children: children
                    .iter()
                    .enumerate()
                    .map(|(index, child)| lower_view(child, document, &format!("{path}/{index}")))
                    .collect::<Result<Vec<_>, _>>()?,
            })
        }
        _ => Err(lowering_error(
            node,
            node.span(),
            "this node family is not live yet",
        )),
    }
}

fn live_key(document: &CheckedDocument, path: &str, kind: &str) -> String {
    format!("{}/live/{path}/{kind}", document.app)
}

fn lower_route(
    route: &Route,
    node: &ViewNode,
    span: &Span,
    document: &CheckedDocument,
) -> Result<LiveRoute, LiveLoweringError> {
    let Some(handler) = document
        .handlers
        .iter()
        .find(|handler| handler.name == route.handler && handler.name != "mount")
    else {
        return Err(lowering_error(
            node,
            span,
            "only top-level app handlers can receive live events",
        ));
    };
    if handler
        .params
        .iter()
        .any(|param| !live_primitive(&param.ty))
    {
        return Err(lowering_error(
            node,
            span,
            "the target handler has a parameter type that is not live yet",
        ));
    }
    let args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Expr(expression) => lower_expression(expression, node, span, document),
            RouteArg::Payload => Err(lowering_error(
                node,
                span,
                "this live event does not provide a payload",
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LiveRoute {
        handler: route.handler.clone(),
        args,
    })
}

fn live_primitive(ty: &Type) -> bool {
    matches!(ty, Type::Bool | Type::I64 | Type::F64 | Type::Str)
}

fn live_binding_type<'a>(document: &'a CheckedDocument, name: &str) -> Option<&'a Type> {
    document
        .states
        .iter()
        .find(|state| state.name == name)
        .map(|state| &state.ty)
        .or_else(|| {
            document
                .derived
                .iter()
                .find(|derived| derived.name == name)
                .map(|derived| &derived.ty)
        })
}

fn lower_expression(
    expression: &Expr,
    node: &ViewNode,
    span: &Span,
    document: &CheckedDocument,
) -> Result<LiveExpression, LiveLoweringError> {
    match expression {
        Expr::Bool(value) => Ok(LiveExpression::Bool(*value)),
        Expr::I64(value) => Ok(LiveExpression::I64(*value)),
        Expr::F64(value) => Ok(LiveExpression::F64(*value)),
        Expr::Str(value) => Ok(LiveExpression::String(value.clone())),
        Expr::Path(path) => match path.as_slice() {
            [name] if live_binding_type(document, name).is_some_and(live_primitive) => {
                Ok(LiveExpression::Path(name.clone()))
            }
            _ => Err(lowering_error(
                node,
                span,
                "only primitive app-state and derived-value paths are live yet",
            )),
        },
        Expr::Unary { op, value } => Ok(LiveExpression::Unary {
            op: match op {
                UnaryOp::Not => LiveUnaryOp::Not,
                UnaryOp::Neg => LiveUnaryOp::Neg,
            },
            value: Box::new(lower_expression(value, node, span, document)?),
        }),
        Expr::Binary { left, op, right } => Ok(LiveExpression::Binary {
            left: Box::new(lower_expression(left, node, span, document)?),
            op: match op {
                BinaryOp::Add => LiveBinaryOp::Add,
                BinaryOp::Sub => LiveBinaryOp::Sub,
                BinaryOp::Mul => LiveBinaryOp::Mul,
                BinaryOp::Div => LiveBinaryOp::Div,
                BinaryOp::Rem => LiveBinaryOp::Rem,
                BinaryOp::Eq => LiveBinaryOp::Eq,
                BinaryOp::NotEq => LiveBinaryOp::NotEq,
                BinaryOp::Lt => LiveBinaryOp::Lt,
                BinaryOp::LtEq => LiveBinaryOp::LtEq,
                BinaryOp::Gt => LiveBinaryOp::Gt,
                BinaryOp::GtEq => LiveBinaryOp::GtEq,
                BinaryOp::And => LiveBinaryOp::And,
                BinaryOp::Or => LiveBinaryOp::Or,
            },
            right: Box::new(lower_expression(right, node, span, document)?),
        }),
        _ => Err(lowering_error(
            node,
            span,
            "this expression is not live yet",
        )),
    }
}

fn is_default<T: Default + fmt::Debug>(value: &T) -> bool {
    format!("{value:?}") == format!("{:?}", T::default())
}

fn lowering_error(node: &ViewNode, span: &Span, message: &str) -> LiveLoweringError {
    LiveLoweringError {
        node: view_node_name(node).into(),
        line: span.line,
        column: span.column,
        message: message.into(),
    }
}

fn view_node_name(node: &ViewNode) -> &'static str {
    match node {
        ViewNode::Layout { .. } => "layout",
        ViewNode::Container { .. } => "box",
        ViewNode::Overlay { .. } => "overlay",
        ViewNode::PaneGrid { .. } => "pane-grid",
        ViewNode::Text { .. } => "text",
        ViewNode::RichText { .. } => "rich-text",
        ViewNode::Input { .. } => "input",
        ViewNode::Button { .. } => "button",
        ViewNode::Checkbox { .. } => "checkbox",
        ViewNode::Toggler { .. } => "toggler",
        ViewNode::Slider { .. } => "slider",
        ViewNode::Progress { .. } => "progress",
        ViewNode::Radio { .. } => "radio",
        ViewNode::PickList { .. } => "pick-list",
        ViewNode::ComboBox { .. } => "combo-box",
        ViewNode::Rule { .. } => "rule",
        ViewNode::QrCode { .. } => "qr-code",
        ViewNode::Space { .. } => "space",
        ViewNode::If { .. } => "if",
        ViewNode::Match { .. } => "match",
        ViewNode::For { .. } => "for",
        ViewNode::KeyedColumn { .. } => "keyed-column",
        ViewNode::Lazy { .. } => "lazy",
        ViewNode::Markdown { .. } => "markdown",
        ViewNode::TextEditor { .. } => "editor",
        ViewNode::Table { .. } => "table",
        ViewNode::Component { .. } => "component",
        ViewNode::Slot { .. } => "slot",
        ViewNode::ExternComponent { .. } => "extern-component",
        ViewNode::Themer { .. } => "themer",
        ViewNode::Shader { .. } => "shader",
        ViewNode::Media { .. } => "media",
        ViewNode::Tooltip { .. } => "tooltip",
        ViewNode::MouseArea { .. } => "mouse-area",
        ViewNode::ResizeHandle { .. } => "resize-handle",
        ViewNode::Canvas { .. } => "canvas",
        ViewNode::Theme { .. } => "theme",
        ViewNode::Float { .. } => "float",
        ViewNode::Pin { .. } => "pin",
        ViewNode::Sensor { .. } => "sensor",
        ViewNode::Responsive { .. } => "responsive",
    }
}

pub fn live_program_contract(document: &CheckedDocument) -> LiveProgramContract {
    LiveProgramContract {
        protocol_version: LIVE_PROTOCOL_VERSION,
        abi: LiveProgramAbi {
            app: document.app.clone(),
            mode: if document.daemon {
                LiveProgramMode::Daemon
            } else {
                LiveProgramMode::Application
            },
            bootstrap: bootstrap_contract(document),
            aot_semantics_digest: aot_semantics_digest(document),
            named_types: document.enums.iter().map(named_type_abi).collect(),
            palette_types: document
                .theme_contract
                .iter()
                .map(|contract| {
                    let variants = document
                        .palettes
                        .iter()
                        .map(|palette| palette.name.as_str())
                        .collect::<Vec<_>>()
                        .join(",");
                    format!("{}[{variants}]", contract.name)
                })
                .collect(),
            extern_structs: document.structs.iter().map(extern_struct_abi).collect(),
            extern_functions: document.functions.iter().map(extern_function_abi).collect(),
        },
        state: state_schema(document),
    }
}

fn aot_semantics_digest(document: &CheckedDocument) -> String {
    let canonical = format!(
        "settings={:?};presets={:?};recipes={:?};subscriptions={:?};theme_contract={:?};palettes={:?};states={:?};derived={:?};components={:?};handlers={:?}",
        document.settings,
        document.presets,
        document.recipes,
        document.subscriptions,
        document.theme_contract,
        document.palettes,
        document.states,
        document.derived,
        document.components,
        document.handlers,
    );
    format!("{:x}", Sha256::digest(canonical.as_bytes()))
}

fn bootstrap_contract(document: &CheckedDocument) -> String {
    let settings = &document.settings;
    format!(
        "id={:?};executor={:?};renderer={:?};font_assets={:?};fonts={:?};default_text_size={:?};antialiasing={:?};vsync={:?};window={:?};windows={:?}",
        settings.id,
        settings.executor,
        settings.renderer,
        settings
            .fonts
            .iter()
            .map(|font| font.path.as_str())
            .collect::<Vec<_>>(),
        document
            .fonts
            .iter()
            .map(|font| format!(
                "{}:{:?}:{:?}:{:?}:{:?}:{}",
                font.name, font.family, font.weight, font.stretch, font.style, font.default
            ))
            .collect::<Vec<_>>(),
        settings.default_text_size,
        settings.antialiasing,
        settings.vsync,
        window_contract(settings.window.as_ref()),
        settings
            .windows
            .iter()
            .map(|window| (&window.name, window_contract(Some(&window.settings))))
            .collect::<Vec<_>>(),
    )
}

fn window_contract(settings: Option<&crate::WindowSettings>) -> Option<String> {
    settings.map(|settings| {
        format!(
            "size={:?};maximized={:?};fullscreen={:?};position={:?};min_size={:?};max_size={:?};visible={:?};resizable={:?};closeable={:?};minimizable={:?};decorations={:?};transparent={:?};blur={:?};level={:?};icon={:?};exit_on_close={:?};linux={:?};windows={:?};macos={:?};wasm={:?}",
            settings.size,
            settings.maximized,
            settings.fullscreen,
            settings.position,
            settings.min_size,
            settings.max_size,
            settings.visible,
            settings.resizable,
            settings.closeable,
            settings.minimizable,
            settings.decorations,
            settings.transparent,
            settings.blur,
            settings.level,
            settings
                .icon
                .as_ref()
                .map(|icon| (&icon.path, icon.width, icon.height, icon.byte_len)),
            settings.exit_on_close_request,
            settings.linux,
            settings.windows,
            settings.macos,
            settings.wasm,
        )
    })
}

fn named_type_abi(item: &UiEnum) -> LiveNamedTypeAbi {
    LiveNamedTypeAbi {
        name: item.name.clone(),
        variants: item
            .variants
            .iter()
            .map(|variant| {
                (
                    variant.name.clone(),
                    variant.payload.as_ref().map(Type::display),
                )
            })
            .collect(),
    }
}

fn extern_struct_abi(item: &ExternStruct) -> LiveExternStructAbi {
    LiveExternStructAbi {
        name: item.name.clone(),
        rust_path: item.rust_path.clone(),
        fields: item
            .fields
            .iter()
            .map(|(name, ty)| (name.clone(), ty.display()))
            .collect(),
    }
}

fn extern_function_abi(item: &ExternFn) -> LiveExternFunctionAbi {
    LiveExternFunctionAbi {
        kind: format!("{:?}", item.kind),
        name: item.name.clone(),
        rust_path: item.rust_path.clone(),
        params: item
            .params
            .iter()
            .map(|(name, ty)| (name.clone(), ty.display()))
            .collect(),
        borrowed: item.borrowed.clone(),
        progress: item.progress.as_ref().map(Type::display),
        output: item.output.display(),
        error: item.error.as_ref().map(Type::display),
    }
}

fn state_schema(document: &CheckedDocument) -> LiveStateSchema {
    let mut slots = document
        .states
        .iter()
        .map(|state| state_slot("app", state, LiveStateStorage::App))
        .collect::<Vec<_>>();
    for component in &document.components {
        let storage = match component.lifetime {
            ComponentLifetime::Retained => LiveStateStorage::RetainedComponent,
            ComponentLifetime::Mounted => LiveStateStorage::MountedComponent,
        };
        slots.extend(
            component
                .states
                .iter()
                .map(|state| state_slot(&component.name, state, storage)),
        );
    }
    slots.sort_by(|left, right| left.id.cmp(&right.id));
    LiveStateSchema { slots }
}

fn state_slot(owner: &str, state: &State, storage: LiveStateStorage) -> LiveStateSlot {
    LiveStateSlot {
        id: LiveStateId {
            owner: owner.to_owned(),
            name: state.name.clone(),
        },
        ty: state.ty.display(),
        storage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze;

    const BASE: &str = r#"app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  count = 0
on bump
  count = count + 1
view
  text "one"
"#;

    fn contract(source: &str) -> LiveProgramContract {
        live_program_contract(&analyze(source).unwrap())
    }

    #[test]
    fn view_edits_keep_the_live_abi() {
        let previous = contract(BASE);
        let next = contract(&BASE.replace("text \"one\"", "text \"two\""));

        assert_eq!(previous.abi_fingerprint(), next.abi_fingerprint());
        assert_eq!(
            evaluate_live_reload(&previous, &next),
            LiveReloadDecision::Reload {
                state_changes: Vec::new()
            }
        );
    }

    #[test]
    fn generated_behavior_changes_restart_until_their_live_ir_lands() {
        let previous = contract(BASE);
        let next = contract(
            &BASE
                .replace("  count = 0", "  count = \"zero\"\n  ready = true")
                .replace("  count = count + 1", "  count = \"one\""),
        );

        assert_eq!(
            evaluate_live_reload(&previous, &next),
            LiveReloadDecision::RestartRequired {
                reasons: vec![LiveRestartReason::AotSemantics]
            }
        );

        let handler = contract(&BASE.replace("count + 1", "count + 2"));
        assert_eq!(
            evaluate_live_reload(&previous, &handler),
            LiveReloadDecision::RestartRequired {
                reasons: vec![LiveRestartReason::AotSemantics]
            }
        );
    }

    #[test]
    fn compile_time_boundaries_require_restart_with_all_reasons() {
        let previous = contract(BASE);
        let next = contract(
            &BASE
                .replace(
                    "app Demo",
                    "daemon Demo\n  executor iced::executor::Default",
                )
                .replace(
                    "theme contract AppTheme",
                    "enum Mode\n  idle\n  ready\ntheme contract AppTheme",
                ),
        );

        assert_eq!(
            evaluate_live_reload(&previous, &next),
            LiveReloadDecision::RestartRequired {
                reasons: vec![
                    LiveRestartReason::ProgramIdentity,
                    LiveRestartReason::Bootstrap,
                    LiveRestartReason::AotSemantics,
                    LiveRestartReason::NamedTypes,
                ]
            }
        );
    }

    #[test]
    fn fingerprints_are_deterministic_and_distinguish_state_schema() {
        let first = contract(BASE);
        let same = contract(BASE);
        let changed = contract(
            &BASE
                .replace("  count = 0", "  count = \"zero\"")
                .replace("  count = count + 1", "  count = \"one\""),
        );

        assert_eq!(first.abi_fingerprint(), same.abi_fingerprint());
        assert_eq!(first.state_fingerprint(), same.state_fingerprint());
        assert_ne!(first.abi_fingerprint(), changed.abi_fingerprint());
        assert_ne!(first.state_fingerprint(), changed.state_fingerprint());
    }

    #[test]
    fn lowers_the_initial_static_live_view_slice() {
        let source = BASE.replace(
            "view\n  text \"one\"",
            "view\n  col\n    text \"one\"\n    row\n      text 2\n      text 2.5",
        );
        let document = analyze(&source).unwrap();

        assert_eq!(
            live_plan(&document, 7).unwrap().view,
            Some(LiveView::Column {
                key: "Demo/live/root/column".into(),
                children: vec![
                    LiveView::Text {
                        key: "Demo/live/root/0/text".into(),
                        value: LiveExpression::String("one".into()),
                    },
                    LiveView::Row {
                        key: "Demo/live/root/1/row".into(),
                        children: vec![
                            LiveView::Text {
                                key: "Demo/live/root/1/0/text".into(),
                                value: LiveExpression::I64(2),
                            },
                            LiveView::Text {
                                key: "Demo/live/root/1/1/text".into(),
                                value: LiveExpression::F64(2.5),
                            },
                        ],
                    },
                ],
            })
        );
    }

    #[test]
    fn lowers_state_expressions_conditionals_and_aot_handler_routes() {
        let source = BASE.replace(
            "view\n  text \"one\"",
            "view\n  col\n    text count\n    button \"+1\" disabled=(count > 9) -> bump\n    if count != 0\n      text \"changed\"",
        );
        let document = analyze(&source).unwrap();
        let view = live_plan(&document, 2).unwrap().view.unwrap();

        let LiveView::Column { children, .. } = view else {
            panic!("expected a live column");
        };
        assert_eq!(
            children[0],
            LiveView::Text {
                key: "Demo/live/root/0/text".into(),
                value: LiveExpression::Path("count".into()),
            }
        );
        assert!(matches!(
            &children[1],
            LiveView::Button {
                route: LiveRoute { handler, args },
                disabled: Some(LiveExpression::Binary { op: LiveBinaryOp::Gt, .. }),
                ..
            } if handler == "bump" && args.is_empty()
        ));
        assert!(matches!(
            &children[2],
            LiveView::If {
                condition: LiveExpression::Binary {
                    op: LiveBinaryOp::NotEq,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn rejects_live_nodes_before_their_semantics_are_implemented() {
        let source = BASE.replace("text \"one\"", "text \"one\" size=20.0");
        let document = analyze(&source).unwrap();

        let error = live_plan(&document, 1).unwrap_err();
        assert_eq!(error.node, "text");
        assert!(error.message.contains("options are not live"));
    }

    #[test]
    fn rejects_control_flow_outside_a_live_layout() {
        let source = BASE.replace(
            "view\n  text \"one\"",
            "view\n  if count == 0\n    text \"zero\"",
        );
        let document = analyze(&source).unwrap();

        let error = live_plan(&document, 1).unwrap_err();
        assert_eq!(error.node, "if");
        assert!(error.message.contains("child of a live row or column"));
    }
}
