use super::*;
use crate::Warning;

#[derive(Clone)]
pub(in crate::check) struct WidgetIdSlot {
    entries: Vec<(String, ViewNode, HashMap<String, Type>)>,
    parent: Option<Box<Self>>,
}

pub(in crate::check) fn widget_operation_ids(
    root: &ViewNode,
    env: &dyn ExprTypeEnv,
    document: &Document,
) -> Result<Vec<WidgetIdPath>, Error> {
    Ok(collect_widget_ids(root, env, document, false)?.targets)
}

pub(in crate::check) struct TestWidgetIds {
    pub targets: Vec<WidgetIdPath>,
    pub component_scopes: Vec<WidgetIdPath>,
}

pub(in crate::check) fn test_widget_ids(
    root: &ViewNode,
    env: &dyn ExprTypeEnv,
    document: &Document,
) -> Result<TestWidgetIds, Error> {
    collect_widget_ids(root, env, document, true)
}

fn collect_widget_ids(
    root: &ViewNode,
    env: &dyn ExprTypeEnv,
    document: &Document,
    inspect_all: bool,
) -> Result<TestWidgetIds, Error> {
    fn segment(
        id: &Id,
        env: &dyn ExprTypeEnv,
        document: &Document,
        span: &Span,
    ) -> Result<(String, Option<Type>), Error> {
        Ok((
            id.name.clone(),
            id.key
                .as_ref()
                .map(|key| expr_type(key, env, document, span))
                .transpose()?,
        ))
    }

    fn scoped(
        scope: &WidgetIdPath,
        id: &Option<Id>,
        env: &dyn ExprTypeEnv,
        document: &Document,
        span: &Span,
    ) -> Result<WidgetIdPath, Error> {
        let mut scope = scope.clone();
        if let Some(id) = id {
            scope.push(segment(id, env, document, span)?);
        }
        Ok(scope)
    }

    fn record(
        scope: &WidgetIdPath,
        id: &Option<Id>,
        env: &dyn ExprTypeEnv,
        document: &Document,
        span: &Span,
        output: &mut Vec<WidgetIdPath>,
    ) -> Result<(), Error> {
        if id.is_some() {
            let path = scoped(scope, id, env, document, span)?;
            if !output.contains(&path) {
                output.push(path);
            }
        }
        Ok(())
    }

    fn match_binding_type(
        pattern: &MatchPattern,
        value_ty: &Type,
        document: &Document,
    ) -> Option<(String, Type)> {
        match (pattern, value_ty) {
            (MatchPattern::Some(binding), Type::Option(inner)) => {
                Some((binding.clone(), inner.as_ref().clone()))
            }
            (MatchPattern::Ok(binding), Type::Result(output, _)) => {
                Some((binding.clone(), output.as_ref().clone()))
            }
            (MatchPattern::Err(binding), Type::Result(_, error)) => {
                Some((binding.clone(), error.as_ref().clone()))
            }
            (
                MatchPattern::Enum {
                    enum_name,
                    variant,
                    binding: Some(binding),
                },
                Type::Named(name),
            ) if enum_name == name => document
                .enums
                .iter()
                .find(|item| item.name == *enum_name)
                .and_then(|item| item.variants.iter().find(|item| item.name == *variant))
                .and_then(|variant| variant.payload.clone())
                .map(|payload| (binding.clone(), payload)),
            _ => None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn collect(
        node: &ViewNode,
        env: &dyn ExprTypeEnv,
        document: &Document,
        scope: &WidgetIdPath,
        slot: Option<&WidgetIdSlot>,
        components: &mut Vec<(String, Span)>,
        output: &mut Vec<WidgetIdPath>,
        component_scopes: &mut Vec<WidgetIdPath>,
        inspect_all: bool,
    ) -> Result<(), Error> {
        match node {
            ViewNode::Layout {
                kind,
                id,
                children,
                span,
                ..
            } => {
                if inspect_all || *kind == Layout::Scroll {
                    record(scope, id, env, document, span, output)?;
                }
                let child_scope = scoped(scope, id, env, document, span)?;
                for child in children {
                    collect(
                        child,
                        env,
                        document,
                        &child_scope,
                        slot,
                        components,
                        output,
                        component_scopes,
                        inspect_all,
                    )?;
                }
            }
            ViewNode::Input { id, span, .. } | ViewNode::TextEditor { id, span, .. } => {
                record(scope, id, env, document, span, output)?;
            }
            ViewNode::Text { id, span, .. }
            | ViewNode::RichText { id, span, .. }
            | ViewNode::Toggler { id, span, .. }
            | ViewNode::Slider { id, span, .. }
            | ViewNode::Progress { id, span, .. }
            | ViewNode::Radio { id, span, .. }
            | ViewNode::PickList { id, span, .. }
            | ViewNode::ComboBox { id, span, .. }
            | ViewNode::Rule { id, span, .. }
            | ViewNode::QrCode { id, span, .. }
            | ViewNode::Space { id, span, .. }
            | ViewNode::Markdown { id, span, .. }
            | ViewNode::ExternComponent { id, span, .. }
            | ViewNode::Themer { id, span, .. }
            | ViewNode::Shader { id, span, .. }
            | ViewNode::Media { id, span, .. }
            | ViewNode::Canvas { id, span, .. }
                if inspect_all =>
            {
                record(scope, id, env, document, span, output)?;
            }
            ViewNode::Container {
                id, content, span, ..
            } => {
                if inspect_all {
                    record(scope, id, env, document, span, output)?;
                }
                let child_scope = scoped(scope, id, env, document, span)?;
                collect(
                    content,
                    env,
                    document,
                    &child_scope,
                    slot,
                    components,
                    output,
                    component_scopes,
                    inspect_all,
                )?;
            }
            ViewNode::Button {
                id, content, span, ..
            } => {
                if inspect_all {
                    record(scope, id, env, document, span, output)?;
                }
                if let Some(content) = content {
                    let child_scope = scoped(scope, id, env, document, span)?;
                    collect(
                        content,
                        env,
                        document,
                        &child_scope,
                        slot,
                        components,
                        output,
                        component_scopes,
                        inspect_all,
                    )?;
                }
            }
            ViewNode::Checkbox { id, span, .. } if inspect_all => {
                record(scope, id, env, document, span, output)?;
            }
            ViewNode::If { children, .. } => {
                for child in children {
                    collect(
                        child,
                        env,
                        document,
                        scope,
                        slot,
                        components,
                        output,
                        component_scopes,
                        inspect_all,
                    )?;
                }
            }
            ViewNode::For {
                item,
                items,
                children,
                span,
            } => {
                let Type::List(inner) = expr_type(items, env, document, span)? else {
                    unreachable!("checker validates for lists")
                };
                let mut child_env = ScopedTypeEnv::new(env);
                child_env.insert(item.clone(), *inner);
                for child in children {
                    collect(
                        child,
                        &child_env,
                        document,
                        scope,
                        slot,
                        components,
                        output,
                        component_scopes,
                        inspect_all,
                    )?;
                }
            }
            ViewNode::Match { value, arms, span } => {
                let value_ty = expr_type(value, env, document, span)?;
                for arm in arms {
                    let mut child_env = ScopedTypeEnv::new(env);
                    if let Some((name, ty)) = match_binding_type(&arm.pattern, &value_ty, document)
                    {
                        child_env.insert(name, ty);
                    }
                    for child in &arm.children {
                        collect(
                            child,
                            &child_env,
                            document,
                            scope,
                            slot,
                            components,
                            output,
                            component_scopes,
                            inspect_all,
                        )?;
                    }
                }
            }
            ViewNode::KeyedColumn {
                item,
                items,
                key,
                id,
                child,
                span,
                ..
            } => {
                if inspect_all {
                    record(scope, id, env, document, span, output)?;
                }
                let Type::List(inner) = expr_type(items, env, document, span)? else {
                    unreachable!("checker validates keyed lists")
                };
                let mut child_env = ScopedTypeEnv::new(env);
                child_env.insert(item.clone(), *inner);
                let mut child_scope = scoped(scope, id, env, document, span)?;
                child_scope.push((
                    "key".into(),
                    Some(expr_type(key, &child_env, document, span)?),
                ));
                collect(
                    child,
                    &child_env,
                    document,
                    &child_scope,
                    slot,
                    components,
                    output,
                    component_scopes,
                    inspect_all,
                )?;
            }
            ViewNode::Lazy {
                dependency,
                keys: _,
                binding,
                id,
                child,
                span,
            } => {
                if inspect_all {
                    record(scope, id, env, document, span, output)?;
                }
                let mut child_env = ScopedTypeEnv::new(env);
                child_env.insert(binding.clone(), expr_type(dependency, env, document, span)?);
                let child_scope = scoped(scope, id, env, document, span)?;
                collect(
                    child,
                    &child_env,
                    document,
                    &child_scope,
                    slot,
                    components,
                    output,
                    component_scopes,
                    inspect_all,
                )?;
            }
            ViewNode::Tooltip {
                id,
                content,
                tip,
                span,
                ..
            } => {
                if inspect_all {
                    record(scope, id, env, document, span, output)?;
                }
                let child_scope = scoped(scope, id, env, document, span)?;
                collect(
                    content,
                    env,
                    document,
                    &child_scope,
                    slot,
                    components,
                    output,
                    component_scopes,
                    inspect_all,
                )?;
                collect(
                    tip,
                    env,
                    document,
                    &child_scope,
                    slot,
                    components,
                    output,
                    component_scopes,
                    inspect_all,
                )?;
            }
            ViewNode::Overlay {
                id,
                content,
                layer,
                span,
                ..
            } => {
                if inspect_all {
                    record(scope, id, env, document, span, output)?;
                }
                let child_scope = scoped(scope, id, env, document, span)?;
                collect(
                    content,
                    env,
                    document,
                    &child_scope,
                    slot,
                    components,
                    output,
                    component_scopes,
                    inspect_all,
                )?;
                collect(
                    layer,
                    env,
                    document,
                    &child_scope,
                    slot,
                    components,
                    output,
                    component_scopes,
                    inspect_all,
                )?;
            }
            ViewNode::PaneGrid {
                name,
                panes,
                templates,
                ..
            } => {
                let mut grid_scope = scope.clone();
                grid_scope.push((name.clone(), None));
                if inspect_all && !output.contains(&grid_scope) {
                    output.push(grid_scope.clone());
                }
                for pane in panes {
                    let mut pane_env = ScopedTypeEnv::new(env);
                    if let Some(binding) = &pane.maximized {
                        pane_env.insert(binding.clone(), Type::Bool);
                    }
                    let mut pane_scope = grid_scope.clone();
                    pane_scope.push((pane.name.clone(), None));
                    for node in pane.nodes() {
                        collect(
                            node,
                            &pane_env,
                            document,
                            &pane_scope,
                            slot,
                            components,
                            output,
                            component_scopes,
                            inspect_all,
                        )?;
                    }
                }
                for template in templates {
                    let Type::List(item_type) = env
                        .get_type(&template.items)
                        .expect("checker validates dynamic pane state")
                    else {
                        unreachable!("checker validates dynamic pane lists")
                    };
                    let mut template_env = ScopedTypeEnv::new(env);
                    template_env.insert(template.item.clone(), (**item_type).clone());
                    if let Some(binding) = &template.pane.maximized {
                        template_env.insert(binding.clone(), Type::Bool);
                    }
                    let mut pane_scope = grid_scope.clone();
                    pane_scope.push((
                        template.item.clone(),
                        Some(expr_type(
                            &template.key,
                            &template_env,
                            document,
                            &template.span,
                        )?),
                    ));
                    for node in template.pane.nodes() {
                        collect(
                            node,
                            &template_env,
                            document,
                            &pane_scope,
                            slot,
                            components,
                            output,
                            component_scopes,
                            inspect_all,
                        )?;
                    }
                }
            }
            ViewNode::Table {
                item,
                rows,
                id,
                columns,
                span,
                ..
            } => {
                if inspect_all {
                    record(scope, id, env, document, span, output)?;
                }
                let child_scope = scoped(scope, id, env, document, span)?;
                let Type::List(inner) = expr_type(rows, env, document, span)? else {
                    unreachable!("checker validates table rows")
                };
                let mut cell_env = ScopedTypeEnv::new(env);
                cell_env.insert(item.clone(), *inner);
                for column in columns {
                    let mut header_scope = child_scope.clone();
                    header_scope.push(("header".into(), Some(Type::I64)));
                    collect(
                        &column.header,
                        env,
                        document,
                        &header_scope,
                        slot,
                        components,
                        output,
                        component_scopes,
                        inspect_all,
                    )?;
                    let mut cell_scope = child_scope.clone();
                    cell_scope.push(("row".into(), Some(Type::I64)));
                    cell_scope.push(("col".into(), Some(Type::I64)));
                    collect(
                        &column.cell,
                        &cell_env,
                        document,
                        &cell_scope,
                        slot,
                        components,
                        output,
                        component_scopes,
                        inspect_all,
                    )?;
                }
            }
            ViewNode::Component {
                name,
                id,
                slots,
                span,
                ..
            } => {
                let Some(id) = id else {
                    return Ok(());
                };
                let call = (name.clone(), span.clone());
                if components.contains(&call) {
                    return Err(Error::new(
                        "E122",
                        span,
                        format!("recursive component `{name}` cannot define widget targets"),
                    ));
                }
                let component = document
                    .components
                    .iter()
                    .find(|component| component.name == *name)
                    .expect("checker validates component names");
                let mut component_scope = scope.clone();
                component_scope.push(segment(id, env, document, span)?);
                if inspect_all && !component_scopes.contains(&component_scope) {
                    component_scopes.push(component_scope.clone());
                }
                let mut component_env: HashMap<String, Type> = component
                    .params
                    .iter()
                    .map(|param| (param.name.clone(), param.ty.clone()))
                    .collect();
                component_env.extend(
                    component
                        .states
                        .iter()
                        .map(|state| (state.name.clone(), state.ty.clone())),
                );
                let component_slot = (!slots.is_empty()).then(|| WidgetIdSlot {
                    entries: slots
                        .iter()
                        .map(|slot| (slot.name.clone(), (*slot.content).clone(), env.snapshot()))
                        .collect(),
                    parent: slot.cloned().map(Box::new),
                });
                components.push(call);
                collect(
                    &component.root,
                    &component_env,
                    document,
                    &component_scope,
                    component_slot.as_ref(),
                    components,
                    output,
                    component_scopes,
                    inspect_all,
                )?;
                components.pop();
            }
            ViewNode::Slot { name, .. } => {
                if let Some(slot) = slot
                    && let Some((_, content, content_env)) =
                        slot.entries.iter().find(|(entry, ..)| entry == name)
                {
                    collect(
                        content,
                        content_env,
                        document,
                        scope,
                        slot.parent.as_deref(),
                        components,
                        output,
                        component_scopes,
                        inspect_all,
                    )?;
                }
            }
            ViewNode::MouseArea {
                id, content, span, ..
            }
            | ViewNode::ResizeHandle {
                id, content, span, ..
            }
            | ViewNode::Theme {
                id, content, span, ..
            }
            | ViewNode::Float {
                id, content, span, ..
            }
            | ViewNode::Pin {
                id, content, span, ..
            }
            | ViewNode::Sensor {
                id, content, span, ..
            } => {
                if inspect_all {
                    record(scope, id, env, document, span, output)?;
                }
                let child_scope = scoped(scope, id, env, document, span)?;
                collect(
                    content,
                    env,
                    document,
                    &child_scope,
                    slot,
                    components,
                    output,
                    component_scopes,
                    inspect_all,
                )?;
            }
            ViewNode::Responsive {
                id, content, span, ..
            } => {
                if inspect_all {
                    record(scope, id, env, document, span, output)?;
                }
                let child_scope = scoped(scope, id, env, document, span)?;
                match content {
                    ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                        collect(
                            narrow,
                            env,
                            document,
                            &child_scope,
                            slot,
                            components,
                            output,
                            component_scopes,
                            inspect_all,
                        )?;
                        collect(
                            wide,
                            env,
                            document,
                            &child_scope,
                            slot,
                            components,
                            output,
                            component_scopes,
                            inspect_all,
                        )?;
                    }
                    ResponsiveContent::Size {
                        width,
                        height,
                        content,
                    } => {
                        let mut child_env = ScopedTypeEnv::new(env);
                        child_env.insert(width.clone(), Type::F64);
                        child_env.insert(height.clone(), Type::F64);
                        collect(
                            content,
                            &child_env,
                            document,
                            &child_scope,
                            slot,
                            components,
                            output,
                            component_scopes,
                            inspect_all,
                        )?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    let mut output = Vec::new();
    let mut component_scopes = Vec::new();
    collect(
        root,
        env,
        document,
        &Vec::new(),
        None,
        &mut Vec::new(),
        &mut output,
        &mut component_scopes,
        inspect_all,
    )?;
    Ok(TestWidgetIds {
        targets: output,
        component_scopes,
    })
}

pub(in crate::check) fn unscoped_component_widget_warnings(
    document: &Document,
    reachable_components: &HashSet<String>,
) -> Vec<Warning> {
    fn visit(node: &ViewNode, target_counts: &HashMap<String, usize>, warnings: &mut Vec<Warning>) {
        match node {
            ViewNode::Component {
                name,
                id,
                slots,
                span,
                ..
            } => {
                let count = target_counts.get(name).copied().unwrap_or_default();
                if id.is_none() && count > 0 {
                    let noun = if count == 1 { "ID" } else { "IDs" };
                    warnings.push(
                        Warning::new(
                            "W015",
                            span,
                            format!(
                                "component `{name}` is mounted without an ID, so its {count} widget {noun} cannot be targeted from the caller"
                            ),
                        )
                        .hint(
                            "add an explicit `#id` to the component call to expose those widget targets under that scope",
                        ),
                    );
                }
                for slot in slots {
                    visit(&slot.content, target_counts, warnings);
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    visit(child, target_counts, warnings);
                }
            }
            ViewNode::Match { arms, .. } => {
                for child in arms.iter().flat_map(|arm| &arm.children) {
                    visit(child, target_counts, warnings);
                }
            }
            ViewNode::Container { content, .. }
            | ViewNode::Lazy { child: content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::MouseArea { content, .. }
            | ViewNode::ResizeHandle { content, .. }
            | ViewNode::KeyedColumn { child: content, .. } => {
                visit(content, target_counts, warnings);
            }
            ViewNode::Button {
                content: Some(content),
                ..
            } => visit(content, target_counts, warnings),
            ViewNode::Overlay { content, layer, .. } => {
                visit(content, target_counts, warnings);
                visit(layer, target_counts, warnings);
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for child in panes
                    .iter()
                    .flat_map(PaneView::nodes)
                    .chain(templates.iter().flat_map(|template| template.pane.nodes()))
                {
                    visit(child, target_counts, warnings);
                }
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    visit(&column.header, target_counts, warnings);
                    visit(&column.cell, target_counts, warnings);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                visit(content, target_counts, warnings);
                visit(tip, target_counts, warnings);
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    visit(narrow, target_counts, warnings);
                    visit(wide, target_counts, warnings);
                }
                ResponsiveContent::Size { content, .. } => {
                    visit(content, target_counts, warnings);
                }
            },
            _ => {}
        }
    }

    let target_counts = document
        .components
        .iter()
        .map(|component| {
            let mut env: HashMap<String, Type> = component
                .params
                .iter()
                .map(|param| (param.name.clone(), param.ty.clone()))
                .collect();
            env.extend(
                component
                    .states
                    .iter()
                    .map(|state| (state.name.clone(), state.ty.clone())),
            );
            (
                component.name.clone(),
                widget_operation_ids(&component.root, &env, document).map_or(0, |ids| ids.len()),
            )
        })
        .collect::<HashMap<_, _>>();

    let mut warnings = Vec::new();
    visit(&document.view, &target_counts, &mut warnings);
    for component in document
        .components
        .iter()
        .filter(|component| reachable_components.contains(&component.name))
    {
        visit(&component.root, &target_counts, &mut warnings);
    }
    for mount in document.tests.iter().filter_map(|test| test.mount.as_ref()) {
        visit(mount, &target_counts, &mut warnings);
    }
    warnings
}

fn widget_path_label(path: &WidgetIdPath) -> String {
    format!(
        "#{}",
        path.iter()
            .map(|(name, key)| if key.is_some() {
                format!("{name}(key)")
            } else {
                name.clone()
            })
            .collect::<Vec<_>>()
            .join("/")
    )
}

fn edit_distance(left: &str, right: &str) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];
    for (left_index, left_byte) in left.bytes().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_byte) in right.bytes().enumerate() {
            current[right_index + 1] = if left_byte == right_byte {
                previous[right_index]
            } else {
                1 + previous[right_index]
                    .min(previous[right_index + 1])
                    .min(current[right_index])
            };
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

fn unknown_widget_target_hint(label: &str, operation_ids: &[WidgetIdPath]) -> String {
    let mut candidates = operation_ids
        .iter()
        .map(widget_path_label)
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    candidates.sort_by_key(|candidate| (edit_distance(label, candidate), candidate.clone()));
    candidates.truncate(3);
    if candidates.is_empty() {
        return "use the full component, layout, keyed, table, or pane identity path from the app view"
            .into();
    }
    format!(
        "nearest valid widget targets: {}",
        candidates
            .iter()
            .map(|candidate| format!("`{candidate}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(in crate::check) fn check_widget_target(
    target: &WidgetTarget,
    env: &dyn ExprTypeEnv,
    document: &Document,
    operation_ids: &[WidgetIdPath],
    span: &Span,
) -> Result<(), Error> {
    let mut actual = Vec::with_capacity(target.segments.len());
    for segment in &target.segments {
        let key = segment
            .key
            .as_ref()
            .map(|key| expr_type(key, env, document, span))
            .transpose()?;
        if let Some(key) = &key
            && !matches!(key, Type::Bool | Type::I64 | Type::F64 | Type::Str)
        {
            return Err(Error::new(
                "E172",
                span,
                "widget target keys must be bool, i64, f64, or str",
            ));
        }
        actual.push((segment.name.clone(), key));
    }
    if let Some(window) = &target.window {
        let ty = expr_type(window, env, document, span)?;
        if !matches!(ty, Type::WindowId) {
            return Err(Error::new(
                "E172",
                span,
                format!(
                    "a `window=` qualifier takes a window-id, found {}",
                    ty.display()
                ),
            ));
        }
    }
    if operation_ids.iter().any(|expected| {
        expected.len() == actual.len()
            && expected
                .iter()
                .zip(&actual)
                .all(|((expected_name, expected_key), (name, key))| {
                    expected_name == name
                        && match (expected_key, key) {
                            (None, None) => true,
                            (Some(expected), Some(actual)) => compatible(expected, actual),
                            _ => false,
                        }
                })
    }) {
        return Ok(());
    }
    let label = format!(
        "#{}",
        target
            .segments
            .iter()
            .map(|segment| if segment.key.is_some() {
                format!("{}(key)", segment.name)
            } else {
                segment.name.clone()
            })
            .collect::<Vec<_>>()
            .join("/")
    );
    let same_shape = operation_ids
        .iter()
        .filter(|expected| {
            expected.len() == actual.len()
                && expected.iter().zip(&actual).all(
                    |((expected_name, expected_key), (name, key))| {
                        expected_name == name && expected_key.is_some() == key.is_some()
                    },
                )
        })
        .collect::<Vec<_>>();
    let mismatch = (!same_shape.is_empty())
        .then(|| {
            (0..actual.len()).find(|index| {
                let Some(actual) = &actual[*index].1 else {
                    return false;
                };
                same_shape.iter().all(|path| {
                    path[*index]
                        .1
                        .as_ref()
                        .is_some_and(|expected| !compatible(expected, actual))
                })
            })
        })
        .flatten();
    if let Some(index) = mismatch {
        let expected = same_shape
            .iter()
            .filter_map(|path| path[index].1.as_ref())
            .map(Type::display)
            .collect::<HashSet<_>>();
        return Err(Error::new(
            "E172",
            span,
            format!(
                "widget target segment `{}` expects key type {}, got `{}`",
                actual[index].0,
                expected
                    .into_iter()
                    .map(|ty| format!("`{ty}`"))
                    .collect::<Vec<_>>()
                    .join(" or "),
                actual[index].1.as_ref().unwrap().display()
            ),
        ));
    }
    Err(
        Error::new("E172", span, format!("unknown app widget target `{label}`"))
            .hint(unknown_widget_target_hint(&label, operation_ids)),
    )
}

pub(in crate::check) fn widget_selector_output(
    selector: &WidgetSelector,
    document: &Document,
    span: &Span,
) -> Result<Type, Error> {
    match selector {
        WidgetSelector::Extern { function, .. } => {
            Ok(
                extern_function(document, function, ExternKind::Selector, span)?
                    .output
                    .clone(),
            )
        }
        WidgetSelector::Id(_)
        | WidgetSelector::Text(_)
        | WidgetSelector::Point { .. }
        | WidgetSelector::Focused => Ok(Type::WidgetTarget),
    }
}

pub(in crate::check) fn check_widget_selector(
    selector: &WidgetSelector,
    env: &dyn ExprTypeEnv,
    document: &Document,
    operation_ids: &[WidgetIdPath],
    span: &Span,
) -> Result<Type, Error> {
    match selector {
        WidgetSelector::Id(target) => {
            check_widget_target(target, env, document, operation_ids, span)?;
        }
        WidgetSelector::Text(value) => {
            require_type(&expr_type(value, env, document, span)?, &Type::Str, span)?;
        }
        WidgetSelector::Point { x, y } => {
            for value in [x, y] {
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
            }
        }
        WidgetSelector::Focused => {}
        WidgetSelector::Extern { function, args } => {
            let function = extern_function(document, function, ExternKind::Selector, span)?;
            check_call_args(function, args, env, document, span)?;
        }
    }
    widget_selector_output(selector, document, span)
}

pub(in crate::check) struct PaneGridNames {
    pub(in crate::check) panes: HashSet<String>,
    pub(in crate::check) templates: HashMap<String, Type>,
    pub(in crate::check) splits: HashSet<String>,
}

pub(in crate::check) fn pane_split_names(
    configuration: &PaneConfiguration,
    output: &mut HashSet<String>,
) {
    if let PaneConfiguration::Split { name, a, b, .. } = configuration {
        if let Some(name) = name {
            output.insert(name.clone());
        }
        pane_split_names(a, output);
        pane_split_names(b, output);
    }
}

pub(in crate::check) fn static_pane_grids(
    root: &ViewNode,
    states: &HashMap<String, Type>,
    document: &Document,
) -> Result<HashMap<String, PaneGridNames>, Error> {
    fn collect(
        node: &ViewNode,
        states: &HashMap<String, Type>,
        document: &Document,
        output: &mut HashMap<String, PaneGridNames>,
    ) -> Result<(), Error> {
        match node {
            ViewNode::PaneGrid {
                name,
                configuration,
                panes,
                templates,
                span,
                ..
            } => {
                let mut splits = HashSet::new();
                pane_split_names(configuration, &mut splits);
                let mut template_types = HashMap::new();
                for template in templates {
                    let Some(Type::List(item_type)) = states.get(&template.items) else {
                        return Err(Error::new(
                            "E187",
                            &template.span,
                            format!(
                                "dynamic pane template `{}` requires list state `{}`",
                                template.item, template.items
                            ),
                        ));
                    };
                    let mut env = states.clone();
                    env.insert(template.item.clone(), (**item_type).clone());
                    template_types.insert(
                        template.item.clone(),
                        expr_type(&template.key, &env, document, &template.span)?,
                    );
                }
                if output
                    .insert(
                        name.clone(),
                        PaneGridNames {
                            panes: panes.iter().map(|pane| pane.name.clone()).collect(),
                            templates: template_types,
                            splits,
                        },
                    )
                    .is_some()
                {
                    return Err(Error::new(
                        "E187",
                        span,
                        format!("duplicate persistent panes `#{name}`"),
                    ));
                }
                for pane in panes {
                    for node in pane.nodes() {
                        collect(node, states, document, output)?;
                    }
                }
                for template in templates {
                    for node in template.pane.nodes() {
                        collect(node, states, document, output)?;
                    }
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, states, document, output)?;
                }
            }
            ViewNode::Match { arms, .. } => {
                for child in arms.iter().flat_map(|arm| &arm.children) {
                    collect(child, states, document, output)?;
                }
            }
            ViewNode::Tooltip { content, tip, .. }
            | ViewNode::Overlay {
                content,
                layer: tip,
                ..
            } => {
                collect(content, states, document, output)?;
                collect(tip, states, document, output)?;
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, states, document, output)?;
                    collect(&column.cell, states, document, output)?;
                }
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::ResizeHandle { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. }
            | ViewNode::Button {
                content: Some(content),
                ..
            } => collect(content, states, document, output)?,
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, states, document, output)?;
                }
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, states, document, output)?;
                    collect(wide, states, document, output)?;
                }
                ResponsiveContent::Size { content, .. } => {
                    collect(content, states, document, output)?
                }
            },
            _ => {}
        }
        Ok(())
    }
    let mut output = HashMap::new();
    collect(root, states, document, &mut output)?;
    Ok(output)
}
