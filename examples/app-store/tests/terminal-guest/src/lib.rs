pub mod data {
    pub fn label(value: bool) -> String {
        value.to_string()
    }
    use iced::futures::{Stream, StreamExt};
    use ui_lang_wire::SurfaceValue;
    #[derive(Clone, Debug, PartialEq)]
    pub struct TerminalNotice {
        pub running: bool,
        pub title: String,
        pub attention: bool,
    }
    #[derive(Clone, Debug, PartialEq)]
    pub struct TerminalError {
        pub message: String,
    }
    pub fn events() -> impl Stream<Item = Result<TerminalNotice, TerminalError>> + Send + 'static {
        ui_lang_guest::host::subscribe("terminal.events", &[]).map(|answer| {
            let bytes = answer.map_err(|message| TerminalError { message })?;
            let value = ui_lang_wire::decode::<SurfaceValue>(&bytes)
                .map_err(|message| TerminalError { message })?;
            if let SurfaceValue::Record { name, fields } = value {
                if name == "TerminalNotice" {
                    if let [
                        (running, SurfaceValue::Bool(r)),
                        (title, SurfaceValue::Str(t)),
                        (attention, SurfaceValue::Bool(a)),
                    ] = fields.as_slice()
                    {
                        if running == "running" && title == "title" && attention == "attention" {
                            return Ok(TerminalNotice {
                                running: *r,
                                title: t.clone(),
                                attention: *a,
                            });
                        }
                    }
                }
            }
            Err(TerminalError {
                message: "invalid terminal notice".into(),
            })
        })
    }
}
ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    TerminalFixture,
    "Terminal fixture",
    "Host-owned native PTY",
    ["terminal"]
);
