use super::*;

pub(super) fn identify_rendered(
    rendered: String,
    id: Option<&Id>,
    message: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
    scope: &str,
) -> Result<String, Error> {
    let Some(id) = id else {
        return Ok(rendered);
    };
    let id = id_code(id, scope, env, document)?;
    Ok(format!(
        "{{ let __identified: __IceElement<'_, {message}> = {rendered}; ::iced::widget::container(__identified).id(::iced::widget::Id::from({id})).into() }}"
    ))
}

pub(super) fn rendered_child_scope(
    id: Option<&Id>,
    scope: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    id.map_or_else(
        || Ok(scope.to_owned()),
        |id| id_code(id, scope, env, document),
    )
}

pub(in crate::codegen) fn component_slot_context(
    slots: &[ComponentSlot],
    document: &Document,
    env: &HashMap<String, Binding>,
    parent: Option<&SlotContext>,
) -> Result<Option<SlotContext>, Error> {
    let mut entries = Vec::new();
    for slot in slots {
        if !node_is_omitted(&slot.content, document, env, parent)? {
            entries.push(SlotContent {
                name: slot.name.clone(),
                node: (*slot.content).clone(),
                env: env.clone(),
            });
        }
    }
    Ok((!entries.is_empty()).then(|| SlotContext {
        entries,
        parent: parent.cloned().map(Box::new),
    }))
}

pub(in crate::codegen) fn render_node_if_present(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    if node_is_omitted(node, document, env, slot)? {
        Ok(None)
    } else {
        render_node(node, document, message, env, scope, slot).map(Some)
    }
}

fn node_is_omitted(
    node: &ViewNode,
    document: &Document,
    env: &HashMap<String, Binding>,
    slot: Option<&SlotContext>,
) -> Result<bool, Error> {
    let omitted = match node {
        ViewNode::Slot { name, .. } => {
            let Some((content, parent)) = slot.and_then(|slot| {
                slot.entries
                    .iter()
                    .find(|entry| entry.name == *name)
                    .map(|content| (content, slot.parent.as_deref()))
            }) else {
                return Ok(true);
            };
            node_is_omitted(&content.node, document, &content.env, parent)?
        }
        ViewNode::Component {
            name, slots, span, ..
        } => {
            let component = document
                .components
                .iter()
                .find(|component| component.name == *name)
                .ok_or_else(|| Error::new("E122", span, format!("unknown component `{name}`")))?;
            let slots = component_slot_context(slots, document, env, slot)?;
            node_is_omitted(&component.root, document, env, slots.as_ref())?
        }
        ViewNode::Layout {
            kind: Layout::Scroll,
            children,
            ..
        } => node_is_omitted(&children[0], document, env, slot)?,
        ViewNode::Container { content, .. }
        | ViewNode::MouseArea { content, .. }
        | ViewNode::ResizeHandle { content, .. }
        | ViewNode::Theme { content, .. }
        | ViewNode::Float { content, .. }
        | ViewNode::Pin { content, .. }
        | ViewNode::Sensor { content, .. }
        | ViewNode::KeyedColumn { child: content, .. }
        | ViewNode::Lazy { child: content, .. } => node_is_omitted(content, document, env, slot)?,
        ViewNode::Button {
            content: Some(content),
            ..
        } => node_is_omitted(content, document, env, slot)?,
        ViewNode::Tooltip { content, tip, .. } => {
            node_is_omitted(content, document, env, slot)?
                || node_is_omitted(tip, document, env, slot)?
        }
        ViewNode::Overlay { content, layer, .. } => {
            node_is_omitted(content, document, env, slot)?
                || node_is_omitted(layer, document, env, slot)?
        }
        ViewNode::Responsive { content, .. } => match content {
            ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                node_is_omitted(narrow, document, env, slot)?
                    || node_is_omitted(wide, document, env, slot)?
            }
            ResponsiveContent::Size { content, .. } => {
                node_is_omitted(content, document, env, slot)?
            }
        },
        ViewNode::Table { columns, .. } => {
            let mut omitted = false;
            for column in columns {
                omitted |= node_is_omitted(&column.header, document, env, slot)?
                    || node_is_omitted(&column.cell, document, env, slot)?;
            }
            omitted
        }
        ViewNode::PaneGrid {
            panes, templates, ..
        } => {
            let mut omitted = false;
            for child in panes
                .iter()
                .flat_map(PaneView::nodes)
                .chain(templates.iter().flat_map(|template| template.pane.nodes()))
            {
                omitted |= node_is_omitted(child, document, env, slot)?;
            }
            omitted
        }
        _ => false,
    };
    Ok(omitted)
}

pub(in crate::codegen) fn render_node(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    if let Some(rendered) = render_foundation(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    if let Some(rendered) = render_controls(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    if let Some(rendered) = render_content(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    if let Some(rendered) = render_media(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    if let Some(rendered) = render_structure(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    if let Some(rendered) = render_documents(node, document, message, env, scope, slot)? {
        return Ok(rendered);
    }
    unreachable!("every view node belongs to a render group")
}

mod container;
mod content;
mod controls;
mod documents;
mod foundation;
mod layout;
mod media;
mod pane;
mod structure;
mod table;

pub(super) use container::*;
pub(super) use content::*;
pub(super) use controls::*;
pub(super) use documents::*;
pub(super) use foundation::*;
pub(super) use layout::*;
pub(super) use media::*;
pub(super) use pane::*;
pub(super) use structure::*;
pub(super) use table::*;
