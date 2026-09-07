//! The default named markdown surface owns the parsed document its widgets borrow.
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::{Element, Event, Font, Length, Rectangle, Size, Theme, mouse, widget};
use ui_lang_wire::{MarkdownDocument, SurfaceValue};

/// Register under `ice.markdown` in the embedding host's surface registry.
pub fn markdown_surface(_key: &str, args: &[SurfaceValue]) -> Element<'static, SurfaceValue> {
    let [document] = args else {
        return widget::text("invalid markdown arguments").into();
    };
    let Some(document) = MarkdownDocument::from_surface_value(document) else {
        return widget::text("invalid markdown document").into();
    };
    // Iced's cache is local to this widget Tree. Unmount drops the document;
    // an unrelated guest frame does not reparse or reset link/scroll state.
    let dependency = ui_lang_wire::encode(&args);
    widget::lazy(dependency, move |_| -> Element<'static, SurfaceValue> {
        match HostMarkdown::new(document.clone()) {
            Ok(markdown) => Element::new(markdown),
            Err(error) => widget::text(error).into(),
        }
    })
    .into()
}

const MAX_MARKDOWN_DEPTH: usize = 32;
const MAX_MARKDOWN_EVENTS: usize = 4096;

fn source_is_bounded(source: &str) -> bool {
    use pulldown_cmark::{Event, Options, Parser};
    let options = Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;
    let mut depth = 0;
    for (count, event) in Parser::new_ext(source, options).enumerate() {
        if count >= MAX_MARKDOWN_EVENTS {
            return false;
        }
        match event {
            Event::Start(_) => {
                depth += 1;
                if depth > MAX_MARKDOWN_DEPTH {
                    return false;
                }
            }
            Event::End(_) => depth -= 1,
            _ => {}
        }
    }
    true
}

#[ouroboros::self_referencing]
struct ParsedMarkdown {
    content: widget::markdown::Content,
    #[borrows(content)]
    #[not_covariant]
    element: Element<'this, SurfaceValue>,
}

struct HostMarkdown {
    parsed: ParsedMarkdown,
    theme: Theme,
}
impl HostMarkdown {
    fn new(document: MarkdownDocument) -> Result<Self, &'static str> {
        if !source_is_bounded(&document.source) {
            return Err("markdown document exceeds structural limits");
        }
        let color = |[r, g, b, a]: [f32; 4]| iced::Color { r, g, b, a };
        let [background, text, primary, success, warning, danger] = document.palette.map(color);
        let theme = Theme::custom(
            String::from("Guest markdown"),
            iced::theme::Palette {
                background,
                text,
                primary,
                success,
                warning,
                danger,
            },
        );
        let mut settings = widget::markdown::Settings::from(&theme);
        let [body, h1, h2, h3, h4, h5, h6, code, spacing] = document.metrics.map(iced::Pixels);
        settings.text_size = body;
        settings.h1_size = h1;
        settings.h2_size = h2;
        settings.h3_size = h3;
        settings.h4_size = h4;
        settings.h5_size = h5;
        settings.h6_size = h6;
        settings.code_size = code;
        settings.spacing = spacing;
        let [body, inline, code] = document
            .monospace
            .map(|mono| if mono { Font::MONOSPACE } else { Font::DEFAULT });
        settings.style.font = body;
        settings.style.inline_code_font = inline;
        settings.style.code_block_font = code;
        let [background, text, link, border] = document.colors.map(color);
        settings.style.inline_code_highlight.background = background.into();
        settings.style.inline_code_color = text;
        settings.style.link_color = link;
        let [top, right, bottom, left] = document.padding;
        settings.style.inline_code_padding = iced::Padding {
            top,
            right,
            bottom,
            left,
        };
        let [top_left, top_right, bottom_right, bottom_left] = document.radii;
        settings.style.inline_code_highlight.border = iced::Border {
            color: border,
            width: document.border_width,
            radius: iced::border::Radius {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            },
        };
        let parsed = ParsedMarkdownBuilder {
            content: widget::markdown::Content::parse(&document.source),
            element_builder: |content| {
                widget::markdown::view(content.items(), settings).map(SurfaceValue::Str)
            },
        }
        .build();
        Ok(Self { parsed, theme })
    }
}
impl Widget<SurfaceValue, Theme, iced::Renderer> for HostMarkdown {
    fn tag(&self) -> tree::Tag {
        self.parsed
            .with_element(|element| element.as_widget().tag())
    }
    fn state(&self) -> tree::State {
        self.parsed
            .with_element(|element| element.as_widget().state())
    }
    fn children(&self) -> Vec<Tree> {
        self.parsed
            .with_element(|element| element.as_widget().children())
    }
    fn diff(&self, tree: &mut Tree) {
        self.parsed
            .with_element(|element| element.as_widget().diff(tree));
    }
    fn size(&self) -> Size<Length> {
        self.parsed
            .with_element(|element| element.as_widget().size())
    }
    fn size_hint(&self) -> Size<Length> {
        self.parsed
            .with_element(|element| element.as_widget().size_hint())
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.parsed
            .with_element_mut(|element| element.as_widget_mut().layout(tree, renderer, limits))
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let style = renderer::Style {
            text_color: self.theme.palette().text,
        };
        self.parsed.with_element(|element| {
            element.as_widget().draw(
                tree,
                renderer,
                &self.theme,
                &style,
                layout,
                cursor,
                viewport,
            )
        });
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.parsed.with_element_mut(|element| {
            element
                .as_widget_mut()
                .operate(tree, layout, renderer, operation)
        });
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, SurfaceValue>,
        viewport: &Rectangle,
    ) {
        self.parsed.with_element_mut(|element| {
            element.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            )
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
        self.parsed.with_element(|element| {
            element
                .as_widget()
                .mouse_interaction(tree, layout, cursor, viewport, renderer)
        })
    }
    // DefaultViewer uses text, rules, checkboxes, containers and scrollables;
    // none creates an overlay. Custom viewer providers keep their own Element,
    // whose overlays are forwarded by the ordinary Surface rendering path.
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::renderer::Headless;
    use iced::{Color, Pixels, Point, window};
    use iced_test::runtime::{UserInterface, user_interface};

    fn document(source: &str) -> MarkdownDocument {
        MarkdownDocument {
            source: source.into(),
            palette: [
                [1.0; 4],
                [1.0, 0.0, 0.0, 1.0],
                [0.0, 1.0, 0.0, 1.0],
                [0.0, 1.0, 0.0, 1.0],
                [1.0, 1.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 1.0],
            ],
            metrics: [18.0, 32.0, 28.0, 24.0, 20.0, 18.0, 16.0, 14.0, 8.0],
            monospace: [false, true, true],
            colors: [
                [0.9, 0.9, 0.9, 1.0],
                [0.0, 0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0, 1.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            padding: [2.0; 4],
            border_width: 1.0,
            radii: [3.0; 4],
        }
    }
    fn renderer() -> iced::Renderer {
        iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer")
    }

    #[test]
    fn markdown_links_keep_hover_state_and_return_the_clicked_uri() {
        let mut renderer = renderer();
        let element = markdown_surface(
            "doc",
            &[document("[Open](duck://docs/start)").into_surface_value()],
        );
        let mut ui = UserInterface::build(
            element,
            Size::new(200.0, 80.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        let cursor = mouse::Cursor::Available(Point::new(10.0, 8.0));
        let redraw = Event::Window(window::Event::RedrawRequested(std::time::Instant::now()));
        let mut messages = vec![];
        let mut clipboard = iced::advanced::clipboard::Null;
        ui.update(
            std::slice::from_ref(&redraw),
            cursor,
            &mut renderer,
            &mut clipboard,
            &mut messages,
        );
        let (state, _) = ui.update(
            &[redraw],
            cursor,
            &mut renderer,
            &mut clipboard,
            &mut messages,
        );
        assert!(
            matches!(
                state,
                user_interface::State::Updated {
                    mouse_interaction: mouse::Interaction::Pointer,
                    redraw_request: window::RedrawRequest::Wait,
                    ..
                }
            ),
            "stable link hover must retain its pointer without a redraw loop: {state:?}"
        );
        let cache = ui.into_cache();
        let element = markdown_surface(
            "doc",
            &[document("[Open](duck://docs/start)").into_surface_value()],
        );
        let mut ui = UserInterface::build(element, Size::new(200.0, 80.0), cache, &mut renderer);
        let (state, _) = ui.update(
            &[Event::Window(window::Event::RedrawRequested(
                std::time::Instant::now(),
            ))],
            cursor,
            &mut renderer,
            &mut clipboard,
            &mut messages,
        );
        assert!(
            matches!(
                state,
                user_interface::State::Updated {
                    mouse_interaction: mouse::Interaction::Pointer,
                    redraw_request: window::RedrawRequest::Wait,
                    ..
                }
            ),
            "an unchanged document must reuse its native element: {state:?}"
        );
        ui.update(
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            cursor,
            &mut renderer,
            &mut clipboard,
            &mut messages,
        );
        assert_eq!(
            messages,
            vec![SurfaceValue::Str("duck://docs/start".into())]
        );
        let cache = ui.into_cache();
        let element = markdown_surface(
            "doc",
            &[document("[Next](duck://docs/next)").into_surface_value()],
        );
        let mut ui = UserInterface::build(element, Size::new(200.0, 80.0), cache, &mut renderer);
        messages.clear();
        ui.update(
            &[
                Event::Window(window::Event::RedrawRequested(std::time::Instant::now())),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            cursor,
            &mut renderer,
            &mut clipboard,
            &mut messages,
        );
        assert_eq!(
            messages,
            vec![SurfaceValue::Str("duck://docs/next".into())],
            "changed source must replace the cached link destination"
        );
    }

    #[test]
    fn markdown_body_uses_guest_text_color_in_the_rendered_pixels() {
        let mut renderer = renderer();
        let element = markdown_surface("doc", &[document("Visible").into_surface_value()]);
        let mut ui = UserInterface::build(
            element,
            Size::new(200.0, 80.0),
            user_interface::Cache::default(),
            &mut renderer,
        );
        ui.draw(
            &mut renderer,
            &Theme::Light,
            &renderer::Style {
                text_color: Color::from_rgb(0.0, 0.0, 1.0),
            },
            mouse::Cursor::Unavailable,
        );
        let pixels = renderer.screenshot(Size::new(200, 80), 1.0, Color::WHITE);
        assert!(
            pixels
                .chunks_exact(4)
                .any(|p| p[0] > 180 && p[1] < 100 && p[2] < 100),
            "body glyphs must be painted in the guest's red text color"
        );
        assert!(
            !pixels
                .chunks_exact(4)
                .any(|p| p[2] > 180 && p[0] < 100 && p[1] < 100),
            "host blue text must not leak into the document"
        );
    }

    #[test]
    fn markdown_structure_is_bounded_before_native_parsing() {
        assert!(source_is_bounded(
            "# Title\n\n> A [link](duck://docs)\n\n- [x] Done\n"
        ));
        assert!(
            !source_is_bounded(&format!("{}x", "> ".repeat(MAX_MARKDOWN_DEPTH + 1))),
            "deep quotes must be rejected before recursive native construction"
        );
        assert!(
            HostMarkdown::new(document(&format!(
                "{}x",
                "> ".repeat(MAX_MARKDOWN_DEPTH + 1)
            )))
            .is_err(),
            "native construction must enforce structural limits"
        );
        assert!(
            !source_is_bounded(&"# a\n\n".repeat(MAX_MARKDOWN_EVENTS)),
            "many shallow blocks must also be bounded"
        );
    }
}
