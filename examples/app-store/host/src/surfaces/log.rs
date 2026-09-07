//! A host-owned log session with an independently retained native view per mount.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::{Element, Event, Length, Rectangle, Size, mouse, widget, window};
use ui_lang_runtime::{
    LogTimelineEvent, LogTimelineState, VirtualListConfig, VirtualListId, log_timeline,
};
use ui_lang_wire::SurfaceValue as Value;

#[derive(Clone)]
struct Row {
    id: u64,
    text: String,
}
#[derive(Default)]
struct Data {
    next: u64,
    rows: Vec<Row>,
}
#[derive(Default)]
pub(crate) struct Session(Mutex<Data>);
impl Session {
    pub(crate) fn append(&self, text: &str) {
        let mut data = self.0.lock().expect("log session");
        data.next += 1;
        let id = data.next;
        if data.rows.len() == 256 {
            data.rows.remove(0);
        }
        data.rows.push(Row {
            id,
            text: text.chars().take(512).collect(),
        });
    }
}

pub(super) fn provider(session: Arc<Session>) -> ui_lang_runtime::view_tree::Surface {
    // A registry is an instance boundary even when two registries bind the same
    // backing session. This identity never crosses to the guest.
    static NEXT_SCOPE: AtomicU64 = AtomicU64::new(1);
    let scope = NEXT_SCOPE.fetch_add(1, Ordering::Relaxed);
    Box::new(move |key, args| {
        if !args.is_empty() {
            return widget::text("invalid session_log arguments").into();
        }
        let session = session.clone();
        let content = widget::lazy(
            (scope, key.to_owned()),
            move |(_, key)| -> Element<'static, Value> {
                Element::new(LogView::new(session.clone(), format!("{scope}/{key}")))
            },
        );
        widget::container(content).width(Length::Fill).into()
    })
}

#[ouroboros::self_referencing]
struct RenderedLog {
    rows: Vec<Row>,
    #[borrows(rows)]
    #[not_covariant]
    element: Element<'this, LogTimelineEvent<u64>>,
}
fn render(rows: Vec<Row>, timeline: &LogTimelineState<u64>) -> RenderedLog {
    RenderedLogBuilder {
        rows,
        element_builder: |rows| {
            widget::container(widget::column![
                widget::button("Resume tail").on_press(LogTimelineEvent::ResumeTail),
                log_timeline(
                    timeline,
                    rows,
                    config(),
                    "Session log",
                    |row| row.id,
                    |row| row.text.clone(),
                    |_, row, selected| widget::text(if selected {
                        format!("> {}", row.text)
                    } else {
                        row.text.clone()
                    })
                    .size(14)
                    .width(Length::Fill)
                    .wrapping(iced::advanced::text::Wrapping::None)
                    .into(),
                    std::convert::identity
                ),
            ])
            .height(240)
            .width(Length::Fill)
            .into()
        },
    }
    .build()
}
struct LogView {
    session: Arc<Session>,
    timeline: LogTimelineState<u64>,
    rendered: RenderedLog,
    revision: u64,
    notice_pending: bool,
}
fn config() -> VirtualListConfig {
    VirtualListConfig::new(24.0)
        .expect("fixed row height")
        .overscan(2)
}
impl LogView {
    fn new(session: Arc<Session>, key: String) -> Self {
        let timeline = LogTimelineState::new(VirtualListId::new(key));
        let rendered = render(Vec::new(), &timeline);
        Self {
            session,
            timeline,
            rendered,
            revision: 0,
            notice_pending: true,
        }
    }
    fn sync(&mut self) -> bool {
        let data = self.session.0.lock().expect("log session");
        if data.next == self.revision {
            return false;
        }
        let old = self.rendered.borrow_rows();
        let removed = old
            .iter()
            .take_while(|row| data.rows.first().is_none_or(|first| row.id < first.id))
            .count();
        self.timeline
            .reconcile_trimmed(&data.rows, |row| row.id, removed, config())
            .expect("host log appends and evicts only its prefix");
        self.revision = data.next;
        self.rendered = render(data.rows.clone(), &self.timeline);
        self.notice_pending = true;
        true
    }
    fn rebuild(&mut self) {
        self.rendered = render(self.rendered.borrow_rows().clone(), &self.timeline);
        self.notice_pending = true;
    }
    fn notice(&self) -> Value {
        Value::Record {
            name: "LogNotice".into(),
            fields: vec![
                (
                    "selected".into(),
                    Value::I64(self.timeline.selected().copied().map_or(-1, |id| id as i64)),
                ),
                (
                    "following".into(),
                    Value::Bool(self.timeline.is_following_tail()),
                ),
                (
                    "unread".into(),
                    Value::I64(self.timeline.unread_count() as i64),
                ),
                (
                    "offset".into(),
                    Value::F64(f64::from(self.timeline.scroll_offset())),
                ),
                (
                    "rows".into(),
                    Value::I64(self.rendered.borrow_rows().len() as i64),
                ),
            ],
        }
    }
}
impl Widget<Value, iced::Theme, iced::Renderer> for LogView {
    fn tag(&self) -> tree::Tag {
        self.rendered
            .with_element(|element| element.as_widget().tag())
    }
    fn state(&self) -> tree::State {
        self.rendered
            .with_element(|element| element.as_widget().state())
    }
    fn children(&self) -> Vec<Tree> {
        self.rendered
            .with_element(|element| element.as_widget().children())
    }
    fn diff(&self, tree: &mut Tree) {
        self.rendered
            .with_element(|element| element.as_widget().diff(tree));
    }
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(240.0))
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.sync();
        self.rendered.with_element_mut(|element| {
            element.as_widget().diff(tree);
            element.as_widget_mut().layout(tree, renderer, limits)
        })
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
        self.rendered.with_element(|element| {
            element
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport)
        });
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.rendered.with_element_mut(|element| {
            element
                .as_widget_mut()
                .operate(tree, layout, renderer, operation)
        });
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.rendered.with_element(|element| {
            element
                .as_widget()
                .mouse_interaction(tree, layout, cursor, viewport, renderer)
        })
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Value>,
        viewport: &Rectangle,
    ) {
        let changed = if let Event::Window(window::Event::RedrawRequested(now)) = event {
            shell.request_redraw_at(*now + Duration::from_millis(100));
            self.sync()
        } else {
            false
        };
        if changed {
            // The existing Layout belongs to the previous row window. Relayout
            // before passing any event through the newly rendered list.
            shell.invalidate_layout();
        } else {
            let mut events = Vec::new();
            {
                let mut local = Shell::new(&mut events);
                self.rendered.with_element_mut(|element| {
                    element.as_widget_mut().update(
                        tree, event, layout, cursor, renderer, clipboard, &mut local, viewport,
                    )
                });
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
            if !events.is_empty() {
                for event in events {
                    self.timeline.apply(event, config());
                }
                self.rebuild();
                shell.invalidate_layout();
            }
        }
        if self.notice_pending {
            shell.publish(self.notice());
            self.notice_pending = false;
            shell.request_redraw();
        }
    }
}
