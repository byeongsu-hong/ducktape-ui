ui_lang::include_app!("src/ui/notion.ice");

mod editor;
mod pages;

mod helpers {
    pub fn page_link(page: String) -> String {
        format!("https://notion.local/{page}")
    }

    pub fn selected_access(access: Option<String>) -> String {
        access.unwrap_or_else(|| "Can edit".into())
    }
}

fn main() -> iced::Result {
    Notion::run()
}

#[cfg(test)]
mod tests {
    use super::{__NotionMessage, ModalState, Notion, helpers};

    fn notion_settings() -> iced::Settings {
        iced::Settings {
            fonts: vec![
                include_bytes!("../assets/fonts/Inter-Regular.ttf")
                    .as_slice()
                    .into(),
                include_bytes!("../assets/fonts/Inter-Bold.ttf")
                    .as_slice()
                    .into(),
            ],
            default_font: iced::Font::with_name("Inter"),
            ..iced::Settings::default()
        }
    }

    #[test]
    fn navigation_keeps_a_retained_editor_per_page() {
        let (mut app, _) = Notion::__boot();
        assert_eq!(app.pages.selected_id(), "home");
        assert!(app.pages.document("home").block_count() > 1);

        let _ = app.__update(__NotionMessage::Navigate("launch".into()));
        assert_eq!(app.pages.selected_id(), "launch");
        assert!(app.pages.document("launch").block_count() > 1);
        assert!(app.pages.document("home").thread_count() > 0);
    }

    #[test]
    fn editor_draws_at_its_default_viewport() {
        let (app, _) = Notion::__boot();
        let mut screen = iced_test::Simulator::with_size(
            notion_settings(),
            iced::Size::new(1280.0, 800.0),
            app.__view(),
        );

        screen.find("Eddy's Notion").expect("workspace sidebar");
        screen
            .find("Product strategy")
            .expect("editable page title");
        assert!(
            screen
                .find("Can we link the customer research notes here?")
                .is_err(),
            "comments stay out of the writing surface until opened"
        );
        assert!(screen.find("BLOCKS").is_err(), "editor chrome stays hidden");

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
        assert_eq!(app.pages.selected_id(), "launch");

        assert!(
            app.pages
                .document("launch")
                .markdown()
                .contains("Finalize announcement")
        );
    }

    #[test]
    fn rendered_search_opens_with_recent_pages() {
        let (mut app, _) = Notion::__boot();
        let viewport = iced::Size::new(1280.0, 800.0);
        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen.click("Search").expect("sidebar search button");
        for message in screen.into_messages() {
            let _ = app.__update(message);
        }

        assert!(matches!(app.modal, ModalState::Search));
        assert!(app.search_query.is_empty());
        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen.find("RECENT").expect("recent pages heading");
        screen.find("Untitled").expect("untitled recent page");

        if let Ok(path) = std::env::var("NOTION_SEARCH_SNAPSHOT") {
            let snapshot = screen.snapshot(&app.__theme()).expect("render snapshot");
            assert!(snapshot.matches_image(path).expect("write snapshot"));
        }
    }

    #[test]
    fn rendered_comments_support_reply_resolve_and_reopen() {
        let (mut app, _) = Notion::__boot();
        let viewport = iced::Size::new(1280.0, 800.0);
        assert!(!crate::editor::block_editor_comments_open(
            app.pages.document("home").clone()
        ));

        let _ = app.__update(__NotionMessage::CommentsToggled);
        assert!(crate::editor::block_editor_comments_open(
            app.pages.document("home").clone()
        ));
        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen.find("Comments").expect("comments pane");
        screen.find("Open ▾").expect("open comment filter");
        screen
            .find("Keep decisions, plans, and their context in one clear place.")
            .expect("comment source context");

        if let Ok(path) = std::env::var("NOTION_COMMENTS_SNAPSHOT") {
            let snapshot = screen.snapshot(&app.__theme()).expect("render snapshot");
            assert!(snapshot.matches_image(path).expect("write snapshot"));
        }

        screen.click("Reply…").expect("thread reply field");
        screen.typewrite("Research notes are linked.");
        for message in screen.into_messages() {
            let _ = app.__update(message);
        }

        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen.click("Send").expect("send reply");
        for message in screen.into_messages() {
            let _ = app.__update(message);
        }
        assert_eq!(app.pages.document("home").thread_message_count(1), 2);

        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen
            .find("Research notes are linked.")
            .expect("submitted reply");
        screen.click("Resolve").expect("resolve comment");
        for message in screen.into_messages() {
            let _ = app.__update(message);
        }
        assert!(app.pages.document("home").thread_resolved(1));

        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen.find("No open comments").expect("empty open filter");
        screen.click("Open ▾").expect("comment status filter");
        for message in screen.into_messages() {
            let _ = app.__update(message);
        }

        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen
            .find("Research notes are linked.")
            .expect("resolved thread");
        screen.click("Reopen").expect("reopen comment");
        for message in screen.into_messages() {
            let _ = app.__update(message);
        }
        assert!(!app.pages.document("home").thread_resolved(1));
    }

    #[test]
    fn rendered_share_dialog_supports_access_and_invites() {
        let (mut app, _) = Notion::__boot();
        let viewport = iced::Size::new(1280.0, 800.0);
        let _ = app.__update(__NotionMessage::OpenShare);
        assert_eq!(
            helpers::selected_access(app.invite_access_choice.clone()),
            "Can edit"
        );

        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen.find("Share this page").expect("share dialog");
        screen
            .find("Only people invited")
            .expect("private general access");
        screen.find("Copy link").expect("copy page link");
        drop(screen);

        let _ = app.__update(__NotionMessage::InviteAccessChanged("Can view".into()));
        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen.click("Email or name").expect("invite email field");
        screen.typewrite("collaborator@example.com");
        for message in screen.into_messages() {
            let _ = app.__update(message);
        }

        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        if let Ok(path) = std::env::var("NOTION_SHARE_SNAPSHOT") {
            let snapshot = screen.snapshot(&app.__theme()).expect("render snapshot");
            assert!(snapshot.matches_image(path).expect("write snapshot"));
        }
        screen.click("Invite").expect("invite collaborator");
        for message in screen.into_messages() {
            let _ = app.__update(message);
        }
        assert_eq!(app.invited_email, "collaborator@example.com");
        assert_eq!(app.invited_access, "Can view");

        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        screen
            .find("collaborator@example.com")
            .expect("invited collaborator");
        screen.find("Can view").expect("invited access");
        screen.click("Copy link").expect("copy page link");
        for message in screen.into_messages() {
            let _ = app.__update(message);
        }
        assert!(app.link_copied);
    }
}
