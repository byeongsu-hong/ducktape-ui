//! Publishes a view as data instead of Rust.
//!
//! Every construct here has an inline emitter elsewhere in `codegen::view`
//! that produces the same widgets as compiled Rust. This module builds the
//! `ui_lang_template` tree that `ui_lang_runtime::template` renders, plus the
//! Rust expressions that fill the slot table each frame.
//!
//! The node types come from `ui_lang_template` rather than being restated
//! here: a format written by one definition and read by another is a format
//! that can drift, and the drift would surface as a view that renders wrong
//! rather than as a build failure.
//!
//! The node vocabulary is deliberately narrow — layouts, containers, text,
//! inputs, buttons. A construct outside it is not a failure: it becomes a hole
//! whose subtree the inline emitter renders as before, spliced back through
//! the slot table. Widening the vocabulary shrinks the holes; it is never
//! required for a view to publish.

use super::*;
use ui_lang_template::{
    A11y, AlignX, AlignY, Axis, BoolSlot, ButtonFace, ButtonStyle, ColorBase, ColorRef, Edges,
    GroupSlot, HandlerSlot, MessageSlot, Node, Size, SlotCounts, Source, StateSlot, SubtreeSlot,
    Template, TextSlot, Value,
};

/// A view published as data, with the compiled expressions that feed it.
pub(in crate::codegen) struct TemplateEmission {
    /// The template itself, in the JSON form the runtime parses.
    pub(in crate::codegen) json: String,
    /// Rust statements, one per slot, each appending to the table its kind
    /// belongs to. They are emitted in the order the tables are indexed, so a
    /// statement's position in its own table is the index the template holds.
    pub(in crate::codegen) slots: Vec<String>,
    /// How many slots of each kind the statements above produce.
    pub(in crate::codegen) counts: SlotCounts,
    /// The `.ice` files this template's nodes cite, in index order. They are
    /// compiled in as `&'static str`, which is why a reload can move a node
    /// between these files but not introduce a new one.
    pub(in crate::codegen) paths: Vec<String>,
}

/// Publishes `program`'s app view as a template.
///
/// `None` means this view has no published form at all — it renders nothing,
/// or the app retains mounted component state the vocabulary cannot express.
/// An ordinary unmodelled construct does not land here; it becomes a hole.
pub(in crate::codegen) fn emit(
    program: &LoweredProgram,
    message: &str,
    env: &dyn BindingEnvironment,
    source_path: &str,
    root_scope: &str,
) -> Result<Option<TemplateEmission>, Error> {
    // Mounted components carry per-scope retained state that the template
    // vocabulary has no node for, so those views stay inline wholesale.
    if retains_mounted_components(program) {
        return Ok(None);
    }
    let mut builder = Builder {
        program,
        message,
        env,
        slots: Vec::new(),
        counts: SlotCounts::default(),
        paths: Vec::new(),
        source_path,
    };
    if builder.is_omitted(program.app_view())? {
        return Ok(None);
    }
    let root = builder.node(program.app_view(), root_scope)?;
    let template = Template {
        root,
        slots: builder.counts,
    };
    let origin = program.resolved_view(program.app_view())?.origin;
    let json = template.to_json().map_err(|error| {
        program.invariant_at_origin(
            origin,
            format!("view template failed to serialize: {error}"),
        )
    })?;
    Ok(Some(TemplateEmission {
        json,
        slots: builder.slots,
        counts: builder.counts,
        paths: builder.paths,
    }))
}

struct Builder<'a> {
    program: &'a LoweredProgram,
    message: &'a str,
    env: &'a dyn BindingEnvironment,
    slots: Vec<String>,
    counts: SlotCounts,
    paths: Vec<String>,
    /// The root being compiled. A node lowered from the root file carries no
    /// path of its own — only imported origins do — so this stands in for it,
    /// the same way the inline emitter stamps its source markers.
    source_path: &'a str,
}

impl Builder<'_> {
    // One push per slot kind. Each hands back the index type its table is
    // addressed by, so a node built from the wrong one does not compile.

    fn push_text(&mut self, code: String) -> TextSlot {
        let index = self.counts.texts;
        self.counts.texts += 1;
        self.slots.push(format!("__ice_slots.texts.push({code});"));
        TextSlot(index)
    }

    fn push_state(&mut self, code: String) -> StateSlot {
        let index = self.counts.states;
        self.counts.states += 1;
        self.slots.push(format!("__ice_slots.states.push({code});"));
        StateSlot(index)
    }

    fn push_message(&mut self, code: String) -> MessageSlot {
        let index = self.counts.messages;
        self.counts.messages += 1;
        self.slots
            .push(format!("__ice_slots.messages.push({code});"));
        MessageSlot(index)
    }

    fn push_handler(&mut self, code: String) -> HandlerSlot {
        let index = self.counts.handlers;
        self.counts.handlers += 1;
        self.slots
            .push(format!("__ice_slots.handlers.push({code});"));
        HandlerSlot(index)
    }

    fn push_subtree(&mut self, code: String) -> SubtreeSlot {
        let index = self.counts.subtrees;
        self.counts.subtrees += 1;
        self.slots
            .push(format!("__ice_slots.push_subtree({code});"));
        SubtreeSlot(index)
    }

    fn push_bool(&mut self, code: String) -> BoolSlot {
        let index = self.counts.bools;
        self.counts.bools += 1;
        self.slots.push(format!("__ice_slots.bools.push({code});"));
        BoolSlot(index)
    }

    fn push_group(&mut self, code: String) -> GroupSlot {
        let index = self.counts.groups;
        self.counts.groups += 1;
        self.slots.push(format!("__ice_slots.push_group({code});"));
        GroupSlot(index)
    }

    /// Interns a `.ice` path and returns its index in the emitted table.
    fn path_index(&mut self, path: String) -> usize {
        match self.paths.iter().position(|known| known == &path) {
            Some(index) => index,
            None => {
                self.paths.push(path);
                self.paths.len() - 1
            }
        }
    }

    fn source(&mut self, origin: OriginId) -> Option<Source> {
        let origin = self.program.origin(origin);
        let (line, column) = (origin.line, origin.column);
        let path = match &origin.path {
            Some(path) => path.to_string_lossy().into_owned(),
            None => self.source_path.to_owned(),
        };
        let path = self.path_index(path);
        Some(Source { path, line, column })
    }

    /// The accessibility identity of a node, matching what the inline path
    /// composes: an author's `#name`, or `@kind:line` when unnamed.
    fn segment(
        &mut self,
        identity: Option<&ResolvedViewIdentity>,
        kind: &str,
        origin: OriginId,
    ) -> Option<A11y> {
        let source = self.source(origin);
        match identity {
            // A keyed identity resolves at run time, which the template's
            // static segment cannot express.
            Some(identity) if identity.key.is_some() => None,
            Some(identity) => Some(A11y {
                segment: identity.name.clone(),
                named: true,
                source,
            }),
            None => Some(A11y {
                segment: format!("@{kind}:{}", self.program.origin(origin).line),
                named: false,
                source,
            }),
        }
    }

    /// Publishes a node as data, or — when the vocabulary cannot express it —
    /// as a hole the compiled tree fills.
    ///
    /// Falling back per subtree rather than per view is what lets a real app
    /// reload at all: one `if` no longer forces its whole screen back onto the
    /// compiled path, it just stops that branch's contents from reloading.
    fn node(&mut self, id: ViewId, scope: &str) -> Result<Node, Error> {
        let view = self.program.resolved_view(id)?;
        let modelled = match &view.kind {
            ResolvedViewKind::Container { content } => self.container(id, view, *content, scope)?,
            ResolvedViewKind::Layout { children } => self.linear(id, view, children, scope)?,
            ResolvedViewKind::Text => self.text(id, view)?,
            ResolvedViewKind::Input => self.input(id, view)?,
            ResolvedViewKind::Button { content } => self.button(id, view, content.as_ref())?,
            ResolvedViewKind::Overlay { content, layer } => {
                self.overlay(id, view, *content, *layer, scope)?
            }
            _ => None,
        };
        match modelled {
            Some(node) => Ok(node),
            None => self.subtree(id, scope),
        }
    }

    /// Whether a node renders nothing — an optional slot with no content, or
    /// a wrapper around one.
    fn is_omitted(&self, id: ViewId) -> Result<bool, Error> {
        node_is_omitted(id, self.program, self.env, None)
    }

    /// Whether a node contributes a variable number of children to its parent
    /// rather than rendering as one element.
    ///
    /// `if`, `for`, and `match` are lowered as layout children precisely
    /// because their child count is not known until they run. A layout gives
    /// each of them a group slot; anything else that can hold only one child
    /// refuses the node, and becomes a subtree hole itself.
    fn splices_into_parent(&self, id: ViewId) -> Result<bool, Error> {
        Ok(matches!(
            self.program.resolved_view(id)?.kind,
            ResolvedViewKind::If { .. }
                | ResolvedViewKind::Match { .. }
                | ResolvedViewKind::For { .. }
        ))
    }

    /// The scope a node's descendants render under. Mirrors the inline
    /// emitter: only an identified node extends the path.
    fn child_scope(&self, view: &ResolvedView, scope: &str) -> Result<String, Error> {
        rendered_child_scope(view.identity.as_ref(), scope, self.env, self.program)
    }

    /// Emits the compiled rendering of a whole subtree into a slot, and
    /// returns the hole that reads it.
    fn subtree(&mut self, id: ViewId, scope: &str) -> Result<Node, Error> {
        let rendered = render_node(id, self.program, self.message, self.env, scope, None)?;
        Ok(Node::Subtree {
            slot: self.push_subtree(rendered),
        })
    }

    /// Emits the compiled child list a spliced construct produces into a slot,
    /// and returns the group that reads it.
    ///
    /// This is the same code the inline emitter writes for an `if`, `for` or
    /// `match` among a layout's children — the statements that append to
    /// `__children` — captured into a vector instead of a surrounding layout's
    /// own. Reusing it is the point: the branch renders exactly as before, and
    /// only the layout around it becomes data.
    fn group(&mut self, id: ViewId, scope: &str) -> Result<Node, Error> {
        let mut body = format!(
            "{{ let mut __children: ::std::vec::Vec<__IceElement<'_, {}>> = ::std::vec::Vec::new();",
            self.message
        );
        render_children(
            &mut body,
            &[id],
            self.program,
            self.message,
            self.env,
            scope,
            None,
        )?;
        body.push_str(" __children }");
        Ok(Node::Group {
            slot: self.push_group(body),
        })
    }

    fn container(
        &mut self,
        id: ViewId,
        view: &ResolvedView,
        content: ViewId,
        scope: &str,
    ) -> Result<Option<Node>, Error> {
        let container = self.program.resolved_container(id)?;
        // Anything beyond geometry and a flat background still needs compiled
        // Rust; refuse the whole view rather than drop the styling silently.
        if container.max_width.is_some()
            || container.max_height.is_some()
            || container.clip.is_some()
            || container.custom_style.is_some()
            || !container.border_dash.is_empty()
            || !surface_is_plain_background(&container.surface)
            || !style_is_empty(&container.utility_style)
            || !flex_item_is_empty(&container.flex_item)
        {
            return Ok(None);
        }
        let Some(a11y) = self.segment(view.identity.as_ref(), "container", container.origin) else {
            return Ok(None);
        };
        let (Some(width), Some(height)) = (
            self.optional_size(container.width.as_ref())?,
            self.optional_size(container.height.as_ref())?,
        ) else {
            return Ok(None);
        };
        let Some(padding) = self.padding(&container.padding)? else {
            return Ok(None);
        };
        let background = match &container.surface.background {
            Some(ResolvedContainerBackground::Color(color)) => match color_ref(color) {
                Some(color) => Some(color),
                None => return Ok(None),
            },
            Some(_) => return Ok(None),
            None => None,
        };
        if self.splices_into_parent(content)? || self.is_omitted(content)? {
            return Ok(None);
        }
        let child_scope = self.child_scope(view, scope)?;
        let content = self.node(content, &child_scope)?;
        Ok(Some(Node::Container {
            a11y,
            width,
            height,
            padding,
            align_x: container.align_x.map(align_x),
            align_y: container.align_y.map(align_y),
            background,
            content: Box::new(content),
        }))
    }

    fn linear(
        &mut self,
        id: ViewId,
        view: &ResolvedView,
        children: &[ViewId],
        scope: &str,
    ) -> Result<Option<Node>, Error> {
        let layout = self.program.resolved_layout(id)?;
        let ResolvedLayoutMode::Linear(linear) = &layout.mode else {
            return Ok(None);
        };
        if linear.max_width.is_some()
            || linear.virtual_row.is_some()
            || linear.clip.is_some()
            || linear.wrap
            || linear.wrap_spacing.is_some()
            || linear.wrap_align.is_some()
            || !style_is_empty(&layout.utility_style)
        {
            return Ok(None);
        }
        let Some(a11y) = self.segment(view.identity.as_ref(), "layout", layout.origin) else {
            return Ok(None);
        };
        let Some(spacing) = self.optional_f64(linear.spacing)? else {
            return Ok(None);
        };
        let (Some(width), Some(height)) = (
            self.optional_size(linear.width.as_ref())?,
            self.optional_size(linear.height.as_ref())?,
        ) else {
            return Ok(None);
        };
        let Some(padding) = self.padding(&linear.padding)? else {
            return Ok(None);
        };
        let axis = match linear.axis {
            ResolvedLinearAxis::Column => Axis::Column,
            ResolvedLinearAxis::Row => Axis::Row,
        };
        // A linear layout aligns along its cross axis only.
        let (align_x, align_y) = match (&axis, linear.align) {
            (Axis::Column, Some(align)) => (Some(align_x(align)), None),
            (Axis::Row, Some(align)) => (None, Some(align_y(align))),
            (_, None) => (None, None),
        };
        let child_scope = self.child_scope(view, scope)?;
        let mut rendered = Vec::with_capacity(children.len());
        for child in children {
            // An unsupplied optional slot renders nothing, and the layout's
            // spacing and fill are computed from the surviving child count.
            if self.is_omitted(*child)? {
                continue;
            }
            rendered.push(if self.splices_into_parent(*child)? {
                self.group(*child, &child_scope)?
            } else {
                self.node(*child, &child_scope)?
            });
        }
        Ok(Some(Node::Linear {
            a11y,
            axis,
            spacing,
            padding,
            width,
            height,
            align_x,
            align_y,
            children: rendered,
        }))
    }

    /// Publishes an overlay, keeping only its panel compiled.
    ///
    /// The base is the whole application beneath the modal, so publishing it
    /// is the point: an overlay is usually the outermost node of a real view,
    /// and leaving it unmodelled cost that view its entire template. The panel
    /// stays a hole — and stays lazy, built only on the frames the overlay is
    /// open, exactly as the inline path builds it.
    fn overlay(
        &mut self,
        id: ViewId,
        view: &ResolvedView,
        content: ViewId,
        layer: ViewId,
        scope: &str,
    ) -> Result<Option<Node>, Error> {
        let overlay = self.program.resolved_overlay(id)?;
        let Some(backdrop) = color_ref(&overlay.backdrop) else {
            return Ok(None);
        };
        let Some(padding) = self.literal_f64(overlay.padding) else {
            return Ok(None);
        };
        let Some(a11y) = self.segment(view.identity.as_ref(), "overlay", overlay.origin) else {
            return Ok(None);
        };
        if self.splices_into_parent(content)? || self.is_omitted(content)? {
            return Ok(None);
        }
        let child_scope = self.child_scope(view, scope)?;
        let visible =
            resolved_expr_use_code(self.program, overlay.visible, self.env, ValueMode::Owned)?;
        let (layer_code, panel_code) = overlay_layer_and_panel(
            layer,
            self.program,
            self.message,
            self.env,
            &child_scope,
            None,
        )?;
        let dismiss = match overlay.dismiss.as_ref() {
            Some(route) => {
                resolved_interaction_route_code(route, &[], self.env, self.program, self.message)?
            }
            None => format!("{}::__ExternNoop", self.message),
        };
        let message = self.message;
        // Built only while the overlay is open, so a shut modal costs one
        // `space` rather than its whole panel.
        let panel = format!(
            "{{ let __overlay_panel: __IceElement<'_, {message}> = if {visible} {{              let __overlay_layer: __IceElement<'_, {message}> = {layer_code}; {panel_code}.into()              }} else {{ ::iced::widget::Space::new().into() }}; __overlay_panel }}"
        );
        let content = self.node(content, &child_scope)?;
        Ok(Some(Node::Overlay {
            a11y,
            visible: self.push_bool(visible.clone()),
            backdrop,
            padding: padding as f32,
            align_x: overlay_align_x(overlay.align_x),
            align_y: overlay_align_y(overlay.align_y),
            dismiss: self.push_message(dismiss),
            noop: self.push_message(format!("{message}::__ExternNoop")),
            content: Box::new(content),
            panel: self.push_subtree(panel),
        }))
    }

    fn text(&mut self, id: ViewId, view: &ResolvedView) -> Result<Option<Node>, Error> {
        let text = self.program.resolved_text(id)?;
        let options = &text.options;
        if options.width.is_some()
            || options.height.is_some()
            || options.line_height.is_some()
            || options.font.is_some()
            || options.align_x.is_some()
            || options.align_y.is_some()
            || options.shaping.is_some()
            || options.wrapping.is_some()
            // Zero tracking is the absence of tracking: the inline path emits
            // one plain widget for it, so the template can carry it too.
            || options.tracking.is_some_and(|tracking| tracking != 0.0)
            || options.custom_style.is_some()
            || !style_is_only_text_color(&text.utility_style)
        {
            return Ok(None);
        }
        let ResolvedTextContent::Plain { value } = &text.content else {
            return Ok(None);
        };
        // `@token` colouring reaches text through the utility style rather
        // than the node's own options.
        let color = match text.utility_style.text_color.as_ref() {
            None => None,
            Some(color) => match color_ref(color) {
                Some(color) => Some(color),
                None => return Ok(None),
            },
        };
        let Some(a11y) = self.segment(view.identity.as_ref(), "text", text.origin) else {
            return Ok(None);
        };
        let Some(size) = self.optional_f32(options.size)? else {
            return Ok(None);
        };
        // A literal stays in the template; anything reading state becomes a
        // slot the compiled `__view` refills each frame.
        let value = match self.literal_string(*value) {
            Some(literal) => Value::Literal(literal),
            None => {
                let code =
                    resolved_expr_use_code(self.program, *value, self.env, ValueMode::Owned)?;
                Value::Slot(self.push_text(format!("({code}).to_string()")))
            }
        };
        Ok(Some(Node::Text {
            a11y,
            value,
            size,
            color,
        }))
    }

    fn input(&mut self, id: ViewId, view: &ResolvedView) -> Result<Option<Node>, Error> {
        let input = self.program.resolved_input(id)?;
        if input.disabled.is_some()
            || input.accessibility_label.is_some()
            || input.accessibility_description.is_some()
            || input.change.is_some()
            || input.submit.is_some()
            || input.paste.is_some()
            || input.padding.is_some()
            || input.text_size.is_some()
            || input.line_height.is_some()
            || input.align.is_some()
            || input.font.is_some()
            || input.icon.is_some()
            || input.custom_style.is_some()
            || !input_styles_are_empty(&input.styles)
            || !style_is_empty(&input.utility_style)
            || !input.hint.is_empty()
        {
            return Ok(None);
        }
        // `secure` flips the widget's role and value reporting, so a dynamic
        // one cannot be baked into the template.
        let secure = match input.secure {
            None => false,
            Some(expression) => match self.literal_bool(expression) {
                Some(value) => value,
                None => return Ok(None),
            },
        };
        let Some(a11y) = self.segment(view.identity.as_ref(), "input", input.origin) else {
            return Ok(None);
        };
        let Some(width) = self.optional_size(input.width.as_ref())? else {
            return Ok(None);
        };
        // Only app state is modelled: a component binding's constructor
        // captures a runtime scope, which is not a plain `fn` pointer.
        let state = self.env.get(input.binding.name()).ok_or_else(|| {
            self.program
                .invariant_at_origin(input.origin, "input state is absent from its render scope")
        })?;
        let Some(StateBinding::App(name)) = &state.state else {
            return Ok(None);
        };
        let value = self.push_state(format!("&{}", state.code));
        let on_input = self.push_handler(format!(
            "{}::{} as fn(::std::string::String) -> {}",
            self.message,
            binding_variant(name),
            self.message
        ));
        Ok(Some(Node::Input {
            a11y,
            label: input.label.clone(),
            value,
            on_input,
            width,
            secure,
        }))
    }

    fn button(
        &mut self,
        id: ViewId,
        view: &ResolvedView,
        content: Option<&ViewId>,
    ) -> Result<Option<Node>, Error> {
        let button = self.program.resolved_button(id)?;
        if content.is_some()
            || button.disabled.is_some()
            || button.accessibility_label.is_some()
            || button.accessibility_description.is_some()
            || button.width.is_some()
            || button.height.is_some()
            || button.padding.is_some()
            || button.clip.is_some()
            || button.custom_style.is_some()
            || button.preset != ResolvedButtonPreset::Primary
            || button.styles.disabled.is_some()
            || !style_is_empty(&button.utility_style)
            || !button.route.args.is_empty()
        {
            return Ok(None);
        }
        let ResolvedButtonContent::Label(label) = &button.content else {
            return Ok(None);
        };
        let Some(a11y) = self.segment(view.identity.as_ref(), "button", button.origin) else {
            return Ok(None);
        };
        let (Some(active), Some(hovered), Some(pressed)) = (
            button_face(self, button.styles.active.as_ref()),
            button_face(self, button.styles.hovered.as_ref()),
            button_face(self, button.styles.pressed.as_ref()),
        ) else {
            return Ok(None);
        };
        let activate = resolved_interaction_route_code(
            &button.route,
            &[],
            self.env,
            self.program,
            self.message,
        )?;
        let on_press = self.push_message(activate);
        Ok(Some(Node::Button {
            a11y,
            label: label.clone(),
            on_press,
            style: ButtonStyle {
                active: active.unwrap_or_default(),
                hovered,
                pressed,
            },
        }))
    }

    /// Reads a literal string out of the expression arena, or reports `None`
    /// when the expression must run.
    fn literal_string(&self, expression: CheckedExprUseId) -> Option<String> {
        let expressions = self.program.expressions();
        let resolved = expressions.expression_use(expression);
        if !matches!(resolved.coercion, ResolvedInitializerCoercion::None) {
            return None;
        }
        match &expressions.expression(resolved.root).kind {
            ResolvedExpressionKind::Str(value) => Some(value.clone()),
            _ => None,
        }
    }

    fn literal_f64(&self, expression: CheckedExprUseId) -> Option<f64> {
        let expressions = self.program.expressions();
        let resolved = expressions.expression_use(expression);
        if !matches!(resolved.coercion, ResolvedInitializerCoercion::None) {
            return None;
        }
        match &expressions.expression(resolved.root).kind {
            ResolvedExpressionKind::F64(value) => Some(*value),
            ResolvedExpressionKind::I64(value) => Some(*value as f64),
            _ => None,
        }
    }

    fn literal_bool(&self, expression: CheckedExprUseId) -> Option<bool> {
        let expressions = self.program.expressions();
        let resolved = expressions.expression_use(expression);
        if !matches!(resolved.coercion, ResolvedInitializerCoercion::None) {
            return None;
        }
        match &expressions.expression(resolved.root).kind {
            ResolvedExpressionKind::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// `Ok(None)` refuses the view; `Ok(Some(None))` means the option is unset.
    fn optional_f64(
        &self,
        expression: Option<CheckedExprUseId>,
    ) -> Result<Option<Option<f64>>, Error> {
        Ok(match expression {
            None => Some(None),
            Some(expression) => self.literal_f64(expression).map(Some),
        })
    }

    fn optional_f32(
        &self,
        expression: Option<CheckedExprUseId>,
    ) -> Result<Option<Option<f32>>, Error> {
        Ok(self
            .optional_f64(expression)?
            .map(|value| value.map(|value| value as f32)))
    }

    fn optional_size(
        &self,
        length: Option<&ResolvedContainerLength>,
    ) -> Result<Option<Option<Size>>, Error> {
        Ok(match length {
            None => Some(None),
            Some(ResolvedContainerLength::Fill) => Some(Some(Size::Fill)),
            Some(ResolvedContainerLength::Shrink) => Some(Some(Size::Shrink)),
            Some(ResolvedContainerLength::FixedF64(expression))
            | Some(ResolvedContainerLength::FixedLength(expression)) => self
                .literal_f64(*expression)
                .map(|value| Some(Size::Fixed(value as f32))),
            Some(ResolvedContainerLength::FillPortion(_)) => None,
        })
    }

    fn padding(&self, padding: &ResolvedContainerPadding) -> Result<Option<Option<Edges>>, Error> {
        if padding.all.is_none()
            && padding.x.is_none()
            && padding.y.is_none()
            && padding.top.is_none()
            && padding.right.is_none()
            && padding.bottom.is_none()
            && padding.left.is_none()
        {
            return Ok(Some(None));
        }
        let mut edges = Edges {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        };
        let read = |expression: Option<CheckedExprUseId>| match expression {
            None => Ok(None),
            Some(expression) => self.literal_f64(expression).map(Some).ok_or(()),
        };
        let Ok(all) = read(padding.all) else {
            return Ok(None);
        };
        if let Some(all) = all {
            edges = Edges {
                top: all,
                right: all,
                bottom: all,
                left: all,
            };
        }
        let (Ok(x), Ok(y)) = (read(padding.x), read(padding.y)) else {
            return Ok(None);
        };
        if let Some(x) = x {
            edges.left = x;
            edges.right = x;
        }
        if let Some(y) = y {
            edges.top = y;
            edges.bottom = y;
        }
        let (Ok(top), Ok(right), Ok(bottom), Ok(left)) = (
            read(padding.top),
            read(padding.right),
            read(padding.bottom),
            read(padding.left),
        ) else {
            return Ok(None);
        };
        for (slot, value) in [
            (&mut edges.top, top),
            (&mut edges.right, right),
            (&mut edges.bottom, bottom),
            (&mut edges.left, left),
        ] {
            if let Some(value) = value {
                *slot = value;
            }
        }
        Ok(Some(Some(edges)))
    }
}

/// The template addresses the palette by index, so only token colours are
/// expressible; a literal colour refuses the view rather than render wrong.
fn overlay_align_x(align: ResolvedOverlayAlignment) -> AlignX {
    match align {
        ResolvedOverlayAlignment::Start => AlignX::Left,
        ResolvedOverlayAlignment::Center => AlignX::Center,
        ResolvedOverlayAlignment::End => AlignX::Right,
    }
}

fn overlay_align_y(align: ResolvedOverlayAlignment) -> AlignY {
    match align {
        ResolvedOverlayAlignment::Start => AlignY::Top,
        ResolvedOverlayAlignment::Center => AlignY::Center,
        ResolvedOverlayAlignment::End => AlignY::Bottom,
    }
}

fn color_ref(color: &ResolvedThemeColor) -> Option<ColorRef> {
    let base = match color.base {
        ResolvedThemeColorBase::Token(token) => ColorBase::Token(token.index as usize),
        ResolvedThemeColorBase::White => ColorBase::White,
        ResolvedThemeColorBase::Black => ColorBase::Black,
        ResolvedThemeColorBase::Transparent => ColorBase::Transparent,
    };
    Some(ColorRef {
        base,
        alpha: color.opacity.map(|opacity| opacity as f32 / 100.0),
    })
}

/// `None` refuses the view; `Some(None)` means the status has no override.
fn button_face(
    builder: &Builder<'_>,
    style: Option<&ResolvedButtonStatusStyle>,
) -> Option<Option<ButtonFace>> {
    let Some(style) = style else {
        return Some(None);
    };
    let surface = &style.surface;
    if surface.border_color.is_some()
        || surface.border_width.is_some()
        || surface.shadow_color.is_some()
        || surface.shadow_x.is_some()
        || surface.shadow_y.is_some()
        || surface.shadow_blur.is_some()
        || surface.pixel_snap.is_some()
        || surface.background_alpha.is_some()
    {
        return None;
    }
    let background = match &surface.background {
        None => None,
        Some(ResolvedContainerBackground::Color(color)) => Some(color_ref(color)?),
        Some(_) => return None,
    };
    // Only the uniform `r=` form is modelled; per-corner radii are not.
    if surface.radius.top_left.is_some()
        || surface.radius.top_right.is_some()
        || surface.radius.bottom_right.is_some()
        || surface.radius.bottom_left.is_some()
    {
        return None;
    }
    let radius = match surface.radius.all {
        None => None,
        Some(expression) => Some(builder.literal_f64(expression)? as f32),
    };
    let text_color = match surface.text_color.as_ref() {
        None => None,
        Some(color) => Some(color_ref(color)?),
    };
    Some(Some(ButtonFace {
        background,
        text_color,
        radius,
    }))
}

/// Whether a utility style carries nothing at all.
fn style_is_empty(style: &ResolvedStyle) -> bool {
    style_is_only_text_color(style) && style.text_color.is_none()
}

/// Whether a utility style carries at most a text colour — the one utility the
/// template's text node can express.
fn style_is_only_text_color(style: &ResolvedStyle) -> bool {
    !style.width_fill
        && !style.height_fill
        && style.max_width.is_none()
        && style.padding == [0; 4]
        && style.gap.is_none()
        && !style.items_center
        && !style.self_center
        && !style.clip
        && style.text_size.is_none()
        && style.text_line_height.is_none()
        && !style.font_monospace
        && style.font_weight.is_none()
        && style.background.is_none()
        && style.hover_background.is_none()
        && style.pressed_background.is_none()
        && style.disabled_background.is_none()
        && style.disabled_text_color.is_none()
        && style.border_color.is_none()
        && style.focus_border_color.is_none()
        && style.border_width == 0
        && style.radius == 0
        && style.disabled_opacity.is_none()
}

fn flex_item_is_empty(item: &ResolvedContainerFlexItem) -> bool {
    item.order.is_none()
        && item.grow.is_none()
        && item.shrink.is_none()
        && item.basis.is_none()
        && item.align_self.is_none()
        && item.margins.is_none()
}

fn input_styles_are_empty(styles: &ResolvedInputStyleSet) -> bool {
    styles.active.is_none()
        && styles.hovered.is_none()
        && styles.focused.is_none()
        && styles.focused_hovered.is_none()
        && styles.disabled.is_none()
}

fn surface_is_plain_background(surface: &ResolvedContainerSurface) -> bool {
    // A computed opacity is per-frame Rust; `ColorRef` holds a constant.
    surface.background_alpha.is_none()
        && surface.text_color.is_none()
        && surface.border_color.is_none()
        && surface.border_width.is_none()
        && surface.shadow_color.is_none()
        && surface.shadow_x.is_none()
        && surface.shadow_y.is_none()
        && surface.shadow_blur.is_none()
        && surface.pixel_snap.is_none()
        && surface.radius.all.is_none()
        && surface.radius.top_left.is_none()
        && surface.radius.top_right.is_none()
        && surface.radius.bottom_right.is_none()
        && surface.radius.bottom_left.is_none()
}

fn align_x(align: ResolvedContainerAlignment) -> AlignX {
    match align {
        ResolvedContainerAlignment::Start => AlignX::Left,
        ResolvedContainerAlignment::Center => AlignX::Center,
        ResolvedContainerAlignment::End => AlignX::Right,
    }
}

fn align_y(align: ResolvedContainerAlignment) -> AlignY {
    match align {
        ResolvedContainerAlignment::Start => AlignY::Top,
        ResolvedContainerAlignment::Center => AlignY::Center,
        ResolvedContainerAlignment::End => AlignY::Bottom,
    }
}
