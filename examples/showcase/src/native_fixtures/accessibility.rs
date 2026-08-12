ui_lang::include_app!("tests/cases/ui/accessibility.ice");

fn snapshot(app: &Accessibility) -> ui_lang_runtime::Snapshot<__AccessibilityMessage> {
    use iced::advanced::renderer::Headless;
    use iced_test::futures::futures::StreamExt;

    let mut renderer = iced_test::futures::futures::executor::block_on(
        <iced::Renderer as Headless>::new(iced::Font::DEFAULT, iced::Pixels(16.0), None),
    )
    .expect("headless renderer");
    let mut ui = iced_test::runtime::UserInterface::build(
        app.__view(),
        iced::Size::new(640.0, 480.0),
        iced_test::runtime::user_interface::Cache::default(),
        &mut renderer,
    );
    let task = ui_lang_runtime::snapshot::<__AccessibilityMessage>("Accessibility");
    let mut stream = iced_test::runtime::task::into_stream(task).expect("snapshot task");
    let action =
        iced_test::futures::futures::executor::block_on(stream.next()).expect("widget operation");
    let iced_test::runtime::Action::Widget(mut operation) = action else {
        panic!("snapshot task must begin with a widget operation");
    };
    ui.operate(&renderer, operation.as_mut());
    let _ = operation.finish();
    let output =
        iced_test::futures::futures::executor::block_on(stream.next()).expect("snapshot output");
    let iced_test::runtime::Action::Output(snapshot) = output else {
        panic!("snapshot operation must produce a tree");
    };
    snapshot
}

#[test]
fn builds_first_class_accessibility_contract() {
    let (mut app, _) = Accessibility::__boot();
    let initial = snapshot(&app);
    let nodes = initial
        .update
        .nodes
        .iter()
        .map(|(_, node)| node)
        .collect::<Vec<_>>();
    let by_role_and_label = |role, label| {
        nodes
            .iter()
            .copied()
            .find(|node| node.role() == role && node.label() == Some(label))
            .expect("semantic node")
    };

    let root = nodes
        .iter()
        .copied()
        .find(|node| node.role() == ui_lang_runtime::Role::Window)
        .expect("named root");
    assert_eq!(root.label(), Some("Accessibility"));
    assert!(nodes.iter().any(|node| {
        node.role() == ui_lang_runtime::Role::Label && node.value() == Some("Accessible form")
    }));
    let input = by_role_and_label(ui_lang_runtime::Role::TextInput, "Full name");
    assert_eq!(input.description(), Some("Name used on your profile"));
    assert_eq!(input.value(), Some(""));
    let password = by_role_and_label(ui_lang_runtime::Role::PasswordInput, "Account password");
    assert_eq!(password.value(), None);
    let checkbox = by_role_and_label(ui_lang_runtime::Role::CheckBox, "Terms consent");
    assert_eq!(checkbox.toggled(), Some(ui_lang_runtime::Toggled::False));
    let submit = by_role_and_label(ui_lang_runtime::Role::Button, "Submit");
    assert!(submit.is_disabled());
    assert_eq!(submit.description(), Some("Save the accessible form"));
    assert_eq!(
        by_role_and_label(ui_lang_runtime::Role::Button, "Open help").description(),
        Some("Show keyboard and screen-reader help")
    );
    assert_eq!(
        by_role_and_label(ui_lang_runtime::Role::Image, "Ice accessibility example").description(),
        Some("A decorative sample promoted into the accessibility tree")
    );

    let _ = app.__update(__AccessibilityMessage::Toggle(true));
    assert!(app.accepted);
    let updated = snapshot(&app);
    let checkbox = updated
        .update
        .nodes
        .iter()
        .map(|(_, node)| node)
        .find(|node| {
            node.role() == ui_lang_runtime::Role::CheckBox && node.label() == Some("Terms consent")
        })
        .expect("updated checkbox");
    assert_eq!(checkbox.toggled(), Some(ui_lang_runtime::Toggled::True));
}
