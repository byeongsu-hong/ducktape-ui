ui_lang::include_app!("src/ui/notion.ice");

mod editor;

mod helpers {
    pub fn page_matches(query: String, title: String) -> bool {
        title.to_lowercase().contains(&query.trim().to_lowercase())
    }
}

fn main() -> iced::Result {
    Notion::run()
}

#[cfg(test)]
mod tests {
    use super::{__NotionMessage, Notion, helpers};

    #[test]
    fn navigation_keeps_a_retained_editor_per_page() {
        let (mut app, _) = Notion::__boot();
        assert_eq!(app.selected_page, "home");
        assert!(app.home_document.block_count() > 1);

        let _ = app.__update(__NotionMessage::Navigate("launch".into()));
        assert_eq!(app.selected_page, "launch");
        assert!(app.launch_document.block_count() > 1);
        assert!(app.home_document.thread_count() > 0);
    }

    #[test]
    fn page_search_is_case_insensitive() {
        assert!(helpers::page_matches(
            "road".into(),
            "Product Roadmap".into()
        ));
        assert!(!helpers::page_matches(
            "road".into(),
            "Meeting Notes".into()
        ));
    }

    #[test]
    fn editor_draws_at_its_default_viewport() {
        let (app, _) = Notion::__boot();
        let mut screen = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1280.0, 800.0),
            app.__view(),
        );

        screen.find("Eddy's Notion").expect("workspace sidebar");
        screen
            .find("Building a home for your work")
            .expect("editable page title");
        screen.find("BLOCKS").expect("dynamic block toolbar");
        screen.find("Thread 1").expect("floating comment thread");

        let snapshot = screen.snapshot(&app.__theme()).expect("render snapshot");
        if let Ok(path) = std::env::var("NOTION_SNAPSHOT") {
            assert!(snapshot.matches_image(path).expect("write snapshot"));
        }
    }

    #[test]
    fn rendered_navigation_routes_to_the_selected_editor() {
        let (mut app, _) = Notion::__boot();
        let viewport = iced::Size::new(1280.0, 800.0);
        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen.click("Launch plan").expect("launch page link");
        for message in screen.into_messages() {
            let _ = app.__update(message);
        }
        assert_eq!(app.selected_page, "launch");

        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen.find("BLOCKS").expect("launch editor toolbar");
        screen.find("Finalize announcement").expect("launch block");
    }
}
