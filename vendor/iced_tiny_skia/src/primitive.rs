use crate::core::Rectangle;

#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    /// A path filled with some paint.
    Fill {
        /// The frame clipping region in canvas coordinates.
        clip_bounds: Rectangle,
        /// The path to fill.
        path: tiny_skia::Path,
        /// The paint to use.
        paint: tiny_skia::Paint<'static>,
        /// The fill rule to follow.
        rule: tiny_skia::FillRule,
    },
    /// A path stroked with some paint.
    Stroke {
        /// The frame clipping region in canvas coordinates.
        clip_bounds: Rectangle,
        /// The path to stroke.
        path: tiny_skia::Path,
        /// The paint to use.
        paint: tiny_skia::Paint<'static>,
        /// The stroke settings.
        stroke: tiny_skia::Stroke,
    },
}

impl Primitive {
    /// Returns the clipping region retained from the drawing frame.
    pub fn clip_bounds(&self) -> Rectangle {
        match self {
            Self::Fill { clip_bounds, .. } | Self::Stroke { clip_bounds, .. } => *clip_bounds,
        }
    }

    /// Returns the visible bounds of the [`Primitive`].
    pub fn visible_bounds(&self) -> Rectangle {
        let (bounds, reach) = match self {
            Primitive::Fill { path, .. } => (path.bounds(), 0.0),
            // A stroke paints half its width either side of the path it
            // follows, and a mitred corner reaches further still. Without
            // that, a path with no area of its own — a horizontal rule, a
            // chart's grid line — measures as a rectangle of zero height.
            Primitive::Stroke { path, stroke, .. } => (
                path.bounds(),
                stroke.width / 2.0
                    * match stroke.line_join {
                        tiny_skia::LineJoin::Miter
                        | tiny_skia::LineJoin::MiterClip => {
                            stroke.miter_limit.max(1.0)
                        }
                        tiny_skia::LineJoin::Round
                        | tiny_skia::LineJoin::Bevel => 1.0,
                    },
            ),
        };

        Rectangle {
            x: bounds.x() - reach,
            y: bounds.y() - reach,
            width: bounds.width() + reach * 2.0,
            height: bounds.height() + reach * 2.0,
        }
    }
}
