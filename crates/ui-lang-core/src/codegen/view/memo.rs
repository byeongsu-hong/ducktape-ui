use super::*;
use std::collections::BTreeSet;

/// The revision reads that key the layout memo the compiler wraps around a
/// component use, or `None` when the use must lay out on every pass.
///
/// A cached layout node is correct while it is a function of what the key
/// sees. That holds when every expression the use evaluates — its arguments,
/// its slot content, the component's body, and the bodies of the components
/// that body uses — reads only app state, derived values, this instance's
/// component state, palette entries, and locals declared inside the subtree;
/// and when every widget in the subtree lays out from its own element and
/// `Limits` alone. A row local from outside, a secret, a clock builtin, a
/// nested component with state of its own (its revisions live in an
/// instance this site cannot name), or a widget whose layout reads state
/// written elsewhere — a virtual window, an editor, an extern, a sensor —
/// leaves the use unmemoized, exactly as `lazy` refuses such a dependency.
pub(in crate::codegen) fn component_use_memo_reads(
    program: &LoweredProgram,
    call: &ComponentCall,
    component: &ComponentContract,
    component_env: &dyn BindingEnvironment,
    use_env: &dyn BindingEnvironment,
) -> Result<BTreeSet<String>, String> {
    let mut fold = MemoFold {
        program,
        reads: BTreeSet::new(),
        subtree: HashSet::new(),
        depth: 0,
    };
    let markers = fold.arguments(call, use_env)?;
    for slot in &call.slots {
        if let Some(content) = slot.content {
            fold.view(content, use_env)?;
        }
    }
    let mut body_env = ScopedBindingEnv::new(component_env);
    for (key, marker) in markers {
        body_env.insert(key, marker);
    }
    fold.view(component.root, &body_env)?;
    Ok(fold.reads)
}

/// `file:line` of a view, for the `ICE_MEMO_DEBUG` refusal trace.
fn where_is(program: &LoweredProgram, view: &ResolvedView) -> String {
    let origin = program.origin(view.origin);
    let path = origin
        .path
        .as_ref()
        .map_or_else(|| "<input>".to_owned(), |path| path.display().to_string());
    format!("{path}:{}", origin.line)
}

fn kind_name(view: &ResolvedView) -> String {
    let debug = format!("{:?}", view.kind);
    debug
        .split([' ', '{', '('])
        .next()
        .unwrap_or("view")
        .to_owned()
}

struct MemoFold<'a> {
    program: &'a LoweredProgram,
    reads: BTreeSet<String>,
    /// Views walked so far; a local one of them declares is internal.
    subtree: HashSet<ViewId>,
    depth: u32,
}

impl MemoFold<'_> {
    fn reads_of(
        &self,
        expression: CheckedExprUseId,
        env: &dyn BindingEnvironment,
    ) -> Option<BTreeSet<String>> {
        let program = self.program;
        let subtree = &self.subtree;
        revision_reads_within(program, expression, env, &|local| {
            program
                .local_view(local)
                .is_some_and(|view| subtree.contains(&view))
        })
    }

    /// Folds the call's arguments into the key and returns the parameter
    /// revision markers the callee body resolves its parameter reads to.
    fn arguments(
        &mut self,
        call: &ComponentCall,
        env: &dyn BindingEnvironment,
    ) -> Result<HashMap<String, Binding>, String> {
        let mut markers = HashMap::new();
        for argument in &call.arguments {
            let reads = if argument.uses_definition_scope() {
                BTreeSet::new()
            } else {
                self.reads_of(argument.expression, env).ok_or_else(|| {
                    format!(
                        "argument `{}` reads a row local, a secret, or a clock",
                        argument.name
                    )
                })?
            };
            markers.insert(
                param_revisions_key(&argument.name),
                Binding {
                    code: reads
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(&REVISION_SEPARATOR.to_string()),
                    ty: Type::Bool,
                    local: true,
                    state: None,
                    owner: None,
                },
            );
            self.reads.extend(reads);
        }
        Ok(markers)
    }

    fn view(&mut self, id: ViewId, env: &dyn BindingEnvironment) -> Result<(), String> {
        let program = self.program;
        let view = program
            .resolved_view(id)
            .map_err(|error| error.to_string())?;
        if !layout_pure(program, view) {
            return Err(format!(
                "{} at {} lays out from state outside its subtree",
                kind_name(view),
                where_is(program, view)
            ));
        }
        self.subtree.insert(id);
        for expression in program.expression_uses_of_view(id) {
            let reads = self.reads_of(expression, env).ok_or_else(|| {
                format!(
                    "an expression of {} at {} reads a row local, a secret, or a clock",
                    kind_name(view),
                    where_is(program, view)
                )
            })?;
            self.reads.extend(reads);
        }
        if let ResolvedViewKind::Component { call } = &view.kind {
            let call = program
                .component_call_by_id(*call)
                .map_err(|error| error.to_string())?;
            let component = program.component(call.component);
            if !component.states.is_empty() {
                return Err(format!(
                    "nested component `{}` at {} owns state",
                    component.name,
                    where_is(program, view)
                ));
            }
            if self.depth == 64 {
                return Err("components nest deeper than 64".to_owned());
            }
            self.depth += 1;
            let markers = self.arguments(call, env)?;
            for slot in &call.slots {
                if let Some(content) = slot.content {
                    self.view(content, env)?;
                }
            }
            self.view(component.root, &markers)?;
            self.depth -= 1;
            return Ok(());
        }
        for child in program
            .resolved_view_children(view)
            .map_err(|error| error.to_string())?
        {
            self.view(child, env)?;
        }
        Ok(())
    }
}

/// Whether the widget this view lowers to computes its layout node from its
/// own element and `Limits` alone, so a node cached under a key over the
/// element's reads stays correct.
fn layout_pure(program: &LoweredProgram, view: &ResolvedView) -> bool {
    match &view.kind {
        // A virtual window reads the viewport the enclosing scrollable
        // writes; an auto-scrolling scrollable corrects its offset while it
        // lays out.
        ResolvedViewKind::Layout { .. } => program.resolved_layout(view.id).is_ok_and(|layout| {
            !matches!(
                &layout.mode,
                ResolvedLayoutMode::Linear(linear) if linear.virtual_row.is_some()
            ) && !matches!(
                &layout.mode,
                ResolvedLayoutMode::Scroll(scroll) if scroll.auto_scroll.is_some()
            )
        }),
        ResolvedViewKind::KeyedColumn { .. } => program
            .resolved_keyed_column(view.id)
            .is_ok_and(|column| column.virtual_row.is_none()),
        ResolvedViewKind::Container { .. }
        | ResolvedViewKind::Overlay { .. }
        | ResolvedViewKind::Text
        | ResolvedViewKind::RichText
        | ResolvedViewKind::Input
        | ResolvedViewKind::Button { .. }
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
        | ResolvedViewKind::If { .. }
        | ResolvedViewKind::Match { .. }
        | ResolvedViewKind::For { .. }
        | ResolvedViewKind::Lazy { .. }
        | ResolvedViewKind::Markdown
        | ResolvedViewKind::Table { .. }
        | ResolvedViewKind::Component { .. }
        | ResolvedViewKind::Slot { .. }
        | ResolvedViewKind::Tooltip { .. }
        | ResolvedViewKind::MouseArea { .. }
        | ResolvedViewKind::Canvas
        | ResolvedViewKind::Theme { .. }
        | ResolvedViewKind::Pin { .. } => true,
        // An editor mutates highlighter state across frames; an extern,
        // a themer, a shader, and media are opaque to the compiler; a
        // sensor, a resize handle, a responsive size, a float, and a pane
        // grid read or publish geometry from outside their subtree.
        ResolvedViewKind::TextEditor
        | ResolvedViewKind::ExternComponent
        | ResolvedViewKind::Themer
        | ResolvedViewKind::Shader
        | ResolvedViewKind::Media
        | ResolvedViewKind::ResizeHandle { .. }
        | ResolvedViewKind::Sensor { .. }
        | ResolvedViewKind::ResponsiveSize { .. }
        | ResolvedViewKind::Float { .. }
        | ResolvedViewKind::PaneGrid { .. } => false,
    }
}
