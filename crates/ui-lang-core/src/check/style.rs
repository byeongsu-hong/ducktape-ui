use super::*;

pub(in crate::check) fn check_id(
    id: &Option<Id>,
    env: &dyn ExprTypeEnv,
    document: &Document,
    ids: &mut HashSet<String>,
    span: &Span,
) -> Result<(), Error> {
    let Some(id) = id else {
        return Ok(());
    };
    if let Some(key) = &id.key {
        let ty =
            retained_view_expr_type(key, env, document, span, CheckedViewExprRole::IdentityKey)?;
        if !matches!(ty, Type::I64 | Type::Str) {
            return Err(Error::new(
                "E160",
                span,
                "dynamic id keys must be i64 or str",
            ));
        }
    } else if !ids.insert(id.name.clone()) {
        return Err(Error::new(
            "E161",
            span,
            format!("duplicate local id `#{}`", id.name),
        ));
    }
    Ok(())
}

// Recipes expand in place before utility validation and lowering. Direct typed fields are applied
// after recipe defaults, while setting the same field through a typed property and a direct
// utility remains an error. font= and font-weight utilities compose, with the utility selecting
// the weight.
#[derive(Clone, Copy)]
pub(in crate::check) enum StyleTarget<'a> {
    Layout(Layout, &'a LayoutOptions),
    Container(&'a ContainerOptions),
    PaneContent(&'a ContainerStyleOptions),
    PaneTitle(&'a ContainerStyleOptions),
    Text(&'a TextOptions),
    RichText {
        typed_color: bool,
        typed_size: bool,
        typed_line_height: bool,
        typed_font: bool,
    },
    RichSpan(&'a RichSpanOptions),
    Input(&'a InputOptions),
    Button(&'a ButtonOptions),
    Checkbox,
    Toggler,
    Slider,
    Progress,
    Radio,
    Rule,
    Space,
}

pub(in crate::check) fn check_recipes(document: &Document) -> Result<(), Error> {
    for recipe in &document.recipes {
        let mut seen = HashSet::from([recipe.name.as_str()]);
        let mut current = recipe;
        while let Some(base_name) = &current.base {
            let base = document.style_recipe(base_name).ok_or_else(|| {
                Error::new(
                    "E046",
                    &recipe.span,
                    format!(
                        "recipe `{}` extends unknown recipe `{base_name}`",
                        recipe.name
                    ),
                )
            })?;
            if current.target != base.target {
                return Err(Error::new(
                    "E046",
                    &recipe.span,
                    format!(
                        "recipe `{}` targets `{}` but extends `{base_name}`, which targets `{}`",
                        current.name,
                        current.target.source_name(),
                        base.target.source_name()
                    ),
                ));
            }
            if !seen.insert(base.name.as_str()) {
                return Err(Error::new(
                    "E046",
                    &recipe.span,
                    format!("recipe inheritance cycle includes `{}`", base.name),
                ));
            }
            current = base;
        }
    }

    for recipe in &document.recipes {
        let utilities = document.expand_styles(std::slice::from_ref(&recipe.name));
        match recipe.target {
            StyleRecipeTarget::Column => check_styles(
                &utilities,
                document,
                &recipe.span,
                StyleTarget::Layout(Layout::Column, &LayoutOptions::default()),
            )?,
            StyleRecipeTarget::Row => check_styles(
                &utilities,
                document,
                &recipe.span,
                StyleTarget::Layout(Layout::Row, &LayoutOptions::default()),
            )?,
            StyleRecipeTarget::Flex => {
                let options = LayoutOptions {
                    flexbox: Some(FlexboxOptions::default()),
                    ..LayoutOptions::default()
                };
                check_styles(
                    &utilities,
                    document,
                    &recipe.span,
                    StyleTarget::Layout(Layout::Row, &options),
                )?;
            }
            StyleRecipeTarget::Grid => check_styles(
                &utilities,
                document,
                &recipe.span,
                StyleTarget::Layout(Layout::Grid, &LayoutOptions::default()),
            )?,
            StyleRecipeTarget::Stack => check_styles(
                &utilities,
                document,
                &recipe.span,
                StyleTarget::Layout(Layout::Stack, &LayoutOptions::default()),
            )?,
            StyleRecipeTarget::Container => check_styles(
                &utilities,
                document,
                &recipe.span,
                StyleTarget::Container(&ContainerOptions::default()),
            )?,
            StyleRecipeTarget::Text => check_styles(
                &utilities,
                document,
                &recipe.span,
                StyleTarget::Text(&TextOptions::default()),
            )?,
            StyleRecipeTarget::Input => check_styles(
                &utilities,
                document,
                &recipe.span,
                StyleTarget::Input(&InputOptions::default()),
            )?,
            StyleRecipeTarget::Button => check_styles(
                &utilities,
                document,
                &recipe.span,
                StyleTarget::Button(&ButtonOptions::default()),
            )?,
        }
    }
    Ok(())
}

fn style_target_name(target: StyleTarget<'_>) -> &'static str {
    match target {
        StyleTarget::Layout(_, options) if options.flexbox.is_some() => "flex",
        StyleTarget::Layout(Layout::Column, _) => "col",
        StyleTarget::Layout(Layout::Row, _) => "row",
        StyleTarget::Layout(Layout::Scroll, _) => "scroll",
        StyleTarget::Layout(Layout::Grid, _) => "grid",
        StyleTarget::Layout(Layout::Stack, _) => "stack",
        StyleTarget::Layout(Layout::Hover, _) => "hover",
        StyleTarget::Container(_) => "box",
        StyleTarget::PaneContent(_) => "pane",
        StyleTarget::PaneTitle(_) => "pane title",
        StyleTarget::Text(_) | StyleTarget::RichText { .. } | StyleTarget::RichSpan(_) => "text",
        StyleTarget::Input(_) => "input",
        StyleTarget::Button(_) => "button",
        StyleTarget::Checkbox => "checkbox",
        StyleTarget::Toggler => "toggler",
        StyleTarget::Slider => "slider",
        StyleTarget::Progress => "progress",
        StyleTarget::Radio => "radio",
        StyleTarget::Rule => "rule",
        StyleTarget::Space => "space",
    }
}

fn recipe_matches(target: StyleTarget<'_>, recipe: StyleRecipeTarget) -> bool {
    match recipe {
        StyleRecipeTarget::Column => matches!(
            target,
            StyleTarget::Layout(Layout::Column, options) if options.flexbox.is_none()
        ),
        StyleRecipeTarget::Row => matches!(
            target,
            StyleTarget::Layout(Layout::Row, options) if options.flexbox.is_none()
        ),
        StyleRecipeTarget::Flex => matches!(
            target,
            StyleTarget::Layout(_, options) if options.flexbox.is_some()
        ),
        StyleRecipeTarget::Grid => matches!(target, StyleTarget::Layout(Layout::Grid, _)),
        StyleRecipeTarget::Stack => matches!(target, StyleTarget::Layout(Layout::Stack, _)),
        StyleRecipeTarget::Container => matches!(target, StyleTarget::Container(_)),
        StyleRecipeTarget::Text => matches!(
            target,
            StyleTarget::Text(_) | StyleTarget::RichText { .. } | StyleTarget::RichSpan(_)
        ),
        StyleRecipeTarget::Input => matches!(target, StyleTarget::Input(_)),
        StyleRecipeTarget::Button => matches!(target, StyleTarget::Button(_)),
    }
}

pub(in crate::check) fn valid_theme_color(value: &str, document: &Document) -> bool {
    let (name, opacity) = value
        .split_once('/')
        .map_or((value, None), |(name, opacity)| (name, Some(opacity)));
    (["white", "black", "transparent"].contains(&name)
        || document
            .theme_contract
            .as_ref()
            .is_some_and(|contract| contract.tokens.iter().any(|token| token == name)))
        && opacity.is_none_or(|opacity| opacity.parse::<u8>().is_ok_and(|opacity| opacity <= 100))
}

pub(in crate::check) fn require_theme_color(
    value: &str,
    document: &Document,
    span: &Span,
    code: &'static str,
    label: &str,
) -> Result<(), Error> {
    if valid_theme_color(value, document) {
        Ok(())
    } else {
        Err(Error::new(
            code,
            span,
            format!("unknown {label} color `{value}`"),
        ))
    }
}

pub(in crate::check) fn check_styles(
    styles: &[String],
    document: &Document,
    span: &Span,
    target: StyleTarget<'_>,
) -> Result<(), Error> {
    let declared_styles = styles;
    for style in declared_styles {
        if let Some(recipe) = document.style_recipe(style)
            && !recipe_matches(target, recipe.target)
        {
            return Err(Error::new(
                "E046",
                span,
                format!(
                    "recipe `{}` targets `{}`, not `{}`",
                    recipe.name,
                    recipe.target.source_name(),
                    style_target_name(target)
                ),
            ));
        }
    }
    let expanded_styles = document.expand_styles(declared_styles);
    let styles = expanded_styles.as_slice();
    let spacing = [
        "0", "1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20", "24",
    ];
    let is_css_flex = matches!(
        target,
        StyleTarget::Layout(_, options) if options.flexbox.is_some()
    );
    let is_layout_box = matches!(
        target,
        StyleTarget::Layout(
            Layout::Column | Layout::Row | Layout::Grid | Layout::Stack,
            _
        )
    );
    let is_box = is_layout_box || matches!(target, StyleTarget::Container(_));
    let is_visual_box = is_box
        || matches!(
            target,
            StyleTarget::PaneContent(_) | StyleTarget::PaneTitle(_)
        );
    let target_name = style_target_name(target);

    for original in styles {
        let (variant, utility) = original
            .split_once(':')
            .map_or((None, original.as_str()), |(variant, utility)| {
                (Some(variant), utility)
            });
        let color = ["bg-", "border-"]
            .iter()
            .find_map(|prefix| utility.strip_prefix(prefix))
            .or_else(|| {
                is_text_color_utility(utility).then(|| {
                    utility
                        .strip_prefix("text-")
                        .expect("text color utility has a text prefix")
                })
            });
        let valid_color = color.is_some_and(|value| valid_theme_color(value, document));
        let valid_spacing = ["p-", "px-", "py-", "gap-"].iter().any(|prefix| {
            utility
                .strip_prefix(prefix)
                .is_some_and(|value| spacing.contains(&value) || exact_pixels(value).is_some())
        });
        let valid_radius = utility
            .strip_prefix("rounded-")
            .is_some_and(|value| exact_radius(value).is_some());
        let known = matches!(
            utility,
            "w-full"
                | "h-full"
                | "max-w-sm"
                | "max-w-md"
                | "max-w-lg"
                | "max-w-xl"
                | "max-w-2xl"
                | "items-center"
                | "self-center"
                | "overflow-hidden"
                | "text-xs"
                | "text-sm"
                | "text-base"
                | "text-lg"
                | "text-xl"
                | "text-2xl"
                | "leading-tight"
                | "leading-snug"
                | "leading-normal"
                | "leading-relaxed"
                | "font-mono"
                | "font-medium"
                | "font-semibold"
                | "font-bold"
                | "border"
                | "border-2"
                | "rounded-sm"
                | "rounded"
                | "rounded-md"
                | "rounded-lg"
                | "rounded-full"
        ) || valid_spacing
            || valid_radius
            || is_exact_text_size_utility(utility)
            || valid_color
            || utility
                .strip_prefix("opacity-")
                .is_some_and(|value| ["0", "25", "50", "75", "100"].contains(&value));

        if !known {
            return Err(Error::new(
                "E041",
                span,
                format!("unsupported utility `{original}`"),
            ));
        }

        let supported = match variant {
            Some("hover" | "pressed") => {
                matches!(target, StyleTarget::Button(_))
                    && utility
                        .strip_prefix("bg-")
                        .is_some_and(|color| valid_theme_color(color, document))
            }
            Some("focus") => {
                matches!(target, StyleTarget::Input(_))
                    && utility
                        .strip_prefix("border-")
                        .is_some_and(|color| valid_theme_color(color, document))
            }
            Some("focus-visible") => {
                matches!(target, StyleTarget::Button(_))
                    && utility
                        .strip_prefix("border-")
                        .is_some_and(|color| valid_theme_color(color, document))
            }
            Some("disabled") => {
                let disabled_text_ink = utility
                    .strip_prefix("text-")
                    .is_some_and(|color| valid_theme_color(color, document));
                let on_button = matches!(target, StyleTarget::Button(_))
                    && (utility.starts_with("opacity-")
                        || utility
                            .strip_prefix("bg-")
                            .is_some_and(|color| valid_theme_color(color, document))
                        || disabled_text_ink);
                // A text child inside a button's content may pin its disabled
                // ink: the arm keys on the BUTTON's status, which is how an
                // explicitly-colored glyph follows the disabled ramp.
                let on_button_child_text = matches!(target, StyleTarget::Text(_))
                    && disabled_text_ink
                    && inside_button_content();
                on_button || on_button_child_text
            }
            Some(_) => false,
            None => match utility {
                "w-full" | "h-full" => {
                    is_box || matches!(target, StyleTarget::Input(_)) && utility == "w-full"
                }
                "max-w-sm" | "max-w-md" | "max-w-lg" | "max-w-xl" | "max-w-2xl" => is_box,
                "items-center" => {
                    matches!(target, StyleTarget::Layout(Layout::Column | Layout::Row, _))
                }
                "self-center" => is_box && !is_css_flex,
                "overflow-hidden" => is_box,
                utility if is_text_size_utility(utility) => matches!(
                    target,
                    StyleTarget::Text(_)
                        | StyleTarget::RichText { .. }
                        | StyleTarget::RichSpan(_)
                        | StyleTarget::Button(_)
                ),
                utility if is_text_line_height_utility(utility) => matches!(
                    target,
                    StyleTarget::Text(_)
                        | StyleTarget::RichText { .. }
                        | StyleTarget::RichSpan(_)
                        | StyleTarget::Button(_)
                ),
                "font-mono" | "font-medium" | "font-semibold" | "font-bold" => matches!(
                    target,
                    StyleTarget::Text(_)
                        | StyleTarget::RichText { .. }
                        | StyleTarget::RichSpan(_)
                        | StyleTarget::Button(_)
                ),
                "border" | "border-2" => {
                    is_visual_box
                        || matches!(target, StyleTarget::Input(_) | StyleTarget::Button(_))
                }
                "rounded-sm" | "rounded" | "rounded-md" | "rounded-lg" | "rounded-full" => {
                    is_visual_box
                        || matches!(target, StyleTarget::Input(_) | StyleTarget::Button(_))
                }
                utility if utility.starts_with("rounded-") => {
                    is_visual_box
                        || matches!(target, StyleTarget::Input(_) | StyleTarget::Button(_))
                }
                _ if utility.starts_with("gap-") => matches!(
                    target,
                    StyleTarget::Layout(
                        Layout::Column | Layout::Row | Layout::Grid | Layout::Stack,
                        _
                    )
                ),
                _ if utility.starts_with("p-")
                    || utility.starts_with("px-")
                    || utility.starts_with("py-") =>
                {
                    is_box || matches!(target, StyleTarget::Input(_) | StyleTarget::Button(_))
                }
                _ if utility.starts_with("bg-") => {
                    is_visual_box
                        || matches!(target, StyleTarget::Input(_) | StyleTarget::Button(_))
                }
                _ if is_text_color_utility(utility) => {
                    is_visual_box
                        || matches!(
                            target,
                            StyleTarget::Text(_)
                                | StyleTarget::RichText { .. }
                                | StyleTarget::RichSpan(_)
                                | StyleTarget::Button(_)
                        )
                }
                _ if utility.starts_with("border-") => {
                    is_visual_box
                        || matches!(target, StyleTarget::Input(_) | StyleTarget::Button(_))
                }
                _ => false,
            },
        };
        if !supported {
            let error = Error::new(
                "E042",
                span,
                format!("utility `{original}` has no effect on `{target_name}`"),
            );
            let stranded_disabled_text = variant == Some("disabled")
                && matches!(target, StyleTarget::Text(_))
                && utility.starts_with("text-");
            if stranded_disabled_text {
                return Err(error.hint(
                    "`disabled:text-*` on text keys on a button's status, so it only works on text inside a button's content in the same view body",
                ));
            }
            return Err(error);
        }
    }

    let has_border = styles
        .iter()
        .map(|style| base_utility(style))
        .any(|utility| matches!(utility, "border" | "border-2"));
    let has_typed_border = match target {
        StyleTarget::Container(options) => options.style.border_width.is_some(),
        StyleTarget::PaneContent(style) | StyleTarget::PaneTitle(style) => {
            style.border_width.is_some()
        }
        _ => false,
    };
    // A `focus-visible:border-*` ring is an overlay with its own two-pixel
    // stroke; unlike `focus:border-*` it never recolors the widget's own
    // border, so it does not require a base border width.
    let has_border_color = styles
        .iter()
        .filter(|style| !style.starts_with("focus-visible:"))
        .map(|style| base_utility(style))
        .any(|utility| utility.starts_with("border-") && utility != "border-2");
    if (is_visual_box || matches!(target, StyleTarget::Input(_) | StyleTarget::Button(_)))
        && has_border_color
        && !has_border
        && !has_typed_border
    {
        return Err(Error::new(
            "E044",
            span,
            "border colors require `border-w=` on the same node",
        ));
    }
    let has_radius = styles
        .iter()
        .map(|style| base_utility(style))
        .any(|utility| utility.starts_with("rounded"));
    let has_background = styles
        .iter()
        .map(|style| base_utility(style))
        .any(|utility| utility.starts_with("bg-"));
    if is_visual_box && has_radius && !has_background && !has_border {
        return Err(Error::new(
            "E044",
            span,
            "rounded layout requires a background or border on the same node",
        ));
    }
    check_style_ownership(declared_styles, span, target)?;
    Ok(())
}

fn check_style_ownership(
    styles: &[String],
    span: &Span,
    target: StyleTarget<'_>,
) -> Result<(), Error> {
    match target {
        StyleTarget::Layout(kind, options) => {
            reject_duplicate_style_property(
                span,
                options.clip.is_some(),
                "overflow",
                "clip=",
                last_utility(styles, None, |utility| utility == "overflow-hidden"),
            )?;
            reject_duplicate_style_property(
                span,
                options.spacing.is_some(),
                "spacing",
                "gap=",
                last_utility(styles, None, |utility| utility.starts_with("gap-")),
            )?;
            reject_duplicate_style_property(
                span,
                has_padding(&options.padding),
                "padding",
                "p=",
                last_utility(styles, None, is_padding_utility),
            )?;
            reject_duplicate_style_property(
                span,
                options.align.is_some()
                    || options
                        .flexbox
                        .as_ref()
                        .is_some_and(|flex| flex.align_items.is_some()),
                "alignment",
                if options.flexbox.is_some() {
                    "items="
                } else {
                    "align="
                },
                last_utility(styles, None, |utility| utility == "items-center"),
            )?;
            match kind {
                Layout::Scroll | Layout::Column | Layout::Row | Layout::Grid => {}
                Layout::Stack | Layout::Hover => {
                    reject_stack_size_overlap(
                        span,
                        options.width.is_some(),
                        "width",
                        "w=",
                        last_utility(styles, None, |utility| utility == "w-full"),
                    )?;
                    reject_stack_size_overlap(
                        span,
                        options.height.is_some(),
                        "height",
                        "h=",
                        last_utility(styles, None, |utility| utility == "h-full"),
                    )?;
                }
            }
        }
        StyleTarget::Container(options) => {
            for (typed, property, owner, utility) in [
                (
                    options.width.is_some(),
                    "width",
                    "w=",
                    last_utility(styles, None, |utility| utility == "w-full"),
                ),
                (
                    options.height.is_some(),
                    "height",
                    "h=",
                    last_utility(styles, None, |utility| utility == "h-full"),
                ),
                (
                    options.max_width.is_some(),
                    "max-width",
                    "max-w=",
                    last_utility(styles, None, |utility| utility.starts_with("max-w-")),
                ),
                (
                    has_padding(&options.padding),
                    "padding",
                    "p=",
                    last_utility(styles, None, is_padding_utility),
                ),
                (
                    options.clip.is_some(),
                    "overflow",
                    "clip=",
                    last_utility(styles, None, |utility| utility == "overflow-hidden"),
                ),
            ] {
                reject_duplicate_style_property(span, typed, property, owner, utility)?;
            }
            check_direct_surface_ownership(styles, span, &options.style)?;
        }
        StyleTarget::PaneContent(style) | StyleTarget::PaneTitle(style) => {
            check_direct_surface_ownership(styles, span, style)?;
        }
        StyleTarget::Text(options) => {
            reject_duplicate_style_property(
                span,
                options.size.is_some(),
                "text size",
                "size=",
                last_utility(styles, None, is_text_size_utility),
            )?;
            reject_duplicate_style_property(
                span,
                options.line_height.is_some(),
                "text line height",
                "line-h=",
                last_utility(styles, None, is_text_line_height_utility),
            )?;
            reject_duplicate_style_property(
                span,
                options.font.is_some(),
                "font family",
                "font=",
                last_utility(styles, None, |utility| utility == "font-mono"),
            )?;
        }
        StyleTarget::RichText {
            typed_color,
            typed_size,
            typed_line_height,
            typed_font,
        } => {
            reject_duplicate_style_property(
                span,
                typed_size,
                "text size",
                "size=",
                last_utility(styles, None, is_text_size_utility),
            )?;
            reject_duplicate_style_property(
                span,
                typed_line_height,
                "text line height",
                "line-h=",
                last_utility(styles, None, is_text_line_height_utility),
            )?;
            reject_duplicate_style_property(
                span,
                typed_color,
                "text color",
                "color=",
                last_utility(styles, None, is_text_color_utility),
            )?;
            reject_duplicate_style_property(
                span,
                typed_font,
                "font family",
                "font=",
                last_utility(styles, None, |utility| utility == "font-mono"),
            )?;
        }
        StyleTarget::RichSpan(options) => {
            reject_duplicate_style_property(
                span,
                options.size.is_some(),
                "text size",
                "size=",
                last_utility(styles, None, is_text_size_utility),
            )?;
            reject_duplicate_style_property(
                span,
                options.line_height.is_some(),
                "text line height",
                "line-h=",
                last_utility(styles, None, is_text_line_height_utility),
            )?;
            reject_duplicate_style_property(
                span,
                options.font.is_some(),
                "font family",
                "font=",
                last_utility(styles, None, |utility| utility == "font-mono"),
            )?;
            reject_duplicate_style_property(
                span,
                options.color.is_some(),
                "text color",
                "color=",
                last_utility(styles, None, is_text_color_utility),
            )?;
        }
        StyleTarget::Input(options) => {
            reject_duplicate_style_property(
                span,
                options.width.is_some(),
                "width",
                "w=",
                last_utility(styles, None, |utility| utility == "w-full"),
            )?;
            reject_duplicate_style_property(
                span,
                options.padding.is_some(),
                "padding",
                "p=",
                last_utility(styles, None, |utility| {
                    utility.starts_with("p-")
                        || utility.starts_with("px-")
                        || utility.starts_with("py-")
                }),
            )?;
            for (name, status, focused) in [
                ("active", &options.style.active, false),
                ("hovered", &options.style.hovered, false),
                ("focused", &options.style.focused, true),
                ("focused-hovered", &options.style.focused_hovered, true),
                ("disabled", &options.style.disabled, false),
            ] {
                if let Some(status) = status {
                    check_input_status_ownership(styles, span, name, &status.options, focused)?;
                }
            }
        }
        StyleTarget::Button(options) => {
            reject_duplicate_style_property(
                span,
                options.padding.is_some(),
                "padding",
                "p=",
                last_utility(styles, None, |utility| {
                    utility.starts_with("p-")
                        || utility.starts_with("px-")
                        || utility.starts_with("py-")
                }),
            )?;
            for (name, status) in [
                ("active", &options.style.active),
                ("hovered", &options.style.hovered),
                ("pressed", &options.style.pressed),
                ("disabled", &options.style.disabled),
            ] {
                if let Some(status) = status {
                    check_button_status_ownership(styles, span, name, &status.options)?;
                }
            }
        }
        StyleTarget::Checkbox
        | StyleTarget::Toggler
        | StyleTarget::Slider
        | StyleTarget::Progress
        | StyleTarget::Radio
        | StyleTarget::Rule
        | StyleTarget::Space => {}
    }
    Ok(())
}

fn check_direct_surface_ownership(
    styles: &[String],
    span: &Span,
    style: &ContainerStyleOptions,
) -> Result<(), Error> {
    for (typed, property, owner, utility) in [
        (
            style.background.is_some(),
            "background",
            "bg=",
            last_utility(styles, None, |utility| utility.starts_with("bg-")),
        ),
        (
            style.text_color.is_some(),
            "text color",
            "text=",
            last_utility(styles, None, is_text_color_utility),
        ),
        (
            style.border_color.is_some(),
            "border color",
            "border=",
            last_utility(styles, None, is_border_color_utility),
        ),
        (
            style.border_width.is_some(),
            "border width",
            "border-w=",
            last_utility(styles, None, |utility| {
                matches!(utility, "border" | "border-2")
            }),
        ),
        (
            has_radius(style),
            "radius",
            "r=",
            last_utility(styles, None, |utility| utility.starts_with("rounded")),
        ),
    ] {
        reject_duplicate_style_property(span, typed, property, owner, utility)?;
    }
    Ok(())
}

fn check_input_status_ownership(
    styles: &[String],
    span: &Span,
    status: &str,
    options: &ContainerStyleOptions,
    focused: bool,
) -> Result<(), Error> {
    let background = last_utility(styles, None, |utility| utility.starts_with("bg-"));
    let border_color = focused
        .then(|| {
            last_utility(styles, Some("focus"), |utility| {
                utility.starts_with("border-")
            })
        })
        .flatten()
        .or_else(|| last_utility(styles, None, is_border_color_utility));
    let owners = [
        (
            options.background.is_some(),
            "background",
            "bg=",
            background,
        ),
        (
            options.border_width.is_some(),
            "border width",
            "border-w=",
            last_utility(styles, None, |utility| {
                matches!(utility, "border" | "border-2")
            }),
        ),
        (
            options.border_color.is_some(),
            "border color",
            "border=",
            border_color,
        ),
        (
            has_radius(options),
            "radius",
            "r=",
            last_utility(styles, None, |utility| utility.starts_with("rounded")),
        ),
    ];
    for (typed, property, owner, utility) in owners {
        let property = format!("{status} {property}");
        let owner = format!("{status} {owner}");
        reject_duplicate_style_property(span, typed, &property, &owner, utility)?;
    }
    Ok(())
}

fn check_button_status_ownership(
    styles: &[String],
    span: &Span,
    status: &str,
    options: &ContainerStyleOptions,
) -> Result<(), Error> {
    let background = match status {
        "hovered" => last_utility(styles, Some("hover"), |utility| utility.starts_with("bg-"))
            .or_else(|| last_utility(styles, None, |utility| utility.starts_with("bg-"))),
        "pressed" => last_utility(styles, Some("pressed"), |utility| {
            utility.starts_with("bg-")
        })
        .or_else(|| last_utility(styles, Some("hover"), |utility| utility.starts_with("bg-")))
        .or_else(|| last_utility(styles, None, |utility| utility.starts_with("bg-"))),
        _ => last_utility(styles, None, |utility| utility.starts_with("bg-")),
    };
    for (typed, property, owner, utility) in [
        (
            options.background.is_some(),
            "background",
            "bg=",
            background,
        ),
        (
            options.text_color.is_some(),
            "text color",
            "text=",
            last_utility(styles, None, is_text_color_utility),
        ),
        (
            has_radius(options),
            "radius",
            "r=",
            last_utility(styles, None, |utility| utility.starts_with("rounded")),
        ),
    ] {
        let property = format!("{status} {property}");
        let owner = format!("{status} {owner}");
        reject_duplicate_style_property(span, typed, &property, &owner, utility)?;
    }
    Ok(())
}

fn reject_duplicate_style_property(
    span: &Span,
    typed: bool,
    property: &str,
    typed_owner: &str,
    utility: Option<&str>,
) -> Result<(), Error> {
    let Some(utility) = utility.filter(|_| typed) else {
        return Ok(());
    };
    Err(Error::new(
        "E045",
        span,
        format!("style property `{property}` is set by both `{typed_owner}` and `@{utility}`"),
    )
    .hint(format!(
        "choose one owner; `{typed_owner}` currently overrides `@{utility}` on this node"
    )))
}

fn reject_stack_size_overlap(
    span: &Span,
    typed: bool,
    property: &str,
    typed_owner: &str,
    utility: Option<&str>,
) -> Result<(), Error> {
    let Some(utility) = utility.filter(|_| typed) else {
        return Ok(());
    };
    Err(Error::new(
        "E045",
        span,
        format!("style property `{property}` is set by both `{typed_owner}` and `@{utility}`"),
    )
    .hint(format!(
        "remove `{typed_owner}`; `@{utility}` sizes both the stack and its generated outer wrapper"
    )))
}

fn last_utility<'a>(
    styles: &'a [String],
    variant: Option<&str>,
    predicate: impl Fn(&str) -> bool,
) -> Option<&'a str> {
    styles.iter().rev().find_map(|style| {
        let (actual_variant, utility) = style
            .split_once(':')
            .map_or((None, style.as_str()), |(variant, utility)| {
                (Some(variant), utility)
            });
        (actual_variant == variant && predicate(utility)).then_some(style.as_str())
    })
}

fn is_text_color_utility(utility: &str) -> bool {
    utility.starts_with("text-") && !is_text_size_utility(utility)
}

fn is_text_size_utility(utility: &str) -> bool {
    matches!(
        utility,
        "text-xs" | "text-sm" | "text-base" | "text-lg" | "text-xl" | "text-2xl"
    ) || is_exact_text_size_utility(utility)
}

fn is_exact_text_size_utility(utility: &str) -> bool {
    utility
        .strip_prefix("text-")
        .and_then(|value| value.strip_suffix("px"))
        .and_then(|value| value.parse::<f32>().ok())
        .is_some_and(|value| value.is_finite() && value > 0.0)
}

fn exact_pixels(value: &str) -> Option<u16> {
    value.strip_suffix("px")?.parse().ok()
}

fn exact_radius(value: &str) -> Option<u16> {
    exact_pixels(value).filter(|value| *value > 0)
}

fn is_text_line_height_utility(utility: &str) -> bool {
    matches!(
        utility,
        "leading-tight" | "leading-snug" | "leading-normal" | "leading-relaxed"
    )
}

fn is_border_color_utility(utility: &str) -> bool {
    utility.starts_with("border-") && utility != "border-2"
}

fn is_padding_utility(utility: &str) -> bool {
    utility.starts_with("p-") || utility.starts_with("px-") || utility.starts_with("py-")
}

fn has_padding(options: &PaddingOptions) -> bool {
    options.all.is_some()
        || options.x.is_some()
        || options.y.is_some()
        || options.top.is_some()
        || options.right.is_some()
        || options.bottom.is_some()
        || options.left.is_some()
}

fn has_radius(options: &ContainerStyleOptions) -> bool {
    options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some()
}

pub(in crate::check) fn base_utility(style: &str) -> &str {
    style.split_once(':').map_or(style, |(_, utility)| utility)
}

pub(in crate::check) fn require_type(
    actual: &Type,
    expected: &Type,
    span: &Span,
) -> Result<(), Error> {
    if compatible(actual, expected) {
        Ok(())
    } else {
        Err(type_error(span, expected, actual))
    }
}

pub(crate) fn compatible(left: &Type, right: &Type) -> bool {
    left == right
        || *left == Type::Unknown
        || *right == Type::Unknown
        || match (left, right) {
            (Type::List(left), Type::List(right)) | (Type::Option(left), Type::Option(right)) => {
                compatible(left, right)
            }
            (Type::Result(left_output, left_error), Type::Result(right_output, right_error)) => {
                compatible(left_output, right_output) && compatible(left_error, right_error)
            }
            _ => false,
        }
}

pub(in crate::check) fn type_error(span: &Span, expected: &Type, actual: &Type) -> Error {
    let error = Error::new(
        "E101",
        span,
        format!(
            "expected `{}`, got `{}`",
            expected.display(),
            actual.display()
        ),
    );
    // Every way a secret could become an ordinary value arrives here, because
    // no operation on `secret` produces anything else. One sentence at the
    // single funnel beats a rule per widget.
    if *actual == Type::Secret {
        return error.hint(
            "a secret never becomes a value: bind one `input` to it, ask it `empty` or `len`, clear it with `= \"\"`, and pass it to an extern parameter declared `secret`",
        );
    }
    error
}
