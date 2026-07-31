use super::*;

pub(in crate::codegen) fn render_foundation(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<Option<String>, Error> {
    let rendered = match node {
        ViewNode::Layout {
            kind,
            options,
            id,
            styles,
            children,
            span,
        } => render_layout(
            *kind, options, id, styles, children, span, document, message, env, scope, slot,
        ),
        ViewNode::Container {
            options,
            id,
            styles,
            content,
            span,
        } => render_container(
            options, id, styles, content, span, document, message, env, scope, slot,
        ),
        ViewNode::Overlay {
            id,
            options,
            content,
            layer,
            ..
        } => render_overlay(
            id, options, content, layer, document, message, env, scope, slot,
        ),
        ViewNode::PaneGrid {
            name,
            options,
            panes,
            templates,
            ..
        } => render_pane_grid(
            name, options, panes, templates, document, message, env, scope, slot,
        ),
        ViewNode::Text {
            value,
            id,
            options,
            styles,
            span,
        } => {
            let style = Style::parse(styles, document);
            let value = expr_code(value, env, document, ValueMode::Borrowed)?;
            let accessibility_key =
                accessibility_key_code(id.as_ref(), "text", span, scope, env, document)?;
            let code = text_code(options, &style, message, env, document)?;
            let selection = options.tracking.filter(|tracking| *tracking > 0.0).map_or(
                "let __text = ::ui_lang_runtime::selectable_text(__text);",
                |_| "",
            );
            Ok(format!(
                "{{ let __a11y_key = {accessibility_key}; let __text_value = ({value}).to_string(); let __text = {code}; {selection} ::ui_lang_runtime::accessible(__text, ::ui_lang_runtime::StableId::new(&__a11y_key), ::ui_lang_runtime::Role::Label).logical_id(__a11y_key.clone()).value(__text_value).into() }}"
            ))
        }
        ViewNode::RichText {
            id,
            options,
            color,
            spans,
            styles,
            route,
            ..
        } => render_rich_text(
            id, options, color, spans, styles, route, document, message, env, scope,
        ),
        ViewNode::Input {
            label,
            id,
            binding,
            hint,
            disabled,
            options,
            styles,
            span,
        } => {
            let style = Style::parse(styles, document);
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
                Some(&style),
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

/// Builds the widget behind `text`, reading `__text_value` from the scope the
/// caller opened.
///
/// Without `tracking=` that is a single iced text widget. With it, iced has no
/// letter spacing to set — `iced::advanced::text` carries no such field — so
/// the run is lowered to one text widget per grapheme inside a row whose
/// spacing is the tracking, and the properties that describe the run as a
/// whole (bounds and alignment) move to a container around that row.
fn text_code(
    options: &TextOptions,
    style: &Style,
    message: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let mut glyph = String::new();
    let Some(tracking) = options.tracking.filter(|tracking| *tracking > 0.0) else {
        glyph.push_str("::iced::widget::text(__text_value.clone())");
        append_glyph_options(&mut glyph, options, style, env, document)?;
        return Ok(glyph);
    };
    let mut run = options.clone();
    run.width = None;
    run.height = None;
    run.align_x = None;
    run.align_y = None;
    glyph.push_str("::iced::widget::text(__grapheme.to_owned())");
    append_glyph_options(&mut glyph, &run, style, env, document)?;
    let mut code = format!(
        "{{ let mut __tracked: ::std::vec::Vec<__IceElement<'_, {message}>> = ::std::vec::Vec::new(); for __grapheme in ::ui_lang_runtime::graphemes(&__text_value) {{ __tracked.push({glyph}.into()); }} let __spacing = ::ui_lang_runtime::bounded_spacing({}, __tracked.len()); let __run = ::iced::widget::row(__tracked).spacing(__spacing);",
        rust_f64(tracking)
    );
    let bounded = options.width.is_some()
        || options.height.is_some()
        || options.align_x.is_some()
        || options.align_y.is_some();
    if !bounded {
        code.push_str(" __run }");
        return Ok(code);
    }
    let mut wrapper = String::from("::iced::widget::container(__run)");
    append_dimensions(
        &mut wrapper,
        [&options.width, &options.height],
        env,
        document,
    )?;
    if let Some(alignment) = options.align_x {
        let alignment = match alignment {
            TextAlignment::Default | TextAlignment::Left => "Left",
            TextAlignment::Center => "Center",
            TextAlignment::Right => "Right",
            TextAlignment::Justified => unreachable!("checker rejects justified tracking"),
        };
        write!(
            wrapper,
            ".align_x(::iced::alignment::Horizontal::{alignment})"
        )
        .unwrap();
    }
    if let Some(alignment) = options.align_y {
        let alignment = match alignment {
            VerticalAlignment::Top => "Top",
            VerticalAlignment::Center => "Center",
            VerticalAlignment::Bottom => "Bottom",
        };
        write!(
            wrapper,
            ".align_y(::iced::alignment::Vertical::{alignment})"
        )
        .unwrap();
    }
    write!(code, " {wrapper} }}").unwrap();
    Ok(code)
}

fn append_glyph_options(
    code: &mut String,
    options: &TextOptions,
    style: &Style,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    append_text_options(code, options, style, env, document)?;
    if let Some(color) = &style.text_color {
        write!(code, ".color({})", theme_color(document, color)).unwrap();
    }
    Ok(())
}
