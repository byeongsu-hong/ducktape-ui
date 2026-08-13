use super::*;
use crate::Warning;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
struct ScopeRisk {
    positional: bool,
    dynamic: bool,
}

impl ScopeRisk {
    fn with_id(self, id: &Option<Id>) -> Self {
        if id.as_ref().is_some_and(|id| id.key.is_some()) {
            Self {
                dynamic: true,
                ..self
            }
        } else {
            self
        }
    }

    fn positional(self) -> Self {
        Self {
            positional: true,
            dynamic: true,
        }
    }

    fn keyed(self) -> Self {
        Self {
            dynamic: true,
            ..self
        }
    }
}

pub(in crate::check) fn component_identity_warnings(document: &Document) -> Vec<Warning> {
    let mut session = Session {
        document,
        warnings: Vec::new(),
        emitted: HashSet::new(),
        visiting: HashSet::new(),
    };
    session.visit(&document.view, ScopeRisk::default());
    for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
        session.visit(mount, ScopeRisk::default());
    }
    session.warnings
}

struct Session<'a> {
    document: &'a Document,
    warnings: Vec<Warning>,
    emitted: HashSet<(&'static str, usize, usize)>,
    visiting: HashSet<(String, ScopeRisk)>,
}

impl Session<'_> {
    fn emit(&mut self, warning: Warning) {
        if self
            .emitted
            .insert((warning.code, warning.line, warning.column))
        {
            self.warnings.push(warning);
        }
    }

    fn visit(&mut self, node: &ViewNode, risk: ScopeRisk) {
        match node {
            ViewNode::Layout { id, children, .. } => self.visit_all(children, risk.with_id(id)),
            ViewNode::Container { id, content, .. }
            | ViewNode::Lazy {
                id, child: content, ..
            }
            | ViewNode::Theme { id, content, .. }
            | ViewNode::Float { id, content, .. }
            | ViewNode::Pin { id, content, .. }
            | ViewNode::Sensor { id, content, .. }
            | ViewNode::MouseArea { id, content, .. }
            | ViewNode::ResizeHandle { id, content, .. } => {
                self.visit(content, risk.with_id(id));
            }
            ViewNode::Overlay {
                id, content, layer, ..
            } => {
                let risk = risk.with_id(id);
                self.visit(content, risk);
                self.visit(layer, risk);
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for child in panes.iter().flat_map(PaneView::nodes) {
                    self.visit(child, risk);
                }
                for child in templates.iter().flat_map(|template| template.pane.nodes()) {
                    self.visit(child, risk.keyed());
                }
            }
            ViewNode::Button {
                id,
                content: Some(content),
                ..
            } => self.visit(content, risk.with_id(id)),
            ViewNode::If { children, .. } => self.visit_all(children, risk),
            ViewNode::Match { arms, .. } => {
                for children in arms.iter().map(|arm| arm.children.as_slice()) {
                    self.visit_all(children, risk);
                }
            }
            ViewNode::For { children, .. } => self.visit_all(children, risk.positional()),
            ViewNode::KeyedColumn { id, child, .. } => {
                self.visit(child, risk.with_id(id).keyed());
            }
            ViewNode::Table { id, columns, .. } => {
                let risk = risk.with_id(id);
                for column in columns {
                    self.visit(&column.header, risk);
                    self.visit(&column.cell, risk.positional());
                }
            }
            ViewNode::Component {
                name,
                id,
                slots,
                span,
                ..
            } => {
                let risk = risk.with_id(id);
                let component = self
                    .document
                    .components
                    .iter()
                    .find(|component| component.name == *name)
                    .expect("checker resolves component calls");
                let stateful = component_has_identity_state(component);
                if stateful && risk.positional {
                    self.emit(
                        Warning::new(
                            "W008",
                            span,
                            format!(
                                "stateful component `{name}` has position-based identity inside a repeated view"
                            ),
                        )
                        .hint(
                            "render it with `keyed item in items by=stable_key` so inserts and reordering cannot transfer component state",
                        ),
                    );
                }
                if stateful && risk.dynamic && component.lifetime == ComponentLifetime::Retained {
                    self.emit(
                        Warning::new(
                            "W009",
                            span,
                            format!(
                                "retained component `{name}` can accumulate state for dynamic identities"
                            ),
                        )
                        .hint(
                            "declare `lifetime mounted` to discard state after an identity leaves the rendered tree, or keep the identity set bounded",
                        ),
                    );
                }

                let visit_key = (name.clone(), risk);
                if self.visiting.insert(visit_key.clone()) {
                    self.visit(&component.root, risk);
                    self.visiting.remove(&visit_key);
                }
                for slot in slots {
                    self.visit(&slot.content, risk);
                }
            }
            ViewNode::Tooltip {
                id, content, tip, ..
            } => {
                let risk = risk.with_id(id);
                self.visit(content, risk);
                self.visit(tip, risk);
            }
            ViewNode::Responsive { id, content, .. } => {
                let risk = risk.with_id(id);
                match content {
                    ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                        self.visit(narrow, risk);
                        self.visit(wide, risk);
                    }
                    ResponsiveContent::Size { content, .. } => self.visit(content, risk),
                }
            }
            ViewNode::Text { .. }
            | ViewNode::RichText { .. }
            | ViewNode::Input { .. }
            | ViewNode::Button { content: None, .. }
            | ViewNode::Checkbox { .. }
            | ViewNode::Toggler { .. }
            | ViewNode::Slider { .. }
            | ViewNode::Progress { .. }
            | ViewNode::Radio { .. }
            | ViewNode::PickList { .. }
            | ViewNode::ComboBox { .. }
            | ViewNode::Rule { .. }
            | ViewNode::QrCode { .. }
            | ViewNode::Space { .. }
            | ViewNode::Markdown { .. }
            | ViewNode::TextEditor { .. }
            | ViewNode::Slot { .. }
            | ViewNode::ExternComponent { .. }
            | ViewNode::Themer { .. }
            | ViewNode::Shader { .. }
            | ViewNode::Media { .. }
            | ViewNode::Canvas { .. } => {}
        }
    }

    fn visit_all(&mut self, nodes: &[ViewNode], risk: ScopeRisk) {
        for node in nodes {
            self.visit(node, risk);
        }
    }
}

fn component_has_identity_state(component: &Component) -> bool {
    !component.states.is_empty()
        || component
            .handlers
            .iter()
            .any(|handler| handler.statements.iter().any(statement_has_delivery_lane))
}

fn statement_has_delivery_lane(statement: &Statement) -> bool {
    match statement {
        Statement::Run {
            mode: DeliveryMode::Latest | DeliveryMode::Replace,
            ..
        } => true,
        Statement::TaskGroup { statements, .. } => {
            statements.iter().any(statement_has_delivery_lane)
        }
        Statement::Match { arms, .. } => arms
            .iter()
            .flat_map(|arm| &arm.statements)
            .any(statement_has_delivery_lane),
        Statement::Abortable { task, .. } => statement_has_delivery_lane(task),
        _ => false,
    }
}
