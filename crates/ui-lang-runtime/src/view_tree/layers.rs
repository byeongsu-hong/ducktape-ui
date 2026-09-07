//! Native layering for copied module views.
use super::*;

fn surface(
    content: IceElement<'static, Output>,
    padding: Option<wire::Edges>,
    background: Option<wire::Rgba>,
    edge: Option<wire::Border>,
) -> IceElement<'static, Output> {
    let mut container = widget::container(content);
    if let Some(edges) = padding {
        container = container.padding(super::padding(edges));
    }
    container
        .style(move |_| widget::container::Style {
            background: background.map(|value| Background::Color(color(value))),
            border: edge.map(border).unwrap_or_default(),
            ..Default::default()
        })
        .into()
}

pub(super) fn render(node: &wire::Node, kept: &Kept<'_>) -> IceElement<'static, Output> {
    match node {
        wire::Node::Stack {
            key,
            width,
            height,
            padding,
            background,
            border,
            clip,
            under,
            children,
        } => {
            let mut children = selected_children(children, kept)
                .into_iter()
                .map(|child| render_node(child, kept))
                .collect::<Vec<_>>();
            let content: IceElement<'static, Output> = if *under == 0 {
                let mut stack = crate::zstack(children).clip(*clip);
                if let Some(width) = width {
                    stack = stack.width(length(*width));
                }
                if let Some(height) = height {
                    stack = stack.height(length(*height));
                }
                stack.into()
            } else {
                let above = children.split_off((*under as usize).min(children.len()));
                let stack = above
                    .into_iter()
                    .fold(widget::Stack::new(), |stack, child| stack.push(child));
                let mut stack = children
                    .into_iter()
                    .rev()
                    .fold(stack, |stack, child| stack.push_under(child))
                    .clip(*clip);
                if let Some(width) = width {
                    stack = stack.width(length(*width));
                }
                if let Some(height) = height {
                    stack = stack.height(length(*height));
                }
                stack.into()
            };
            accessible(
                surface(content, *padding, *background, *border),
                StableId::new(key),
                Role::GenericContainer,
            )
            .logical_id_maybe(cfg!(test).then_some(key.as_str()))
            .into()
        }
        wire::Node::Hover {
            key,
            width,
            height,
            padding,
            background,
            border,
            tint,
            radius,
            open,
            children,
        } => {
            let child = |index| {
                children
                    .get(index)
                    .map(|node| render_node(node, kept))
                    .unwrap_or_else(|| widget::Space::new().into())
            };
            let mut hover = crate::hover_reveal(child(0), child(1))
                .radius(*radius)
                .open(*open);
            if let Some(tint) = tint {
                hover = hover.tint(color(*tint));
            }
            let mut sized = widget::container(hover);
            if let Some(width) = width {
                sized = sized.width(length(*width));
            }
            if let Some(height) = height {
                sized = sized.height(length(*height));
            }
            accessible(
                surface(sized.into(), *padding, *background, *border),
                StableId::new(key),
                Role::GenericContainer,
            )
            .logical_id_maybe(cfg!(test).then_some(key.as_str()))
            .into()
        }
        wire::Node::Overlay {
            key,
            padding,
            backdrop,
            align_x,
            align_y,
            on_dismiss,
            children,
        } => {
            let base = children
                .first()
                .map(|child| render_node(child, kept))
                .unwrap_or_else(|| widget::Space::new().into());
            let open = children.get(1);
            let base: IceElement<'static, Output> = if open.is_some() {
                crate::focus_barrier(base).into()
            } else {
                base
            };
            let mut stack = widget::Stack::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .push(base);
            if let Some(layer) = open {
                let background = color(*backdrop);
                let backdrop = widget::container(widget::Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(move |_| widget::container::Style {
                        background: Some(Background::Color(background)),
                        ..Default::default()
                    });
                let backdrop = widget::mouse_area(backdrop)
                    .on_press(on_dismiss.map(Output::Activate).unwrap_or(Output::Ignore))
                    .on_release(Output::Ignore)
                    .on_right_press(Output::Ignore)
                    .on_right_release(Output::Ignore)
                    .on_middle_press(Output::Ignore)
                    .on_middle_release(Output::Ignore)
                    .on_scroll(|_| Output::Ignore);
                let panel = widget::mouse_area(render_node(layer, kept))
                    .on_press(Output::Ignore)
                    .on_release(Output::Ignore)
                    .on_right_press(Output::Ignore)
                    .on_right_release(Output::Ignore)
                    .on_middle_press(Output::Ignore)
                    .on_middle_release(Output::Ignore)
                    .on_scroll(|_| Output::Ignore);
                let panel = widget::container(panel)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(*padding)
                    .align_x(horizontal(*align_x))
                    .align_y(vertical(*align_y));
                let modal = widget::Stack::new()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .push(backdrop)
                    .push(panel);
                stack = stack.push(
                    widget::float(modal).translate(|_, _| iced::Vector::new(f32::EPSILON, 0.0)),
                );
            }
            accessible(stack, StableId::new(key), Role::GenericContainer)
                .logical_id_maybe(cfg!(test).then_some(key.as_str()))
                .into()
        }
        _ => unreachable!("layered node"),
    }
}
