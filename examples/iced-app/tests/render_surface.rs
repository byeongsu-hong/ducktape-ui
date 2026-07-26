#[allow(dead_code)]
#[path = "../src/backend/shader.rs"]
mod shader;

mod backend {
    pub use super::shader::status_shader;

    pub fn native_help(active: bool) -> iced::Element<'static, bool> {
        iced::widget::button("Extern component")
            .on_press(!active)
            .into()
    }

    #[allow(clippy::type_complexity)]
    pub fn alternate_panel(
        active: bool,
    ) -> (
        Option<iced::Theme>,
        iced::Element<'static, (), iced::Theme>,
        Option<fn(&iced::Theme) -> iced::Color>,
        Option<fn(&iced::Theme) -> iced::Background>,
    ) {
        (
            active.then_some(iced::Theme::Dark),
            iced::widget::Space::new().width(24).height(24).into(),
            None,
            None,
        )
    }
}

ui_lang::include_app!("src/ui/render_surface.ice");
