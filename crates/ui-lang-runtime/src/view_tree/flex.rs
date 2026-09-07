//! Render copied flex rules with the native engine.
use super::*;

pub(super) fn render(node: &wire::Node, kept: &Kept<'_>) -> IceElement<'static, Output> {
    let wire::Node::Flex {
        key,
        layout,
        background,
        border,
        items,
        children,
    } = node
    else {
        unreachable!("flex node")
    };
    let items = children
        .iter()
        .enumerate()
        .map(|(index, child)| {
            let options = items.get(index).copied().unwrap_or_default();
            let mut item = crate::flex_item(render_node(child, kept))
                .order(options.order)
                .shrink(options.shrink)
                .basis(match options.basis {
                    wire::FlexBasis::Auto => crate::FlexBasis::Auto,
                    wire::FlexBasis::Content => crate::FlexBasis::Content,
                    wire::FlexBasis::Fixed(value) => crate::FlexBasis::Fixed(value),
                    wire::FlexBasis::Percent(value) => crate::FlexBasis::Percent(value),
                })
                .margins(crate::FlexMargins {
                    top: margin(options.margins.top),
                    right: margin(options.margins.right),
                    bottom: margin(options.margins.bottom),
                    left: margin(options.margins.left),
                });
            if let Some(grow) = options.grow {
                item = item.grow(grow);
            }
            if let Some(align) = options.align {
                item = item.align_self(item_alignment(align));
            }
            item
        })
        .collect();
    let mut content = crate::flex(items)
        .direction(match layout.direction {
            wire::FlexDirection::Row => crate::FlexDirection::Row,
            wire::FlexDirection::RowReverse => crate::FlexDirection::RowReverse,
            wire::FlexDirection::Column => crate::FlexDirection::Column,
            wire::FlexDirection::ColumnReverse => crate::FlexDirection::ColumnReverse,
        })
        .wrap(match layout.wrap {
            wire::FlexWrap::NoWrap => crate::FlexWrap::NoWrap,
            wire::FlexWrap::Wrap => crate::FlexWrap::Wrap,
            wire::FlexWrap::WrapReverse => crate::FlexWrap::WrapReverse,
        })
        .clip(layout.clip);
    if let Some(value) = layout.justify {
        content = content.justify_content(justify(value));
    }
    if let Some(value) = layout.items {
        content = content.align_items(item_alignment(value));
    }
    if let Some(value) = layout.content {
        content = content.align_content(content_alignment(value));
    }
    if let Some(value) = layout.row_gap {
        content = content.row_gap(value);
    }
    if let Some(value) = layout.column_gap {
        content = content.column_gap(value);
    }
    if let Some(value) = layout.padding {
        content = content.padding(padding(value));
    }
    if let Some(value) = layout.width {
        content = content.width(length(value));
    }
    if let Some(value) = layout.height {
        content = content.height(length(value));
    }
    if let Some(value) = layout.max_width {
        content = content.max_width(value);
    }
    if let Some(value) = layout.max_height {
        content = content.max_height(value);
    }
    let mut surface = surfaced(content, *background, *border);
    if let Some(value) = layout.surface_width {
        surface = surface.width(length(value));
    }
    if let Some(value) = layout.surface_height {
        surface = surface.height(length(value));
    }
    if let Some(value) = layout.surface_max_width {
        surface = surface.max_width(value);
    }
    accessible(surface, StableId::new(key), Role::GenericContainer)
        .logical_id_maybe(cfg!(test).then_some(key.as_str()))
        .into()
}

fn margin(value: wire::FlexMargin) -> crate::FlexMargin {
    match value {
        wire::FlexMargin::Zero => crate::FlexMargin::Zero,
        wire::FlexMargin::Auto => crate::FlexMargin::Auto,
        wire::FlexMargin::Fixed(value) => crate::FlexMargin::Fixed(value),
        wire::FlexMargin::Percent(value) => crate::FlexMargin::Percent(value),
    }
}
fn item_alignment(value: wire::FlexItemAlignment) -> crate::AlignItems {
    match value {
        wire::FlexItemAlignment::Start => crate::AlignItems::Start,
        wire::FlexItemAlignment::End => crate::AlignItems::End,
        wire::FlexItemAlignment::FlexStart => crate::AlignItems::FlexStart,
        wire::FlexItemAlignment::FlexEnd => crate::AlignItems::FlexEnd,
        wire::FlexItemAlignment::Center => crate::AlignItems::Center,
        wire::FlexItemAlignment::Baseline => crate::AlignItems::Baseline,
        wire::FlexItemAlignment::Stretch => crate::AlignItems::Stretch,
    }
}
fn justify(value: wire::FlexContentAlignment) -> crate::JustifyContent {
    match value {
        wire::FlexContentAlignment::Start => crate::JustifyContent::Start,
        wire::FlexContentAlignment::End => crate::JustifyContent::End,
        wire::FlexContentAlignment::FlexStart => crate::JustifyContent::FlexStart,
        wire::FlexContentAlignment::FlexEnd => crate::JustifyContent::FlexEnd,
        wire::FlexContentAlignment::Center => crate::JustifyContent::Center,
        wire::FlexContentAlignment::Stretch => crate::JustifyContent::Stretch,
        wire::FlexContentAlignment::SpaceBetween => crate::JustifyContent::SpaceBetween,
        wire::FlexContentAlignment::SpaceAround => crate::JustifyContent::SpaceAround,
        wire::FlexContentAlignment::SpaceEvenly => crate::JustifyContent::SpaceEvenly,
    }
}
fn content_alignment(value: wire::FlexContentAlignment) -> crate::AlignContent {
    match value {
        wire::FlexContentAlignment::Start => crate::AlignContent::Start,
        wire::FlexContentAlignment::End => crate::AlignContent::End,
        wire::FlexContentAlignment::FlexStart => crate::AlignContent::FlexStart,
        wire::FlexContentAlignment::FlexEnd => crate::AlignContent::FlexEnd,
        wire::FlexContentAlignment::Center => crate::AlignContent::Center,
        wire::FlexContentAlignment::Stretch => crate::AlignContent::Stretch,
        wire::FlexContentAlignment::SpaceBetween => crate::AlignContent::SpaceBetween,
        wire::FlexContentAlignment::SpaceAround => crate::AlignContent::SpaceAround,
        wire::FlexContentAlignment::SpaceEvenly => crate::AlignContent::SpaceEvenly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::{layout, renderer::Headless, widget::Tree};
    use iced::{Font, Pixels, Rectangle, Size};

    fn geometry(node: &wire::Node) -> (Rectangle, Vec<Rectangle>) {
        let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let mut element = super::super::render(
            node,
            &Inputs::default(),
            &Pictures::default(),
            &Surfaces::new(),
        );
        let mut tree = Tree::new(element.as_widget());
        let layout = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(600.0, 400.0)),
        );
        fn walk(node: &layout::Node, parent: iced::Vector, result: &mut Vec<Rectangle>) {
            let bounds = node.bounds();
            let offset = parent + iced::Vector::new(bounds.x, bounds.y);
            if node.children().is_empty() {
                result.push(Rectangle::new(
                    iced::Point::new(offset.x, offset.y),
                    bounds.size(),
                ));
            }
            for child in node.children() {
                walk(child, offset, result);
            }
        }
        let mut result = vec![];
        walk(&layout, iced::Vector::ZERO, &mut result);
        (layout.bounds(), result)
    }
    fn leaves(node: &wire::Node) -> Vec<Rectangle> {
        geometry(node).1
    }
    fn flex(layout: wire::FlexLayout, items: Vec<wire::FlexItem>, width: f32) -> wire::Node {
        wire::Node::Flex {
            key: "flex".into(),
            layout,
            background: None,
            border: None,
            items,
            children: (0..3)
                .map(|_| wire::Node::Space {
                    width: Some(wire::Length::Fixed(width)),
                    height: Some(wire::Length::Fixed(10.0)),
                })
                .collect(),
        }
    }
    #[test]
    fn wire_flex_keeps_utility_surface_dimensions_separate_from_explicit_inner_size() {
        let node = flex(
            wire::FlexLayout {
                width: Some(wire::Length::Fixed(100.0)),
                surface_width: Some(wire::Length::Fill),
                surface_height: Some(wire::Length::Fill),
                surface_max_width: Some(180.0),
                ..Default::default()
            },
            vec![wire::FlexItem::default(); 3],
            20.0,
        );
        let (outer, children) = geometry(&node);
        assert_eq!(
            outer.width, 180.0,
            "outer fill surface must honor its utility cap"
        );
        assert_eq!(
            outer.height, 400.0,
            "outer height utility must fill available space"
        );
        assert_eq!(
            children.iter().map(|r| r.width).collect::<Vec<_>>(),
            vec![20.0; 3]
        );
    }

    #[test]
    fn wire_flex_wraps_with_independent_gaps_and_reflows_after_width_changes() {
        let mut node = flex(
            wire::FlexLayout {
                width: Some(wire::Length::Fixed(100.0)),
                wrap: wire::FlexWrap::Wrap,
                row_gap: Some(5.0),
                column_gap: Some(7.0),
                ..Default::default()
            },
            vec![wire::FlexItem::default(); 3],
            40.0,
        );
        let positions =
            |node: &wire::Node| leaves(node).iter().map(|r| (r.x, r.y)).collect::<Vec<_>>();
        assert_eq!(positions(&node), vec![(0.0, 0.0), (47.0, 0.0), (0.0, 15.0)]);
        let wire::Node::Flex { layout, .. } = &mut node else {
            unreachable!()
        };
        layout.width = Some(wire::Length::Fixed(50.0));
        assert_eq!(positions(&node), vec![(0.0, 0.0), (0.0, 15.0), (0.0, 30.0)]);
    }
    #[test]
    fn wire_flex_preserves_item_order_grow_and_percentage_basis() {
        let node = flex(
            wire::FlexLayout {
                width: Some(wire::Length::Fixed(120.0)),
                ..Default::default()
            },
            vec![
                wire::FlexItem {
                    order: 2,
                    grow: Some(1.0),
                    basis: wire::FlexBasis::Fixed(20.0),
                    ..Default::default()
                },
                wire::FlexItem {
                    grow: Some(2.0),
                    basis: wire::FlexBasis::Percent(1.0 / 6.0),
                    ..Default::default()
                },
                wire::FlexItem::default(),
            ],
            20.0,
        );
        let bounds = leaves(&node);
        assert_eq!(
            bounds.iter().map(|r| (r.x, r.width)).collect::<Vec<_>>(),
            vec![(80.0, 40.0), (0.0, 60.0), (60.0, 20.0)]
        );
    }
}
