use super::*;

pub(in crate::check) fn infer_documents_group(
    node: &ViewNode,
    env: &dyn ExprTypeEnv,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<bool, Error> {
    match node {
        ViewNode::If {
            condition,
            children,
            span,
        } => {
            require_type(
                &retained_view_expr_type(
                    condition,
                    env,
                    document,
                    span,
                    CheckedViewExprRole::IfCondition,
                )?,
                &Type::Bool,
                span,
            )?;
            for child in children {
                infer_view(child, env, document, signatures, ids)?;
            }
        }
        ViewNode::For {
            item,
            items,
            children,
            span,
        } => {
            let Type::List(inner) =
                retained_view_expr_type(items, env, document, span, CheckedViewExprRole::ForItems)?
            else {
                return Err(Error::new("E121", span, "for expects a list expression"));
            };
            let mut child_env = scoped_view_env(env);
            child_env.insert(item.clone(), *inner);
            for child in children {
                infer_view(child, &child_env, document, signatures, ids)?;
            }
        }
        ViewNode::Match { value, arms, span } => {
            let value_ty = retained_view_expr_type(
                value,
                env,
                document,
                span,
                CheckedViewExprRole::MatchValue,
            )?;
            infer_match_arms(&value_ty, arms, env, document, signatures, ids, span)?;
        }
        ViewNode::KeyedColumn {
            item,
            items,
            key,
            id,
            options,
            child,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let Type::List(inner) = retained_view_expr_type(
                items,
                env,
                document,
                span,
                CheckedViewExprRole::KeyedItems,
            )?
            else {
                return Err(Error::new("E138", span, "keyed expects a list expression"));
            };
            let mut child_env = scoped_view_env(env);
            child_env.insert(item.clone(), *inner);
            let key_type = retained_view_expr_type(
                key,
                &child_env,
                document,
                span,
                CheckedViewExprRole::KeyedKey,
            )?;
            if !matches!(key_type, Type::Bool | Type::I64 | Type::F64) {
                return Err(Error::new(
                    "E138",
                    span,
                    "keyed keys must be copyable bool, i64, or f64 values",
                ));
            }
            for (length, role) in [
                (&options.width, CheckedViewExprRole::KeyedWidth),
                (&options.height, CheckedViewExprRole::KeyedHeight),
            ] {
                if let Some(LengthValue::Fixed(value)) = length {
                    let actual = retained_view_expr_type(value, env, document, span, role)?;
                    if !matches!(actual, Type::F64 | Type::Length) {
                        return Err(Error::new(
                            "E101",
                            span,
                            format!(
                                "expected `f64` or `length`, got `{}` for keyed size",
                                actual.display()
                            ),
                        ));
                    }
                    if actual == Type::F64 {
                        require_f32_literal_range(value, 0.0, None, "keyed size", span)?;
                    }
                }
            }
            for (value, role) in [
                (&options.spacing, CheckedViewExprRole::KeyedSpacing),
                (&options.padding.all, CheckedViewExprRole::KeyedPaddingAll),
                (&options.padding.x, CheckedViewExprRole::KeyedPaddingX),
                (&options.padding.y, CheckedViewExprRole::KeyedPaddingY),
                (&options.padding.top, CheckedViewExprRole::KeyedPaddingTop),
                (
                    &options.padding.right,
                    CheckedViewExprRole::KeyedPaddingRight,
                ),
                (
                    &options.padding.bottom,
                    CheckedViewExprRole::KeyedPaddingBottom,
                ),
                (&options.padding.left, CheckedViewExprRole::KeyedPaddingLeft),
                (&options.max_width, CheckedViewExprRole::KeyedMaxWidth),
            ] {
                if let Some(value) = value {
                    let actual = retained_view_expr_type(value, env, document, span, role)?;
                    require_type(&actual, &Type::F64, span)?;
                    require_f32_literal_range(value, 0.0, None, "keyed metric", span)?;
                }
            }
            infer_view(child, &child_env, document, signatures, ids)?;
        }
        ViewNode::Lazy {
            dependency,
            binding,
            id,
            child,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let dependency_type = retained_view_expr_type(
                dependency,
                env,
                document,
                span,
                CheckedViewExprRole::LazyDependency,
            )?;
            if !lazy_hashable(&dependency_type) || contains_ui_enum(&dependency_type, document) {
                return Err(Error::new(
                    "E139",
                    span,
                    format!(
                        "lazy dependency type `{}` does not implement stable hashing",
                        dependency_type.display()
                    ),
                )
                .hint("use bool, i64, str, an extern type with Hash + Clone, or a list/optional of those"));
            }
            check_lazy_subtree(child, document, &mut HashSet::new(), false)?;
            let child_env = HashMap::from([(binding.clone(), dependency_type)]);
            let mut child_ids = HashSet::new();
            infer_view(child, &child_env, document, signatures, &mut child_ids)?;
        }
        ViewNode::Markdown {
            content,
            id,
            options,
            route,
            span,
        } => {
            record_read(content, span);
            check_id(id, env, document, ids, span)?;
            let content_type = env.get_type(content).ok_or_else(|| {
                Error::new("E139", span, format!("unknown markdown state `{content}`"))
            })?;
            require_type(content_type, &Type::Markdown, span)?;
            for (value, label, min) in [
                (&options.text_size, "markdown text size", f64::EPSILON),
                (&options.h1_size, "markdown h1 size", f64::EPSILON),
                (&options.h2_size, "markdown h2 size", f64::EPSILON),
                (&options.h3_size, "markdown h3 size", f64::EPSILON),
                (&options.h4_size, "markdown h4 size", f64::EPSILON),
                (&options.h5_size, "markdown h5 size", f64::EPSILON),
                (&options.h6_size, "markdown h6 size", f64::EPSILON),
                (&options.code_size, "markdown code size", f64::EPSILON),
                (&options.spacing, "markdown spacing", 0.0),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_f32_literal_range(value, min, None, label, span)?;
                }
            }
            check_markdown_style(&options.style, env, document, span)?;
            let payload = if let Some(viewer) = &options.viewer {
                let function =
                    extern_function(document, &viewer.function, ExternKind::MarkdownViewer, span)?;
                check_call_args(function, &viewer.args, env, document, span)?;
                function.output.clone()
            } else {
                Type::Str
            };
            infer_route(route, Some(payload), env, document, signatures)?;
        }
        ViewNode::TextEditor {
            binding,
            id,
            disabled,
            options,
            span,
        } => {
            record_read(binding, span);
            record_write(binding, span);
            check_id(id, env, document, ids, span)?;
            let binding_type = env.get_type(binding).ok_or_else(|| {
                Error::new("E139", span, format!("unknown editor state `{binding}`"))
            })?;
            require_type(binding_type, &Type::Editor, span)?;
            if let Some(disabled) = disabled {
                require_type(
                    &expr_type(disabled, env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            for (value, label, min) in [
                (&options.width, "editor width", 0.0),
                (&options.min_height, "editor minimum height", 0.0),
                (&options.max_height, "editor maximum height", 0.0),
                (&options.size, "editor text size", f64::EPSILON),
                (&options.padding, "editor padding", 0.0),
            ] {
                if let Some(value) = value {
                    require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                    require_f32_literal_range(value, min, None, label, span)?;
                }
            }
            if let Some(length) = &options.height {
                check_length_value(length, env, document, span, "editor height")?;
            }
            if let Some(line_height) = &options.line_height {
                let value = match line_height {
                    TextLineHeight::Relative(value) | TextLineHeight::Absolute(value) => value,
                };
                require_type(&expr_type(value, env, document, span)?, &Type::F64, span)?;
                require_f32_literal_range(value, f64::EPSILON, None, "editor line height", span)?;
            }
            if let (Some(Expr::F64(min)), Some(Expr::F64(max))) =
                (&options.min_height, &options.max_height)
                && min > max
            {
                return Err(Error::new(
                    "E139",
                    span,
                    "editor min-height cannot exceed max-height",
                ));
            }
            check_font(options.font.as_ref(), document, span)?;
            if let Some(highlighter) = &options.highlighter {
                let function = extern_function(
                    document,
                    &highlighter.function,
                    ExternKind::EditorHighlighter,
                    span,
                )?;
                check_call_args(function, &highlighter.args, env, document, span)?;
            }
            if let Some(binding) = &options.key_binding {
                let function =
                    extern_function(document, &binding.function, ExternKind::EditorBinding, span)?;
                check_call_args(function, &binding.args, env, document, span)?;
                infer_route(
                    options
                        .key_binding_route
                        .as_ref()
                        .expect("parser requires a key-binding route"),
                    Some(function.output.clone()),
                    env,
                    document,
                    signatures,
                )?;
            }
            if let Some(action) = &options.action {
                let function =
                    extern_function(document, &action.function, ExternKind::EditorAction, span)?;
                check_call_args(function, &action.args, env, document, span)?;
            }
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::EditorStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_text_input_styles(&options.style, env, document, span, "editor")?;
        }
        ViewNode::Table {
            item,
            rows,
            id,
            options,
            columns,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let Type::List(inner) =
                retained_view_expr_type(rows, env, document, span, CheckedViewExprRole::TableRows)?
            else {
                return Err(Error::new("E139", span, "table expects a list of rows"));
            };
            if let Some(LengthValue::Fixed(value)) = &options.width {
                let actual = retained_view_expr_type(
                    value,
                    env,
                    document,
                    span,
                    CheckedViewExprRole::TableWidth,
                )?;
                if !matches!(actual, Type::F64 | Type::Length) {
                    return Err(Error::new(
                        "E101",
                        span,
                        format!(
                            "expected `f64` or `length`, got `{}` for table width",
                            actual.display()
                        ),
                    ));
                }
                if actual == Type::F64 {
                    require_f32_literal_range(value, 0.0, None, "table width", span)?;
                }
            }
            for (value, label, role) in [
                (
                    &options.padding,
                    "table padding",
                    CheckedViewExprRole::TablePadding,
                ),
                (
                    &options.padding_x,
                    "table horizontal padding",
                    CheckedViewExprRole::TablePaddingX,
                ),
                (
                    &options.padding_y,
                    "table vertical padding",
                    CheckedViewExprRole::TablePaddingY,
                ),
                (
                    &options.separator,
                    "table separator",
                    CheckedViewExprRole::TableSeparator,
                ),
                (
                    &options.separator_x,
                    "table horizontal separator",
                    CheckedViewExprRole::TableSeparatorX,
                ),
                (
                    &options.separator_y,
                    "table vertical separator",
                    CheckedViewExprRole::TableSeparatorY,
                ),
            ] {
                if let Some(value) = value {
                    let actual = retained_view_expr_type(value, env, document, span, role)?;
                    require_type(&actual, &Type::F64, span)?;
                    require_f32_literal_range(value, 0.0, None, label, span)?;
                }
            }
            let mut cell_env = scoped_view_env(env);
            cell_env.insert(item.clone(), *inner);
            for (index, column) in columns.iter().enumerate() {
                if let Some(LengthValue::Fixed(value)) = &column.width {
                    let actual = retained_view_expr_type_at(
                        value,
                        env,
                        document,
                        span,
                        &column.span,
                        CheckedViewExprRole::TableColumnWidth(index as u32),
                    )?;
                    if !matches!(actual, Type::F64 | Type::Length) {
                        return Err(Error::new(
                            "E101",
                            &column.span,
                            format!(
                                "expected `f64` or `length`, got `{}` for table column width",
                                actual.display()
                            ),
                        ));
                    }
                    if actual == Type::F64 {
                        require_f32_literal_range(
                            value,
                            0.0,
                            None,
                            "table column width",
                            &column.span,
                        )?;
                    }
                }
                let mut header_ids = HashSet::new();
                infer_view(&column.header, env, document, signatures, &mut header_ids)?;
                let mut cell_ids = HashSet::new();
                infer_view(&column.cell, &cell_env, document, signatures, &mut cell_ids)?;
            }
        }
        _ => return Ok(false),
    };
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn infer_match_arms(
    value_ty: &Type,
    arms: &[MatchArm],
    env: &dyn ExprTypeEnv,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
    span: &Span,
) -> Result<(), Error> {
    let mut covered = HashSet::new();
    let wildcard = arms
        .iter()
        .any(|arm| matches!(arm.pattern, MatchPattern::Wildcard));
    for arm in arms {
        let binding = match (value_ty, &arm.pattern) {
            (Type::Option(inner), MatchPattern::Some(name)) => {
                if !covered.insert("some".to_owned()) {
                    return Err(Error::new("E195", &arm.span, "duplicate `some` match arm"));
                }
                Some((name, inner.as_ref().clone()))
            }
            (Type::Option(_), MatchPattern::None) => {
                if !covered.insert("none".to_owned()) {
                    return Err(Error::new("E195", &arm.span, "duplicate `none` match arm"));
                }
                None
            }
            (Type::Result(output, _), MatchPattern::Ok(name)) => {
                if !covered.insert("ok".to_owned()) {
                    return Err(Error::new("E195", &arm.span, "duplicate `ok` match arm"));
                }
                Some((name, output.as_ref().clone()))
            }
            (Type::Result(_, error_ty), MatchPattern::Err(name)) => {
                if !covered.insert("err".to_owned()) {
                    return Err(Error::new("E195", &arm.span, "duplicate `err` match arm"));
                }
                Some((name, error_ty.as_ref().clone()))
            }
            (
                Type::Named(name),
                MatchPattern::Enum {
                    enum_name,
                    variant,
                    binding,
                },
            ) => {
                let item = document
                    .enums
                    .iter()
                    .find(|item| item.name == *name)
                    .ok_or_else(|| pattern_type_error(value_ty, &arm.span))?;
                if enum_name != name {
                    return Err(Error::new(
                        "E195",
                        &arm.span,
                        format!("expected `{name}` pattern, got `{enum_name}.{variant}`"),
                    ));
                }
                let declared = item
                    .variants
                    .iter()
                    .find(|declared| declared.name == *variant)
                    .ok_or_else(|| {
                        Error::new(
                            "E195",
                            &arm.span,
                            format!("unknown `{name}` variant `{variant}`"),
                        )
                    })?;
                if !covered.insert(variant.clone()) {
                    return Err(Error::new(
                        "E195",
                        &arm.span,
                        format!("duplicate `{name}.{variant}` match arm"),
                    ));
                }
                match (&declared.payload, binding) {
                    (Some(payload), Some(binding)) => Some((binding, payload.clone())),
                    (Some(_), None) => {
                        return Err(Error::new(
                            "E195",
                            &arm.span,
                            format!("`{name}.{variant}` pattern must bind its payload"),
                        ));
                    }
                    (None, Some(_)) => {
                        return Err(Error::new(
                            "E195",
                            &arm.span,
                            format!("`{name}.{variant}` has no payload to bind"),
                        ));
                    }
                    (None, None) => None,
                }
            }
            (
                Type::Palette(contract),
                MatchPattern::Enum {
                    enum_name,
                    variant,
                    binding,
                },
            ) => {
                if enum_name != contract {
                    return Err(Error::new(
                        "E195",
                        &arm.span,
                        format!(
                            "expected `{contract}` palette pattern, got `{enum_name}.{variant}`"
                        ),
                    ));
                }
                if binding.is_some() {
                    return Err(Error::new(
                        "E195",
                        &arm.span,
                        format!("`{contract}.{variant}` has no payload to bind"),
                    ));
                }
                if !document
                    .palettes
                    .iter()
                    .any(|palette| palette.name == *variant)
                {
                    return Err(Error::new(
                        "E195",
                        &arm.span,
                        format!("unknown `{contract}` palette `{variant}`"),
                    ));
                }
                if !covered.insert(variant.clone()) {
                    return Err(Error::new(
                        "E195",
                        &arm.span,
                        format!("duplicate `{contract}.{variant}` match arm"),
                    ));
                }
                None
            }
            (_, MatchPattern::Wildcard) => None,
            _ => return Err(pattern_type_error(value_ty, &arm.span)),
        };
        let mut child_env = scoped_view_env(env);
        if let Some((name, ty)) = binding {
            child_env.insert(name.clone(), ty);
        }
        for child in &arm.children {
            infer_view(child, &child_env, document, signatures, ids)?;
        }
    }

    if wildcard {
        return Ok(());
    }
    let missing = match value_ty {
        Type::Option(_) => ["some", "none"]
            .into_iter()
            .filter(|name| !covered.contains(*name))
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        Type::Result(_, _) => ["ok", "err"]
            .into_iter()
            .filter(|name| !covered.contains(*name))
            .map(str::to_owned)
            .collect(),
        Type::Named(name) => document
            .enums
            .iter()
            .find(|item| item.name == *name)
            .ok_or_else(|| pattern_type_error(value_ty, span))?
            .variants
            .iter()
            .filter(|variant| !covered.contains(&variant.name))
            .map(|variant| format!("{name}.{}", variant.name))
            .collect(),
        Type::Palette(contract) => document
            .palettes
            .iter()
            .filter(|palette| !covered.contains(&palette.name))
            .map(|palette| format!("{contract}.{}", palette.name))
            .collect(),
        _ => return Err(pattern_type_error(value_ty, span)),
    };
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::new(
            "E195",
            span,
            format!("non-exhaustive match; missing {}", missing.join(", ")),
        ))
    }
}

fn pattern_type_error(ty: &Type, span: &Span) -> Error {
    Error::new(
        "E195",
        span,
        format!(
            "typed match patterns require option, result, UI enum, or palette; got `{}`",
            ty.display()
        ),
    )
}
