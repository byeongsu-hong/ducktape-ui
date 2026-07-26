use super::*;

#[derive(Debug)]
struct DrawCounts {
    quads: usize,
    primitives: usize,
    images: usize,
    text: usize,
}

fn draw_headlessly<Message>(
    element: iced::Element<'_, Message>,
    theme: &iced::Theme,
    viewport: iced::Size,
) -> DrawCounts {
    use iced::advanced::renderer::Headless as _;
    use iced::theme::Base as _;

    let mut renderer = iced_test::futures::futures::executor::block_on(iced::Renderer::new(
        iced::Font::with_name("Fira Sans"),
        iced::Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("tiny-skia headless renderer");
    let mut ui = iced_test::runtime::UserInterface::build(
        element,
        viewport,
        iced_test::runtime::user_interface::Cache::default(),
        &mut renderer,
    );
    let mut messages = Vec::new();
    let _ = ui.update(
        &[iced::Event::Window(iced::window::Event::RedrawRequested(
            iced::time::Instant::now(),
        ))],
        iced::mouse::Cursor::Unavailable,
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut messages,
    );
    ui.draw(
        &mut renderer,
        theme,
        &iced::advanced::renderer::Style {
            text_color: theme.base().text_color,
        },
        iced::mouse::Cursor::Unavailable,
    );

    let iced_test::renderer::fallback::Renderer::Secondary(renderer) = &mut renderer else {
        panic!("tiny-skia backend was not selected");
    };
    renderer.layers().iter().fold(
        DrawCounts {
            quads: 0,
            primitives: 0,
            images: 0,
            text: 0,
        },
        |counts, layer| DrawCounts {
            quads: counts.quads + layer.quads.len(),
            primitives: counts.primitives + layer.primitives.len(),
            images: counts.images + layer.images.len(),
            text: counts.text + layer.text.len(),
        },
    )
}

mod accessibility;
mod application;
mod events;
mod showcase;
mod tasks;
mod values;
mod widgets;
