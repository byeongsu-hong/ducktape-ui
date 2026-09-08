//! Native pick-list layout, handles, and lifecycle routes.
use super::*;

pub(super) fn render(node: &wire::Node) -> IceElement<'static, Output> {
    let wire::Node::PickList {
        settings,
        key,
        options,
        selected,
        placeholder,
        on_select,
        width,
        style,
    } = node
    else {
        unreachable!("pick node")
    };
    let style = *style;
    let choices: Vec<Choice> = options
        .iter()
        .enumerate()
        .map(|(index, option)| Choice(index as u32, option.clone()))
        .collect();
    let chosen = selected.and_then(|index| choices.get(index as usize).cloned());
    let handler = *on_select;
    let mut pick = widget::pick_list(choices, chosen.clone(), move |choice: Choice| {
        Output::Select {
            handler,
            index: choice.0,
        }
    })
    .style(move |theme, status| pick_list_style(style, theme, status));
    if let Some(menu) = style.menu {
        pick = pick.menu_style(move |theme| menu_style(menu, theme));
    }
    if let Some(placeholder) = placeholder {
        pick = pick.placeholder(placeholder.clone());
    }
    if let Some(width) = width {
        pick = pick.width(length(*width));
    }
    if let Some(value) = settings.menu_height {
        pick = pick.menu_height(length(value));
    }
    if let Some(value) = settings.padding {
        pick = pick.padding(crate::bounded_table_metric(f64::from(value), options.len()));
    }
    if let Some(value) = settings.text_size {
        pick = pick.text_size(value);
    }
    if let Some(value) = settings.line_height {
        pick = pick.text_line_height(widget::text::LineHeight::Relative(value));
    }
    if let Some(value) = settings.shaping {
        pick = pick.text_shaping(shaping(value));
    }
    if let Some(value) = &settings.font {
        pick = pick.font(text::named_font(value));
    }
    if let Some(value) = &settings.handle {
        pick = pick.handle(handle(value));
    }
    if let Some(route) = settings.on_open {
        pick = pick.on_open(Output::Activate(route));
    }
    if let Some(route) = settings.on_close {
        pick = pick.on_close(Output::Activate(route));
    }
    accessible(pick, StableId::new(key), Role::ComboBox)
        .logical_id_maybe(cfg!(test).then_some(key.as_str()))
        .label(placeholder.clone().unwrap_or_default())
        .value(chosen.map(|choice| choice.1).unwrap_or_default())
        .into()
}
pub(super) fn shaping(value: wire::Shaping) -> widget::text::Shaping {
    match value {
        wire::Shaping::Auto => widget::text::Shaping::Auto,
        wire::Shaping::Basic => widget::text::Shaping::Basic,
        wire::Shaping::Advanced => widget::text::Shaping::Advanced,
    }
}
fn icon(value: &wire::PickIcon) -> widget::pick_list::Icon<iced::Font> {
    widget::pick_list::Icon {
        font: value
            .font
            .as_ref()
            .map(text::named_font)
            .unwrap_or(iced::Font::DEFAULT),
        code_point: value.code_point,
        size: value.size.map(iced::Pixels),
        line_height: value
            .line_height
            .map(widget::text::LineHeight::Relative)
            .unwrap_or_default(),
        shaping: value.shaping.map(shaping).unwrap_or_default(),
    }
}
fn handle(value: &wire::PickHandle) -> widget::pick_list::Handle<iced::Font> {
    match value {
        wire::PickHandle::Arrow { size } => widget::pick_list::Handle::Arrow {
            size: size.map(iced::Pixels),
        },
        wire::PickHandle::Static(value) => widget::pick_list::Handle::Static(icon(value)),
        wire::PickHandle::Dynamic { closed, open } => widget::pick_list::Handle::Dynamic {
            closed: icon(closed),
            open: icon(open),
        },
        wire::PickHandle::None => widget::pick_list::Handle::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::{layout::Limits, renderer::Headless, widget::Tree};
    use iced::mouse;
    use iced_test::runtime::{UserInterface, user_interface};

    // Real layout and overlay events must distinguish dropped metrics/routes.
    #[test]
    fn native_pick_metrics_and_menu_routes_use_copied_options() {
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let node = wire::Node::PickList {
            key: "pick".into(),
            options: vec!["First".into(), "Second".into()],
            selected: None,
            placeholder: Some("Choose".into()),
            on_select: 7,
            width: Some(wire::Length::Fixed(180.0)),
            style: Default::default(),
            settings: Box::new(wire::PickOptions {
                padding: Some(9.0),
                text_size: Some(20.0),
                line_height: Some(1.5),
                menu_height: Some(wire::Length::Fixed(100.0)),
                handle: Some(wire::PickHandle::None),
                on_open: Some(11),
                on_close: Some(12),
                ..Default::default()
            }),
        };
        let mut element = render(&node);
        let mut tree = Tree::new(&element);
        let layout = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &Limits::new(iced::Size::ZERO, iced::Size::new(300.0, 250.0)),
        );
        assert_eq!(
            layout.size(),
            iced::Size::new(180.0, 48.0),
            "copied padding and text metrics determine the closed face"
        );
        let mut ui = UserInterface::build(
            element,
            iced::Size::new(300.0, 250.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let mut outputs = Vec::new();
        let click = |ui: &mut UserInterface<'_, Output, iced::Theme, iced::Renderer>,
                     renderer: &mut iced::Renderer,
                     outputs: &mut Vec<Output>,
                     x,
                     y| {
            let point = iced::Point::new(x, y);
            ui.update(
                &[
                    iced::Event::Mouse(mouse::Event::CursorMoved { position: point }),
                    iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(point),
                renderer,
                &mut iced::advanced::clipboard::Null,
                outputs,
            );
        };
        click(&mut ui, &mut renderer, &mut outputs, 10.0, 10.0);
        assert!(
            matches!(outputs.as_slice(), [Output::Activate(11)]),
            "opening emits its route exactly once: {outputs:?}"
        );
        outputs.clear();
        click(&mut ui, &mut renderer, &mut outputs, 250.0, 180.0);
        assert!(
            matches!(outputs.as_slice(), [Output::Activate(12)]),
            "outside click closes through its route: {outputs:?}"
        );
        outputs.clear();
        click(&mut ui, &mut renderer, &mut outputs, 10.0, 10.0);
        outputs.clear();
        click(&mut ui, &mut renderer, &mut outputs, 20.0, 70.0);
        assert!(
            matches!(
                outputs.as_slice(),
                [Output::Select {
                    handler: 7,
                    index: 0
                }]
            ),
            "native menu chooses the first copied option: {outputs:?}"
        );
    }
}
