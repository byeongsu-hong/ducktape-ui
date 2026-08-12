use super::*;

pub(in crate::check) fn infer_controls_group(
    node: &ViewNode,
    env: &dyn ExprTypeEnv,
    document: &Document,
    signatures: &mut HashMap<String, Vec<Option<Type>>>,
    ids: &mut HashSet<String>,
) -> Result<bool, Error> {
    match node {
        ViewNode::Button {
            label,
            id,
            disabled,
            options,
            content,
            styles,
            route,
            span,
            ..
        } => {
            check_id(id, env, document, ids, span)?;
            let button_analysis_guard = expr::HandlerAnalysisGuard::start();
            if let Some(disabled) = disabled {
                let ty = expr_type(disabled, env, document, span)?;
                require_type(&ty, &Type::Bool, span)?;
            }
            if let Some(checked) = &options.checked {
                require_type(&expr_type(checked, env, document, span)?, &Type::Bool, span)?;
            }
            check_accessibility_options(&options.accessibility, env, document, span)?;
            if label.is_none() && options.accessibility.label.is_none() {
                return Err(Error::new(
                    "E105",
                    span,
                    "a button with child content requires `label=...` for accessibility",
                ));
            }
            for length in [&options.width, &options.height].into_iter().flatten() {
                check_length_value(length, env, document, span, "button size")?;
            }
            if let Some(padding) = &options.padding {
                require_nonnegative_f64(padding, env, document, "button padding", span)?;
            }
            if let Some(clip) = &options.clip {
                require_type(&expr_type(clip, env, document, span)?, &Type::Bool, span)?;
            }
            if let Some(style) = &options.style.custom {
                let function =
                    extern_function(document, &style.function, ExternKind::ButtonStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            for status in [
                &options.style.active,
                &options.style.hovered,
                &options.style.pressed,
                &options.style.disabled,
            ]
            .into_iter()
            .flatten()
            {
                check_container_style_options(
                    &status.options,
                    env,
                    document,
                    &status.span,
                    "E129",
                    false,
                )?;
            }
            infer_route(route, None, env, document, signatures)?;
            check_styles(styles, document, span, StyleTarget::Button(options))?;
            retain_interaction_analyses(span, button_analysis_guard.finish())?;
            if let Some(content) = content {
                infer_view(content, env, document, signatures, ids)?;
            }
        }
        ViewNode::Checkbox {
            label,
            id,
            checked,
            disabled,
            options,
            style,
            styles,
            route,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let interaction_analysis_guard = expr::HandlerAnalysisGuard::start();
            require_type(&expr_type(label, env, document, span)?, &Type::Str, span)?;
            require_type(&expr_type(checked, env, document, span)?, &Type::Bool, span)?;
            if let Some(disabled) = disabled {
                require_type(
                    &expr_type(disabled, env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            check_accessibility_options(&options.accessibility, env, document, span)?;
            check_bool_control_options(options, env, document, span)?;
            if let Some(style) = &style.custom {
                let function =
                    extern_function(document, &style.function, ExternKind::CheckboxStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_checkbox_styles(style, env, document, span)?;
            infer_route(route, Some(Type::Bool), env, document, signatures)?;
            check_styles(styles, document, span, StyleTarget::Checkbox)?;
            retain_interaction_analyses(span, interaction_analysis_guard.finish())?;
        }
        ViewNode::Toggler {
            label,
            id,
            checked,
            disabled,
            options,
            style,
            styles,
            route,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let interaction_analysis_guard = expr::HandlerAnalysisGuard::start();
            require_type(&expr_type(label, env, document, span)?, &Type::Str, span)?;
            require_type(&expr_type(checked, env, document, span)?, &Type::Bool, span)?;
            if let Some(disabled) = disabled {
                require_type(
                    &expr_type(disabled, env, document, span)?,
                    &Type::Bool,
                    span,
                )?;
            }
            check_accessibility_options(&options.accessibility, env, document, span)?;
            check_bool_control_options(options, env, document, span)?;
            if let Some(style) = &style.custom {
                let function =
                    extern_function(document, &style.function, ExternKind::TogglerStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_toggler_styles(style, env, document, span)?;
            infer_route(route, Some(Type::Bool), env, document, signatures)?;
            check_styles(styles, document, span, StyleTarget::Toggler)?;
            retain_interaction_analyses(span, interaction_analysis_guard.finish())?;
        }
        ViewNode::Slider {
            value,
            id,
            min,
            max,
            step,
            options,
            vertical,
            styles,
            route,
            release,
            span,
            ..
        } => {
            check_id(id, env, document, ids, span)?;
            let interaction_analysis_guard = expr::HandlerAnalysisGuard::start();
            let value_type = expr_type(value, env, document, span)?;
            if !matches!(&value_type, Type::F64 | Type::Named(_)) {
                return Err(Error::new(
                    "E125",
                    span,
                    "slider values must be f64 or an extern numeric type",
                ));
            }
            for expr in [min, max, step] {
                require_type(&expr_type(expr, env, document, span)?, &value_type, span)?;
            }
            for expr in [&options.default, &options.shift_step]
                .into_iter()
                .flatten()
            {
                require_type(&expr_type(expr, env, document, span)?, &value_type, span)?;
            }
            if value_type == Type::F64 {
                require_literal_range(step, f64::EPSILON, None, "slider step", span)?;
                if let Some(shift_step) = &options.shift_step {
                    require_literal_range(
                        shift_step,
                        f64::EPSILON,
                        None,
                        "slider shift step",
                        span,
                    )?;
                }
                if let (Some(min), Some(max)) = (f64_literal(min), f64_literal(max))
                    && min > max
                {
                    return Err(Error::new("E128", span, "slider min cannot exceed max"));
                }
                if let Some(default) = options.default.as_ref().and_then(f64_literal)
                    && (f64_literal(min).is_some_and(|min| default < min)
                        || f64_literal(max).is_some_and(|max| default > max))
                {
                    return Err(Error::new(
                        "E128",
                        span,
                        "slider default is outside its range",
                    ));
                }
            }
            for (length, fluid, label) in [
                (&options.width, !*vertical, "slider width"),
                (&options.height, *vertical, "slider height"),
            ] {
                if let Some(length) = length {
                    match length {
                        LengthValue::Fixed(value) if !fluid => {
                            require_nonnegative_f64(value, env, document, label, span)?;
                        }
                        LengthValue::Fixed(_) => {
                            check_length_value(length, env, document, span, label)?;
                        }
                        _ if !fluid => {
                            return Err(Error::new(
                                "E129",
                                span,
                                format!("{label} must be fixed on this axis"),
                            ));
                        }
                        _ => {}
                    }
                }
            }
            if let Some(style) = &options.style.custom {
                let function =
                    extern_function(document, &style.function, ExternKind::SliderStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_slider_styles(&options.style, env, document, span)?;
            infer_route(route, Some(value_type), env, document, signatures)?;
            if let Some(release) = release {
                infer_route(release, None, env, document, signatures)?;
            }
            check_styles(styles, document, span, StyleTarget::Slider)?;
            retain_interaction_analyses(span, interaction_analysis_guard.finish())?;
        }
        ViewNode::Progress {
            value,
            id,
            min,
            max,
            options,
            styles,
            span,
            ..
        } => {
            check_id(id, env, document, ids, span)?;
            let interaction_analysis_guard = expr::HandlerAnalysisGuard::start();
            for expr in [value, min, max] {
                require_f32_value(expr, env, document, "progress value", span)?;
            }
            if let (Some(min), Some(max)) = (f64_literal(min), f64_literal(max))
                && min > max
            {
                return Err(Error::new("E128", span, "progress min cannot exceed max"));
            }
            for length in [&options.length, &options.girth].into_iter().flatten() {
                check_length_value(length, env, document, span, "progress size")?;
            }
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::ProgressStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            for (background, label) in [
                (&options.background, "progress background"),
                (&options.bar, "progress bar"),
            ] {
                if let Some(background) = background {
                    check_background_value(background, env, document, span, "E129", label)?;
                }
            }
            if let Some(color) = &options.border_color {
                require_theme_color(color, document, span, "E129", "progress")?;
            }
            for (value, label) in [
                (&options.border_width, "progress border width"),
                (&options.radius, "progress radius"),
                (&options.radius_top_left, "progress radius"),
                (&options.radius_top_right, "progress radius"),
                (&options.radius_bottom_right, "progress radius"),
                (&options.radius_bottom_left, "progress radius"),
            ] {
                if let Some(value) = value {
                    require_nonnegative_f64(value, env, document, label, span)?;
                }
            }
            check_styles(styles, document, span, StyleTarget::Progress)?;
            retain_interaction_analyses(span, interaction_analysis_guard.finish())?;
        }
        ViewNode::Radio {
            label,
            id,
            value,
            selected,
            options,
            style,
            styles,
            route,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let interaction_analysis_guard = expr::HandlerAnalysisGuard::start();
            require_type(&expr_type(label, env, document, span)?, &Type::Str, span)?;
            let value_type = expr_type(value, env, document, span)?;
            if !matches!(
                value_type,
                Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Named(_)
            ) {
                return Err(Error::new(
                    "E125",
                    span,
                    "radio values must be bool, i64, f64, str, or an extern type",
                ));
            }
            require_type(
                &expr_type(selected, env, document, span)?,
                &Type::Bool,
                span,
            )?;
            check_bool_control_options(options, env, document, span)?;
            if let Some(style) = &style.custom {
                let function =
                    extern_function(document, &style.function, ExternKind::RadioStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_radio_styles(style, env, document, span)?;
            infer_route(route, Some(value_type), env, document, signatures)?;
            check_styles(styles, document, span, StyleTarget::Radio)?;
            retain_interaction_analyses(span, interaction_analysis_guard.finish())?;
        }
        ViewNode::PickList {
            options,
            id,
            selected,
            options_config,
            route,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let selection_analysis_guard = expr::HandlerAnalysisGuard::start();
            let Type::List(option_type) = expr_type(options, env, document, span)? else {
                return Err(Error::new("E129", span, "pick options must be a list"));
            };
            let Type::Option(selected_type) = expr_type(selected, env, document, span)? else {
                return Err(Error::new(
                    "E129",
                    span,
                    "pick selection must use an optional `T?` value",
                ));
            };
            require_type(&option_type, &selected_type, span)?;
            if !matches!(
                option_type.as_ref(),
                Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Named(_)
            ) {
                return Err(Error::new(
                    "E129",
                    span,
                    "pick values must be bool, i64, f64, str, or an extern type",
                ));
            }
            if let Some(placeholder) = &options_config.placeholder {
                require_type(
                    &expr_type(placeholder, env, document, span)?,
                    &Type::Str,
                    span,
                )?;
            }
            for length in [&options_config.width, &options_config.menu_height]
                .into_iter()
                .flatten()
            {
                check_length_value(length, env, document, span, "pick size")?;
            }
            for (value, label) in [
                (&options_config.padding, "pick padding"),
                (&options_config.text_size, "pick text size"),
                (&options_config.line_height, "pick line height"),
            ] {
                if let Some(value) = value {
                    require_nonnegative_f64(value, env, document, label, span)?;
                }
            }
            check_font(options_config.font.as_ref(), document, span)?;
            check_pick_list_handle(options_config.handle.as_ref(), env, document, span)?;
            if let Some(style) = &options_config.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::PickListStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            if let Some(style) = &options_config.custom_menu_style {
                let function =
                    extern_function(document, &style.function, ExternKind::MenuStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_pick_list_styles(options_config, env, document, span)?;
            infer_route(route, Some(*option_type), env, document, signatures)?;
            for route in [&options_config.open, &options_config.close]
                .into_iter()
                .flatten()
            {
                infer_route(route, None, env, document, signatures)?;
            }
            retain_interaction_analyses(span, selection_analysis_guard.finish())?;
        }
        ViewNode::ComboBox {
            state,
            id,
            selected,
            options,
            route,
            span,
            ..
        } => {
            record_read(state, span);
            check_id(id, env, document, ids, span)?;
            let selection_analysis_guard = expr::HandlerAnalysisGuard::start();
            let Some(Type::Combo(option_type)) = env.get_type(state) else {
                return Err(Error::new(
                    "E129",
                    span,
                    format!("combo state `{state}` must have type `combo[T]`"),
                ));
            };
            let Type::Option(selected_type) = expr_type(selected, env, document, span)? else {
                return Err(Error::new(
                    "E129",
                    span,
                    "combo selection must use an optional `T?` value",
                ));
            };
            require_type(option_type, &selected_type, span)?;
            if !matches!(
                option_type.as_ref(),
                Type::Bool | Type::I64 | Type::F64 | Type::Str | Type::Named(_)
            ) {
                return Err(Error::new(
                    "E129",
                    span,
                    "combo values must be bool, i64, f64, str, or an extern type",
                ));
            }
            for length in [&options.width, &options.menu_height].into_iter().flatten() {
                check_length_value(length, env, document, span, "combo size")?;
            }
            for (value, label) in [
                (&options.padding, "combo padding"),
                (&options.text_size, "combo text size"),
                (&options.line_height, "combo line height"),
            ] {
                if let Some(value) = value {
                    require_nonnegative_f64(value, env, document, label, span)?;
                }
            }
            check_font(options.font.as_ref(), document, span)?;
            check_text_input_icon(options.icon.as_ref(), env, document, "combo")?;
            if let Some(style) = &options.custom_style {
                let function =
                    extern_function(document, &style.function, ExternKind::InputStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            if let Some(style) = &options.custom_menu_style {
                let function =
                    extern_function(document, &style.function, ExternKind::MenuStyle, span)?;
                check_call_args(function, &style.args, env, document, span)?;
            }
            check_text_input_styles(&options.style, env, document, span, "combo")?;
            check_menu_style(options.menu_style.as_deref(), env, document, span)?;
            for (route, payload, label) in [
                (Some(route), Some((**option_type).clone()), "selection"),
                (options.input.as_ref(), Some(Type::Str), "input"),
                (
                    options.hover.as_ref(),
                    Some((**option_type).clone()),
                    "hover",
                ),
            ] {
                if let Some(route) = route {
                    if route
                        .args
                        .iter()
                        .any(|arg| !matches!(arg, RouteArg::Payload))
                    {
                        return Err(Error::new(
                            "E129",
                            span,
                            format!("combo {label} routes only accept `_` payloads"),
                        ));
                    }
                    infer_route(route, payload, env, document, signatures)?;
                }
            }
            for route in [&options.open, &options.close].into_iter().flatten() {
                infer_route(route, None, env, document, signatures)?;
            }
            retain_interaction_analyses(span, selection_analysis_guard.finish())?;
        }
        ViewNode::Rule {
            thickness,
            id,
            options,
            styles,
            span,
            ..
        } => {
            check_id(id, env, document, ids, span)?;
            let interaction_analysis_guard = expr::HandlerAnalysisGuard::start();
            require_nonnegative_f64(thickness, env, document, "rule thickness", span)?;
            if let Some(RuleFill::Percent(percent)) = &options.fill {
                require_type(&expr_type(percent, env, document, span)?, &Type::F64, span)?;
                require_literal_range(percent, 0.0, Some(100.0), "rule percent", span)?;
            }
            if let Some(color) = &options.color {
                require_theme_color(color, document, span, "E129", "rule")?;
            }
            for radius in [
                &options.radius,
                &options.radius_top_left,
                &options.radius_top_right,
                &options.radius_bottom_right,
                &options.radius_bottom_left,
            ]
            .into_iter()
            .flatten()
            {
                require_nonnegative_f64(radius, env, document, "rule radius", span)?;
            }
            if let Some(snap) = &options.snap {
                require_type(&expr_type(snap, env, document, span)?, &Type::Bool, span)?;
            }
            check_styles(styles, document, span, StyleTarget::Rule)?;
            retain_interaction_analyses(span, interaction_analysis_guard.finish())?;
        }
        ViewNode::QrCode {
            payload,
            id,
            correction,
            version,
            cell_size,
            total_size,
            cell,
            background,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let interaction_analysis_guard = expr::HandlerAnalysisGuard::start();
            let ty = expr_type(payload, env, document, span)?;
            if !matches!(ty, Type::Str | Type::Bytes) {
                return Err(type_error(span, &Type::Str, &ty).hint("qr accepts str or bytes"));
            }
            check_qr_payload(payload, *correction, *version, span)?;
            for (value, label) in [
                (cell_size.as_ref(), "qr cell size"),
                (total_size.as_ref(), "qr total size"),
            ] {
                if let Some(value) = value {
                    require_nonnegative_f64(value, env, document, label, span)?;
                }
            }
            for (color, label) in [(cell, "cell"), (background, "background")] {
                if let Some(color) = color {
                    require_theme_color(color, document, span, "E136", &format!("qr {label}"))?;
                }
            }
            retain_interaction_analyses(span, interaction_analysis_guard.finish())?;
        }
        ViewNode::Space {
            id,
            width,
            height,
            styles,
            span,
        } => {
            check_id(id, env, document, ids, span)?;
            let interaction_analysis_guard = expr::HandlerAnalysisGuard::start();
            for length in [width, height].into_iter().flatten() {
                check_length_value(length, env, document, span, "space length")?;
            }
            check_styles(styles, document, span, StyleTarget::Space)?;
            retain_interaction_analyses(span, interaction_analysis_guard.finish())?;
        }
        _ => return Ok(false),
    };
    Ok(true)
}
