//! Transfer scroll positions into fresh native state without retaining guest widgets.
use super::focus::WithoutSurfaces;
use iced::advanced::widget::{Operation, operation::Scrollable};
use iced::{Rectangle, Vector, widget::Id};
use std::collections::{HashMap, HashSet};
use ui_lang_wire::{Node, ScrollAnchor, ScrollDirection};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Kind(ScrollDirection, ScrollAnchor, ScrollAnchor);

#[derive(Clone, Copy)]
pub(super) struct Position {
    kind: Kind,
    offset: Vector,
}
pub(super) type Positions = HashMap<Id, Position>;

#[derive(Debug, Default)]
pub(super) struct Targets {
    scrolls: HashMap<Id, Option<Kind>>,
    surfaces: HashSet<crate::StableId>,
}
impl Targets {
    pub fn new(root: &Node) -> Self {
        fn collect(node: &Node, targets: &mut Targets) {
            if let Node::Surface { key, .. } = node {
                targets.surfaces.insert(crate::StableId::new(key));
            }
            if let Node::Scroll {
                key,
                direction,
                anchor_x,
                anchor_y,
                ..
            } = node
                && !key.is_empty()
            {
                targets
                    .scrolls
                    .entry(Id::from(key.clone()))
                    .and_modify(|kind| *kind = None)
                    .or_insert(Some(Kind(*direction, *anchor_x, *anchor_y)));
            }
            for child in node.children() {
                collect(child, targets);
            }
        }
        let mut targets = Self::default();
        collect(root, &mut targets);
        targets
    }

    pub fn capture(&self, mut operate: impl FnMut(&mut dyn Operation)) -> Positions {
        if self.scrolls.is_empty() {
            return Positions::new();
        }
        struct Capture<'a> {
            targets: &'a Targets,
            positions: Positions,
        }
        impl Operation for Capture<'_> {
            fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
                visit(self);
            }
            fn scrollable(
                &mut self,
                id: Option<&Id>,
                bounds: Rectangle,
                content: Rectangle,
                translation: Vector,
                _: &mut dyn Scrollable,
            ) {
                if let Some(id) = id
                    && let Some(Some(kind)) = self.targets.scrolls.get(id)
                {
                    let offset = Vector::new(
                        anchored(translation.x, bounds.width, content.width, kind.1),
                        anchored(translation.y, bounds.height, content.height, kind.2),
                    );
                    if offset.x.is_finite() && offset.y.is_finite() {
                        self.positions.insert(
                            id.clone(),
                            Position {
                                kind: *kind,
                                offset,
                            },
                        );
                    }
                }
            }
        }
        let mut capture = Capture {
            targets: self,
            positions: Positions::new(),
        };
        operate(&mut WithoutSurfaces::new(&self.surfaces, &mut capture));
        // Count the complete native tree too: a host surface must not collide
        // with an eligible guest ID, even when excluded from capture.
        struct Count<'a> {
            counts: HashMap<&'a Id, usize>,
        }
        impl Operation for Count<'_> {
            fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
                visit(self);
            }
            fn scrollable(
                &mut self,
                id: Option<&Id>,
                _: Rectangle,
                _: Rectangle,
                _: Vector,
                _: &mut dyn Scrollable,
            ) {
                if let Some(id) = id
                    && let Some(count) = self.counts.get_mut(id)
                {
                    *count += 1;
                }
            }
        }
        let mut count = Count {
            counts: capture.positions.keys().map(|id| (id, 0)).collect(),
        };
        operate(&mut count);
        let unique: HashSet<_> = count
            .counts
            .into_iter()
            .filter_map(|(id, count)| (count == 1).then_some(id.clone()))
            .collect();
        capture.positions.retain(|id, _| unique.contains(id));
        capture.positions
    }

    pub fn restore(&self, saved: Positions, mut operate: impl FnMut(&mut dyn Operation)) {
        if saved.is_empty() {
            return;
        }
        let current = self.capture(&mut operate);
        let eligible: Positions = saved
            .into_iter()
            .filter(|(id, old)| current.get(id).is_some_and(|new| new.kind == old.kind))
            .collect();
        struct Restore(Positions);
        impl Operation for Restore {
            fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
                visit(self);
            }
            fn scrollable(
                &mut self,
                id: Option<&Id>,
                bounds: Rectangle,
                content: Rectangle,
                _: Vector,
                state: &mut dyn Scrollable,
            ) {
                if let Some(position) = id.and_then(|id| self.0.get(id)) {
                    state.scroll_to(
                        iced::advanced::widget::operation::scrollable::AbsoluteOffset {
                            x: Some(
                                position
                                    .offset
                                    .x
                                    .clamp(0.0, (content.width - bounds.width).max(0.0)),
                            ),
                            y: Some(
                                position
                                    .offset
                                    .y
                                    .clamp(0.0, (content.height - bounds.height).max(0.0)),
                            ),
                        },
                    );
                }
            }
        }
        operate(&mut WithoutSurfaces::new(
            &self.surfaces,
            &mut Restore(eligible),
        ));
    }
}

fn anchored(translation: f32, viewport: f32, content: f32, anchor: ScrollAnchor) -> f32 {
    match anchor {
        ScrollAnchor::End => (content - viewport).max(0.0) - translation,
        ScrollAnchor::Start | ScrollAnchor::Keep => translation,
    }
}
