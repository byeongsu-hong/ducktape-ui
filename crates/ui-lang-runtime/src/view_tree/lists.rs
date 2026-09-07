//! Host-owned keyed row state and viewport-based layout.
use super::*;

pub(super) fn render(node: &wire::Node, kept: &Kept<'_>) -> IceElement<'static, Output> {
    let wire::Node::KeyedColumn {
        key,
        keys,
        background,
        border,
        spacing,
        padding: edges,
        width,
        height,
        max_width,
        align,
        virtual_row,
        children,
    } = node
    else {
        unreachable!("keyed column")
    };
    let selected = if keys.is_some() {
        children.iter().collect()
    } else {
        selected_children(children, kept)
    };
    let count = keys
        .as_ref()
        .map_or(selected.len(), |keys| selected.len().min(keys.len()));
    let children = selected
        .into_iter()
        .take(count)
        .map(|child| bounded_fill_element(render_node(child, kept), count, false))
        .collect::<Vec<_>>();
    let spacing = bounded_spacing(f64::from(spacing.unwrap_or(0.0)), count);
    let content: IceElement<'static, Output> = if let Some(estimate) = virtual_row {
        let rows = if let Some(keys) = keys {
            crate::virtual_keyed_children(
                keys.iter()
                    .copied()
                    .zip(children)
                    .map(|(key, child)| (key.virtual_key(), child))
                    .collect(),
                *estimate,
            )
        } else {
            crate::virtual_children(children, *estimate)
        };
        let mut column = widget::column(vec![rows.spacing(spacing).into()]);
        if let Some(edges) = edges {
            column = column.padding(padding(*edges));
        }
        if let Some(width) = width {
            column = column.width(length(*width));
        }
        if let Some(height) = height {
            column = column.height(length(*height));
        }
        if let Some(max_width) = max_width {
            column = column.max_width(*max_width);
        }
        column.into()
    } else {
        let keyed = children
            .into_iter()
            .enumerate()
            .map(|(index, child)| {
                let key = keys
                    .as_ref()
                    .and_then(|keys| keys.get(index))
                    .copied()
                    .unwrap_or(wire::ListKey::Integer(index as i64));
                (NativeKey(key), child)
            })
            .collect::<Vec<_>>();
        let mut column = crate::keyed_column(keyed).spacing(spacing);
        if let Some(edges) = edges {
            column = column.padding(padding(*edges));
        }
        if let Some(width) = width {
            column = column.width(length(*width));
        }
        if let Some(height) = height {
            column = column.height(length(*height));
        }
        if let Some(max_width) = max_width {
            column = column.max_width(*max_width);
        }
        if let Some(align) = align {
            column = column.align_items(horizontal(*align).into());
        }
        column.into()
    };
    accessible(
        surfaced(content, *background, *border),
        StableId::new(key),
        Role::GenericContainer,
    )
    .logical_id_maybe(cfg!(test).then_some(key.as_str()))
    .into()
}

// Wire equality preserves bits; native nonvirtual keys use numeric PartialEq.
#[derive(Clone, Copy)]
struct NativeKey(wire::ListKey);
impl PartialEq for NativeKey {
    fn eq(&self, other: &Self) -> bool {
        match (self.0, other.0) {
            (wire::ListKey::Float(a), wire::ListKey::Float(b)) => a == b,
            (a, b) => a == b,
        }
    }
}
