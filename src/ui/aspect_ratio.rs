use iced::widget::{Responsive, container, responsive};
use iced::{Element, Length, Size};

/// Keeps content inside the largest rectangle of `ratio` that fits its parent.
///
/// The parent must bound at least one axis. The factory is called again when
/// available space changes, as required by iced's native [`Responsive`] widget.
pub fn aspect_ratio<'a, Message>(
    ratio: f32,
    content: impl Fn() -> Element<'a, Message> + 'a,
) -> Responsive<'a, Message>
where
    Message: 'a,
{
    let ratio = normalized_ratio(ratio);

    responsive(move |bounds| {
        let size = fitted_size(bounds, ratio);

        container(content())
            .width(size.width)
            .height(size.height)
            .into()
    })
    .width(Length::Shrink)
    .height(Length::Shrink)
}

fn normalized_ratio(ratio: f32) -> f32 {
    if ratio.is_finite() && ratio > 0.0 {
        ratio
    } else {
        1.0
    }
}

fn fitted_size(bounds: Size, ratio: f32) -> Size {
    let width = match (bounds.width.is_finite(), bounds.height.is_finite()) {
        (true, true) => bounds.width.min(bounds.height * ratio),
        (true, false) => bounds.width,
        (false, true) => bounds.height * ratio,
        (false, false) => 0.0,
    }
    .max(0.0);

    Size::new(width, width / ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_the_ratio_to_either_bounding_axis() {
        assert_eq!(
            fitted_size(Size::new(1600.0, 1200.0), 16.0 / 9.0),
            Size::new(1600.0, 900.0)
        );
        assert_eq!(
            fitted_size(Size::new(1600.0, 450.0), 16.0 / 9.0),
            Size::new(800.0, 450.0)
        );
    }

    #[test]
    fn invalid_ratios_fall_back_to_a_square() {
        for ratio in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(normalized_ratio(ratio), 1.0);
        }
    }
}
