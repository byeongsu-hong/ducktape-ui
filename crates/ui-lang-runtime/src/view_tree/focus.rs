//! Eligible focus transferred into a replacement native tree.
use iced::advanced::widget::{Operation, operation::Focusable};
use iced::{Rectangle, widget::Id};
use std::collections::{HashMap, HashSet};
use ui_lang_wire::{Node, ToggleKind};

#[derive(Clone, Copy, Debug, PartialEq)]
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

#[derive(Debug, Default)]
pub(super) struct Targets {
    controls: HashMap<Id, Option<Kind>>,
    surfaces: HashSet<crate::StableId>,
}
#[derive(Clone, Debug, Default)]
pub(in crate::view_tree) struct Cache(std::sync::Arc<std::sync::Mutex<Option<Cached>>>);

#[derive(Debug)]
struct Cached {
    // None marks a host surface; controls retain their exact kind and order,
    // including duplicate keys. No hashes authorize cache reuse.
    entries: Vec<(String, Option<Kind>)>,
    targets: std::sync::Arc<Targets>,
}

fn visit(node: &Node, visitor: &mut impl FnMut(&str, Option<Kind>)) {
    if let Node::Surface { key, .. } = node {
        visitor(key, None);
    }
    if let Some((key, kind)) = control(node).filter(|(key, _)| !key.is_empty()) {
        visitor(key, Some(kind));
    }
    for child in node.children() {
        visit(child, visitor);
    }
}

impl Cache {
    #[cfg(test)]
    pub(super) fn snapshot(&self) -> Option<std::sync::Arc<Targets>> {
        self.0
            .lock()
            .unwrap()
            .as_ref()
            .map(|cached| cached.targets.clone())
    }

    pub(super) fn get(&self, root: &Node) -> std::sync::Arc<Targets> {
        let mut cached = self.0.lock().expect("focus metadata cache poisoned");
        if let Some(current) = cached.as_ref() {
            let mut entries = current.entries.iter();
            let mut matches = true;
            visit(root, &mut |key, kind| {
                matches &= entries
                    .next()
                    .is_some_and(|entry| entry.0 == key && entry.1 == kind);
            });
            if matches && entries.next().is_none() {
                return current.targets.clone();
            }
        }
        let mut entries = Vec::new();
        visit(root, &mut |key, kind| entries.push((key.to_owned(), kind)));
        let targets = std::sync::Arc::new(Targets::new(root));
        *cached = Some(Cached {
            entries,
            targets: targets.clone(),
        });
        targets
    }
}

fn control(node: &Node) -> Option<(&str, Kind)> {
    match node {
        Node::Input { key, options, .. } if !options.disabled => Some((key.as_str(), Kind::Input)),
        Node::Editor {
            key,
            editable: true,
            ..
        } => Some((key.as_str(), Kind::Editor)),
        Node::Button {
            key,
            on_press: Some(_),
            ..
        } => Some((key.as_str(), Kind::Button)),
        Node::Toggle {
            key,
            kind,
            on_toggle: Some(_),
            ..
        } => Some((key.as_str(), Kind::Toggle(*kind))),
        Node::Radio { key, .. } => Some((key.as_str(), Kind::Radio)),
        Node::Slider { key, .. } => Some((key.as_str(), Kind::Slider)),
        Node::PickList { key, .. } => Some((key.as_str(), Kind::Pick)),
        Node::ComboBox { key, .. } => Some((key.as_str(), Kind::Combo)),
        _ => None,
    }
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
        if let Some((key, kind)) = control(node).filter(|(key, _)| !key.is_empty()) {
            // An ambiguous identity cannot authorize selecting one of its uses.
            self.controls
                .entry(Id::from(key.to_owned()))
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
pub(super) struct WithoutSurfaces<'a> {
    surfaces: &'a HashSet<crate::StableId>,
    inner: &'a mut dyn Operation,
    excluded: bool,
}
impl<'a> WithoutSurfaces<'a> {
    pub(super) fn new(
        surfaces: &'a HashSet<crate::StableId>,
        inner: &'a mut dyn Operation,
    ) -> Self {
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
    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content: Rectangle,
        translation: iced::Vector,
        state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if !self.excluded {
            self.inner
                .scrollable(id, bounds, content, translation, state);
        }
    }
    fn focusable(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        if !self.excluded {
            self.inner.focusable(id, bounds, state);
        }
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::sync::Arc;

    fn input(key: &str) -> Node {
        Node::Input {
            key: key.into(),
            options: Default::default(),
            placeholder: String::new(),
            value: String::new(),
            on_input: 0,
            on_submit: None,
            width: None,
            secure: false,
            style: Box::default(),
        }
    }

    #[test]
    fn exact_focus_metadata_reuses_values_but_rejects_authority_changes() {
        let cache = Cache::default();
        let original = input("draft");
        let first = cache.get(&original);
        let target = Target {
            id: Id::from("draft"),
            kind: Kind::Input,
        };
        assert!(first.contains(&target));
        let mut edited = original.clone();
        if let Node::Input { value, .. } = &mut edited {
            *value = "edited".into();
        }
        assert!(Arc::ptr_eq(&first, &cache.get(&edited)));
        let mut disabled = original.clone();
        if let Node::Input { options, .. } = &mut disabled {
            options.disabled = true;
        }
        assert!(!cache.get(&disabled).contains(&target));
        assert!(!cache.get(&input("other")).contains(&target));
        let mut root = Node::Linear {
            key: "root".into(),
            axis: ui_lang_wire::Axis::Column,
            max_width: None,
            clip: false,
            wrap: None,
            spacing: None,
            padding: None,
            width: None,
            height: None,
            align: None,
            background: None,
            border: None,
            children: vec![original.clone(), original.clone()],
        };
        assert!(
            !cache.get(&root).contains(&target),
            "duplicate keys cannot authorize focus"
        );
        if let Node::Linear { children, .. } = &mut root {
            children.pop();
        }
        assert!(cache.get(&root).contains(&target));
        let changed_kind = Node::Radio {
            key: "draft".into(),
            label: String::new(),
            selected: false,
            on_select: 0,
            width: None,
            style: Default::default(),
        };
        assert!(!cache.get(&changed_kind).contains(&target));
        let surface = Node::Surface {
            key: "host".into(),
            name: "surface".into(),
            args: vec![],
            on_event: None,
        };
        let surfaces = cache.get(&surface);
        assert!(surfaces.surfaces.contains(&crate::StableId::new("host")));
        assert!(
            !cache
                .get(&Node::empty())
                .surfaces
                .contains(&crate::StableId::new("host"))
        );
        assert!(
            first.contains(&target),
            "mounted scope keeps its immutable authority snapshot"
        );
    }
}
