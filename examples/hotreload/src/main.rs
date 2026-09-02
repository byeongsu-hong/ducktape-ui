#[cfg(test)]
mod frame_probe;

ui_lang::include_app!("src/ui/app.ice");

mod backend {
    const SOURCE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui/screen.ice");

    #[derive(Clone, Debug)]
    pub struct SourceError {
        pub message: String,
    }

    #[derive(Clone, Debug)]
    pub struct SaveCommand;

    pub fn source_keys(
        event: iced::widget::text_editor::KeyPress,
    ) -> Option<iced::widget::text_editor::Binding<SaveCommand>> {
        if event.key.to_latin(event.physical_key) == Some('s')
            && (event.modifiers.control() || event.modifiers.logo())
        {
            Some(iced::widget::text_editor::Binding::Custom(SaveCommand))
        } else {
            iced::widget::text_editor::Binding::from_key_press(event)
        }
    }

    pub async fn load_source() -> Result<String, SourceError> {
        std::fs::read_to_string(SOURCE_PATH).map_err(source_error)
    }

    pub async fn save_source(source: String) -> Result<String, SourceError> {
        #[cfg(not(test))]
        std::fs::write(SOURCE_PATH, &source).map_err(source_error)?;

        Ok(format!(
            "Saved {} bytes — cargo ice dev will apply compatible edits.",
            source.len()
        ))
    }

    fn source_error(error: std::io::Error) -> SourceError {
        SourceError {
            message: format!("{SOURCE_PATH}: {error}"),
        }
    }
}

fn main() -> iced::Result {
    HotReload::run()
}
