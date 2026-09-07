ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    WidgetFixture,
    "Widget fixture",
    "Mounted widget commands.",
    ["clock"]
);

mod bridge {
    pub fn canceled_focus() -> iced::Task<()> {
        iced::Task::future(async {
            let command = ui_lang_guest::wire::WidgetCommand::Focus {
                target: "WidgetFixture/second".into(),
            };
            let request =
                ui_lang_guest::host::request("host.widget", &ui_lang_guest::wire::encode(&command));
            drop(request);
        })
    }
}
