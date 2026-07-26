use super::theme::Theme;
use iced::Theme as IcedTheme;
use iced::widget::rule;
use iced::widget::rule::{FillMode, Rule, Style};

pub fn horizontal(theme: &Theme) -> Rule<'static, IcedTheme> {
    styled(rule::horizontal(1), theme)
}

pub fn vertical(theme: &Theme) -> Rule<'static, IcedTheme> {
    styled(rule::vertical(1), theme)
}

fn styled(rule: Rule<'static, IcedTheme>, theme: &Theme) -> Rule<'static, IcedTheme> {
    let color = theme.palette.border;
    rule.style(move |_iced_theme| Style {
        color,
        radius: 0.0.into(),
        fill_mode: FillMode::Full,
        snap: true,
    })
}

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;
    use iced::Length;
    use iced::advanced::Widget;

    #[test]
    fn orientation_keeps_a_one_pixel_rule() {
        assert_eq!(
            Widget::<(), IcedTheme, iced::Renderer>::size(&horizontal(&LIGHT)),
            iced::Size::new(Length::Fill, Length::Fixed(1.0))
        );
        assert_eq!(
            Widget::<(), IcedTheme, iced::Renderer>::size(&vertical(&LIGHT)),
            iced::Size::new(Length::Fixed(1.0), Length::Fill)
        );
    }
}
