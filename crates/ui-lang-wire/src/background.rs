//! Copied container backgrounds; gradients retain the native eight-stop limit.
use super::{Rgba, bound_color, finite};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorStop {
    pub offset: f32,
    pub color: Rgba,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Background {
    Color(Rgba),
    Linear {
        angle: f32,
        stops: [Option<ColorStop>; 8],
    },
}
impl Background {
    pub(crate) fn sanitize(&mut self) {
        match self {
            Self::Color(color) => sanitize_color(color),
            Self::Linear { angle, stops } => {
                *angle = finite(*angle);
                let mut previous = None;
                for slot in stops {
                    let Some(stop) = slot else { continue };
                    if !stop.offset.is_finite()
                        || !(0.0..=1.0).contains(&stop.offset)
                        || previous.is_some_and(|offset| stop.offset <= offset)
                    {
                        *slot = None;
                    } else {
                        previous = Some(stop.offset);
                        sanitize_color(&mut stop.color);
                    }
                }
            }
        }
    }
}
fn sanitize_color(color: &mut Rgba) {
    let mut bounded = Some(*color);
    bound_color(&mut bounded);
    *color = bounded.unwrap();
}

#[cfg(feature = "iced")]
impl From<iced_core::Background> for Background {
    fn from(value: iced_core::Background) -> Self {
        let rgba = |color: iced_core::Color| Rgba([color.r, color.g, color.b, color.a]);
        match value {
            iced_core::Background::Color(color) => Self::Color(rgba(color)),
            iced_core::Background::Gradient(iced_core::Gradient::Linear(linear)) => Self::Linear {
                angle: linear.angle.0,
                stops: linear.stops.map(|stop| {
                    stop.map(|stop| ColorStop {
                        offset: stop.offset,
                        color: rgba(stop.color),
                    })
                }),
            },
        }
    }
}
