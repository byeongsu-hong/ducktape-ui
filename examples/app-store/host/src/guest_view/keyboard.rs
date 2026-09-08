//! Captured overlay keys never reach the base widget, so forward them here.
use super::*;

pub(super) fn wrap<'a>(
    content: overlay::Element<'a, String, iced::Theme, iced::Renderer>,
    guest: Arc<Mutex<Guest>>,
    instance: Arc<AtomicBool>,
) -> overlay::Element<'a, String, iced::Theme, iced::Renderer> {
    overlay::Element::new(Box::new(Keyboard {
        content,
        guest,
        instance,
    }))
}
struct Keyboard<'a> {
    content: overlay::Element<'a, String, iced::Theme, iced::Renderer>,
    guest: Arc<Mutex<Guest>>,
    instance: Arc<AtomicBool>,
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
        if !self.instance.load(Ordering::Relaxed) {
            shell.invalidate_widgets();
            shell.publish("wake".into());
            return;
        }
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
            .map(|content| wrap(content, self.guest.clone(), self.instance.clone()))
    }
    fn index(&self) -> f32 {
        self.content.as_overlay().index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CapturingOverlay;
    impl overlay::Overlay<String, iced::Theme, iced::Renderer> for CapturingOverlay {
        fn layout(&mut self, _: &iced::Renderer, bounds: Size) -> layout::Node {
            layout::Node::new(bounds)
        }
        fn draw(
            &self,
            _: &mut iced::Renderer,
            _: &iced::Theme,
            _: &renderer::Style,
            _: Layout<'_>,
            _: mouse::Cursor,
        ) {
        }
        fn update(
            &mut self,
            _: &Event,
            _: Layout<'_>,
            _: mouse::Cursor,
            _: &iced::Renderer,
            _: &mut dyn Clipboard,
            shell: &mut Shell<'_, String>,
        ) {
            shell.capture_event();
        }
    }

    #[test]
    #[ignore = "requires two bundled reload fixtures"]
    fn bundled_reload_old_overlay_keyboard_cannot_reach_new_instance() {
        let load = |version| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../target/reload-v{version}-fixture/app_store_reload_v{version}_fixture.wasm"
            ));
            let bytes = std::fs::read(&path).expect("bundle both reload fixtures first");
            iced::futures::executor::block_on(crate::store::install_app(
                crate::store::CatalogEntry {
                    id: "overlay-reload".into(),
                    name: "Overlay reload".into(),
                    description: String::new(),
                    capabilities: vec![],
                    path: path.to_string_lossy().into(),
                    mark: "O".into(),
                    hash: crate::catalog::sha256_hex(&bytes),
                },
            ))
            .unwrap()
            .surface
        };
        let guest = load(1).0;
        let instance = guest.lock().unwrap().alive.clone();
        let mut overlay = wrap(
            overlay::Element::new(Box::new(CapturingOverlay)),
            guest.clone(),
            instance,
        );
        let renderer = iced::futures::executor::block_on(
            <iced::Renderer as iced::advanced::renderer::Headless>::new(
                iced::Font::DEFAULT,
                iced::Pixels(16.0),
                Some("tiny-skia"),
            ),
        )
        .unwrap();
        let node = overlay
            .as_overlay_mut()
            .layout(&renderer, Size::new(100.0, 100.0));
        let event = Event::Keyboard(iced::keyboard::Event::KeyReleased {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::empty(),
        });
        let mut messages = vec![];
        overlay.as_overlay_mut().update(
            &event,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
        );
        assert!(
            matches!(
                guest.lock().unwrap().pending.as_slice(),
                [ui_lang_wire::Event::Keyboard { captured: true, .. }]
            ),
            "a live overlay must forward captured keys"
        );
        // The overlay is still mounted while the same shared surface changes generation.
        let next = load(2);
        std::mem::swap(&mut *guest.lock().unwrap(), &mut *next.0.lock().unwrap());
        drop(next);
        overlay.as_overlay_mut().update(
            &event,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
        );
        assert!(
            guest.lock().unwrap().pending.is_empty(),
            "old overlay keyboard events must be refused"
        );
    }
}
