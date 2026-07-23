ui_lang::include_app!("src/showcase.ice");

fn main() -> iced::Result {
    Showcase::run()
}

#[cfg(test)]
mod tests {
    use super::{__ShowcaseMessage, Showcase};

    #[test]
    fn showcase_boots_with_default_component_state() {
        let (mut showcase, _) = Showcase::__boot();

        assert_eq!(showcase.clicks, 0);
        assert!(!showcase.accepted);
        assert!(showcase.notifications);
        assert!(!showcase.dialog_open);

        let _ = showcase.__update(__ShowcaseMessage::Clicked);
        let _ = showcase.__update(__ShowcaseMessage::DismissToast);
        assert_eq!(showcase.clicks, 1);
        assert!(!showcase.toast_visible);

        let _ = showcase.__update(__ShowcaseMessage::ShowToast);
        assert!(showcase.toast_visible);
    }
}
