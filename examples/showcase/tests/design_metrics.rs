mod backend {
    pub fn native_role(content: String, role: i64) -> iced::Element<'static, ()> {
        use ui_lang_components::ui::typography::{TextRole, typography};
        let role = match role {
            0 => TextRole::Display,
            1 => TextRole::SectionTitle,
            2 => TextRole::Body,
            3 => TextRole::Caption,
            4 => TextRole::Machine,
            5 => TextRole::ScreenTitle,
            6 => TextRole::PaneHeader,
            7 => TextRole::List,
            8 => TextRole::Meta,
            9 => TextRole::MetaCompact,
            10 => TextRole::FieldLabel,
            11 => TextRole::NavLabel,
            _ => TextRole::Badge,
        };
        let theme = ui_lang_components::ui::theme::LIGHT.with_fonts(
            iced::Font::with_name("IBM Plex Sans KR"),
            iced::Font::MONOSPACE,
        );
        typography(content, role, &theme).into()
    }
}

ui_lang::include_app!("tests/cases/ui/design_metrics.ice");
