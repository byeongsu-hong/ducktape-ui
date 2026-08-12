use super::*;

#[cfg(test)]
pub fn native_help(active: bool) -> iced::Element<'static, bool> {
    let hint = if active {
        "Pointer entered the external component"
    } else {
        "This tooltip and mouse area are built in Rust"
    };
    iced::widget::mouse_area(iced::widget::tooltip(
        iced::widget::text("Extern component"),
        iced::widget::text(hint),
        iced::widget::tooltip::Position::Bottom,
    ))
    .on_enter(true)
    .on_exit(false)
    .into()
}

#[cfg(test)]
pub fn borrowed_help<'a>(
    label: &'a str,
    active: &'a bool,
) -> iced::Element<'a, bool, iced::Theme, AppRenderer> {
    iced::widget::button(iced::widget::text(if label.is_empty() {
        "Borrowed extern component"
    } else {
        label
    }))
    .on_press(!*active)
    .into()
}

#[cfg(test)]
pub struct IndexedOverlayHost {
    index: f32,
}

#[cfg(test)]
pub struct IndexedOverlay {
    pub index: f32,
}

#[cfg(test)]
impl iced::advanced::Widget<(), iced::Theme, iced::Renderer> for IndexedOverlayHost {
    fn size(&self) -> iced::Size<iced::Length> {
        iced::Size::new(iced::Length::Shrink, iced::Length::Shrink)
    }

    fn layout(
        &mut self,
        _tree: &mut iced::advanced::widget::Tree,
        _renderer: &iced::Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        iced::advanced::layout::atomic(limits, iced::Length::Shrink, iced::Length::Shrink)
    }

    fn draw(
        &self,
        _tree: &iced::advanced::widget::Tree,
        _renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &iced::advanced::renderer::Style,
        _layout: iced::advanced::Layout<'_>,
        _cursor: iced::mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
    }

    fn overlay<'a>(
        &'a mut self,
        _tree: &'a mut iced::advanced::widget::Tree,
        _layout: iced::advanced::Layout<'a>,
        _renderer: &iced::Renderer,
        _viewport: &iced::Rectangle,
        _translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'a, (), iced::Theme, iced::Renderer>> {
        Some(iced::advanced::overlay::Element::new(Box::new(
            IndexedOverlay { index: self.index },
        )))
    }
}

#[cfg(test)]
impl iced::advanced::Overlay<(), iced::Theme, iced::Renderer> for IndexedOverlay {
    fn layout(
        &mut self,
        _renderer: &iced::Renderer,
        _bounds: iced::Size,
    ) -> iced::advanced::layout::Node {
        iced::advanced::layout::Node::new(iced::Size::new(1.0, 1.0))
    }

    fn draw(
        &self,
        _renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &iced::advanced::renderer::Style,
        _layout: iced::advanced::Layout<'_>,
        _cursor: iced::mouse::Cursor,
    ) {
    }

    fn index(&self) -> f32 {
        self.index
    }
}

#[cfg(test)]
pub fn native_overlay(index: f64) -> iced::Element<'static, ()> {
    iced::Element::new(IndexedOverlayHost {
        index: index as f32,
    })
}

#[cfg(test)]
pub struct DocsViewer {
    prefix: String,
}

#[cfg(test)]
pub fn docs_viewer(prefix: String) -> DocsViewer {
    DocsViewer { prefix }
}

#[cfg(test)]
pub fn summary_text(theme: &iced::Theme, busy: bool) -> iced::widget::text::Style {
    if busy {
        iced::widget::text::warning(theme)
    } else {
        iced::widget::text::primary(theme)
    }
}

#[cfg(test)]
pub fn volume_slider(
    theme: &iced::Theme,
    status: iced::widget::slider::Status,
    busy: bool,
) -> iced::widget::slider::Style {
    let mut style = iced::widget::slider::default(theme, status);
    if busy {
        style.handle.border_color = theme.palette().danger;
    }
    style
}

#[cfg(test)]
pub fn loading_progress(theme: &iced::Theme, active: bool) -> iced::widget::progress_bar::Style {
    if active {
        iced::widget::progress_bar::warning(theme)
    } else {
        iced::widget::progress_bar::success(theme)
    }
}

#[cfg(test)]
pub fn action_button(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
    busy: bool,
) -> iced::widget::button::Style {
    if busy {
        iced::widget::button::secondary(theme, status)
    } else {
        iced::widget::button::primary(theme, status)
    }
}

#[cfg(test)]
pub fn task_checkbox(
    theme: &iced::Theme,
    status: iced::widget::checkbox::Status,
    busy: bool,
) -> iced::widget::checkbox::Style {
    if busy {
        iced::widget::checkbox::secondary(theme, status)
    } else {
        iced::widget::checkbox::primary(theme, status)
    }
}

#[cfg(test)]
pub fn notification_toggler(
    theme: &iced::Theme,
    status: iced::widget::toggler::Status,
    busy: bool,
) -> iced::widget::toggler::Style {
    let mut style = iced::widget::toggler::default(theme, status);
    if busy {
        style.text_color = Some(theme.palette().text);
    }
    style
}

#[cfg(test)]
pub fn view_radio(
    theme: &iced::Theme,
    status: iced::widget::radio::Status,
    busy: bool,
) -> iced::widget::radio::Style {
    let mut style = iced::widget::radio::default(theme, status);
    if busy {
        style.text_color = Some(theme.palette().text);
    }
    style
}

#[cfg(test)]
pub fn summary_container(theme: &iced::Theme, busy: bool) -> iced::widget::container::Style {
    if busy {
        iced::widget::container::bordered_box(theme)
    } else {
        iced::widget::container::rounded_box(theme)
    }
}

#[cfg(test)]
pub fn status_svg(
    theme: &iced::Theme,
    status: iced::widget::svg::Status,
    active: bool,
) -> iced::widget::svg::Style {
    let color = active.then(|| match status {
        iced::widget::svg::Status::Idle => theme.palette().text,
        iced::widget::svg::Status::Hovered => theme.palette().primary,
    });
    iced::widget::svg::Style { color }
}

#[cfg(test)]
pub fn form_input(
    theme: &iced::Theme,
    status: iced::widget::text_input::Status,
    disabled: bool,
) -> iced::widget::text_input::Style {
    let mut style = iced::widget::text_input::default(theme, status);
    if disabled {
        style.value = theme.palette().text;
    }
    style
}

#[cfg(test)]
pub fn task_scroll(
    theme: &iced::Theme,
    status: iced::widget::scrollable::Status,
    active: bool,
) -> iced::widget::scrollable::Style {
    let mut style = iced::widget::scrollable::default(theme, status);
    if active {
        style.container.text_color = Some(theme.palette().text);
    }
    style
}

#[cfg(test)]
pub fn view_picker(
    theme: &iced::Theme,
    status: iced::widget::pick_list::Status,
    active: bool,
) -> iced::widget::pick_list::Style {
    let mut style = iced::widget::pick_list::default(theme, status);
    if active {
        style.handle_color = theme.palette().primary;
    }
    style
}

#[cfg(test)]
pub fn view_menu(theme: &iced::Theme, active: bool) -> iced::overlay::menu::Style {
    let mut style = iced::overlay::menu::default(theme);
    if active {
        style.selected_text_color = theme.palette().text;
    }
    style
}

#[cfg(test)]
pub fn workspace_panes(theme: &iced::Theme, active: bool) -> iced::widget::pane_grid::Style {
    let mut style = iced::widget::pane_grid::default(theme);
    style.hovered_split.width = if active { 5.0 } else { 2.0 };
    style
}

#[cfg(test)]
impl<'a> iced::widget::markdown::Viewer<'a, String> for DocsViewer {
    fn on_link_click(url: iced::widget::markdown::Uri) -> String {
        url
    }

    fn image(
        &self,
        _settings: iced::widget::markdown::Settings,
        url: &'a iced::widget::markdown::Uri,
        _title: &'a str,
        _alt: &iced::widget::markdown::Text,
    ) -> iced::Element<'a, String> {
        iced::widget::text(format!("{} image: {url}", self.prefix)).into()
    }
}
