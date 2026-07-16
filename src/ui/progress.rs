use super::theme::{Theme, mix};
use iced::widget::{ProgressBar, progress_bar};
use iced::{Background, Border};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProgressVariant {
    #[default]
    Default,
    Success,
    Warning,
    Destructive,
}

/// Pair the bar with adjacent visible status text, such as `42%` or `Complete`.
pub fn progress(percent: f32, variant: ProgressVariant, theme: &Theme) -> ProgressBar<'static> {
    let theme = *theme;
    progress_bar(0.0..=100.0, normalized(percent))
        .girth(5)
        .style(move |_iced_theme| style(&theme, variant))
}

pub fn style(theme: &Theme, variant: ProgressVariant) -> iced::widget::progress_bar::Style {
    iced::widget::progress_bar::Style {
        background: Background::Color(mix(
            theme.palette.background,
            theme.palette.foreground,
            0.12,
        )),
        bar: Background::Color(tone(theme, variant)),
        border: Border {
            color: theme.palette.input,
            width: 1.0,
            radius: 999.0.into(),
        },
    }
}

fn tone(theme: &Theme, variant: ProgressVariant) -> iced::Color {
    match variant {
        ProgressVariant::Default => theme.palette.primary,
        ProgressVariant::Success => theme.palette.success,
        ProgressVariant::Warning => theme.palette.warning,
        ProgressVariant::Destructive => theme.palette.destructive,
    }
}

fn normalized(percent: f32) -> f32 {
    if percent.is_nan() {
        0.0
    } else {
        percent.clamp(0.0, 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    #[test]
    fn values_are_bounded_and_nan_is_empty() {
        assert_eq!(normalized(-1.0), 0.0);
        assert_eq!(normalized(f32::NAN), 0.0);
        assert_eq!(normalized(42.0), 42.0);
        assert_eq!(normalized(f32::INFINITY), 100.0);
        assert_eq!(normalized(101.0), 100.0);
    }

    #[test]
    fn variants_select_semantic_colors_with_non_text_contrast() {
        for theme in [LIGHT, DARK] {
            for variant in [
                ProgressVariant::Default,
                ProgressVariant::Success,
                ProgressVariant::Warning,
                ProgressVariant::Destructive,
            ] {
                let expected = tone(&theme, variant);
                let appearance = style(&theme, variant);
                let Background::Color(track) = appearance.background else {
                    panic!("progress track must be a solid color");
                };

                assert_eq!(appearance.bar, Background::Color(expected));
                assert!(
                    appearance
                        .border
                        .color
                        .relative_contrast(theme.palette.background)
                        >= 3.0,
                    "{} progress boundary",
                    theme.name
                );
                assert!(
                    expected.relative_contrast(track) >= 3.0,
                    "{} {variant:?}",
                    theme.name
                );
            }
        }
    }
}
