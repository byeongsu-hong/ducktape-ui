use super::*;

trait CanonicalViewHir {
    fn view_id(&self) -> ViewId;
    fn view_origin(&self) -> OriginId;
}

macro_rules! canonical_view_hir {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl CanonicalViewHir for $ty {
                fn view_id(&self) -> ViewId {
                    self.id
                }

                fn view_origin(&self) -> OriginId {
                    self.origin
                }
            }
        )+
    };
}

canonical_view_hir!(
    ResolvedCanvas,
    ResolvedContainer,
    ResolvedLayout,
    ResolvedText,
    ResolvedInput,
    ResolvedButton,
    ResolvedTextEditor,
    ResolvedMarkdown,
    ResolvedExternComponent,
    ResolvedThemer,
    ResolvedShader,
    ResolvedBooleanControl,
    ResolvedPickList,
    ResolvedSlider,
    ResolvedComboBox,
    ResolvedProgress,
    ResolvedRule,
    ResolvedQrCode,
    ResolvedSpace,
    ResolvedMedia,
    ResolvedOverlay,
    ResolvedTooltip,
    ResolvedFloat,
    ResolvedPin,
    ResolvedResponsive,
    ResolvedLazy,
    ResolvedKeyedColumn,
    ResolvedTable,
    ResolvedPaneGrid,
    ResolvedConditional,
    ResolvedIteration,
    ResolvedMatch,
    ResolvedNestedTheme,
);

impl CanonicalViewHir for ResolvedInteractionWidget {
    fn view_id(&self) -> ViewId {
        match self {
            Self::MouseArea(value) => value.id,
            Self::ResizeHandle(value) => value.id,
            Self::Sensor(value) => value.id,
        }
    }

    fn view_origin(&self) -> OriginId {
        match self {
            Self::MouseArea(value) => value.origin,
            Self::ResizeHandle(value) => value.origin,
            Self::Sensor(value) => value.origin,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedViewIdentity {
    pub(crate) name: String,
    pub(crate) key: Option<CheckedExprUseId>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPaneViewTopology {
    pub(crate) content: ViewId,
    pub(crate) title: Option<ResolvedPaneTitleTopology>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedPaneTitleTopology {
    pub(crate) content: ViewId,
    pub(crate) controls: Option<ViewId>,
    pub(crate) compact_controls: Option<ViewId>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ResolvedTableColumnTopology {
    pub(crate) header: ViewId,
    pub(crate) cell: ViewId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedViewKind {
    Layout {
        children: Vec<ViewId>,
    },
    Container {
        content: ViewId,
    },
    Overlay {
        content: ViewId,
        layer: ViewId,
    },
    PaneGrid {
        panes: Vec<ResolvedPaneViewTopology>,
        templates: Vec<ResolvedPaneViewTopology>,
    },
    Text,
    RichText,
    Input,
    Button {
        content: Option<ViewId>,
    },
    Checkbox,
    Toggler,
    Slider,
    Progress,
    Radio,
    PickList,
    ComboBox,
    Rule,
    QrCode,
    Space,
    If {
        children: Vec<ViewId>,
    },
    Match {
        arms: Vec<Vec<ViewId>>,
    },
    For {
        children: Vec<ViewId>,
    },
    KeyedColumn {
        child: ViewId,
    },
    Lazy {
        child: ViewId,
    },
    Markdown,
    TextEditor,
    Table {
        columns: Vec<ResolvedTableColumnTopology>,
    },
    Component {
        call: ComponentCallId,
    },
    Slot {
        slot: ComponentSlotId,
        name: String,
        optional: bool,
    },
    ExternComponent,
    Themer,
    Shader,
    Media,
    Tooltip {
        content: ViewId,
        tip: ViewId,
    },
    MouseArea {
        content: ViewId,
    },
    ResizeHandle {
        content: ViewId,
    },
    Canvas,
    Theme {
        content: ViewId,
    },
    Float {
        content: ViewId,
    },
    Pin {
        content: ViewId,
    },
    Sensor {
        content: ViewId,
    },
    ResponsiveSize {
        content: ViewId,
    },
}

impl ResolvedViewKind {
    #[cfg(test)]
    fn contract_kind(&self) -> &'static str {
        match self {
            Self::Layout { .. } => "layout",
            Self::Container { .. } => "container",
            Self::Overlay { .. } => "overlay",
            Self::PaneGrid { .. } => "pane-grid",
            Self::Text => "text",
            Self::RichText => "rich-text",
            Self::Input => "input",
            Self::Button { .. } => "button",
            Self::Checkbox => "checkbox",
            Self::Toggler => "toggler",
            Self::Slider => "slider",
            Self::Progress => "progress",
            Self::Radio => "radio",
            Self::PickList => "pick-list",
            Self::ComboBox => "combo-box",
            Self::Rule => "rule",
            Self::QrCode => "qr-code",
            Self::Space => "space",
            Self::If { .. } => "if",
            Self::Match { .. } => "match",
            Self::For { .. } => "for",
            Self::KeyedColumn { .. } => "keyed-column",
            Self::Lazy { .. } => "lazy",
            Self::Markdown => "markdown",
            Self::TextEditor => "text-editor",
            Self::Table { .. } => "table",
            Self::Component { .. } => "component",
            Self::Slot { .. } => "slot",
            Self::ExternComponent => "extern-component",
            Self::Themer => "themer",
            Self::Shader => "shader",
            Self::Media => "media",
            Self::Tooltip { .. } => "tooltip",
            Self::MouseArea { .. } => "mouse-area",
            Self::ResizeHandle { .. } => "resize-handle",
            Self::Canvas => "canvas",
            Self::Theme { .. } => "theme",
            Self::Float { .. } => "float",
            Self::Pin { .. } => "pin",
            Self::Sensor { .. } => "sensor",
            Self::ResponsiveSize { .. } => "responsive",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedView {
    pub(crate) id: ViewId,
    pub(crate) kind: ResolvedViewKind,
    pub(crate) identity: Option<ResolvedViewIdentity>,
    pub(crate) origin: OriginId,
}

impl Lowerer {
    pub(super) fn validate_checked_view_topology(&self) -> Result<(), Error> {
        for (index, view) in self.facts.views().iter().enumerate() {
            let id = ViewId(index as u32);
            let declaration = self.declarations.view(id);
            if view.id != id || declaration.id != id || view.origin != declaration.origin {
                return Err(self.invariant_at_origin(
                    declaration.origin,
                    "checked view identity or origin diverged from its declaration",
                ));
            }
            let mut unique = HashSet::with_capacity(view.children.len());
            for child in &view.children {
                let child_view = self.facts.views().get(child.0 as usize).ok_or_else(|| {
                    self.invariant_at_origin(
                        view.origin,
                        "checked view child ID is outside its arena",
                    )
                })?;
                if !unique.insert(*child)
                    || child_view.id != *child
                    || child_view.parent != Some(id)
                    || child_view.scope != view.scope
                {
                    return Err(self.invariant_at_origin(
                        child_view.origin,
                        "checked view child identity, parent, or scope diverged",
                    ));
                }
            }
            if let Some(parent) = view.parent {
                let parent_view = self.facts.views().get(parent.0 as usize).ok_or_else(|| {
                    self.invariant_at_origin(
                        view.origin,
                        "checked view parent ID is outside its arena",
                    )
                })?;
                if parent_view.id != parent
                    || parent_view.scope != view.scope
                    || !parent_view.children.contains(&id)
                {
                    return Err(self.invariant_at_origin(
                        view.origin,
                        "checked view parent does not own its child",
                    ));
                }
            }
        }
        Ok(())
    }

    pub(super) fn lower_view_topology(
        &mut self,
        node: &ViewNode,
        outer_component: Option<ComponentId>,
    ) -> Result<ViewId, Error> {
        let id = self
            .declarations
            .view_id(node.span())
            .ok_or_else(|| self.invariant(node.span(), "view topology has no shared view ID"))?;
        if id.0 as usize != self.views.len() {
            return Err(self.invariant(node.span(), "resolved view arena order diverged"));
        }
        let checked = self.facts.view(id).clone();
        if checked.id != id
            || checked.origin != self.declarations.view(id).origin
            || checked.kind != crate::hir::view_kind(node)
        {
            return Err(self.invariant_at_origin(
                checked.origin,
                "resolved view identity or kind diverged from checked topology",
            ));
        }
        let expected_scope = match checked.scope {
            CheckedViewScope::Component(component) => Some(component),
            CheckedViewScope::App | CheckedViewScope::Test(_) => None,
        };
        if expected_scope != outer_component {
            return Err(self.invariant_at_origin(
                checked.origin,
                "resolved view scope diverged from checked topology",
            ));
        }
        let source_children = crate::hir::view_children(node)
            .into_iter()
            .map(|child| {
                self.declarations
                    .view_id(child.span())
                    .ok_or_else(|| self.invariant(child.span(), "view child has no shared view ID"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if source_children != checked.children {
            return Err(self.invariant_at_origin(
                checked.origin,
                "resolved view children diverged from checked topology",
            ));
        }
        let identity = self.resolve_view_identity(node, &checked)?;
        let kind = self.resolve_view_kind(node, id, outer_component, &source_children)?;
        self.views.push(ResolvedView {
            id,
            kind,
            identity,
            origin: checked.origin,
        });
        Ok(id)
    }

    fn resolve_view_identity(
        &self,
        node: &ViewNode,
        checked: &crate::check::CheckedView,
    ) -> Result<Option<ResolvedViewIdentity>, Error> {
        let raw = node.identity();
        let Some(retained) = &checked.identity else {
            if raw.is_some() {
                return Err(self.invariant_at_origin(
                    checked.origin,
                    "view identity disappeared from checked topology",
                ));
            }
            return Ok(None);
        };
        let raw = raw.ok_or_else(|| {
            self.invariant_at_origin(
                checked.origin,
                "checked view identity has no source identity",
            )
        })?;
        if raw.name != retained.name || raw.key.is_some() != retained.key.is_some() {
            return Err(self.invariant_at_origin(
                checked.origin,
                "view identity name or key topology diverged",
            ));
        }
        if let Some(key) = retained.key {
            let owner = CheckedExprOwner::View {
                view: checked.id,
                role: CheckedViewExprRole::IdentityKey,
            };
            if self.facts.expression_use_by_owner(owner) != Some(key) {
                return Err(self.invariant_at_origin(
                    checked.origin,
                    "view identity key owner mapping diverged",
                ));
            }
            let expression = self.facts.try_expression_use(key).ok_or_else(|| {
                self.invariant_at_origin(
                    checked.origin,
                    "view identity key expression-use ID is outside its arena",
                )
            })?;
            if expression.owner != owner
                || !matches!(expression.source, Type::I64 | Type::Str)
                || expression.destination != expression.source
                || expression.coercion != CheckedInitializerCoercion::None
            {
                return Err(self.invariant_at_origin(
                    checked.origin,
                    "view identity key type contract diverged",
                ));
            }
            let policy = ViewWidgetExpressionPolicy {
                lowerer: self,
                view: checked.id,
                scope: checked.scope,
                use_id: key,
                span: node.span(),
                canvas_locals: false,
                own_view_locals: false,
                allowed_own_view_locals: None,
                family: "view identity",
            };
            let mut graph = CheckedExpressionGraph::default();
            let scope = graph.root_scope();
            let source =
                self.validate_checked_expression_node(expression.root, &policy, &mut graph, scope)?;
            if source != expression.source {
                return Err(self.invariant_at_origin(
                    checked.origin,
                    "view identity key expression graph type diverged",
                ));
            }
        }
        Ok(Some(ResolvedViewIdentity {
            name: retained.name.clone(),
            key: retained.key,
        }))
    }

    fn resolve_view_kind(
        &self,
        node: &ViewNode,
        id: ViewId,
        outer_component: Option<ComponentId>,
        children: &[ViewId],
    ) -> Result<ResolvedViewKind, Error> {
        let one = |label: &str| {
            children
                .first()
                .copied()
                .filter(|_| children.len() == 1)
                .ok_or_else(|| {
                    self.invariant(node.span(), format!("{label} child topology diverged"))
                })
        };
        let pair = |label: &str| {
            (children.len() == 2)
                .then(|| (children[0], children[1]))
                .ok_or_else(|| {
                    self.invariant(node.span(), format!("{label} child topology diverged"))
                })
        };
        Ok(match node {
            ViewNode::Layout { .. } => ResolvedViewKind::Layout {
                children: children.to_vec(),
            },
            ViewNode::Container { .. } => ResolvedViewKind::Container {
                content: one("container")?,
            },
            ViewNode::Overlay { .. } => {
                let (content, layer) = pair("overlay")?;
                ResolvedViewKind::Overlay { content, layer }
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                let mut cursor = 0;
                let mut resolve_pane =
                    |pane: &PaneView| -> Result<ResolvedPaneViewTopology, Error> {
                        let content = *children.get(cursor).ok_or_else(|| {
                            self.invariant(node.span(), "pane body child disappeared")
                        })?;
                        cursor += 1;
                        let title = pane
                            .title
                            .as_ref()
                            .map(|title| {
                                let content = *children.get(cursor).ok_or_else(|| {
                                    self.invariant(&title.span, "pane title child disappeared")
                                })?;
                                cursor += 1;
                                let controls = title.controls.as_ref().and_then(|_| {
                                    let child = children.get(cursor).copied();
                                    cursor += 1;
                                    child
                                });
                                let compact_controls =
                                    title.compact_controls.as_ref().and_then(|_| {
                                        let child = children.get(cursor).copied();
                                        cursor += 1;
                                        child
                                    });
                                if title.controls.is_some() != controls.is_some()
                                    || title.compact_controls.is_some()
                                        != compact_controls.is_some()
                                {
                                    return Err(self.invariant(
                                        &title.span,
                                        "pane title controls child disappeared",
                                    ));
                                }
                                Ok(ResolvedPaneTitleTopology {
                                    content,
                                    controls,
                                    compact_controls,
                                })
                            })
                            .transpose()?;
                        Ok(ResolvedPaneViewTopology { content, title })
                    };
                let panes = panes
                    .iter()
                    .map(&mut resolve_pane)
                    .collect::<Result<Vec<_>, _>>()?;
                let templates = templates
                    .iter()
                    .map(|template| resolve_pane(&template.pane))
                    .collect::<Result<Vec<_>, _>>()?;
                if cursor != children.len() {
                    return Err(
                        self.invariant(node.span(), "pane grid left child topology unconsumed")
                    );
                }
                ResolvedViewKind::PaneGrid { panes, templates }
            }
            ViewNode::Text { .. } => ResolvedViewKind::Text,
            ViewNode::RichText { .. } => ResolvedViewKind::RichText,
            ViewNode::Input { .. } => ResolvedViewKind::Input,
            ViewNode::Button { content, .. } => {
                if content.is_some() != (children.len() == 1) || children.len() > 1 {
                    return Err(self.invariant(node.span(), "button child topology diverged"));
                }
                ResolvedViewKind::Button {
                    content: children.first().copied(),
                }
            }
            ViewNode::Checkbox { .. } => ResolvedViewKind::Checkbox,
            ViewNode::Toggler { .. } => ResolvedViewKind::Toggler,
            ViewNode::Slider { .. } => ResolvedViewKind::Slider,
            ViewNode::Progress { .. } => ResolvedViewKind::Progress,
            ViewNode::Radio { .. } => ResolvedViewKind::Radio,
            ViewNode::PickList { .. } => ResolvedViewKind::PickList,
            ViewNode::ComboBox { .. } => ResolvedViewKind::ComboBox,
            ViewNode::Rule { .. } => ResolvedViewKind::Rule,
            ViewNode::QrCode { .. } => ResolvedViewKind::QrCode,
            ViewNode::Space { .. } => ResolvedViewKind::Space,
            ViewNode::If { .. } => ResolvedViewKind::If {
                children: children.to_vec(),
            },
            ViewNode::Match { arms, .. } => {
                let mut cursor = 0;
                let arms = arms
                    .iter()
                    .map(|arm| {
                        let end = cursor + arm.children.len();
                        let arm_children = children
                            .get(cursor..end)
                            .ok_or_else(|| {
                                self.invariant(&arm.span, "match arm child topology diverged")
                            })?
                            .to_vec();
                        cursor = end;
                        Ok(arm_children)
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                if cursor != children.len() {
                    return Err(
                        self.invariant(node.span(), "match view left child topology unconsumed")
                    );
                }
                ResolvedViewKind::Match { arms }
            }
            ViewNode::For { .. } => ResolvedViewKind::For {
                children: children.to_vec(),
            },
            ViewNode::KeyedColumn { .. } => ResolvedViewKind::KeyedColumn {
                child: one("keyed column")?,
            },
            ViewNode::Lazy { .. } => ResolvedViewKind::Lazy {
                child: one("lazy")?,
            },
            ViewNode::Markdown { .. } => ResolvedViewKind::Markdown,
            ViewNode::TextEditor { .. } => ResolvedViewKind::TextEditor,
            ViewNode::Table { columns, .. } => {
                if children.len() != columns.len() * 2 {
                    return Err(self.invariant(node.span(), "table column child topology diverged"));
                }
                ResolvedViewKind::Table {
                    columns: children
                        .chunks_exact(2)
                        .map(|pair| ResolvedTableColumnTopology {
                            header: pair[0],
                            cell: pair[1],
                        })
                        .collect(),
                }
            }
            ViewNode::Component { .. } => ResolvedViewKind::Component {
                call: self
                    .declarations
                    .component_call_id(id)
                    .ok_or_else(|| self.invariant(node.span(), "component view has no call ID"))?,
            },
            ViewNode::Slot { .. } => {
                let component = outer_component.ok_or_else(|| {
                    self.invariant(node.span(), "slot view is outside a component")
                })?;
                let checked = self.facts.component_slot_for_view(id).ok_or_else(|| {
                    self.invariant(node.span(), "slot view has no checked slot association")
                })?;
                if checked.id.component != component || checked.view != id {
                    return Err(self.invariant_at_origin(
                        checked.origin,
                        "checked slot view association is inconsistent",
                    ));
                }
                let contract = self
                    .components
                    .get(component.0 as usize)
                    .and_then(|component| component.slots.get(checked.id.index as usize))
                    .filter(|contract| {
                        contract.id == checked.id
                            && contract.name == checked.name
                            && contract.optional == checked.optional
                            && contract.origin == checked.origin
                    })
                    .ok_or_else(|| {
                        self.invariant_at_origin(
                            checked.origin,
                            "slot view has no normalized declaration",
                        )
                    })?;
                ResolvedViewKind::Slot {
                    slot: contract.id,
                    name: contract.name.clone(),
                    optional: contract.optional,
                }
            }
            ViewNode::ExternComponent { .. } => ResolvedViewKind::ExternComponent,
            ViewNode::Themer { .. } => ResolvedViewKind::Themer,
            ViewNode::Shader { .. } => ResolvedViewKind::Shader,
            ViewNode::Media { .. } => ResolvedViewKind::Media,
            ViewNode::Tooltip { .. } => {
                let (content, tip) = pair("tooltip")?;
                ResolvedViewKind::Tooltip { content, tip }
            }
            ViewNode::MouseArea { .. } => ResolvedViewKind::MouseArea {
                content: one("mouse area")?,
            },
            ViewNode::ResizeHandle { .. } => ResolvedViewKind::ResizeHandle {
                content: one("resize handle")?,
            },
            ViewNode::Canvas { .. } => ResolvedViewKind::Canvas,
            ViewNode::Theme { .. } => ResolvedViewKind::Theme {
                content: one("theme")?,
            },
            ViewNode::Float { .. } => ResolvedViewKind::Float {
                content: one("float")?,
            },
            ViewNode::Pin { .. } => ResolvedViewKind::Pin {
                content: one("pin")?,
            },
            ViewNode::Sensor { .. } => ResolvedViewKind::Sensor {
                content: one("sensor")?,
            },
            ViewNode::Responsive { .. } => ResolvedViewKind::ResponsiveSize {
                content: one("responsive")?,
            },
        })
    }
}

impl LoweredProgram {
    #[cfg(test)]
    pub(crate) fn validate_view_hir(&self) -> Result<(), Error> {
        if self.views.len() != self.facts.views().len() {
            return Err(self.invariant_at_origin(
                OriginId(u32::MAX),
                "resolved and checked view arena cardinality diverged",
            ));
        }
        for (index, (view, checked)) in self.views.iter().zip(self.facts.views()).enumerate() {
            let expected_id = ViewId(index as u32);
            if view.id != expected_id
                || checked.id != expected_id
                || view.origin != checked.origin
                || view.kind.contract_kind() != checked.kind
            {
                return Err(self.invariant_at_origin(
                    checked.origin,
                    "resolved view identity, origin, or kind diverged from its checked contract",
                ));
            }
            let resolved_children = self.resolved_view_children(view)?;
            if resolved_children != checked.children {
                return Err(self.invariant_at_origin(
                    view.origin,
                    "resolved view child topology diverged from its checked contract",
                ));
            }
            for child in &resolved_children {
                let child = self.facts.views().get(child.0 as usize).ok_or_else(|| {
                    self.invariant_at_origin(
                        view.origin,
                        "resolved view child ID is outside its arena",
                    )
                })?;
                if child.parent != Some(view.id) {
                    return Err(self.invariant_at_origin(
                        child.origin,
                        "resolved view child belongs to a different parent",
                    ));
                }
            }
            match (&view.identity, &checked.identity) {
                (None, None) => {}
                (Some(resolved), Some(checked))
                    if resolved.name == checked.name && resolved.key == checked.key =>
                {
                    if let Some(key) = resolved.key {
                        let owner = CheckedExprOwner::View {
                            view: view.id,
                            role: CheckedViewExprRole::IdentityKey,
                        };
                        let expression = self.facts.try_expression_use(key).ok_or_else(|| {
                            self.invariant_at_origin(
                                view.origin,
                                "view identity expression-use ID is outside its arena",
                            )
                        })?;
                        if expression.owner != owner
                            || self.facts.expression_use_by_owner(owner) != Some(key)
                            || !matches!(expression.source, Type::I64 | Type::Str)
                            || expression.destination != expression.source
                            || expression.coercion != CheckedInitializerCoercion::None
                        {
                            return Err(self.invariant_at_origin(
                                view.origin,
                                "view identity expression contract diverged",
                            ));
                        }
                    }
                }
                _ => {
                    return Err(self.invariant_at_origin(
                        view.origin,
                        "resolved view identity diverged from its checked contract",
                    ));
                }
            }
            self.validate_view_family(view)?;
        }
        self.validate_view_family_cardinality()?;
        self.validate_view_roots()
    }

    #[cfg(test)]
    fn validate_view_family_cardinality(&self) -> Result<(), Error> {
        macro_rules! expect {
            ($actual:expr, $label:literal, $($pattern:pat_param)|+) => {{
                let expected = self
                    .views
                    .iter()
                    .filter(|view| matches!(&view.kind, $($pattern)|+))
                    .count();
                if $actual != expected {
                    let origin = self
                        .views
                        .iter()
                        .find(|view| matches!(&view.kind, $($pattern)|+))
                        .map_or(OriginId(u32::MAX), |view| view.origin);
                    return Err(self.invariant_at_origin(
                        origin,
                        concat!($label, " normalized HIR cardinality diverged"),
                    ));
                }
            }};
        }

        expect!(
            self.layouts.len(),
            "layout",
            ResolvedViewKind::Layout { .. }
        );
        expect!(
            self.containers.len(),
            "container",
            ResolvedViewKind::Container { .. }
        );
        expect!(
            self.overlays.len(),
            "overlay",
            ResolvedViewKind::Overlay { .. }
        );
        expect!(
            self.pane_grids.len(),
            "pane grid",
            ResolvedViewKind::PaneGrid { .. }
        );
        expect!(
            self.texts.len(),
            "text",
            ResolvedViewKind::Text | ResolvedViewKind::RichText
        );
        expect!(self.inputs.len(), "input", ResolvedViewKind::Input);
        expect!(
            self.buttons.len(),
            "button",
            ResolvedViewKind::Button { .. }
        );
        expect!(
            self.boolean_controls.len(),
            "boolean control",
            ResolvedViewKind::Checkbox | ResolvedViewKind::Toggler | ResolvedViewKind::Radio
        );
        expect!(self.sliders.len(), "slider", ResolvedViewKind::Slider);
        expect!(
            self.progresses.len(),
            "progress",
            ResolvedViewKind::Progress
        );
        expect!(
            self.pick_lists.len(),
            "pick list",
            ResolvedViewKind::PickList
        );
        expect!(
            self.combo_boxes.len(),
            "combo box",
            ResolvedViewKind::ComboBox
        );
        expect!(self.rules.len(), "rule", ResolvedViewKind::Rule);
        expect!(self.qr_codes.len(), "qr code", ResolvedViewKind::QrCode);
        expect!(self.spaces.len(), "space", ResolvedViewKind::Space);
        expect!(
            self.conditionals.len(),
            "conditional",
            ResolvedViewKind::If { .. }
        );
        expect!(
            self.match_views.len(),
            "match",
            ResolvedViewKind::Match { .. }
        );
        expect!(
            self.iterations.len(),
            "iteration",
            ResolvedViewKind::For { .. }
        );
        expect!(
            self.keyed_columns.len(),
            "keyed column",
            ResolvedViewKind::KeyedColumn { .. }
        );
        expect!(self.lazy_views.len(), "lazy", ResolvedViewKind::Lazy { .. });
        expect!(self.markdowns.len(), "markdown", ResolvedViewKind::Markdown);
        expect!(
            self.text_editors.len(),
            "text editor",
            ResolvedViewKind::TextEditor
        );
        expect!(self.tables.len(), "table", ResolvedViewKind::Table { .. });
        expect!(
            self.extern_components.len(),
            "extern component",
            ResolvedViewKind::ExternComponent
        );
        expect!(self.themers.len(), "themer", ResolvedViewKind::Themer);
        expect!(self.shaders.len(), "shader", ResolvedViewKind::Shader);
        expect!(self.media.len(), "media", ResolvedViewKind::Media);
        expect!(
            self.tooltips.len(),
            "tooltip",
            ResolvedViewKind::Tooltip { .. }
        );
        expect!(
            self.interaction_widgets.len(),
            "interaction widget",
            ResolvedViewKind::MouseArea { .. }
                | ResolvedViewKind::ResizeHandle { .. }
                | ResolvedViewKind::Sensor { .. }
        );
        expect!(self.canvases.len(), "canvas", ResolvedViewKind::Canvas);
        expect!(
            self.styles.nested_theme_count(),
            "nested theme",
            ResolvedViewKind::Theme { .. }
        );
        expect!(self.floats.len(), "float", ResolvedViewKind::Float { .. });
        expect!(self.pins.len(), "pin", ResolvedViewKind::Pin { .. });
        expect!(
            self.responsives.len(),
            "responsive",
            ResolvedViewKind::ResponsiveSize { .. }
        );
        Ok(())
    }

    #[cfg(test)]
    fn resolved_view_children(&self, view: &ResolvedView) -> Result<Vec<ViewId>, Error> {
        Ok(match &view.kind {
            ResolvedViewKind::Layout { children }
            | ResolvedViewKind::If { children }
            | ResolvedViewKind::For { children } => children.clone(),
            ResolvedViewKind::Container { content }
            | ResolvedViewKind::KeyedColumn { child: content }
            | ResolvedViewKind::Lazy { child: content }
            | ResolvedViewKind::MouseArea { content }
            | ResolvedViewKind::ResizeHandle { content }
            | ResolvedViewKind::Theme { content }
            | ResolvedViewKind::Float { content }
            | ResolvedViewKind::Pin { content }
            | ResolvedViewKind::Sensor { content }
            | ResolvedViewKind::ResponsiveSize { content } => vec![*content],
            ResolvedViewKind::Overlay { content, layer } => vec![*content, *layer],
            ResolvedViewKind::Tooltip { content, tip } => vec![*content, *tip],
            ResolvedViewKind::Button { content } => content.iter().copied().collect(),
            ResolvedViewKind::Match { arms } => arms.iter().flatten().copied().collect(),
            ResolvedViewKind::Table { columns } => columns
                .iter()
                .flat_map(|column| [column.header, column.cell])
                .collect(),
            ResolvedViewKind::PaneGrid { panes, templates } => panes
                .iter()
                .chain(templates)
                .flat_map(|pane| {
                    let mut children = vec![pane.content];
                    if let Some(title) = &pane.title {
                        children.push(title.content);
                        children.extend(
                            [title.controls, title.compact_controls]
                                .into_iter()
                                .flatten(),
                        );
                    }
                    children
                })
                .collect(),
            ResolvedViewKind::Component { call } => self
                .component_call_by_id(*call)?
                .slots
                .iter()
                .filter_map(|slot| slot.content)
                .collect(),
            ResolvedViewKind::Text
            | ResolvedViewKind::RichText
            | ResolvedViewKind::Input
            | ResolvedViewKind::Checkbox
            | ResolvedViewKind::Toggler
            | ResolvedViewKind::Slider
            | ResolvedViewKind::Progress
            | ResolvedViewKind::Radio
            | ResolvedViewKind::PickList
            | ResolvedViewKind::ComboBox
            | ResolvedViewKind::Rule
            | ResolvedViewKind::QrCode
            | ResolvedViewKind::Space
            | ResolvedViewKind::Markdown
            | ResolvedViewKind::TextEditor
            | ResolvedViewKind::Slot { .. }
            | ResolvedViewKind::ExternComponent
            | ResolvedViewKind::Themer
            | ResolvedViewKind::Shader
            | ResolvedViewKind::Media
            | ResolvedViewKind::Canvas => Vec::new(),
        })
    }

    #[cfg(test)]
    fn validate_view_family(&self, view: &ResolvedView) -> Result<(), Error> {
        match view.kind {
            ResolvedViewKind::Layout { .. } => self.resolved_layout(view.id).map(|_| ()),
            ResolvedViewKind::Container { .. } => self.resolved_container(view.id).map(|_| ()),
            ResolvedViewKind::Overlay { .. } => self.resolved_overlay(view.id).map(|_| ()),
            ResolvedViewKind::PaneGrid { .. } => self.resolved_pane_grid(view.id).map(|_| ()),
            ResolvedViewKind::Text | ResolvedViewKind::RichText => {
                self.resolved_text(view.id).map(|_| ())
            }
            ResolvedViewKind::Input => self.resolved_input(view.id).map(|_| ()),
            ResolvedViewKind::Button { .. } => self.resolved_button(view.id).map(|_| ()),
            ResolvedViewKind::Checkbox | ResolvedViewKind::Toggler | ResolvedViewKind::Radio => {
                self.resolved_boolean_control(view.id).map(|_| ())
            }
            ResolvedViewKind::Slider => self.resolved_slider(view.id).map(|_| ()),
            ResolvedViewKind::Progress => self.resolved_progress(view.id).map(|_| ()),
            ResolvedViewKind::PickList => self.resolved_pick_list(view.id).map(|_| ()),
            ResolvedViewKind::ComboBox => self.resolved_combo_box(view.id).map(|_| ()),
            ResolvedViewKind::Rule => self.resolved_rule(view.id).map(|_| ()),
            ResolvedViewKind::QrCode => self.resolved_qr_code(view.id).map(|_| ()),
            ResolvedViewKind::Space => self.resolved_space(view.id).map(|_| ()),
            ResolvedViewKind::If { .. } => self.resolved_conditional(view.id).map(|_| ()),
            ResolvedViewKind::Match { .. } => self.resolved_match(view.id).map(|_| ()),
            ResolvedViewKind::For { .. } => self.resolved_iteration(view.id).map(|_| ()),
            ResolvedViewKind::KeyedColumn { .. } => self.resolved_keyed_column(view.id).map(|_| ()),
            ResolvedViewKind::Lazy { .. } => self.resolved_lazy(view.id).map(|_| ()),
            ResolvedViewKind::Markdown => self.resolved_markdown(view.id).map(|_| ()),
            ResolvedViewKind::TextEditor => self.resolved_text_editor(view.id).map(|_| ()),
            ResolvedViewKind::Table { .. } => self.resolved_table(view.id).map(|_| ()),
            ResolvedViewKind::ExternComponent => {
                self.resolved_extern_component(view.id).map(|_| ())
            }
            ResolvedViewKind::Themer => self.resolved_themer(view.id).map(|_| ()),
            ResolvedViewKind::Shader => self.resolved_shader(view.id).map(|_| ()),
            ResolvedViewKind::Media => self.resolved_media(view.id).map(|_| ()),
            ResolvedViewKind::Tooltip { .. } => self.resolved_tooltip(view.id).map(|_| ()),
            ResolvedViewKind::MouseArea { .. } => self.resolved_mouse_area(view.id).map(|_| ()),
            ResolvedViewKind::ResizeHandle { .. } => {
                self.resolved_resize_handle(view.id).map(|_| ())
            }
            ResolvedViewKind::Canvas => self.resolved_canvas(view.id).map(|_| ()),
            ResolvedViewKind::Theme { .. } => self.resolved_nested_theme(view.id).map(|_| ()),
            ResolvedViewKind::Float { .. } => self.resolved_float(view.id).map(|_| ()),
            ResolvedViewKind::Pin { .. } => self.resolved_pin(view.id).map(|_| ()),
            ResolvedViewKind::Sensor { .. } => self.resolved_sensor(view.id).map(|_| ()),
            ResolvedViewKind::ResponsiveSize { .. } => {
                self.resolved_responsive(view.id).map(|_| ())
            }
            ResolvedViewKind::Component { call } => self.component_call_by_id(call).map(|_| ()),
            ResolvedViewKind::Slot {
                slot,
                ref name,
                optional,
            } => self
                .components
                .get(slot.component.0 as usize)
                .and_then(|component| component.slots.get(slot.index as usize))
                .filter(|contract| {
                    contract.id == slot && contract.name == *name && contract.optional == optional
                })
                .map(|_| ())
                .ok_or_else(|| {
                    self.invariant_at_origin(view.origin, "slot view contract diverged")
                }),
        }
    }

    #[cfg(test)]
    fn validate_view_roots(&self) -> Result<(), Error> {
        let expected_test_mounts = self
            .tests
            .iter()
            .filter(|test| test.mount.is_some())
            .count();
        if self.test_mounts.len() != expected_test_mounts {
            return Err(self
                .invariant_at_origin(OriginId(u32::MAX), "test mount root cardinality diverged"));
        }
        let mut expected_roots = HashSet::from([self.app_view]);
        expected_roots.extend(self.components.iter().map(|component| component.root));
        expected_roots.extend(self.tests.iter().filter_map(|test| test.mount));
        let actual_roots = self
            .facts
            .views()
            .iter()
            .filter(|view| view.parent.is_none())
            .map(|view| view.id)
            .collect::<HashSet<_>>();
        if actual_roots != expected_roots {
            return Err(self.invariant_at_origin(
                OriginId(u32::MAX),
                "resolved view root set diverged from app, component, and test roots",
            ));
        }
        let app = self.resolved_view(self.app_view)?;
        if self.facts.view(app.id).scope != CheckedViewScope::App
            || self.facts.view(app.id).parent.is_some()
        {
            return Err(self.invariant_at_origin(app.origin, "application root view diverged"));
        }
        for component in &self.components {
            let root = self.resolved_view(component.root)?;
            let checked = self.facts.view(root.id);
            if checked.scope != CheckedViewScope::Component(component.id)
                || checked.parent.is_some()
            {
                return Err(self.invariant_at_origin(
                    root.origin,
                    "component root view diverged from its component scope",
                ));
            }
        }
        for test in &self.tests {
            if test.mount != self.test_mounts.get(&test.id).copied() {
                return Err(self.invariant_at_origin(test.origin, "test mount index diverged"));
            }
            if let Some(mount) = test.mount {
                let mount = self.resolved_view(mount)?;
                let checked = self.facts.view(mount.id);
                if checked.scope != CheckedViewScope::Test(test.id) || checked.parent.is_some() {
                    return Err(self.invariant_at_origin(
                        mount.origin,
                        "test mount root diverged from its test scope",
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn resolved_views(&self) -> impl Iterator<Item = &ResolvedView> {
        self.views.iter()
    }

    pub(crate) fn resolved_canvases(&self) -> Vec<&ResolvedCanvas> {
        let mut canvases = self.canvases.values().collect::<Vec<_>>();
        canvases.sort_by_key(|canvas| canvas.id.0);
        canvases
    }

    fn view_hir<'a, T: CanonicalViewHir>(
        &'a self,
        id: ViewId,
        values: &'a HashMap<ViewId, T>,
        family: &str,
    ) -> Result<&'a T, Error> {
        let origin = self
            .views
            .get(id.0 as usize)
            .filter(|view| view.id == id)
            .map_or(OriginId(u32::MAX), |view| view.origin);
        values
            .get(&id)
            .filter(|value| value.view_id() == id && value.view_origin() == origin)
            .ok_or_else(|| {
                self.invariant_at_origin(
                    origin,
                    format!("{family} normalized HIR identity or origin diverged"),
                )
            })
    }

    pub(crate) fn resolved_canvas(&self, id: ViewId) -> Result<&ResolvedCanvas, Error> {
        self.view_hir(id, &self.canvases, "canvas")
    }
    pub(crate) fn resolved_container(&self, id: ViewId) -> Result<&ResolvedContainer, Error> {
        self.view_hir(id, &self.containers, "container")
    }
    pub(crate) fn resolved_layout(&self, id: ViewId) -> Result<&ResolvedLayout, Error> {
        self.view_hir(id, &self.layouts, "layout")
    }
    pub(crate) fn resolved_text(&self, id: ViewId) -> Result<&ResolvedText, Error> {
        self.view_hir(id, &self.texts, "text")
    }
    pub(crate) fn resolved_input(&self, id: ViewId) -> Result<&ResolvedInput, Error> {
        self.view_hir(id, &self.inputs, "input")
    }
    pub(crate) fn resolved_button(&self, id: ViewId) -> Result<&ResolvedButton, Error> {
        self.view_hir(id, &self.buttons, "button")
    }
    pub(crate) fn resolved_text_editor(&self, id: ViewId) -> Result<&ResolvedTextEditor, Error> {
        self.view_hir(id, &self.text_editors, "text editor")
    }
    pub(crate) fn resolved_markdown(&self, id: ViewId) -> Result<&ResolvedMarkdown, Error> {
        self.view_hir(id, &self.markdowns, "markdown")
    }
    pub(crate) fn resolved_extern_component(
        &self,
        id: ViewId,
    ) -> Result<&ResolvedExternComponent, Error> {
        self.view_hir(id, &self.extern_components, "extern component")
    }
    pub(crate) fn resolved_themer(&self, id: ViewId) -> Result<&ResolvedThemer, Error> {
        self.view_hir(id, &self.themers, "themer")
    }
    pub(crate) fn resolved_shader(&self, id: ViewId) -> Result<&ResolvedShader, Error> {
        self.view_hir(id, &self.shaders, "shader")
    }
    pub(crate) fn resolved_boolean_control(
        &self,
        id: ViewId,
    ) -> Result<&ResolvedBooleanControl, Error> {
        self.view_hir(id, &self.boolean_controls, "boolean control")
    }
    pub(crate) fn resolved_pick_list(&self, id: ViewId) -> Result<&ResolvedPickList, Error> {
        self.view_hir(id, &self.pick_lists, "pick list")
    }
    pub(crate) fn resolved_slider(&self, id: ViewId) -> Result<&ResolvedSlider, Error> {
        self.view_hir(id, &self.sliders, "slider")
    }
    pub(crate) fn resolved_combo_box(&self, id: ViewId) -> Result<&ResolvedComboBox, Error> {
        self.view_hir(id, &self.combo_boxes, "combo box")
    }
    pub(crate) fn resolved_progress(&self, id: ViewId) -> Result<&ResolvedProgress, Error> {
        self.view_hir(id, &self.progresses, "progress")
    }
    pub(crate) fn resolved_rule(&self, id: ViewId) -> Result<&ResolvedRule, Error> {
        self.view_hir(id, &self.rules, "rule")
    }
    pub(crate) fn resolved_qr_code(&self, id: ViewId) -> Result<&ResolvedQrCode, Error> {
        self.view_hir(id, &self.qr_codes, "qr code")
    }
    pub(crate) fn resolved_space(&self, id: ViewId) -> Result<&ResolvedSpace, Error> {
        self.view_hir(id, &self.spaces, "space")
    }
    pub(crate) fn resolved_media(&self, id: ViewId) -> Result<&ResolvedMedia, Error> {
        self.view_hir(id, &self.media, "media")
    }
    pub(crate) fn resolved_overlay(&self, id: ViewId) -> Result<&ResolvedOverlay, Error> {
        self.view_hir(id, &self.overlays, "overlay")
    }
    pub(crate) fn resolved_tooltip(&self, id: ViewId) -> Result<&ResolvedTooltip, Error> {
        self.view_hir(id, &self.tooltips, "tooltip")
    }
    pub(crate) fn resolved_float(&self, id: ViewId) -> Result<&ResolvedFloat, Error> {
        self.view_hir(id, &self.floats, "float")
    }
    pub(crate) fn resolved_pin(&self, id: ViewId) -> Result<&ResolvedPin, Error> {
        self.view_hir(id, &self.pins, "pin")
    }
    pub(crate) fn resolved_responsive(&self, id: ViewId) -> Result<&ResolvedResponsive, Error> {
        self.view_hir(id, &self.responsives, "responsive")
    }
    pub(crate) fn resolved_lazy(&self, id: ViewId) -> Result<&ResolvedLazy, Error> {
        self.view_hir(id, &self.lazy_views, "lazy")
    }
    pub(crate) fn resolved_keyed_column(&self, id: ViewId) -> Result<&ResolvedKeyedColumn, Error> {
        self.view_hir(id, &self.keyed_columns, "keyed column")
    }
    pub(crate) fn resolved_table(&self, id: ViewId) -> Result<&ResolvedTable, Error> {
        self.view_hir(id, &self.tables, "table")
    }
    pub(crate) fn resolved_pane_grid(&self, id: ViewId) -> Result<&ResolvedPaneGrid, Error> {
        self.view_hir(id, &self.pane_grids, "pane grid")
    }
    pub(crate) fn resolved_conditional(&self, id: ViewId) -> Result<&ResolvedConditional, Error> {
        self.view_hir(id, &self.conditionals, "conditional")
    }
    pub(crate) fn resolved_iteration(&self, id: ViewId) -> Result<&ResolvedIteration, Error> {
        self.view_hir(id, &self.iterations, "iteration")
    }
    pub(crate) fn resolved_match(&self, id: ViewId) -> Result<&ResolvedMatch, Error> {
        self.view_hir(id, &self.match_views, "match view")
    }
    pub(crate) fn resolved_nested_theme(&self, id: ViewId) -> Result<&ResolvedNestedTheme, Error> {
        let origin = self
            .resolved_view(id)
            .map_or(OriginId(u32::MAX), |view| view.origin);
        self.styles
            .nested_theme(id)
            .filter(|theme| theme.id == id && theme.origin == origin)
            .ok_or_else(|| {
                self.invariant_at_origin(
                    origin,
                    "nested theme normalized HIR identity or origin diverged",
                )
            })
    }

    fn resolved_interaction(
        &self,
        id: ViewId,
        family: &str,
    ) -> Result<&ResolvedInteractionWidget, Error> {
        self.view_hir(id, &self.interaction_widgets, family)
    }

    pub(crate) fn resolved_mouse_area(&self, id: ViewId) -> Result<&ResolvedMouseArea, Error> {
        match self.resolved_interaction(id, "mouse area")? {
            ResolvedInteractionWidget::MouseArea(value) => Ok(value),
            _ => Err(self.invariant_at_origin(
                self.resolved_view(id)?.origin,
                "mouse area normalized kind diverged",
            )),
        }
    }

    pub(crate) fn resolved_resize_handle(
        &self,
        id: ViewId,
    ) -> Result<&ResolvedResizeHandle, Error> {
        match self.resolved_interaction(id, "resize handle")? {
            ResolvedInteractionWidget::ResizeHandle(value) => Ok(value),
            _ => Err(self.invariant_at_origin(
                self.resolved_view(id)?.origin,
                "resize handle normalized kind diverged",
            )),
        }
    }

    pub(crate) fn resolved_sensor(&self, id: ViewId) -> Result<&ResolvedSensor, Error> {
        match self.resolved_interaction(id, "sensor")? {
            ResolvedInteractionWidget::Sensor(value) => Ok(value),
            _ => Err(self.invariant_at_origin(
                self.resolved_view(id)?.origin,
                "sensor normalized kind diverged",
            )),
        }
    }
}
