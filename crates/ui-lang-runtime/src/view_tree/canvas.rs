//! Paint copied canvas geometry with the same native Iced primitives.
use iced::widget::canvas;
use iced::{Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use ui_lang_wire::{CanvasCommand as Command, CanvasSegment as Segment, CanvasShape as Shape};

pub(super) const MAX_EXPANDED_PARTS: usize = 16_384;
#[derive(Clone)]
pub(super) struct Geometry {
    commands: Vec<Command>,
    paths: Vec<Option<canvas::Path>>,
}
/// A wire frame's prepared paths share one budget, even when a responsive
/// subtree is rebuilt repeatedly during host layout.
pub(super) struct Cache {
    geometries: std::collections::HashMap<String, Geometry>,
}
impl Cache {
    pub(super) fn new(root: &ui_lang_wire::Node) -> Self {
        fn prepare(
            node: &ui_lang_wire::Node,
            budget: &std::cell::Cell<usize>,
            geometries: &mut std::collections::HashMap<String, Geometry>,
        ) {
            if let ui_lang_wire::Node::Canvas { key, commands, .. } = node {
                geometries.insert(key.clone(), Geometry::new(commands.clone(), budget));
            }
            for child in node.children() {
                prepare(child, budget, geometries);
            }
        }
        // Allocate in wire order, including hidden branches. Layout history must
        // never change which paths fit the shared frame budget.
        let mut geometries = Default::default();
        prepare(
            root,
            &std::cell::Cell::new(MAX_EXPANDED_PARTS),
            &mut geometries,
        );
        Self { geometries }
    }
    pub(super) fn get(&self, key: &str) -> Geometry {
        self.geometries
            .get(key)
            .expect("prepared wire canvas")
            .clone()
    }
}
impl Geometry {
    pub(super) fn new(commands: Vec<Command>, budget: &std::cell::Cell<usize>) -> Self {
        let mut scales = vec![1.0_f32];
        let paths = commands
            .iter()
            .filter_map(|command| match command {
                Command::Push { scale, clip, .. } => {
                    scales.push(if clip.is_some() {
                        1.0
                    } else {
                        scales.last().unwrap() * scale[0].abs().max(scale[1].abs())
                    });
                    None
                }
                Command::Pop => {
                    if scales.len() > 1 {
                        scales.pop();
                    }
                    None
                }
                Command::Draw { shape, stroke, .. } => Some(path(shape).and_then(|path| {
                    bounded_path(&path, *scales.last().unwrap(), stroke.as_ref(), budget)
                })),
            })
            .collect();
        Self { commands, paths }
    }
}
impl canvas::Program<super::Output> for Geometry {
    type State = ();
    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        paint(
            &mut frame,
            &mut self.commands.iter(),
            &mut self.paths.iter(),
            0,
        );
        vec![frame.into_geometry()]
    }
}
fn point([x, y]: [f32; 2]) -> Point {
    Point::new(x, y)
}
fn size([width, height]: [f32; 2]) -> Size {
    Size::new(width, height)
}
fn radius([top_left, top_right, bottom_right, bottom_left]: [f32; 4]) -> iced::border::Radius {
    iced::border::Radius {
        top_left,
        top_right,
        bottom_right,
        bottom_left,
    }
}
fn path(shape: &Shape) -> Option<canvas::Path> {
    let arc_starts = if let Shape::Path(segments) = shape {
        validate_arc_tangents(segments)?
    } else {
        Vec::new()
    };
    let mut arc_starts = arc_starts.into_iter();
    Some(match shape {
        Shape::Rectangle {
            position,
            size: dimensions,
            radius: corners,
        } => canvas::Path::rounded_rectangle(point(*position), size(*dimensions), radius(*corners)),
        Shape::Circle { center, radius } => canvas::Path::circle(point(*center), *radius),
        Shape::Line { from, to } => canvas::Path::line(point(*from), point(*to)),
        Shape::Path(segments) => canvas::Path::new(|path| {
            for segment in segments {
                match segment {
                    Segment::Rectangle {
                        position,
                        size: dimensions,
                        radius: corners,
                    } => path.rounded_rectangle(
                        point(*position),
                        size(*dimensions),
                        radius(*corners),
                    ),
                    Segment::Circle { center, radius } => path.circle(point(*center), *radius),
                    Segment::Move(p) => path.move_to(point(*p)),
                    Segment::Line(p) => path.line_to(point(*p)),
                    Segment::Arc {
                        center,
                        radius,
                        start,
                        end,
                    } => path.arc(canvas::path::Arc {
                        center: point(*center),
                        radius: *radius,
                        start_angle: iced::Radians(*start),
                        end_angle: iced::Radians(*end),
                    }),
                    Segment::ArcTo { a, b, radius } => {
                        // Match the validated starting point exactly, including
                        // rounding at the end of preceding curves.
                        if let Some(start) = arc_starts.next().expect("validated arc start") {
                            path.line_to(start);
                        }
                        path.arc_to(point(*a), point(*b), *radius);
                    }
                    Segment::Ellipse {
                        center,
                        radius,
                        rotation,
                        start,
                        end,
                    } => path.ellipse(canvas::path::arc::Elliptical {
                        center: point(*center),
                        radii: Vector::new(radius[0], radius[1]),
                        rotation: iced::Radians(*rotation),
                        start_angle: iced::Radians(*start),
                        end_angle: iced::Radians(*end),
                    }),
                    Segment::Bezier { a, b, end } => {
                        path.bezier_curve_to(point(*a), point(*b), point(*end))
                    }
                    Segment::Quadratic { control, end } => {
                        path.quadratic_curve_to(point(*control), point(*end))
                    }
                    Segment::Close => path.close(),
                }
            }
        }),
    })
}
fn paint(
    frame: &mut canvas::Frame,
    commands: &mut std::slice::Iter<'_, Command>,
    paths: &mut std::slice::Iter<'_, Option<canvas::Path>>,
    depth: usize,
) {
    while let Some(command) = commands.next() {
        match command {
            Command::Pop => return,
            Command::Push {
                translate,
                rotate,
                scale,
                clip,
            } => {
                if depth >= 32 {
                    return;
                }
                frame.with_save(|frame| {
                    frame.translate(Vector::new(translate[0], translate[1]));
                    frame.rotate(*rotate);
                    frame.scale_nonuniform(Vector::new(scale[0], scale[1]));
                    if let Some([x, y, width, height]) = clip {
                        frame.with_clip(
                            Rectangle {
                                x: *x,
                                y: *y,
                                width: *width,
                                height: *height,
                            },
                            |frame| paint(frame, commands, paths, depth + 1),
                        );
                    } else {
                        paint(frame, commands, paths, depth + 1);
                    }
                });
            }
            Command::Draw {
                shape: _,
                fill,
                even_odd,
                stroke,
            } => {
                let Some(Some(path)) = paths.next() else {
                    continue;
                };
                if let Some(color) = fill {
                    frame.fill(
                        path,
                        canvas::Fill {
                            style: canvas::Style::Solid(super::color(*color)),
                            rule: if *even_odd {
                                canvas::fill::Rule::EvenOdd
                            } else {
                                canvas::fill::Rule::NonZero
                            },
                        },
                    );
                }
                if let Some(stroke) = stroke {
                    use ui_lang_wire::{CanvasLineCap as Cap, CanvasLineJoin as Join};
                    frame.stroke(
                        path,
                        canvas::Stroke {
                            style: canvas::Style::Solid(super::color(stroke.color)),
                            width: stroke.width,
                            line_cap: match stroke.cap {
                                Cap::Butt => canvas::LineCap::Butt,
                                Cap::Square => canvas::LineCap::Square,
                                Cap::Round => canvas::LineCap::Round,
                            },
                            line_join: match stroke.join {
                                Join::Miter => canvas::LineJoin::Miter,
                                Join::Round => canvas::LineJoin::Round,
                                Join::Bevel => canvas::LineJoin::Bevel,
                            },
                            line_dash: canvas::LineDash {
                                segments: &stroke.dash,
                                offset: stroke.dash_offset as usize,
                            },
                        },
                    );
                }
            }
        }
    }
}

/// Flatten once, with a shared tree budget, and reuse the bounded line path.
/// This prevents the renderer from expanding the original curves again.
fn bounded_path(
    path: &canvas::Path,
    scale: f32,
    stroke: Option<&ui_lang_wire::CanvasStroke>,
    budget: &std::cell::Cell<usize>,
) -> Option<canvas::Path> {
    use canvas::path::lyon_path::{Event, iterator::PathIterator};
    let scale = scale.max(1.0);
    let min_dash = stroke
        .filter(|stroke| !stroke.dash.is_empty())
        .map(|stroke| stroke.dash.iter().copied().fold(f32::INFINITY, f32::min));
    let mut events = Vec::new();
    for event in path.raw().iter().flattened(0.1 / scale) {
        let (from, to) = match event {
            Event::Begin { at } => (at, at),
            Event::Line { from, to } => (from, to),
            Event::End { last, first, close } => (last, if close { first } else { last }),
            _ => unreachable!("flattened path has no curves"),
        };
        if ![from.x, from.y, to.x, to.y]
            .iter()
            .all(|n| n.is_finite() && n.abs() <= 1_000_000.0)
        {
            return None;
        }
        let cost = 1usize.saturating_add(min_dash.map_or(0, |dash| {
            (((to - from).length() * scale / dash).ceil() as usize).saturating_add(2)
        }));
        if cost > budget.get() {
            return None;
        }
        budget.set(budget.get() - cost);
        events.push(event);
    }
    Some(canvas::Path::new(|builder| {
        for event in events {
            match event {
                Event::Begin { at } => builder.move_to(Point::new(at.x, at.y)),
                Event::Line { to, .. } => builder.line_to(Point::new(to.x, to.y)),
                Event::End { close: true, .. } => builder.close(),
                _ => {}
            }
        }
    }))
}

/// Native arc-to computes radius / tan(angle / 2). Reject unstable derived
/// endpoints before handing them to lyon, even when all inputs are finite.
fn validate_arc_tangents(segments: &[Segment]) -> Option<Vec<Option<Point>>> {
    let mut starts = Vec::new();
    use canvas::path::lyon_path::math::point as p;
    let mut current = p(0.0, 0.0);
    let mut first = current;
    let mut empty = true;
    let ellipse = |center: [f32; 2], radii: [f32; 2], rotation: f32, angle: f32| {
        let x = radii[0] * angle.cos();
        let y = radii[1] * angle.sin();
        p(
            center[0] + x * rotation.cos() - y * rotation.sin(),
            center[1] + x * rotation.sin() + y * rotation.cos(),
        )
    };
    for segment in segments {
        match *segment {
            Segment::Move([x, y]) => {
                current = p(x, y);
                first = current;
            }
            Segment::Line([x, y])
            | Segment::Bezier { end: [x, y], .. }
            | Segment::Quadratic { end: [x, y], .. } => current = p(x, y),
            Segment::Close => current = first,
            Segment::Rectangle {
                position,
                size,
                radius,
            } => {
                current = p(
                    position[0] + radius[0].min(size[0].min(size[1]) / 2.0),
                    position[1],
                );
                first = current;
            }
            Segment::Circle { center, radius } => {
                current = p(center[0] + radius, center[1]);
                first = current;
            }
            Segment::Arc {
                center,
                radius,
                start,
                end,
            } => {
                first = ellipse(center, [radius; 2], 0.0, start);
                current = ellipse(
                    center,
                    [radius; 2],
                    0.0,
                    start + (end - start).clamp(-std::f32::consts::TAU, std::f32::consts::TAU),
                );
            }
            Segment::Ellipse {
                center,
                radius,
                rotation,
                start,
                end,
            } => {
                first = ellipse(center, radius, rotation, start);
                current = ellipse(
                    center,
                    radius,
                    rotation,
                    start + (end - start).clamp(-std::f32::consts::TAU, std::f32::consts::TAU),
                );
            }
            Segment::ArcTo { a, b, radius } => {
                starts.push((!empty).then_some(Point::new(current.x, current.y)));
                let mid = p(a[0], a[1]);
                let end = p(b[0], b[1]);
                let area = current.x * (mid.y - end.y)
                    + mid.x * (end.y - current.y)
                    + end.x * (current.y - mid.y);
                if current == mid || mid == end || radius == 0.0 || area == 0.0 {
                    current = mid;
                    if empty {
                        first = current;
                        empty = false;
                    }
                    continue;
                }
                let u = (current - mid).normalize();
                let v = (end - mid).normalize();
                let distance = radius / (u.dot(v).acos() / 2.0).tan();
                let start = mid + u * distance;
                if empty {
                    first = start;
                }
                current = mid + v * distance;
                if ![start.x, start.y, current.x, current.y]
                    .iter()
                    .all(|n| n.is_finite() && n.abs() <= 1_000_000.0)
                {
                    return None;
                }
            }
        }
        if empty
            && !matches!(
                segment,
                Segment::ArcTo { .. } | Segment::Arc { .. } | Segment::Ellipse { .. }
            )
        {
            first = current;
        }
        if !matches!(segment, Segment::Close) {
            empty = false;
        }
    }
    Some(starts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_lang_wire::{CanvasLineCap, CanvasLineJoin, CanvasStroke, Rgba};
    #[test]
    fn alternate_canvases_have_the_same_budget_after_either_resize_history() {
        use ui_lang_wire::{ContainerQuery, Node, QueryOp};
        let branch = |key: &str| Node::When {
            key: format!("when-{key}"),
            condition: ContainerQuery {
                ops: vec![QueryOp::Bool(true)],
            },
            children: vec![Node::Canvas {
                key: key.into(),
                width: None,
                height: None,
                commands: vec![Command::Draw {
                    shape: Shape::Line {
                        from: [0.0, 0.0],
                        to: [8192.0, 0.0],
                    },
                    even_odd: false,
                    fill: None,
                    stroke: Some(CanvasStroke {
                        color: Rgba([1.0; 4]),
                        width: 1.0,
                        cap: CanvasLineCap::Butt,
                        join: CanvasLineJoin::Miter,
                        dash: vec![1.0, 1.0],
                        dash_offset: 0,
                    }),
                }],
            }],
        };
        let root = Node::When {
            key: "root".into(),
            condition: ContainerQuery {
                ops: vec![QueryOp::Bool(true)],
            },
            children: vec![branch("wide"), branch("narrow")],
        };
        let wide_first = Cache::new(&root);
        let narrow_first = Cache::new(&root);
        let wide = wide_first.get("wide").paths[0].is_some();
        let narrow = wide_first.get("narrow").paths[0].is_some();
        assert!(wide, "first wire canvas fits its near-limit path");
        assert!(!narrow, "combined paths exceed the shared frame budget");
        assert_eq!(
            narrow_first.get("narrow").paths[0].is_some(),
            narrow,
            "initial narrow layout must match wide then narrow"
        );
        assert_eq!(
            narrow_first.get("wide").paths[0].is_some(),
            wide,
            "narrow then wide must match initial wide layout"
        );
        assert_eq!(
            wide_first.get("wide").paths[0].is_some(),
            wide,
            "wide then narrow then wide must reuse its preparation"
        );
    }
    #[test]
    fn tiny_dashes_on_a_long_line_are_refused_before_native_expansion() {
        let line = canvas::Path::line(Point::ORIGIN, Point::new(8192.0, 0.0));
        let stroke = CanvasStroke {
            color: Rgba([1.0; 4]),
            width: 1.0,
            cap: CanvasLineCap::Round,
            join: CanvasLineJoin::Miter,
            dash: vec![0.01, 0.01],
            dash_offset: 0,
        };
        let budget = std::cell::Cell::new(MAX_EXPANDED_PARTS);
        assert!(bounded_path(&line, 1.0, Some(&stroke), &budget).is_none());
    }
    #[test]
    fn expansion_budget_is_shared_and_curves_are_replaced_by_lines() {
        use canvas::path::lyon_path::Event;
        let circle = canvas::Path::circle(Point::new(20.0, 20.0), 10.0);
        let budget = std::cell::Cell::new(MAX_EXPANDED_PARTS);
        let bounded = bounded_path(&circle, 1.0, None, &budget).unwrap();
        assert!(
            bounded
                .raw()
                .iter()
                .all(|e| !matches!(e, Event::Quadratic { .. } | Event::Cubic { .. }))
        );
        let used = MAX_EXPANDED_PARTS - budget.get();
        assert!(used > 4);
        budget.set(used * 2 - 1);
        assert!(bounded_path(&circle, 1.0, None, &budget).is_some());
        assert!(bounded_path(&circle, 1.0, None, &budget).is_none());
    }
    #[test]
    fn near_parallel_arc_tangents_are_rejected_before_path_construction() {
        let invalid = Shape::Path(vec![
            Segment::Move([0.0, 0.0]),
            Segment::ArcTo {
                a: [1.0, 0.0],
                b: [0.0, 0.000001],
                radius: 8192.0,
            },
        ]);
        let Shape::Path(segments) = &invalid else {
            unreachable!()
        };
        assert!(validate_arc_tangents(segments).is_none());
        let valid = Shape::Path(vec![
            Segment::Move([0.0, 0.0]),
            Segment::ArcTo {
                a: [10.0, 0.0],
                b: [10.0, 10.0],
                radius: 2.0,
            },
        ]);
        assert!(path(&valid).is_some());
    }
}

#[cfg(test)]
mod implicit_path_tests {
    use super::*;
    #[test]
    fn implicit_subpath_close_keeps_the_native_arc_start() {
        let prefix = canvas::Path::new(|p| {
            p.line_to(Point::new(10.0, 10.0));
            p.line_to(Point::new(20.0, 10.0));
            p.close();
        });
        let canvas::path::lyon_path::Event::End { first, .. } = prefix.raw().iter().last().unwrap()
        else {
            panic!("closed path")
        };
        let starts = validate_arc_tangents(&[
            Segment::Line([10.0, 10.0]),
            Segment::Line([20.0, 10.0]),
            Segment::Close,
            Segment::ArcTo {
                a: [10.0, 20.0],
                b: [20.0, 20.0],
                radius: 2.0,
            },
        ])
        .unwrap();
        assert_eq!(starts, vec![Some(Point::new(first.x, first.y))]);
    }
    #[test]
    fn initial_arc_to_does_not_gain_an_origin_connector() {
        use canvas::path::lyon_path::Event;
        let native =
            canvas::Path::new(|p| p.arc_to(Point::new(10.0, 0.0), Point::new(10.0, 10.0), 2.0));
        let copied = path(&Shape::Path(vec![Segment::ArcTo {
            a: [10.0, 0.0],
            b: [10.0, 10.0],
            radius: 2.0,
        }]))
        .unwrap();
        let Some(Event::Begin { at }) = native.raw().iter().next() else {
            panic!("native begin")
        };
        assert_eq!(copied.raw().iter().next(), Some(Event::Begin { at }));
    }
}
