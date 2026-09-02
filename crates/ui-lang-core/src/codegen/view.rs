use super::*;

pub(super) fn identify_rendered(
    rendered: String,
    identity: Option<&ResolvedViewIdentity>,
    message: &str,
) -> Result<String, Error> {
    if identity.is_none() {
        return Ok(rendered);
    }
    Ok(format!(
        "{{ let __identified: __IceElement<'_, {message}> = {rendered}; #[cfg(test)] ::ui_lang_runtime::testing::register_render_source(&{NODE_SCOPE}); ::iced::widget::container(__identified).id(::iced::widget::Id::from({NODE_SCOPE_CLONE})).into() }}"
    ))
}

/// The local `render_node` binds an identified node's scope to, once per
/// pass, before the node's own renderer runs: its id, its accessibility key
/// and every descendant's scope read this binding instead of formatting the
/// ancestor chain again. A nested identified node shadows it with its own.
pub(in crate::codegen) const NODE_SCOPE: &str = "__ice_node_scope";

/// [`NODE_SCOPE`] in the owned form a scope binding takes; `borrowed_scope`
/// strips the clone where the scope is only read.
pub(in crate::codegen) const NODE_SCOPE_CLONE: &str = "__ice_node_scope.clone()";

/// [`NODE_SCOPE`] where the reader takes a `&str`.
pub(in crate::codegen) const NODE_SCOPE_BORROW: &str = "__ice_node_scope.as_str()";

pub(super) fn rendered_child_scope(
    identity: Option<&ResolvedViewIdentity>,
    scope: &str,
) -> Result<String, Error> {
    Ok(match identity {
        Some(_) => NODE_SCOPE_CLONE.to_owned(),
        None => scope.to_owned(),
    })
}

/// The scope an identified node extends its parent's with — the expression
/// `render_node` binds to [`NODE_SCOPE`], and what the template emitter,
/// whose identified nodes are data rather than code, chains for the code
/// slots below them.
pub(in crate::codegen) fn scope_chain_code(
    identity: &ResolvedViewIdentity,
    scope: &str,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    let scope = borrowed_scope(scope);
    if let Some(key) = identity.key {
        let key = resolved_expr_use_code(program, key, env, ValueMode::Borrowed)?;
        Ok(format!(
            "format!(\"{{}}/{}({{}})\", {scope}, {key})",
            identity.name
        ))
    } else {
        Ok(format!("format!(\"{{}}/{}\", {scope})", identity.name))
    }
}

/// A `&str` widget argument. A literal is already one; anything else is
/// borrowed for the call the way a `&str` extern parameter is.
pub(super) fn resolved_str_argument_code(
    program: &LoweredProgram,
    expression: CheckedExprUseId,
    env: &dyn BindingEnvironment,
) -> Result<String, Error> {
    let code = resolved_expr_use_code(program, expression, env, ValueMode::Borrowed)?;
    let expressions = program.expressions();
    let resolved = expressions.expression_use(expression);
    let literal = matches!(resolved.coercion, ResolvedInitializerCoercion::None)
        && matches!(
            expressions.expression(resolved.root).kind,
            ResolvedExpressionKind::Str(_)
        );
    Ok(if literal {
        code
    } else {
        borrowed_argument_code(&Type::Str, &code)
    })
}

/// The accessibility key of a node that only reads it: `StableId::new` hashes
/// it and `logical_id` takes it by reference, so an identified node borrows
/// its scope binding rather than cloning it per frame.
pub(super) fn resolved_accessibility_key_code(
    identity: Option<&ResolvedViewIdentity>,
    kind: &str,
    origin: OriginId,
    scope: &str,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    accessibility_key_code(
        NODE_SCOPE_BORROW,
        identity,
        kind,
        origin,
        scope,
        env,
        program,
    )
}

/// The same key at a node that also moves it into a `widget::Id` — `focus_id`
/// takes the id by value, so this form owns its `String`.
pub(super) fn owned_accessibility_key_code(
    identity: Option<&ResolvedViewIdentity>,
    kind: &str,
    origin: OriginId,
    scope: &str,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    accessibility_key_code(
        NODE_SCOPE_CLONE,
        identity,
        kind,
        origin,
        scope,
        env,
        program,
    )
}

fn accessibility_key_code(
    identified: &str,
    identity: Option<&ResolvedViewIdentity>,
    kind: &str,
    origin: OriginId,
    scope: &str,
    env: &dyn BindingEnvironment,
    program: &LoweredProgram,
) -> Result<String, Error> {
    identity.map_or_else(
        || {
            let scope = borrowed_scope(reconciliation_scope(scope, env));
            Ok(format!(
                "format!(\"{{}}/@{kind}:{}\", {scope})",
                program.origin(origin).line
            ))
        },
        |_| Ok(identified.to_owned()),
    )
}

pub(in crate::codegen) fn component_slot_context(
    slots: &[ResolvedSlot],
    document: &LoweredProgram,
    env: &dyn BindingEnvironment,
    parent: Option<&SlotContext>,
) -> Result<Option<SlotContext>, Error> {
    let mut entries = Vec::new();
    for slot in slots {
        let Some(content) = slot.content else {
            continue;
        };
        if !node_is_omitted(content, document, env, parent)? {
            entries.push(SlotContent {
                slot: slot.slot,
                name: slot.name.clone(),
                view: content,
                env: env.snapshot(),
                recorder: innermost_recorder(),
            });
        }
    }
    Ok((!entries.is_empty()).then(|| SlotContext {
        entries,
        parent: parent.cloned().map(Box::new),
    }))
}

pub(in crate::codegen) fn render_node_if_present(
    node: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    if node_is_omitted(node, document, env, slot)? {
        Ok(None)
    } else {
        render_node(node, document, message, env, scope, slot).map(Some)
    }
}

pub(in crate::codegen) fn node_is_omitted(
    node: ViewId,
    document: &LoweredProgram,
    env: &dyn BindingEnvironment,
    slot: Option<&SlotContext>,
) -> Result<bool, Error> {
    let view = document.resolved_view(node)?;
    let omitted = match &view.kind {
        ResolvedViewKind::Slot { slot: slot_id, .. } => {
            let Some((content, parent)) = slot.and_then(|slot| {
                slot.entries
                    .iter()
                    .find(|entry| entry.slot == *slot_id)
                    .map(|content| (content, slot.parent.as_deref()))
            }) else {
                return Ok(true);
            };
            node_is_omitted(content.view, document, &content.env, parent)?
        }
        ResolvedViewKind::Component { call } => {
            let call = document.component_call_by_id(*call)?;
            let component = document.component(call.component);
            let slots = component_slot_context(&call.slots, document, env, slot)?;
            node_is_omitted(component.root, document, env, slots.as_ref())?
        }
        ResolvedViewKind::Layout { children }
            if matches!(
                document.resolved_layout(node)?.mode,
                ResolvedLayoutMode::Scroll(_)
            ) =>
        {
            node_is_omitted(children[0], document, env, slot)?
        }
        ResolvedViewKind::Container { content }
        | ResolvedViewKind::MouseArea { content }
        | ResolvedViewKind::ResizeHandle { content }
        | ResolvedViewKind::Theme { content }
        | ResolvedViewKind::Float { content }
        | ResolvedViewKind::Pin { content }
        | ResolvedViewKind::Sensor { content }
        | ResolvedViewKind::KeyedColumn { child: content }
        | ResolvedViewKind::Lazy { child: content } => {
            node_is_omitted(*content, document, env, slot)?
        }
        ResolvedViewKind::Button {
            content: Some(content),
        } => node_is_omitted(*content, document, env, slot)?,
        ResolvedViewKind::Tooltip { content, tip } => {
            node_is_omitted(*content, document, env, slot)?
                || node_is_omitted(*tip, document, env, slot)?
        }
        ResolvedViewKind::Overlay { content, layer } => {
            node_is_omitted(*content, document, env, slot)?
                || node_is_omitted(*layer, document, env, slot)?
        }
        ResolvedViewKind::ResponsiveSize { content } => {
            node_is_omitted(*content, document, env, slot)?
        }
        ResolvedViewKind::Table { columns } => {
            let mut omitted = false;
            for column in columns {
                omitted |= node_is_omitted(column.header, document, env, slot)?
                    || node_is_omitted(column.cell, document, env, slot)?;
            }
            omitted
        }
        ResolvedViewKind::PaneGrid { panes, templates } => {
            let mut omitted = false;
            for pane in panes.iter().chain(templates) {
                omitted |= node_is_omitted(pane.content, document, env, slot)?;
                if let Some(title) = &pane.title {
                    omitted |= node_is_omitted(title.content, document, env, slot)?;
                    for child in [title.controls, title.compact_controls]
                        .into_iter()
                        .flatten()
                    {
                        omitted |= node_is_omitted(child, document, env, slot)?;
                    }
                }
            }
            omitted
        }
        _ => false,
    };
    Ok(omitted)
}

pub(in crate::codegen) fn render_node(
    node: ViewId,
    document: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let view = document.resolved_view(node)?;
    let rendered = if let Some(rendered) =
        render_foundation(node, document, message, env, scope, slot)?
    {
        rendered
    } else if let Some(rendered) = render_controls(node, document, message, env, scope, slot)? {
        rendered
    } else if let Some(rendered) = render_content(node, document, message, env, scope, slot)? {
        rendered
    } else if let Some(rendered) = render_media(node, document, message, env, scope, slot)? {
        rendered
    } else if let Some(rendered) = render_structure(node, document, message, env, scope, slot)? {
        rendered
    } else if let Some(rendered) = render_documents(node, document, message, env, scope, slot)? {
        rendered
    } else {
        unreachable!("every view node belongs to a render group")
    };
    let rendered = match &view.identity {
        Some(identity) => format!(
            "{{ let {NODE_SCOPE} = {}; {rendered} }}",
            scope_chain_code(identity, scope, env, document)?
        ),
        None => rendered,
    };
    let track_descendants = matches!(
        view.kind,
        ResolvedViewKind::ExternComponent | ResolvedViewKind::TextEditor
    );
    Ok(source_mapped_expression_origin(
        rendered,
        document,
        view.origin,
        message,
        track_descendants,
    ))
}

pub(in crate::codegen) mod outline;

mod boolean;
mod button;
mod container;
mod content;
mod content_primitives;
mod controls;
mod documents;
mod editor;
mod extern_component;
mod foundation;
mod input;
mod layout;
mod markdown;
mod media;
mod memo;
mod pane;
mod range_controls;
mod selection;
mod structure;
mod table;
mod text;
mod themer_shader;

pub(super) use boolean::*;
pub(super) use button::*;
pub(super) use container::*;
pub(super) use content::*;
pub(super) use content_primitives::*;
pub(super) use controls::*;
pub(super) use documents::*;
pub(super) use editor::*;
pub(super) use extern_component::*;
pub(super) use foundation::*;
pub(super) use input::*;
pub(super) use layout::*;
pub(super) use markdown::*;
pub(super) use media::*;
pub(super) use pane::*;
pub(super) use range_controls::*;
pub(super) use selection::*;
pub(super) use structure::*;
pub(super) use table::*;
pub(super) use text::*;
pub(super) use themer_shader::*;
