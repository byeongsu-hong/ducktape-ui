//! Owned lifetime adapter around Iced's actual combo box and menu overlay.
use super::Output;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Event, Length, Rectangle, Size, Vector, widget};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use super::Choice;
use ui_lang_wire as wire;
type State = widget::combo_box::State<Choice>;
pub(super) type Shared = Arc<Mutex<State>>;

#[derive(Clone, Debug)]
pub(super) struct Field {
    pub options: Vec<String>,
    pub reset: u64,
    pub state: Shared,
    query_bytes: usize,
    restored: bool,
}
impl Field {
    pub fn new(options: &[String], reset: u64) -> Self {
        Self {
            query_bytes: 0,
            restored: false,
            options: options.to_vec(),
            reset,
            state: Arc::new(Mutex::new(State::new(choices(options)))),
        }
    }
    pub fn query_bytes(&self) -> usize {
        self.query_bytes
    }
    pub fn stored_bytes(&self) -> usize {
        self.options.iter().map(String::len).sum::<usize>() + self.query_bytes
    }
    pub fn bound_input(&mut self, text: &mut String, budget: usize) {
        let len = text.len();
        let mut end = text.len().min(budget).min(wire::MAX_STRING_BYTES);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        self.query_bytes = text.len();
        if len != text.len() {
            let query = Choice(u32::MAX, text.clone());
            *self.state.lock().unwrap() =
                State::with_selection(choices(&self.options), Some(&query));
        }
    }
    pub fn adopt(&mut self, options: &[String], reset: u64) {
        if reset == self.reset
            && options.starts_with(&self.options)
            && (!self.restored || options == self.options)
        {
            let mut state = self.state.lock().unwrap();
            for (index, label) in options.iter().enumerate().skip(self.options.len()) {
                state.push(Choice(index as u32, label.clone()));
            }
        } else {
            *self.state.lock().unwrap() = State::new(choices(options));
            self.query_bytes = 0;
        }
        self.options = options.to_vec();
        self.reset = reset;
        self.restored = false;
    }
}
fn choices(options: &[String]) -> Vec<Choice> {
    options
        .iter()
        .enumerate()
        .map(|(index, label)| Choice(index as u32, label.clone()))
        .collect()
}
// App-owned native search outlives a temporarily absent widget. Bound the
// retained inventory across frames, including keys, copied options, and queries.
pub(super) fn stored_bytes(fields: &std::collections::HashMap<String, Field>) -> usize {
    fields
        .iter()
        .map(|(key, field)| key.len() + field.stored_bytes())
        .sum()
}
pub(super) fn adopt(root: &wire::Node, fields: &mut std::collections::HashMap<String, Field>) {
    if let wire::Node::ComboBox {
        state_key: key,
        options,
        reset,
        ..
    } = root
    {
        let old = fields.get(key);
        let retained_query = old
            .filter(|field| {
                field.reset == *reset
                    && options.starts_with(&field.options)
                    && (!field.restored || *options == field.options)
            })
            .map_or(0, |field| field.query_bytes);
        let old_bytes = old.map_or(0, |field| key.len() + field.stored_bytes());
        let new_bytes = key.len() + options.iter().map(String::len).sum::<usize>() + retained_query;
        let fits = (old.is_some() || fields.len() < wire::MAX_NODES)
            && stored_bytes(fields)
                .saturating_sub(old_bytes)
                .saturating_add(new_bytes)
                <= wire::MAX_TEXT_BYTES_PER_FRAME;
        if fits {
            match fields.get_mut(key) {
                Some(field) => field.adopt(options, *reset),
                None => {
                    fields.insert(key.clone(), Field::new(options, *reset));
                }
            }
        } else {
            // Never render the new option table against a retained old state.
            // `render` presents an explicit rejection in place of this combo.
            fields.remove(key);
        }
    }
    for child in root.children() {
        adopt(child, fields);
    }
}
pub(super) fn retain_restored(
    _: &wire::Node,
    fields: &mut std::collections::HashMap<String, Field>,
) {
    // Hidden App-owned bindings are checked when first shown after reload;
    // append is not enough to establish an exact restored-state match.
    for field in fields.values_mut() {
        field.restored = true;
    }
}
pub(super) fn render(
    node: &wire::Node,
    field: Option<&Field>,
) -> super::IceElement<'static, Output> {
    let wire::Node::ComboBox {
        key,
        state_key,
        options,
        selected,
        reset,
        placeholder,
        on_select,
        width,
        settings,
    } = node
    else {
        unreachable!("combo node")
    };
    let Some(field) = field.filter(|field| field.options == *options && field.reset == *reset)
    else {
        return widget::text("combo state rejected: retention budget or conflicting options")
            .into();
    };
    let state = field.state.clone();
    let selected = selected.and_then(|index| {
        options
            .get(index as usize)
            .map(|value| Choice(index, value.clone()))
    });
    super::accessible(
        iced::Element::new(HostCombo {
            status: widget::text_input::Status::Active,
            state,
            key: state_key.clone(),
            selected: selected.clone(),
            placeholder: placeholder.clone(),
            on_select: *on_select,
            width: width.map_or(Length::Fill, super::length),
            settings: settings.clone(),
        }),
        super::StableId::new(key),
        super::Role::ComboBox,
    )
    .logical_id_maybe(cfg!(test).then_some(key.as_str()))
    .label(placeholder.clone())
    .value(selected.map(|value| value.1).unwrap_or_default())
    .into()
}
struct HostCombo {
    status: widget::text_input::Status,
    state: Shared,
    key: String,
    selected: Option<Choice>,
    placeholder: String,
    on_select: u32,
    width: Length,
    settings: Box<wire::ComboOptions>,
}
impl HostCombo {
    fn build<'a>(&self, state: &'a State) -> widget::ComboBox<'a, Choice, Output> {
        let handler = self.on_select;
        let mut combo = widget::combo_box(
            state,
            &self.placeholder,
            self.selected.as_ref(),
            move |value: Choice| Output::Select {
                handler,
                index: value.0,
            },
        )
        .width(self.width);
        let options = &self.settings;
        if let Some(value) = options.padding {
            combo = combo.padding(crate::bounded_table_metric(
                f64::from(value),
                state.options().len(),
            ));
        }
        if let Some(value) = options.menu_height {
            combo = combo.menu_height(super::length(value));
        }
        if let Some(value) = options.text_size {
            combo = combo.size(value);
        }
        if let Some(value) = options.line_height {
            combo = combo.line_height(widget::text::LineHeight::Relative(value));
        }
        if let Some(value) = options.shaping {
            combo = combo.text_shaping(super::pick::shaping(value));
        }
        if let Some(value) = &options.font {
            combo = combo.font(super::text::named_font(value));
        }
        if let Some(value) = &options.icon {
            combo = combo.icon(widget::text_input::Icon {
                font: value
                    .font
                    .as_ref()
                    .map(super::text::named_font)
                    .unwrap_or_default(),
                code_point: value.code_point,
                size: value.size.map(iced::Pixels),
                spacing: value.spacing,
                side: if value.right {
                    widget::text_input::Side::Right
                } else {
                    widget::text_input::Side::Left
                },
            });
        }
        let key = self.key.clone();
        let handler = options.input;
        combo = combo.on_input(move |text| Output::ComboInput {
            key: key.clone(),
            handler,
            text,
        });
        if let Some(handler) = options.hover {
            combo = combo.on_option_hovered(move |value| Output::Select {
                handler,
                index: value.0,
            });
        }
        if let Some(handler) = options.open {
            combo = combo.on_open(Output::Activate(handler));
        }
        if let Some(handler) = options.close {
            combo = combo.on_close(Output::Activate(handler));
        }
        let style = options.style;
        let status = self.status;
        combo = combo.input_style(move |theme, _| super::input_style(style, theme, status));
        if let Some(menu) = options.menu {
            combo = combo.menu_style(move |theme| super::menu_style(menu, theme));
        }
        combo
    }
}
impl Widget<Output, iced::Theme, iced::Renderer> for HostCombo {
    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }
    fn tag(&self) -> tree::Tag {
        self.build(&self.state.lock().unwrap()).tag()
    }
    fn state(&self) -> tree::State {
        self.build(&self.state.lock().unwrap()).state()
    }
    fn children(&self) -> Vec<Tree> {
        self.build(&self.state.lock().unwrap()).children()
    }
    fn diff(&self, tree: &mut Tree) {
        self.build(&self.state.lock().unwrap()).diff(tree);
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.build(&self.state.lock().unwrap())
            .layout(tree, renderer, limits)
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
        self.build(&self.state.lock().unwrap())
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.build(&self.state.lock().unwrap())
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
        shell: &mut Shell<'_, Output>,
        viewport: &Rectangle,
    ) {
        let status = {
            let state = self.state.lock().unwrap();
            let mut combo = self.build(&state);
            combo.update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
            // The native input stays focused when its menu closes; that focus,
            // rather than menu visibility, determines the control's style.
            type Paragraph = <iced::Renderer as iced::advanced::text::Renderer>::Paragraph;
            let input = tree.children[0]
                .state
                .downcast_ref::<widget::text_input::State<Paragraph>>();
            if input.is_focused() {
                widget::text_input::Status::Focused {
                    is_hovered: cursor.is_over(layout.bounds()),
                }
            } else if cursor.is_over(layout.bounds()) {
                widget::text_input::Status::Hovered
            } else {
                widget::text_input::Status::Active
            }
        };
        if self.status != status {
            self.status = status;
            shell.request_redraw();
        }
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.build(&self.state.lock().unwrap())
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }
    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Output, iced::Theme, iced::Renderer>> {
        let state = self.state.lock().unwrap();
        let mut combo = self.build(&state);
        let open = combo
            .overlay(tree, layout, renderer, viewport, translation)
            .is_some();
        open.then(|| {
            overlay::Element::new(Box::new(Menu {
                combo: self,
                tree: RefCell::new(tree),
                layout,
                viewport: *viewport,
                translation,
            }))
        })
    }
}
struct Menu<'a> {
    combo: &'a HostCombo,
    tree: RefCell<&'a mut Tree>,
    layout: Layout<'a>,
    viewport: Rectangle,
    translation: Vector,
}
impl Menu<'_> {
    fn with<R>(
        &self,
        renderer: &iced::Renderer,
        f: impl FnOnce(&mut overlay::Element<'_, Output, iced::Theme, iced::Renderer>) -> R,
    ) -> Option<R> {
        let state = self.combo.state.lock().unwrap();
        let mut combo = self.combo.build(&state);
        let mut tree = self.tree.borrow_mut();
        let mut menu = combo.overlay(
            &mut tree,
            self.layout,
            renderer,
            &self.viewport,
            self.translation,
        )?;
        Some(f(&mut menu))
    }
}
impl overlay::Overlay<Output, iced::Theme, iced::Renderer> for Menu<'_> {
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        self.with(renderer, |menu| {
            menu.as_overlay_mut().layout(renderer, bounds)
        })
        .unwrap_or_else(|| layout::Node::new(Size::ZERO))
    }
    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let state = self.combo.state.lock().unwrap();
        let mut combo = self.combo.build(&state);
        let mut tree = self.tree.borrow_mut();
        if let Some(menu) = combo.overlay(
            &mut tree,
            self.layout,
            renderer,
            &self.viewport,
            self.translation,
        ) {
            menu.as_overlay()
                .draw(renderer, theme, style, layout, cursor);
        }
    }
    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.with(renderer, |menu| {
            menu.as_overlay_mut().operate(layout, renderer, operation)
        });
    }
    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Output>,
    ) {
        self.with(renderer, |menu| {
            menu.as_overlay_mut()
                .update(event, layout, cursor, renderer, clipboard, shell)
        });
    }
    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.with(renderer, |menu| {
            menu.as_overlay()
                .mouse_interaction(layout, cursor, renderer)
        })
        .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::renderer::Headless;
    use iced_test::runtime::{UserInterface, user_interface};

    #[test]
    fn native_focus_drives_combo_style_and_default_width_fills() {
        let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let mut combo = HostCombo {
            status: widget::text_input::Status::Active,
            state: Arc::new(Mutex::new(State::new(choices(&["Apple".into()])))),
            key: "combo".into(),
            selected: None,
            placeholder: "Search".into(),
            on_select: 1,
            width: Length::Fill,
            settings: Box::default(),
        };
        let mut tree = Tree::new(&combo as &dyn Widget<Output, iced::Theme, iced::Renderer>);
        let bounds = Rectangle::with_size(Size::new(300.0, 200.0));
        let layout_node = combo.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds.size()),
        );
        let layout = Layout::new(&layout_node);
        let mut outputs = Vec::new();
        combo.update(
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            layout,
            mouse::Cursor::Available(iced::Point::new(10.0, 10.0)),
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut outputs),
            &bounds,
        );
        assert!(
            matches!(combo.status, widget::text_input::Status::Focused { .. }),
            "native text input focus must select the focused combo style"
        );
        let mut root = node(0, &["Apple"]);
        if let wire::Node::ComboBox { width, .. } = &mut root {
            *width = None;
        }
        let field = Field::new(&["Apple".into()], 0);
        let mut element = render(&root, Some(&field));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds.size()),
        );
        assert_eq!(
            node.size().width,
            300.0,
            "absent combo width must use native Fill"
        );
    }

    #[test]
    fn owned_combo_delegates_search_and_overlay_selection() {
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        let state = Arc::new(Mutex::new(State::new(choices(&[
            "Apple".into(),
            "Berry".into(),
        ]))));
        let combo = HostCombo {
            status: widget::text_input::Status::Active,
            state,
            key: "combo".into(),
            selected: None,
            placeholder: "Search".into(),
            on_select: 1,
            width: Length::Fixed(200.0),
            settings: Default::default(),
        };
        let mut ui = UserInterface::build(
            iced::Element::new(combo),
            Size::new(300.0, 200.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let mut outputs = Vec::new();
        let point = iced::Point::new(10.0, 10.0);
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
        ui.update(
            &[Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Character("b".into()),
                modified_key: iced::keyboard::Key::Character("b".into()),
                physical_key: iced::keyboard::key::Physical::Unidentified(
                    iced::keyboard::key::NativeCode::Unidentified,
                ),
                location: iced::keyboard::Location::Standard,
                modifiers: iced::keyboard::Modifiers::empty(),
                text: Some("b".into()),
                repeat: false,
            })],
            mouse::Cursor::Available(point),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut outputs,
        );
        // Berry is the only filtered option: choosing the first menu row must
        // return Berry, not the unfiltered first option Apple.
        let point = iced::Point::new(20.0, 40.0);
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
        assert!(
            outputs.iter().any(|output| matches!(
                output,
                Output::Select {
                    handler: 1,
                    index: 1
                }
            )),
            "real filtered menu selection: {outputs:?}"
        );
    }

    type Ui = UserInterface<'static, Output, iced::Theme, iced::Renderer>;
    fn key(value: &str) -> Event {
        Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(value.into()),
            modified_key: iced::keyboard::Key::Character(value.into()),
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::empty(),
            text: Some(value.into()),
            repeat: false,
        })
    }
    fn click(ui: &mut Ui, renderer: &mut iced::Renderer, y: f32, outputs: &mut Vec<Output>) {
        let point = iced::Point::new(20.0, y);
        ui.update(
            &[
                Event::Mouse(mouse::Event::CursorMoved { position: point }),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
            renderer,
            &mut iced::advanced::clipboard::Null,
            outputs,
        );
    }
    fn node(reset: u64, options: &[&str]) -> wire::Node {
        wire::Node::ComboBox {
            key: "search".into(),
            state_key: "search".into(),
            reset,
            options: options.iter().map(|s| (*s).into()).collect(),
            selected: None,
            placeholder: "Search".into(),
            on_select: 1,
            width: Some(wire::Length::Fixed(200.0)),
            settings: Box::default(),
        }
    }
    #[test]
    fn retained_combo_inventory_bounds_new_identities_options_and_queries() {
        let mut fields = std::collections::HashMap::new();
        for i in 0..wire::MAX_NODES {
            let mut root = node(0, &[]);
            if let wire::Node::ComboBox { state_key, .. } = &mut root {
                *state_key = i.to_string();
            }
            adopt(&root, &mut fields);
        }
        assert_eq!(fields.len(), wire::MAX_NODES);
        adopt(&node(0, &["Apple"]), &mut fields);
        assert!(
            !fields.contains_key("search"),
            "a new identity must be rejected at the retained-state count cap"
        );
        let mut fields = std::collections::HashMap::new();
        let large = "a".repeat(wire::MAX_TEXT_BYTES_PER_FRAME - "search".len());
        adopt(&node(0, &[&large]), &mut fields);
        assert_eq!(stored_bytes(&fields), wire::MAX_TEXT_BYTES_PER_FRAME);
        let mut next = node(0, &["Apple"]);
        if let wire::Node::ComboBox { state_key, .. } = &mut next {
            *state_key = "another".into();
        }
        adopt(&next, &mut fields);
        assert!(
            !fields.contains_key("another"),
            "new identities must also respect the cumulative text budget"
        );
        assert!(
            fields.contains_key("search"),
            "budget refusal must not evict a different App state"
        );
        let mut inputs = super::super::Inputs::default();
        inputs.adopt(&node(0, &["Apple"]));
        let mut events = Vec::new();
        inputs.apply(
            Output::ComboInput {
                key: "search".into(),
                handler: Some(3),
                text: "é".repeat(wire::MAX_STRING_BYTES),
            },
            &mut events,
        );
        assert!(
            stored_bytes(&inputs.combos) <= wire::MAX_TEXT_BYTES_PER_FRAME,
            "search text shares the retained inventory budget"
        );
        assert!(
            matches!(&events[..], [wire::Event::Input { handler: 3, text }] if text.len() <= wire::MAX_TEXT_BYTES_PER_FRAME - "searchApple".len())
        );
    }

    #[test]
    fn search_survives_frames_and_push_but_resets_on_assignment_or_reload_mismatch() {
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .unwrap();
        for (name, next, reload, expected) in [
            ("same frame", node(0, &["Apple", "Berry"]), false, 1),
            ("hide and readd", node(0, &["Apple", "Berry"]), false, 1),
            ("push", node(0, &["Apple", "Berry", "Banana"]), false, 1),
            (
                "identical assignment",
                node(1, &["Apple", "Berry"]),
                false,
                0,
            ),
            ("matching reload", node(0, &["Apple", "Berry"]), true, 1),
            (
                "hidden matching reload",
                node(0, &["Apple", "Berry"]),
                true,
                1,
            ),
            (
                "hidden changed reload",
                node(0, &["Apple", "Berry", "Banana"]),
                true,
                0,
            ),
            (
                "changed reload options",
                node(0, &["Apple", "Berry", "Banana"]),
                true,
                0,
            ),
            (
                "changed reload revision",
                node(1, &["Apple", "Berry"]),
                true,
                0,
            ),
        ] {
            let root = node(0, &["Apple", "Berry"]);
            let mut inputs = super::super::Inputs::default();
            inputs.adopt(&root);
            let mut ui = UserInterface::build(
                render(&root, inputs.combos.get("search")),
                Size::new(300.0, 200.0),
                user_interface::Cache::default(),
                &mut renderer,
            );
            let mut outputs = Vec::new();
            click(&mut ui, &mut renderer, 10.0, &mut outputs);
            ui.update(
                &[key("b")],
                mouse::Cursor::Available(iced::Point::new(20.0, 10.0)),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut outputs,
            );
            let cache = ui.into_cache();
            if name == "hide and readd" {
                inputs.adopt(&wire::Node::Space {
                    width: None,
                    height: None,
                });
            }
            if name.starts_with("hidden") {
                inputs.adopt_after_reload(&wire::Node::Space {
                    width: None,
                    height: None,
                });
                inputs.adopt(&next);
            } else if reload {
                inputs.adopt_after_reload(&next);
            } else {
                inputs.adopt(&next);
            }
            let mut ui = UserInterface::build(
                render(&next, inputs.combos.get("search")),
                Size::new(300.0, 200.0),
                cache,
                &mut renderer,
            );
            outputs.clear();
            click(&mut ui, &mut renderer, 40.0, &mut outputs);
            assert!(
                outputs.contains(&Output::Select {
                    handler: 1,
                    index: expected
                }),
                "{name}: expected option {expected}, got {outputs:?}"
            );
        }
    }
}
