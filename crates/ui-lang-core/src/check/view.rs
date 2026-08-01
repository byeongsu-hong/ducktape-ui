use super::*;
use crate::check::expr::analyze_expr_types;
use crate::check::facts::{CheckedAnalyses, CheckedExprOwner, CheckedViewExprRole};
use crate::hir::{ComponentCallId, ComponentParamId, DeclarationIndex, ViewId, view_children};
use std::cell::RefCell;

#[derive(Debug)]
struct ActiveViewAnalyses {
    views: HashMap<(usize, usize), ViewId>,
    calls: HashMap<(usize, usize), ComponentCallId>,
    params: HashMap<(String, String), ComponentParamId>,
    analyses: CheckedAnalyses,
    scope_env_overlays: usize,
    scope_env_full_clones: usize,
}

thread_local! {
    static ACTIVE_VIEW_ANALYSES: RefCell<Option<ActiveViewAnalyses>> = const { RefCell::new(None) };
}

pub(super) struct ViewAnalysisGuard {
    active: bool,
}

impl ViewAnalysisGuard {
    pub(super) fn start(document: &Document, declarations: &DeclarationIndex) -> Self {
        let mut views = HashMap::new();
        let mut calls = HashMap::new();
        fn collect(
            node: &ViewNode,
            declarations: &DeclarationIndex,
            views: &mut HashMap<(usize, usize), ViewId>,
            calls: &mut HashMap<(usize, usize), ComponentCallId>,
        ) {
            if let Some(view) = declarations.view_id(node.span()) {
                let site = (node.span().line, node.span().column);
                views.insert(site, view);
                if let Some(call) = declarations.component_call_id(view) {
                    calls.insert(site, call);
                }
            }
            for child in view_children(node) {
                collect(child, declarations, views, calls);
            }
        }
        for component in &document.components {
            collect(&component.root, declarations, &mut views, &mut calls);
        }
        collect(&document.view, declarations, &mut views, &mut calls);
        for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
            collect(mount, declarations, &mut views, &mut calls);
        }
        let mut params = HashMap::new();
        for (component_index, component) in document.components.iter().enumerate() {
            let component_id = declarations.component(component_index).id;
            for (param_index, param) in component.params.iter().enumerate() {
                params.insert(
                    (component.name.clone(), param.name.clone()),
                    declarations.component_param(component_id, param_index).id,
                );
            }
        }
        ACTIVE_VIEW_ANALYSES.with(|active| {
            let previous = active.replace(Some(ActiveViewAnalyses {
                views,
                calls,
                params,
                analyses: CheckedAnalyses::default(),
                scope_env_overlays: 0,
                scope_env_full_clones: 0,
            }));
            assert!(
                previous.is_none(),
                "view analysis session must not be nested"
            );
        });
        Self { active: true }
    }

    pub(super) fn finish(mut self) -> CheckedAnalyses {
        self.active = false;
        ACTIVE_VIEW_ANALYSES.with(|active| {
            let mut active = active
                .borrow_mut()
                .take()
                .expect("view analysis session must be active");
            active.analyses.view_scope_env_overlays = active.scope_env_overlays;
            active.analyses.view_scope_env_full_clones = active.scope_env_full_clones;
            active.analyses
        })
    }
}

pub(super) fn scoped_view_env(env: &dyn ExprTypeEnv) -> ScopedTypeEnv<'_> {
    ACTIVE_VIEW_ANALYSES.with(|active| {
        if let Some(active) = active.borrow_mut().as_mut() {
            active.scope_env_overlays += 1;
        }
    });
    ScopedTypeEnv::new(env)
}

pub(super) fn record_view_env_full_clone() {
    ACTIVE_VIEW_ANALYSES.with(|active| {
        if let Some(active) = active.borrow_mut().as_mut() {
            active.scope_env_full_clones += 1;
        }
    });
}

pub(super) fn retain_canvas_analyses(
    span: &Span,
    analyses: super::expr::HandlerAnalyses,
) -> Result<(), Error> {
    ACTIVE_VIEW_ANALYSES.with(|active| {
        let mut active = active.borrow_mut();
        let active = active.as_mut().ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "canvas analysis session has no active view analysis",
            )
        })?;
        let canvas = active
            .views
            .get(&(span.line, span.column))
            .copied()
            .ok_or_else(|| Error::new("E196", span, "canvas has no shared view ID"))?;
        active.analyses.retain_canvas(canvas, analyses)
    })
}

pub(super) fn retain_media_analyses(
    span: &Span,
    analyses: super::expr::HandlerAnalyses,
) -> Result<(), Error> {
    ACTIVE_VIEW_ANALYSES.with(|active| {
        let mut active = active.borrow_mut();
        let active = active.as_mut().ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "media analysis session has no active view analysis",
            )
        })?;
        let media = active
            .views
            .get(&(span.line, span.column))
            .copied()
            .ok_or_else(|| Error::new("E196", span, "media has no shared view ID"))?;
        active.analyses.retain_media(media, analyses)
    })
}

pub(super) fn retain_tooltip_analyses(
    span: &Span,
    analyses: super::expr::HandlerAnalyses,
) -> Result<(), Error> {
    ACTIVE_VIEW_ANALYSES.with(|active| {
        let mut active = active.borrow_mut();
        let active = active.as_mut().ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "tooltip analysis session has no active view analysis",
            )
        })?;
        let tooltip = active
            .views
            .get(&(span.line, span.column))
            .copied()
            .ok_or_else(|| Error::new("E196", span, "tooltip has no shared view ID"))?;
        active.analyses.retain_tooltip(tooltip, analyses)
    })
}

pub(super) fn retain_interaction_analyses(
    span: &Span,
    analyses: super::expr::HandlerAnalyses,
) -> Result<(), Error> {
    ACTIVE_VIEW_ANALYSES.with(|active| {
        let mut active = active.borrow_mut();
        let active = active.as_mut().ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "interaction analysis session has no active view analysis",
            )
        })?;
        let widget = active
            .views
            .get(&(span.line, span.column))
            .copied()
            .ok_or_else(|| Error::new("E196", span, "interaction widget has no shared view ID"))?;
        active.analyses.retain_interaction(widget, analyses)
    })
}

pub(super) fn retain_pane_analyses(
    span: &Span,
    analyses: super::expr::HandlerAnalyses,
) -> Result<(), Error> {
    ACTIVE_VIEW_ANALYSES.with(|active| {
        let mut active = active.borrow_mut();
        let active = active.as_mut().ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "pane analysis session has no active view analysis",
            )
        })?;
        let pane = active
            .views
            .get(&(span.line, span.column))
            .copied()
            .ok_or_else(|| Error::new("E196", span, "pane grid has no shared view ID"))?;
        active.analyses.retain_interaction(pane, analyses)
    })
}

pub(super) fn retain_container_analyses(
    span: &Span,
    analyses: super::expr::HandlerAnalyses,
) -> Result<(), Error> {
    ACTIVE_VIEW_ANALYSES.with(|active| {
        let mut active = active.borrow_mut();
        let active = active.as_mut().ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "container analysis session has no active view analysis",
            )
        })?;
        let container = active
            .views
            .get(&(span.line, span.column))
            .copied()
            .ok_or_else(|| Error::new("E196", span, "container has no shared view ID"))?;
        active.analyses.retain_interaction(container, analyses)
    })
}

pub(super) fn retain_layout_analyses(
    span: &Span,
    analyses: super::expr::HandlerAnalyses,
) -> Result<(), Error> {
    ACTIVE_VIEW_ANALYSES.with(|active| {
        let mut active = active.borrow_mut();
        let active = active.as_mut().ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "layout analysis session has no active view analysis",
            )
        })?;
        let layout = active
            .views
            .get(&(span.line, span.column))
            .copied()
            .ok_or_else(|| Error::new("E196", span, "layout has no shared view ID"))?;
        active.analyses.retain_interaction(layout, analyses)
    })
}

pub(super) fn retain_float_analyses(
    span: &Span,
    analyses: super::expr::HandlerAnalyses,
) -> Result<(), Error> {
    ACTIVE_VIEW_ANALYSES.with(|active| {
        let mut active = active.borrow_mut();
        let active = active.as_mut().ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "float analysis session has no active view analysis",
            )
        })?;
        let float = active
            .views
            .get(&(span.line, span.column))
            .copied()
            .ok_or_else(|| Error::new("E196", span, "float has no shared view ID"))?;
        active.analyses.retain_float(float, analyses)
    })
}

pub(super) fn retain_pin_analyses(
    span: &Span,
    analyses: super::expr::HandlerAnalyses,
) -> Result<(), Error> {
    ACTIVE_VIEW_ANALYSES.with(|active| {
        let mut active = active.borrow_mut();
        let active = active.as_mut().ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "pin analysis session has no active view analysis",
            )
        })?;
        let pin = active
            .views
            .get(&(span.line, span.column))
            .copied()
            .ok_or_else(|| Error::new("E196", span, "pin has no shared view ID"))?;
        active.analyses.retain_pin(pin, analyses)
    })
}

impl Drop for ViewAnalysisGuard {
    fn drop(&mut self) {
        if self.active {
            ACTIVE_VIEW_ANALYSES.with(|active| {
                active.borrow_mut().take();
            });
        }
    }
}

pub(super) fn retained_view_expr_type(
    expr: &Expr,
    env: &dyn ExprTypeEnv,
    document: &Document,
    span: &Span,
    role: CheckedViewExprRole,
) -> Result<Type, Error> {
    retained_view_expr_type_at(expr, env, document, span, span, role)
}

pub(super) fn retained_view_expr_type_at(
    expr: &Expr,
    env: &dyn ExprTypeEnv,
    document: &Document,
    owner_span: &Span,
    expression_span: &Span,
    role: CheckedViewExprRole,
) -> Result<Type, Error> {
    let analysis = analyze_expr_types(expr, env, document, expression_span)?;
    let ty = analysis.type_of(expr).cloned().ok_or_else(|| {
        Error::new(
            "E196",
            expression_span,
            "missing retained view expression type",
        )
    })?;
    ACTIVE_VIEW_ANALYSES.with(|active| {
        let mut active = active.borrow_mut();
        let active = active.as_mut().ok_or_else(|| {
            Error::new(
                "E196",
                expression_span,
                "view analysis session is not active",
            )
        })?;
        let view = active
            .views
            .get(&(owner_span.line, owner_span.column))
            .copied()
            .ok_or_else(|| {
                Error::new(
                    "E196",
                    expression_span,
                    "view expression has no shared view ID",
                )
            })?;
        active
            .analyses
            .insert_expression(CheckedExprOwner::View { view, role }, analysis)
    })?;
    Ok(ty)
}

pub(super) fn retained_component_argument_type(
    expr: &Expr,
    env: &dyn ExprTypeEnv,
    document: &Document,
    span: &Span,
    component: &str,
    param: &str,
) -> Result<Type, Error> {
    let analysis = analyze_expr_types(expr, env, document, span)?;
    let ty = analysis
        .type_of(expr)
        .cloned()
        .ok_or_else(|| Error::new("E196", span, "missing retained component argument type"))?;
    ACTIVE_VIEW_ANALYSES.with(|active| {
        let mut active = active.borrow_mut();
        let active = active.as_mut().ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "component argument analysis session is not active",
            )
        })?;
        let site = (span.line, span.column);
        let call =
            active.calls.get(&site).copied().ok_or_else(|| {
                Error::new("E196", span, "component argument has no shared call ID")
            })?;
        let param = active
            .params
            .get(&(component.to_owned(), param.to_owned()))
            .copied()
            .ok_or_else(|| {
                Error::new(
                    "E196",
                    span,
                    "component argument has no shared parameter ID",
                )
            })?;
        active.analyses.insert_expression(
            CheckedExprOwner::ComponentArgument { call, param },
            analysis,
        )
    })?;
    Ok(ty)
}

pub(in crate::check) fn infer_view(
    node: &ViewNode,
    env: &dyn ExprTypeEnv,
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

pub(crate) fn lazy_hashable(ty: &Type) -> bool {
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
        | Type::Palette(_)
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
