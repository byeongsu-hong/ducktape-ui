use super::*;

pub(in crate::codegen) fn render_foundation(
    node: &ViewNode,
    document: &RenderDocument<'_>,
    message: &str,
    env: &dyn BindingEnvironment,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let rendered = match node {
        ViewNode::Layout { id, children, .. } => render_layout(
            document.hir().resolved_layout_for(node)?,
            id,
            children,
            document,
            message,
            env,
            scope,
            slot,
        ),
        ViewNode::Container { id, content, .. } => render_container(
            document.program().resolved_container_for(node)?,
            id,
            content,
            document,
            message,
            env,
            scope,
            slot,
        ),
        ViewNode::Overlay {
            id, content, layer, ..
        } => {
            let overlay = document.program().resolved_overlay_for(node)?;
            render_overlay(
                id, overlay, content, layer, document, message, env, scope, slot,
            )
        }
        ViewNode::PaneGrid {
            panes, templates, ..
        } => render_pane_grid(
            document.program().resolved_pane_grid_for(node)?,
            panes,
            templates,
            document,
            message,
            env,
            scope,
            slot,
        ),
        ViewNode::Text { id, .. } | ViewNode::RichText { id, .. } => render_text(
            document.hir().resolved_text_for(node)?,
            id,
            document,
            message,
            env,
            scope,
        ),
        ViewNode::Input {
            label,
            id,
            binding,
            hint,
            disabled,
            options,
            span,
            ..
        } => {
            let style = &document.program().style_use(span)?.style;
            let state = env.get(binding).ok_or_else(|| {
                Error::new("E150", span, format!("unknown input state `{binding}`"))
            })?;
            let binding_constructor = match &state.state {
                Some(StateBinding::App(name)) => {
                    let variant = binding_variant(name);
                    format!("{message}::{variant} as fn(::std::string::String) -> {message}")
                }
                Some(StateBinding::Component {
                    component,
                    name,
                    scope,
                }) => {
                    let variant = component_binding_variant(component, name);
                    format!(
                        "{{ let __scope = ({scope}).clone(); move |__value| {message}::{variant}(__scope.clone(), __value) }}"
                    )
                }
                None => {
                    return Err(Error::new(
                        "E139",
                        span,
                        "input binding must resolve to state",
                    ));
                }
            };
            let constructor = options
                .change
                .as_ref()
                .map(|route| {
                    route_callback_code(route, "__value", "__value", env, document, message)
                })
                .transpose()?
                .unwrap_or(binding_constructor);
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "input", span, scope, env, document)?;
            let (accessibility_label, accessibility_description) =
                accessibility_code(&options.accessibility, || rust_string(label), env, document)?;
            let disabled_value = disabled
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| "false".into());
            let secure_value = options
                .secure
                .as_ref()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| "false".into());
            let mut input = format!(
                "::iced::widget::text_input({}, &{})",
                rust_string(hint),
                state.code
            );
            input.push_str(".id(::iced::widget::Id::from(__a11y_key.clone()))");
            if let Some(padding) = style.padding_code() {
                write!(input, ".padding({padding})").unwrap();
            }
            if style.width_fill {
                input.push_str(".width(::iced::Fill)");
            }
            input.push_str(".secure(__secure)");
            if let Some(width) = &options.width {
                write!(input, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(padding) = &options.padding {
                write!(
                    input,
                    ".padding({} as f32)",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(size) = &options.text_size {
                write!(
                    input,
                    ".size({})",
                    clamped_f32_code(size, "f32::EPSILON", "f32::MAX", env, document)?
                )
                .unwrap();
            }
            if let Some(height) = &options.line_height {
                write!(
                    input,
                    ".line_height(::iced::widget::text::LineHeight::Relative({}))",
                    clamped_f32_code(height, "f32::EPSILON", "f32::MAX", env, document)?
                )
                .unwrap();
            }
            if let Some(align) = options.align {
                let align = match align {
                    InputAlignment::Left => "Left",
                    InputAlignment::Center => "Center",
                    InputAlignment::Right => "Right",
                };
                write!(input, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
            }
            if let Some(font) = &options.font {
                write!(input, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(icon) = &options.icon {
                write!(
                    input,
                    ".icon({})",
                    text_input_icon_code(icon, env, document)?
                )
                .unwrap();
            }
            write!(
                input,
                ".on_input_maybe(if __disabled {{ None }} else {{ Some({constructor}) }})"
            )
            .unwrap();
            if let Some(route) = &options.submit {
                let submit = route_code(route, "", env, document, message)?;
                write!(
                    input,
                    ".on_submit_maybe(if __disabled {{ None }} else {{ Some({submit}) }})"
                )
                .unwrap();
            }
            if let Some(route) = &options.paste {
                let paste =
                    route_callback_code(route, "__value", "__value", env, document, message)?;
                write!(
                    input,
                    ".on_paste_maybe(if __disabled {{ None }} else {{ Some({paste}) }})"
                )
                .unwrap();
            }
            input.push_str(&text_input_style_code(
                &options.style,
                options.custom_style.as_ref(),
                Some(style),
                env,
                document,
                "style",
                "text_input",
            )?);
            let view = if label.is_empty() {
                "__input.into()".to_owned()
            } else {
                format!(
                    "::iced::widget::column![::iced::widget::text({}), __input].spacing(6).into()",
                    rust_string(label)
                )
            };
            Ok(format!(
                "{{ let __a11y_key = {accessibility_key}; let __a11y_id = ::ui_lang_runtime::StableId::new(&__a11y_key); let __disabled = {disabled_value}; let __secure = {secure_value}; let __role = if __secure {{ ::ui_lang_runtime::Role::PasswordInput }} else {{ ::ui_lang_runtime::Role::TextInput }}; let __input = ::ui_lang_runtime::accessible({input}, __a11y_id, __role).logical_id(__a11y_key.clone()).focus_id(::iced::widget::Id::from(__a11y_key)).label({accessibility_label}).value_maybe((!__secure).then(|| ({}).clone())).disabled(__disabled){accessibility_description}; {view} }}",
                state.code,
            ))
        }
        _ => return Ok(None),
    }?;
    Ok(Some(rendered))
}
