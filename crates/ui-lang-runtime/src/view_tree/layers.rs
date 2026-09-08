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

pub(super) fn floating(
    node: &wire::Node,
    content: IceElement<'static, Output>,
) -> IceElement<'static, Output> {
    let wire::Node::Float {
        x,
        y,
        scale,
        shadow,
        radius: corners,
        ..
    } = node
    else {
        unreachable!("floating node")
    };
    let (x, y, shadow, corners) = (x.clone(), y.clone(), *shadow, *corners);
    widget::float(content)
        .scale(*scale)
        .translate(move |original, viewport| {
            let geometry = [
                original.x,
                original.y,
                original.width,
                original.height,
                viewport.x,
                viewport.y,
                viewport.width,
                viewport.height,
            ]
            .map(f64::from);
            iced::Vector::new(x.evaluate(geometry), y.evaluate(geometry))
        })
        .style(move |_| {
            let mut style = widget::float::Style::default();
            apply_shadow(shadow, &mut style.shadow);
            if let Some(corners) = corners {
                style.shadow_border_radius = radius(corners);
            }
            style
        })
        .into()
}

fn press_guard(content: IceElement<'static, Output>) -> IceElement<'static, Output> {
    widget::mouse_area(content)
        .on_press(Output::Ignore)
        .on_release(Output::Ignore)
        .on_right_press(Output::Ignore)
        .on_right_release(Output::Ignore)
        .on_middle_press(Output::Ignore)
        .on_middle_release(Output::Ignore)
        .on_scroll(|_| Output::Ignore)
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
                // Float relocates its child into an overlay. Guard the translated
                // content itself, matching native overlay lowering.
                let panel = if let wire::Node::Float { content, .. } = layer {
                    floating(layer, press_guard(render_node(content, kept)))
                } else {
                    press_guard(render_node(layer, kept))
                };
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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::renderer::Headless;
    use iced::{Event, Font, Pixels, Point, Size, mouse};
    use iced_test::runtime::{UserInterface, user_interface};

    #[test]
    fn floated_overlay_guards_visible_panel_but_not_its_original_slot() {
        let node = wire::Node::Overlay {
            key: "App/modal".into(),
            padding: 0.0,
            backdrop: wire::Rgba([0.0, 0.0, 0.0, 0.5]),
            align_x: wire::AlignX::Left,
            align_y: wire::AlignY::Top,
            on_dismiss: Some(17),
            children: vec![
                wire::Node::empty(),
                wire::Node::Float {
                    key: "App/modal/panel".into(),
                    x: wire::FloatExpression {
                        ops: vec![wire::FloatOp::Number(150.0)],
                    },
                    y: wire::FloatExpression {
                        ops: vec![wire::FloatOp::Number(90.0)],
                    },
                    scale: 1.0,
                    shadow: wire::Shadow::default(),
                    radius: None,
                    content: Box::new(wire::Node::Space {
                        width: Some(wire::Length::Fixed(80.0)),
                        height: Some(wire::Length::Fixed(60.0)),
                    }),
                },
            ],
        };
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        // The empty panel deliberately has no widget that could consume a click
        // instead of the overlay's guard. Both coordinates are inside the modal.
        for (point, dismisses) in [
            (Point::new(170.0, 110.0), false),
            (Point::new(20.0, 20.0), true),
        ] {
            let content = super::super::render(
                &node,
                &Inputs::default(),
                &Pictures::default(),
                &Surfaces::default(),
            );
            let mut ui = UserInterface::build(
                content,
                Size::new(400.0, 300.0),
                user_interface::Cache::default(),
                &mut renderer,
            );
            let mut outputs = Vec::new();
            ui.update(
                &[
                    Event::Mouse(mouse::Event::CursorMoved { position: point }),
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(point),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut outputs,
            );
            let dismiss_count = outputs
                .iter()
                .filter(|output| matches!(output, Output::Activate(17)))
                .count();
            assert_eq!(
                dismiss_count,
                usize::from(dismisses),
                "the visible panel must guard clicks while its original slot remains backdrop: {point:?}"
            );
            assert!(
                !outputs.is_empty(),
                "the click must reach the guard or backdrop"
            );
        }
    }
}
