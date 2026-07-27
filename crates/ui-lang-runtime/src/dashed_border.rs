//! A dashed border, drawn as a canvas stroke layered over a surface.
//!
//! `iced::Border` is colour, width and radius only — it carries no dash
//! pattern — so a dashed border cannot be part of the quad a `container`
//! draws. `box border-dash=(..)` therefore lowers to this helper: the surface
//! keeps its background, radius and shadow, its *solid* border is dropped, and
//! the dashed stroke is drawn on top of the same rectangle. Layout is
//! untouched: the surface is the base layer of a stack, so the stack takes its
//! size, and the stroke only fills what the surface already measured.
use iced::border::Radius;
use iced::widget::canvas;
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

/// Layers a dashed rounded-rectangle stroke over `content`.
///
/// `segments` is the on/off dash pattern in pixels, and `radius` is the
/// surface's own corner radius; the stroke is inset by half of `width` so it
/// lands inside the surface bounds, the way a CSS border does.
pub fn dashed_border<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    color: Color,
    width: f32,
    radius: Radius,
    segments: Vec<f32>,
) -> Element<'a, Message> {
    let stroke = canvas(DashedBorder {
        color,
        width,
        radius,
        segments,
    })
    .width(Length::Fill)
    .height(Length::Fill);
    iced::widget::stack([content.into(), Element::from(stroke)]).into()
}

struct DashedBorder {
    color: Color,
    width: f32,
    radius: Radius,
    segments: Vec<f32>,
}

impl<Message> canvas::Program<Message> for DashedBorder {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let inset = self.width / 2.0;
        let path = canvas::Path::rounded_rectangle(
            Point::new(inset, inset),
            Size::new(
                (bounds.width - self.width).max(0.0),
                (bounds.height - self.width).max(0.0),
            ),
            inner_radius(self.radius, inset),
        );
        frame.stroke(
            &path,
            canvas::Stroke {
                style: canvas::Style::Solid(self.color),
                width: self.width,
                line_dash: canvas::LineDash {
                    segments: &self.segments,
                    offset: 0,
                },
                ..canvas::Stroke::default()
            },
        );
        vec![frame.into_geometry()]
    }
}

/// Keeps the stroke concentric with the surface it traces: an inset path needs
/// its corners tightened by the same inset, exactly like a CSS inner border.
fn inner_radius(radius: Radius, inset: f32) -> Radius {
    Radius {
        top_left: (radius.top_left - inset).max(0.0),
        top_right: (radius.top_right - inset).max(0.0),
        bottom_right: (radius.bottom_right - inset).max(0.0),
        bottom_left: (radius.bottom_left - inset).max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tightens_corners_by_the_inset_and_never_goes_negative() {
        let radius = inner_radius(
            Radius {
                top_left: 8.0,
                top_right: 8.0,
                bottom_right: 1.0,
                bottom_left: 0.0,
            },
            2.0,
        );
        assert_eq!(radius.top_left, 6.0);
        assert_eq!(radius.bottom_right, 0.0);
        assert_eq!(radius.bottom_left, 0.0);
    }
}
