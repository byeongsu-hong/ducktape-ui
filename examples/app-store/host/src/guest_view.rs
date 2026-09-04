//! The widget that shows a running guest: the tree the guest last sent,
//! rendered with the host's own widgets, wrapped so that every redraw of the
//! window ticks the guest and everything the user does inside it goes back
//! as the guest's own events. A guest the host had to end shows why, with
//! the one button that can bring it back.

use std::sync::{Arc, Mutex};

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector, widget, window};
use ui_lang_runtime::view_tree::{self, Output};

use crate::store::{Guest, Surface};

/// The guest's window. It emits `"restart"` when the user asks for one,
/// `"ended"` when the instance ended on its own, and `"wake"` when the tree
/// changed or a publish must reach the other windows — all the store's
/// business: reloading the module may be a cranelift run, the counts are
/// only recomputed when the app has a message, and a changed tree is shown
/// by rebuilding the view. `dark` is the store's colour mode, which the
/// guest hears about through its theme subscription.
pub fn wasm_view(surface: Surface, dark: bool) -> Element<'static, String> {
    let guest = surface.0;
    let (content, rev) = {
        let locked = guest.lock().expect("guest lock");
        if let Some(fault) = &locked.fault {
            return fault_view(fault);
        }
        let root = locked
            .frame
            .root
            .clone()
            .unwrap_or_else(ui_lang_wire::Node::empty);
        (view_tree::render(&root, &locked.inputs), locked.frame_rev)
    };
    Element::new(GuestView {
        guest,
        rev,
        dark,
        content,
    })
}

/// What an ended app's window shows: the reason, and Restart.
fn fault_view(fault: &str) -> Element<'static, String> {
    widget::container(
        widget::column![
            widget::text("This app was stopped by the store.").size(16),
            widget::text(fault.to_owned()).size(13),
            widget::button(widget::text("Restart")).on_press("restart".to_string()),
        ]
        .spacing(12)
        .align_x(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center(Length::Fill)
    .into()
}

struct GuestView {
    guest: Arc<Mutex<Guest>>,
    /// The frame this element was rendered from.
    rev: u64,
    dark: bool,
    content: Element<'static, Output, iced::Theme, iced::Renderer>,
}

impl Widget<String, iced::Theme, iced::Renderer> for GuestView {
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, String>,
        viewport: &Rectangle,
    ) {
        // The tree's widgets speak `Output`; what they say is the guest's,
        // not the store's, so it is diverted rather than mapped. Everything
        // else the local shell collected — a redraw request, a captured
        // event, an open input method — is the window's and carries over.
        let mut outputs = Vec::new();
        {
            let mut local = Shell::new(&mut outputs);
            self.content.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, &mut local, viewport,
            );
            if local.is_event_captured() {
                shell.capture_event();
            }
            if local.is_layout_invalid() {
                shell.invalidate_layout();
            }
            if local.are_widgets_invalid() {
                shell.invalidate_widgets();
            }
            match local.redraw_request() {
                window::RedrawRequest::NextFrame => shell.request_redraw(),
                window::RedrawRequest::At(at) => shell.request_redraw_at(at),
                window::RedrawRequest::Wait => {}
            }
            shell.input_method_mut().merge(local.input_method());
        }
        let mut guest = self.guest.lock().expect("guest lock");
        if !outputs.is_empty() {
            for output in outputs {
                guest.deliver(output);
            }
            shell.request_redraw();
        }
        let Event::Window(window::Event::RedrawRequested(now)) = event else {
            return;
        };
        // Before any tick, so a guest subscribing in its `on mount` is
        // answered with the mode the window already has.
        guest.set_theme(*now, self.dark);
        // Not every redraw of the window is a tick of the guest: one with
        // nothing to deliver is left alone.
        let wake = guest.redraw(*now);
        if let Some(at) = wake.at {
            shell.request_redraw_at(at);
        }
        // A trap in that tick changed what the monitor counts, and a trap
        // publishes nothing by itself. A new tree has to be rendered, which
        // only a rebuilt view does. A bus publish has to reach the
        // subscribers in the other windows, and the store's update is what
        // redraws every window.
        let faulted = guest.fault.is_some() && !guest.announced_fault;
        if faulted {
            guest.announced_fault = true;
        }
        let changed = guest.frame_rev != self.rev;
        // The two are not the same word: an end changes what the store
        // shows, a wake changes nothing there — its update is only the
        // rebuild, so it must not cost a recompute of the rows.
        if faulted {
            shell.publish("ended".to_string());
        } else if changed || wake.published {
            shell.publish("wake".to_string());
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, String, iced::Theme, iced::Renderer>> {
        // An overlay's messages would be the guest's too; the tree carries
        // no widget that opens one.
        let _ = (tree, layout, renderer, viewport, translation);
        None
    }
}
