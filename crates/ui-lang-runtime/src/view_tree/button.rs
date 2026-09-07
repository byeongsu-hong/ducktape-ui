//! Native preset and copied recipe resolution, in native compiler order.
use super::*;

pub(super) fn style(
    style: &wire::ButtonStyle,
    theme: &iced::Theme,
    status: widget::button::Status,
) -> widget::button::Style {
    use widget::button as native;
    let mut resolved = match style.preset {
        wire::ButtonPreset::Primary => native::primary(theme, status),
        wire::ButtonPreset::Secondary => native::secondary(theme, status),
        wire::ButtonPreset::Success => native::success(theme, status),
        wire::ButtonPreset::Warning => native::warning(theme, status),
        wire::ButtonPreset::Danger => native::danger(theme, status),
        wire::ButtonPreset::Text => native::text(theme, status),
        wire::ButtonPreset::Background => native::background(theme, status),
        wire::ButtonPreset::Subtle => native::subtle(theme, status),
    };
    if let Some(recipe) = &style.recipe {
        apply_face(recipe.base, &mut resolved);
        let background = match status {
            native::Status::Hovered => recipe.hover_background.or(recipe.base.background),
            native::Status::Pressed => recipe
                .pressed_background
                .or(recipe.hover_background)
                .or(recipe.base.background),
            _ => recipe.base.background,
        };
        if let Some(value) = background {
            resolved.background = Some(Background::Color(color(value)));
        }
    }
    apply_face(style.active, &mut resolved);
    let state = match status {
        native::Status::Active => None,
        native::Status::Hovered => style.hovered,
        native::Status::Pressed => style.pressed,
        native::Status::Disabled => style.disabled,
    };
    if let Some(face) = state {
        apply_face(face, &mut resolved);
    }
    if matches!(status, native::Status::Disabled)
        && style.disabled.is_none()
        && let Some(recipe) = &style.recipe
    {
        let alpha = recipe.disabled_opacity.unwrap_or(0.5);
        if let Some(value) = recipe.disabled_background {
            resolved.background = Some(Background::Color(color(value)));
        } else if (recipe.base.background.is_some() || recipe.disabled_opacity.is_some())
            && let Some(Background::Color(mut value)) = resolved.background
        {
            value.a *= alpha;
            resolved.background = Some(Background::Color(value));
        }
        if let Some(value) = recipe.disabled_text {
            resolved.text_color = color(value);
        } else if recipe.base.text.is_some() || recipe.disabled_opacity.is_some() {
            resolved.text_color.a *= alpha;
        }
    }
    resolved
}

pub(super) fn label(
    value: &str,
    recipe: Option<&wire::ButtonRecipe>,
) -> IceElement<'static, Output> {
    let mut text = widget::text(value.to_owned());
    if let Some(recipe) = recipe {
        if let Some(size) = recipe.text_size {
            text = text.size(size.max(f32::EPSILON));
        }
        if let Some(height) = recipe.line_height {
            text = text.line_height(height);
        }
        if let Some(face) = &recipe.font {
            text = text.font(text::named_font(face));
        }
    }
    text.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_disabled_treatment_follows_active_overrides() {
        let recipe = wire::ButtonRecipe {
            base: wire::Face {
                background: Some(wire::Rgba([1.0, 0.0, 0.0, 1.0])),
                text: Some(wire::Rgba([1.0; 4])),
                ..Default::default()
            },
            disabled_opacity: Some(0.25),
            hover_background: Some(wire::Rgba([0.0, 0.0, 1.0, 1.0])),
            ..Default::default()
        };
        let active = wire::Face {
            background: Some(wire::Rgba([0.0, 1.0, 0.0, 1.0])),
            ..Default::default()
        };
        let mut copied = wire::ButtonStyle {
            recipe: Some(recipe),
            active,
            ..Default::default()
        };
        let disabled = style(
            &copied,
            &iced::Theme::Light,
            widget::button::Status::Disabled,
        );
        assert_eq!(
            disabled.background,
            Some(Background::Color(Color::from_rgba(0.0, 1.0, 0.0, 0.25)))
        );
        assert_eq!(disabled.text_color.a, 0.25);
        let hovered = style(
            &copied,
            &iced::Theme::Light,
            widget::button::Status::Hovered,
        );
        assert_eq!(
            hovered.background,
            Some(Background::Color(Color::from_rgb(0.0, 1.0, 0.0))),
            "typed active face follows recipe hover color"
        );
        copied.disabled = Some(wire::Face {
            background: Some(wire::Rgba([0.0, 0.0, 1.0, 1.0])),
            ..Default::default()
        });
        let disabled = style(
            &copied,
            &iced::Theme::Light,
            widget::button::Status::Disabled,
        );
        assert_eq!(
            disabled.background,
            Some(Background::Color(Color::from_rgb(0.0, 0.0, 1.0)))
        );
        assert_eq!(
            disabled.text_color.a, 1.0,
            "explicit disabled face suppresses the recipe disabled pass"
        );
    }

    #[test]
    fn recipe_hover_and_pressed_fallback_preserve_native_preset() {
        let mut copied = wire::ButtonStyle {
            preset: wire::ButtonPreset::Text,
            ..Default::default()
        };
        let theme = iced::Theme::Light;
        let native = widget::button::text(&theme, widget::button::Status::Active);
        assert_eq!(
            style(&copied, &theme, widget::button::Status::Active),
            native
        );
        copied.recipe = Some(wire::ButtonRecipe {
            hover_background: Some(wire::Rgba([1.0, 0.0, 0.0, 1.0])),
            ..Default::default()
        });
        for status in [
            widget::button::Status::Hovered,
            widget::button::Status::Pressed,
        ] {
            assert_eq!(
                style(&copied, &theme, status).background,
                Some(Background::Color(Color::from_rgb(1.0, 0.0, 0.0)))
            );
        }
    }
}
