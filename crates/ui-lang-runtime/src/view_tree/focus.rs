//! The only native interaction state carried to a replacement is eligible focus.
use iced::advanced::widget::{Operation, operation::Focusable};
use iced::{Rectangle, widget::Id};
use std::collections::{HashMap, HashSet};
use ui_lang_wire::{Node, ToggleKind};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Input,
    Editor,
    Button,
    Toggle(ToggleKind),
    Radio,
    Slider,
    Pick,
    Combo,
}

#[derive(Clone, PartialEq)]
pub(super) struct Target {
    id: Id,
    kind: Kind,
}

#[derive(Default)]
pub(super) struct Targets {
    controls: HashMap<Id, Option<Kind>>,
    surfaces: HashSet<crate::StableId>,
}
impl Targets {
    pub fn new(root: &Node) -> Self {
        let mut targets = Self::default();
        targets.collect(root);
        targets
    }

    fn collect(&mut self, node: &Node) {
        if let Node::Surface { key, .. } = node {
            self.surfaces.insert(crate::StableId::new(key));
        }
        let control = match node {
            Node::Input { key, options, .. } if !options.disabled => Some((key, Kind::Input)),
            Node::Editor {
                key,
                editable: true,
                ..
            } => Some((key, Kind::Editor)),
            Node::Button {
                key,
                on_press: Some(_),
                ..
            } => Some((key, Kind::Button)),
            Node::Toggle {
                key,
                kind,
                on_toggle: Some(_),
                ..
            } => Some((key, Kind::Toggle(*kind))),
            Node::Radio { key, .. } => Some((key, Kind::Radio)),
            Node::Slider { key, .. } => Some((key, Kind::Slider)),
            Node::PickList { key, .. } => Some((key, Kind::Pick)),
            Node::ComboBox { key, .. } => Some((key, Kind::Combo)),
            _ => None,
        };
        if let Some((key, kind)) = control.filter(|(key, _)| !key.is_empty()) {
            // An ambiguous identity cannot authorize selecting one of its uses.
            self.controls
                .entry(Id::from(key.clone()))
                .and_modify(|kind| *kind = None)
                .or_insert(Some(kind));
        }
        for child in node.children() {
            self.collect(child);
        }
    }

    pub fn contains(&self, target: &Target) -> bool {
        self.controls.get(&target.id) == Some(&Some(target.kind))
    }

    pub fn capture(&self, mut operate: impl FnMut(&mut dyn Operation)) -> Option<Target> {
        enum Observed {
            None,
            Focused(Target),
            Ineligible,
        }
        struct Capture<'a> {
            targets: &'a Targets,
            observed: Observed,
        }
        impl Operation for Capture<'_> {
            fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
                visit(self);
            }
            fn focusable(&mut self, id: Option<&Id>, _: Rectangle, state: &mut dyn Focusable) {
                if !state.is_focused() {
                    return;
                }
                let target = id.and_then(|id| {
                    self.targets
                        .controls
                        .get(id)
                        .copied()
                        .flatten()
                        .map(|kind| Target {
                            id: id.clone(),
                            kind,
                        })
                });
                self.observed = match (&self.observed, target) {
                    (Observed::None, Some(target)) => Observed::Focused(target),
                    (Observed::Focused(old), Some(target)) if *old == target => {
                        Observed::Focused(target)
                    }
                    _ => Observed::Ineligible,
                };
            }
        }
        let mut capture = Capture {
            targets: self,
            observed: Observed::None,
        };
        operate(&mut WithoutSurfaces::new(&self.surfaces, &mut capture));
        match capture.observed {
            Observed::Focused(target) => unique(&target, &mut operate).then_some(target),
            Observed::None | Observed::Ineligible => None,
        }
    }
}

fn unique(target: &Target, operate: &mut impl FnMut(&mut dyn Operation)) -> bool {
    struct Count<'a> {
        id: &'a Id,
        matches: usize,
    }
    impl Operation for Count<'_> {
        fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
            visit(self);
        }
        fn focusable(&mut self, id: Option<&Id>, _: Rectangle, _: &mut dyn Focusable) {
            if id == Some(self.id) {
                self.matches = (self.matches + 1).min(2);
            }
        }
    }
    let mut count = Count {
        id: &target.id,
        matches: 0,
    };
    operate(&mut count);
    count.matches == 1
}

pub(super) fn restore(
    target: &Target,
    targets: &Targets,
    mut operate: impl FnMut(&mut dyn Operation),
) {
    struct Restore<'a>(&'a Id);
    impl Operation for Restore<'_> {
        fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
            visit(self);
        }
        fn focusable(&mut self, id: Option<&Id>, _: Rectangle, state: &mut dyn Focusable) {
            if id == Some(self.0) {
                state.focus();
            }
        }
    }
    if !unique(target, &mut operate) {
        return;
    }
    // Accessible controls exclude disabled state from this operation. It acts
    // solely on the replacement's freshly constructed widget state.
    operate(&mut WithoutSurfaces::new(
        &targets.surfaces,
        &mut Restore(&target.id),
    ));
}

// Accessible already marks semantic subtree boundaries. Only these private
// handoff operations omit host surfaces; normal focus and accessibility still
// traverse them. A hidden wire control never authorizes a foreign descendant.
struct WithoutSurfaces<'a> {
    surfaces: &'a HashSet<crate::StableId>,
    inner: &'a mut dyn Operation,
    excluded: bool,
}
impl<'a> WithoutSurfaces<'a> {
    fn new(surfaces: &'a HashSet<crate::StableId>, inner: &'a mut dyn Operation) -> Self {
        Self {
            surfaces,
            inner,
            excluded: false,
        }
    }
}
impl Operation for WithoutSurfaces<'_> {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        if !self.excluded {
            visit(self);
        }
    }
    fn custom(&mut self, _: Option<&Id>, _: Rectangle, state: &mut dyn std::any::Any) {
        if let Some(semantics) = state.downcast_ref::<crate::SemanticSnapshot>() {
            self.excluded = self.surfaces.contains(&semantics.id);
        } else if state.is::<crate::SemanticEnd>() {
            self.excluded = false;
        }
    }
    fn focusable(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        if !self.excluded {
            self.inner.focusable(id, bounds, state);
        }
    }
}
