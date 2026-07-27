#[cfg(test)]
mod dynamic_widget_operations {
    ui_lang::include_app!("src/ui/dynamic_widget_operations.ice");

    #[test]
    fn constructs_dynamic_widget_tasks() {
        let (mut app, _) = DynamicOperations::__boot();
        for message in [
            __DynamicOperationsMessage::Focus,
            __DynamicOperationsMessage::FocusNamed,
            __DynamicOperationsMessage::Check,
            __DynamicOperationsMessage::Front,
            __DynamicOperationsMessage::End,
            __DynamicOperationsMessage::Cursor,
            __DynamicOperationsMessage::All,
            __DynamicOperationsMessage::Range,
            __DynamicOperationsMessage::Snap,
            __DynamicOperationsMessage::SnapEnd,
            __DynamicOperationsMessage::ScrollTo,
            __DynamicOperationsMessage::ScrollBy,
        ] {
            assert_eq!(app.__update(message).units(), 2);
        }
    }
}

#[cfg(test)]
mod scoped_widget_operations {
    ui_lang::include_app!("src/ui/scoped_widget_operations.ice");

    #[test]
    fn constructs_scoped_widget_tasks() {
        let (mut app, _) = ScopedOperations::__boot();
        for message in [
            __ScopedOperationsMessage::FocusComponent,
            __ScopedOperationsMessage::FocusDefault,
            __ScopedOperationsMessage::FocusSlot,
            __ScopedOperationsMessage::FocusKeyed,
            __ScopedOperationsMessage::FocusHeader,
            __ScopedOperationsMessage::FocusCell,
            __ScopedOperationsMessage::SnapPane,
        ] {
            assert_eq!(app.__update(message).units(), 2);
        }
    }
}

#[cfg(test)]
mod widget_selectors {
    ui_lang::include_app!("src/ui/widget_selectors.ice");

    #[test]
    fn constructs_native_selector_tasks() {
        let (mut app, _) = WidgetSelectors::__boot();
        for message in [
            __WidgetSelectorsMessage::FindId,
            __WidgetSelectorsMessage::FindText,
            __WidgetSelectorsMessage::FindPoint,
            __WidgetSelectorsMessage::FindFocused,
            __WidgetSelectorsMessage::FindAllText,
            __WidgetSelectorsMessage::FindCustom,
        ] {
            assert_eq!(app.__update(message).units(), 2);
        }
    }
}

#[cfg(test)]
mod component_state {
    ui_lang::include_app!("src/ui/component_state.ice");

    #[test]
    fn selects_pick_list_option_through_rendered_overlay() {
        let (mut app, _) = ComponentState::__boot();
        let viewport = iced::Size::new(1800.0, 1200.0);
        let mut screen =
            iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view());
        let pick = screen
            .find(iced_test::selector::id(
                "ComponentState/interactions/root/pick",
            ))
            .expect("pick")
            .bounds();
        screen.point_at(iced::Point::new(pick.x + 10.0, pick.center_y()));
        screen
            .simulate(iced_test::simulator::click())
            .into_iter()
            .find(|status| *status == iced::event::Status::Captured)
            .expect("pick must capture its opening click");

        let option = iced::Point::new(pick.center_x(), pick.y + pick.height + 10.0);
        screen.point_at(option);
        screen
            .simulate([iced::Event::Touch(iced::touch::Event::FingerPressed {
                id: iced::touch::Finger(0),
                position: option,
            })])
            .into_iter()
            .find(|status| *status == iced::event::Status::Captured)
            .expect("pick option must capture its click");
        for message in screen.into_messages() {
            let _ = app.__update(message);
        }

        iced_test::Simulator::with_size(iced::Settings::default(), viewport, app.__view())
            .find("pick routed")
            .expect("selected pick option rerender");
    }

    #[test]
    fn keeps_component_instances_isolated() {
        let (mut app, _) = ComponentState::__boot();
        let _ = app.__update(__ComponentStateMessage::__CounterHandleIncrement(
            "first".into(),
        ));
        let _ = app.__update(__ComponentStateMessage::__CounterBindDraft(
            "second".into(),
            "local".into(),
        ));
        let _ = app.__update(__ComponentStateMessage::__CounterHandleChanged(
            "first".into(),
            true,
        ));
        let _ = app.__update(__ComponentStateMessage::__FlagHandleChanged(
            "first/flag".into(),
            true,
        ));

        assert_eq!(app.__ice_component_counter["first"].count, 1);
        assert!(app.__ice_component_counter["first"].enabled);
        assert_eq!(app.__ice_component_counter["second"].count, 0);
        assert!(!app.__ice_component_counter["second"].enabled);
        assert_eq!(app.__ice_component_counter["second"].draft, "local");
        assert!(app.__ice_component_flag["first/flag"].checked);
        assert!(!app.__ice_component_flag.contains_key("second/flag"));
    }

    #[test]
    fn drops_stale_component_future_results() {
        let (mut app, _) = ComponentState::__boot();
        let _ = app.__update(__ComponentStateMessage::__LoaderHandleLoad("loader".into()));
        let _ = app.__update(__ComponentStateMessage::__LoaderHandleLoad("loader".into()));
        assert!(app.__ice_component_loader["loader"].loading);
        assert_eq!(app.__ice_component_loader["loader"].__ice_latest_58, 2);

        let stale = __ComponentStateMessage::__LoaderLatest58(
            "loader".into(),
            1,
            Box::new(__ComponentStateMessage::__LoaderHandleLoaded(
                "loader".into(),
                Vec::new(),
            )),
        );
        let _ = app.__update(stale);
        assert!(app.__ice_component_loader["loader"].loading);

        let current = __ComponentStateMessage::__LoaderLatest58(
            "loader".into(),
            2,
            Box::new(__ComponentStateMessage::__LoaderHandleLoaded(
                "loader".into(),
                Vec::new(),
            )),
        );
        let _ = app.__update(current);
        assert!(!app.__ice_component_loader["loader"].loading);
    }
}

#[cfg(test)]
mod component_lifecycle {
    ui_lang::include_app!("src/ui/component_lifecycle.ice");

    #[test]
    fn prunes_mounted_component_state_after_rendered_removal() {
        let (mut app, _) = ComponentLifecycle::__boot();
        let scope = "ComponentLifecycle/search";
        let _ = app.__update(__ComponentLifecycleMessage::__SearchHandleLoad(
            scope.into(),
        ));
        assert!(app.__ice_component_search.values().contains_key(scope));
        let previous = app.__ice_component_search.values()[scope]
            .__ice_replace_30
            .as_ref()
            .unwrap()
            .clone();
        assert!(!previous.is_aborted());
        let _ = app.__update(__ComponentLifecycleMessage::__SearchHandleLoad(
            scope.into(),
        ));
        assert!(previous.is_aborted());
        assert_eq!(
            app.__ice_component_search.values()[scope].__ice_latest_30,
            2
        );

        let _ = app.__view();
        assert!(app.__ice_component_search.values().contains_key(scope));
        app.show = false;
        let _ = app.__view();
        assert!(app.__ice_component_search.values().is_empty());
    }
}

#[cfg(test)]
mod test_mount_features {
    ui_lang::include_app!("src/ui/test_mount_features.ice");
}

#[cfg(test)]
mod component_output {
    mod plugin_backend {
        pub use crate::backend::borrowed_help;
    }

    ui_lang::include_app!("src/ui/component_output.ice");

    #[test]
    fn routes_nested_plugin_component_outputs() {
        let (mut app, _) = ComponentOutput::__boot();
        let _ = app.__update(__ComponentOutputMessage::Changed(true));
        assert!(app.active);
    }
}
