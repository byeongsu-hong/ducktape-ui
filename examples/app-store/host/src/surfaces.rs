//! What the host paints inside a guest's view: the surfaces a
//! `Node::Surface` names. The guest hands over a name and one line of text
//! and never sees a pixel; the host repaints the region on its own clock,
//! without a guest tick.

use std::sync::OnceLock;
use std::time::Duration;

use iced::widget::{canvas, column, text};
use iced::{Length, Point, Rectangle, Renderer, Theme, mouse, window};
use ui_lang_runtime::view_tree::{Output, Surfaces};

use crate::capabilities::clock;

/// The surfaces every guest window can name.
pub fn registry() -> &'static Surfaces {
    static SURFACES: OnceLock<Surfaces> = OnceLock::new();
    SURFACES.get_or_init(|| {
        let mut surfaces = Surfaces::new();
        surfaces.insert(
            "clock_face".into(),
            Box::new(|caption: &str| {
                column![
                    canvas(ClockFace).width(Length::Fill).height(Length::Fill),
                    text(caption.to_owned()).size(12),
                ]
                .align_x(iced::Alignment::Center)
                .into()
            }),
        );
        surfaces
    })
}

/// An analog UTC dial whose second hand sweeps: the host asks for its own
/// redraws, so it moves between the guest's once-a-second ticks.
struct ClockFace;

const SWEEP: Duration = Duration::from_millis(100);

impl canvas::Program<Output> for ClockFace {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        event: &canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Output>> {
        match event {
            canvas::Event::Window(window::Event::RedrawRequested(now)) => {
                Some(canvas::Action::request_redraw_at(*now + SWEEP))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let palette = theme.palette();
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = frame.center();
        let radius = bounds.width.min(bounds.height) / 2.0 - 2.0;
        frame.stroke(
            &canvas::Path::circle(center, radius),
            canvas::Stroke::default()
                .with_width(2.0)
                .with_color(palette.text),
        );
        let ms = clock::unix_ms() % 43_200_000;
        let seconds = ms as f32 / 1000.0;
        let hand = |turns: f32, length: f32, width: f32, color: iced::Color| {
            let angle = turns * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let tip = Point::new(
                center.x + angle.cos() * radius * length,
                center.y + angle.sin() * radius * length,
            );
            (
                canvas::Path::line(center, tip),
                canvas::Stroke::default()
                    .with_width(width)
                    .with_color(color),
            )
        };
        for (path, stroke) in [
            hand(seconds / 43_200.0, 0.5, 3.0, palette.text),
            hand(seconds % 3_600.0 / 3_600.0, 0.75, 2.0, palette.text),
            hand(seconds % 60.0 / 60.0, 0.9, 1.0, palette.primary),
        ] {
            frame.stroke(&path, stroke);
        }
        vec![frame.into_geometry()]
    }
}
