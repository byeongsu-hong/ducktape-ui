//! Captured overlay keys never reach the base widget, so forward them here.
use super::*;

pub(super) fn wrap<'a>(
    content: overlay::Element<'a, String, iced::Theme, iced::Renderer>,
    guest: Arc<Mutex<Guest>>,
) -> overlay::Element<'a, String, iced::Theme, iced::Renderer> {
    overlay::Element::new(Box::new(Keyboard { content, guest }))
}
struct Keyboard<'a> {
    content: overlay::Element<'a, String, iced::Theme, iced::Renderer>,
    guest: Arc<Mutex<Guest>>,
}
impl overlay::Overlay<String, iced::Theme, iced::Renderer> for Keyboard<'_> {
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        self.content.as_overlay_mut().layout(renderer, bounds)
    }
    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content
            .as_overlay()
            .draw(renderer, theme, style, layout, cursor);
    }
    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_overlay_mut()
            .operate(layout, renderer, operation);
    }
    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, String>,
    ) {
        let mut messages = Vec::new();
        let mut local = Shell::new(&mut messages);
        self.content
            .as_overlay_mut()
            .update(event, layout, cursor, renderer, clipboard, &mut local);
        if local.is_event_captured()
            && let Event::Keyboard(event) = event
        {
            self.guest
                .lock()
                .expect("guest lock")
                .pending
                .push(ui_lang_wire::Event::Keyboard {
                    event: event.clone().into(),
                    captured: true,
                });
            local.request_redraw();
        }
        shell.merge(local, std::convert::identity);
    }
    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_overlay()
            .mouse_interaction(layout, cursor, renderer)
    }
    fn overlay<'a>(
        &'a mut self,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
    ) -> Option<overlay::Element<'a, String, iced::Theme, iced::Renderer>> {
        self.content
            .as_overlay_mut()
            .overlay(layout, renderer)
            .map(|content| wrap(content, self.guest.clone()))
    }
    fn index(&self) -> f32 {
        self.content.as_overlay().index()
    }
}
