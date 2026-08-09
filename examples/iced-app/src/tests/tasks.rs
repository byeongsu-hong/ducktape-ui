#[cfg(test)]
mod font_events {
    ui_lang::include_app!("src/ui/font_events.ice");
}

#[cfg(test)]
mod task_groups {
    ui_lang::include_app!("src/ui/task_groups.ice");

    #[test]
    fn compiles_nested_request_lanes_for_a_component_without_declared_state() {
        let _ = TaskGroups::__boot();
    }
}

#[cfg(test)]
mod task_cancel {
    ui_lang::include_app!("src/ui/task_cancel.ice");

    #[test]
    fn aborts_native_task_handle() {
        let (mut app, _) = TaskCancel::__boot();
        let task = app.__update(__TaskCancelMessage::Start);
        assert!(!app.request.as_ref().unwrap().is_aborted());

        let _ = app.__update(__TaskCancelMessage::Cancel);
        assert!(app.request.as_ref().unwrap().is_aborted());
        drop(task);
    }
}

#[cfg(test)]
mod request_lane_lifecycle {
    ui_lang::include_app!("src/ui/request_lane_lifecycle.ice");

    fn output(
        task: iced::Task<__RequestLaneLifecycleMessage>,
    ) -> Option<__RequestLaneLifecycleMessage> {
        use iced::futures::StreamExt;

        let mut stream = iced_runtime::task::into_stream(task).expect("request task stream");
        iced::futures::executor::block_on(async move {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    return Some(message);
                }
            }
            None
        })
    }

    #[test]
    fn latest_lane_filters_a_stale_completion_across_handlers_without_cancelling_it() {
        let (mut app, _) = RequestLaneLifecycle::__boot();
        let first = app.__update(__RequestLaneLifecycleMessage::LatestFirst);
        let second = app.__update(__RequestLaneLifecycleMessage::LatestSecond);
        assert!(!crate::backend::controlled_request_was_cancelled(101));

        crate::backend::complete_controlled_request(102, "current");
        let _ = app.__update(output(second).expect("current latest completion"));
        assert_eq!(app.latest_result, "current");

        crate::backend::complete_controlled_request(101, "stale");
        let _ = app.__update(output(first).expect("stale latest completion"));
        assert_eq!(app.latest_result, "current");
        assert!(!crate::backend::controlled_request_was_cancelled(101));
    }

    #[test]
    fn replace_lane_aborts_prior_work_across_handlers_and_routes_the_replacement() {
        let (mut app, _) = RequestLaneLifecycle::__boot();
        let first = app.__update(__RequestLaneLifecycleMessage::ReplaceFirst);
        let second = app.__update(__RequestLaneLifecycleMessage::ReplaceSecond);
        assert!(app.__ice_run_lane_1_handle.is_some());

        crate::backend::complete_controlled_request(201, "stale");
        assert!(output(first).is_none(), "aborted work must not emit");
        assert!(crate::backend::controlled_request_was_cancelled(201));

        crate::backend::complete_controlled_request(202, "current");
        let _ = app.__update(output(second).expect("replacement completion"));
        assert_eq!(app.replace_result, "current");
        assert!(app.__ice_run_lane_1_handle.is_none());
    }

    #[test]
    fn invalidating_latest_advances_the_generation_without_cancelling_work() {
        let (mut app, _) = RequestLaneLifecycle::__boot();
        let task = app.__update(__RequestLaneLifecycleMessage::LatestForInvalidation);
        let started_generation = app.__ice_run_lane_0_generation;

        let _ = app.__update(__RequestLaneLifecycleMessage::InvalidateLatest);
        assert!(app.__ice_run_lane_0_generation > started_generation);
        assert!(!crate::backend::controlled_request_was_cancelled(103));

        crate::backend::complete_controlled_request(103, "stale");
        let _ = app.__update(output(task).expect("invalidated latest completion"));
        assert_eq!(app.latest_result, "waiting");
        assert!(!crate::backend::controlled_request_was_cancelled(103));
    }

    #[test]
    fn invalidating_replace_aborts_and_clears_the_current_handle() {
        let (mut app, _) = RequestLaneLifecycle::__boot();
        let task = app.__update(__RequestLaneLifecycleMessage::ReplaceForInvalidation);
        let started_generation = app.__ice_run_lane_1_generation;
        let observer = app.__ice_run_lane_1_handle.as_ref().unwrap().clone();

        let _ = app.__update(__RequestLaneLifecycleMessage::InvalidateReplace);
        assert!(app.__ice_run_lane_1_generation > started_generation);
        assert!(observer.is_aborted());
        assert!(app.__ice_run_lane_1_handle.is_none());
        assert!(output(task).is_none(), "invalidated work must not emit");
        assert!(crate::backend::controlled_request_was_cancelled(203));
        assert_eq!(app.replace_result, "waiting");
    }

    #[test]
    fn retained_component_invalidation_is_instance_scoped() {
        let (mut app, _) = RequestLaneLifecycle::__boot();
        let first_scope = "RequestLaneLifecycle/retained-first";
        let second_scope = "RequestLaneLifecycle/retained-second";
        let first = app.__update(__RequestLaneLifecycleMessage::__RetainedHandleStart(
            first_scope.into(),
            301,
        ));
        let second = app.__update(__RequestLaneLifecycleMessage::__RetainedHandleStart(
            second_scope.into(),
            302,
        ));
        let first_observer = app.__ice_component_retained[first_scope]
            .__ice_run_lane_2_handle
            .as_ref()
            .unwrap()
            .clone();

        let _ = app.__update(
            __RequestLaneLifecycleMessage::__RetainedHandleInvalidateRequest(first_scope.into()),
        );
        assert!(first_observer.is_aborted());
        assert!(
            app.__ice_component_retained[first_scope]
                .__ice_run_lane_2_handle
                .is_none()
        );
        assert!(
            app.__ice_component_retained[second_scope]
                .__ice_run_lane_2_handle
                .is_some()
        );
        assert!(
            output(first).is_none(),
            "invalidated instance must not emit"
        );
        assert!(crate::backend::controlled_request_was_cancelled(301));

        crate::backend::complete_controlled_request(302, "second current");
        let _ = app.__update(output(second).expect("second component completion"));
        assert_eq!(app.__ice_component_retained[first_scope].result, "waiting");
        assert_eq!(
            app.__ice_component_retained[second_scope].result,
            "second current"
        );
    }

    #[test]
    fn mounted_invalidation_keeps_generation_monotonic_across_remount() {
        let (mut app, _) = RequestLaneLifecycle::__boot();
        let scope = "RequestLaneLifecycle/mounted";
        let _ = app.__view();
        let old = app.__update(__RequestLaneLifecycleMessage::__MountedHandleStart(
            scope.into(),
            401,
        ));
        let old_generation =
            app.__ice_component_mounted.values()[scope].__ice_run_lane_3_generation;
        crate::backend::complete_controlled_request(401, "old");
        let old_completion = output(old).expect("old mounted completion");

        let _ = app.__update(
            __RequestLaneLifecycleMessage::__MountedHandleInvalidateRequest(scope.into()),
        );
        let invalidated_generation =
            app.__ice_component_mounted.values()[scope].__ice_run_lane_3_generation;
        assert!(invalidated_generation > old_generation);

        app.show_mounted = false;
        let _ = app.__view();
        // Pruning lands one pass late; see `begin_render`.
        let _ = app.__view();
        assert!(app.__ice_component_mounted.values().is_empty());
        app.show_mounted = true;
        let _ = app.__view();

        let current = app.__update(__RequestLaneLifecycleMessage::__MountedHandleStart(
            scope.into(),
            402,
        ));
        let current_generation =
            app.__ice_component_mounted.values()[scope].__ice_run_lane_3_generation;
        assert!(current_generation > invalidated_generation);
        assert_eq!(
            app.__ice_component_mounted.values()[scope].result,
            "waiting"
        );

        let _ = app.__update(old_completion);
        assert_eq!(
            app.__ice_component_mounted.values()[scope].result,
            "waiting"
        );

        crate::backend::complete_controlled_request(402, "current");
        let _ = app.__update(output(current).expect("current mounted completion"));
        assert_eq!(
            app.__ice_component_mounted.values()[scope].result,
            "current"
        );
    }
}

#[cfg(test)]
mod stream_lane_lifecycle {
    ui_lang::include_app!("src/ui/stream_lane_lifecycle.ice");

    fn next_output<Message>(
        stream: &mut (impl iced::futures::Stream<Item = iced_runtime::Action<Message>> + Unpin),
    ) -> Option<Message> {
        use iced::futures::StreamExt;

        iced::futures::executor::block_on(async {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    return Some(message);
                }
            }
            None
        })
    }

    #[test]
    fn restart_aborts_the_old_stream_and_filters_an_already_queued_item() {
        let (mut app, _) = StreamLaneLifecycle::__boot();
        let first = app.__update(__StreamLaneLifecycleMessage::Start(701));
        let mut first = iced_runtime::task::into_stream(first).expect("first stream task");
        let first_observer = app.__ice_run_lane_0_handle.as_ref().unwrap().clone();

        crate::backend::emit_controlled_stream(701, "stale queued");
        let stale = next_output(&mut first).expect("queued stale item");

        let second = app.__update(__StreamLaneLifecycleMessage::Start(702));
        let mut second = iced_runtime::task::into_stream(second).expect("replacement stream task");
        assert!(first_observer.is_aborted());
        assert!(app.__ice_run_lane_0_handle.is_some());
        assert!(
            next_output(&mut first).is_none(),
            "the restarted stream must stop"
        );
        assert!(crate::backend::controlled_stream_was_cancelled(701));

        let _ = app.__update(stale);
        assert_eq!(app.result, "waiting");
        assert!(app.__ice_run_lane_0_handle.is_some());

        crate::backend::emit_controlled_stream(702, "current");
        let _ = app.__update(next_output(&mut second).expect("current item"));
        assert_eq!(app.result, "current");
        assert!(app.__ice_run_lane_0_handle.is_some());

        crate::backend::finish_controlled_stream(702);
        let _ = app.__update(next_output(&mut second).expect("natural terminal"));
        assert!(app.__ice_run_lane_0_handle.is_none());
        assert!(next_output(&mut second).is_none());
    }

    #[test]
    fn multiple_items_keep_the_handle_until_the_natural_terminal() {
        let (mut app, _) = StreamLaneLifecycle::__boot();
        let task = app.__update(__StreamLaneLifecycleMessage::Start(711));
        let mut stream = iced_runtime::task::into_stream(task).expect("stream task");
        let observer = app.__ice_run_lane_0_handle.as_ref().unwrap().clone();

        crate::backend::emit_controlled_stream(711, "first");
        let _ = app.__update(next_output(&mut stream).expect("first item"));
        assert_eq!(app.result, "first");
        assert!(app.__ice_run_lane_0_handle.is_some());
        assert!(!observer.is_aborted());

        crate::backend::emit_controlled_stream(711, "second");
        let _ = app.__update(next_output(&mut stream).expect("second item"));
        assert_eq!(app.result, "second");
        assert!(app.__ice_run_lane_0_handle.is_some());

        crate::backend::finish_controlled_stream(711);
        if let Some(terminal) = next_output(&mut stream) {
            let _ = app.__update(terminal);
        }
        assert!(
            app.__ice_run_lane_0_handle.is_none(),
            "the natural terminal must clear the lane handle"
        );
        assert!(next_output(&mut stream).is_none());
        assert!(!crate::backend::controlled_stream_was_cancelled(711));
    }

    #[test]
    fn invalidate_aborts_the_stream_and_advances_its_generation() {
        let (mut app, _) = StreamLaneLifecycle::__boot();
        let task = app.__update(__StreamLaneLifecycleMessage::Start(721));
        let mut stream = iced_runtime::task::into_stream(task).expect("stream task");
        let started_generation = app.__ice_run_lane_0_generation;
        let observer = app.__ice_run_lane_0_handle.as_ref().unwrap().clone();

        let _ = app.__update(__StreamLaneLifecycleMessage::InvalidateFeed);
        assert!(app.__ice_run_lane_0_generation > started_generation);
        assert!(observer.is_aborted());
        assert!(app.__ice_run_lane_0_handle.is_none());
        assert!(next_output(&mut stream).is_none());
        assert!(crate::backend::controlled_stream_was_cancelled(721));
        assert_eq!(app.result, "waiting");
    }

    #[test]
    fn retained_stream_lanes_are_isolated_by_instance() {
        let (mut app, _) = StreamLaneLifecycle::__boot();
        let first_scope = "StreamLaneLifecycle/retained-first";
        let second_scope = "StreamLaneLifecycle/retained-second";
        let first = app.__update(__StreamLaneLifecycleMessage::__RetainedHandleStart(
            first_scope.into(),
            731,
        ));
        let second = app.__update(__StreamLaneLifecycleMessage::__RetainedHandleStart(
            second_scope.into(),
            732,
        ));
        let mut first = iced_runtime::task::into_stream(first).expect("first retained stream");
        let mut second = iced_runtime::task::into_stream(second).expect("second retained stream");
        let first_observer = app.__ice_component_retained[first_scope]
            .__ice_run_lane_1_handle
            .as_ref()
            .unwrap()
            .clone();

        let _ = app.__update(
            __StreamLaneLifecycleMessage::__RetainedHandleInvalidateFeed(first_scope.into()),
        );
        assert!(first_observer.is_aborted());
        assert!(
            app.__ice_component_retained[first_scope]
                .__ice_run_lane_1_handle
                .is_none()
        );
        assert!(
            app.__ice_component_retained[second_scope]
                .__ice_run_lane_1_handle
                .is_some()
        );
        assert!(next_output(&mut first).is_none());
        assert!(crate::backend::controlled_stream_was_cancelled(731));

        crate::backend::emit_controlled_stream(732, "second current");
        let _ = app.__update(next_output(&mut second).expect("second retained item"));
        assert_eq!(app.__ice_component_retained[first_scope].result, "waiting");
        assert_eq!(
            app.__ice_component_retained[second_scope].result,
            "second current"
        );
        assert!(
            app.__ice_component_retained[second_scope]
                .__ice_run_lane_1_handle
                .is_some()
        );

        crate::backend::finish_controlled_stream(732);
        let _ = app.__update(next_output(&mut second).expect("second retained terminal"));
        assert!(
            app.__ice_component_retained[second_scope]
                .__ice_run_lane_1_handle
                .is_none()
        );
        assert!(!crate::backend::controlled_stream_was_cancelled(732));
    }

    #[test]
    fn mounted_stream_lane_rejects_old_instance_items_after_remount() {
        let (mut app, _) = StreamLaneLifecycle::__boot();
        let scope = "StreamLaneLifecycle/mounted";
        let _ = app.__view();
        let old = app.__update(__StreamLaneLifecycleMessage::__MountedHandleStart(
            scope.into(),
            741,
        ));
        let mut old = iced_runtime::task::into_stream(old).expect("old mounted stream");
        let old_generation =
            app.__ice_component_mounted.values()[scope].__ice_run_lane_2_generation;
        assert!(
            app.__ice_component_mounted.values()[scope]
                .__ice_run_lane_2_handle
                .is_some()
        );

        crate::backend::emit_controlled_stream(741, "old queued");
        let old_item = next_output(&mut old).expect("old queued item");
        let _ = app.__update(__StreamLaneLifecycleMessage::HideMounted);
        let _ = app.__view();
        // Pruning lands one pass late; see `begin_render`.
        let _ = app.__view();
        assert!(app.__ice_component_mounted.values().is_empty());
        assert!(next_output(&mut old).is_none());
        assert!(crate::backend::controlled_stream_was_cancelled(741));

        let _ = app.__update(__StreamLaneLifecycleMessage::ShowMounted);
        let _ = app.__view();
        let current = app.__update(__StreamLaneLifecycleMessage::__MountedHandleStart(
            scope.into(),
            742,
        ));
        let mut current = iced_runtime::task::into_stream(current).expect("current mounted stream");
        let current_generation =
            app.__ice_component_mounted.values()[scope].__ice_run_lane_2_generation;
        assert!(current_generation > old_generation);

        let _ = app.__update(old_item);
        assert_eq!(
            app.__ice_component_mounted.values()[scope].result,
            "waiting"
        );
        assert!(
            app.__ice_component_mounted.values()[scope]
                .__ice_run_lane_2_handle
                .is_some()
        );

        crate::backend::emit_controlled_stream(742, "current");
        let _ = app.__update(next_output(&mut current).expect("current mounted item"));
        assert_eq!(
            app.__ice_component_mounted.values()[scope].result,
            "current"
        );
        assert!(
            app.__ice_component_mounted.values()[scope]
                .__ice_run_lane_2_handle
                .is_some()
        );

        crate::backend::finish_controlled_stream(742);
        let _ = app.__update(next_output(&mut current).expect("current mounted terminal"));
        assert!(
            app.__ice_component_mounted.values()[scope]
                .__ice_run_lane_2_handle
                .is_none()
        );
        assert!(!crate::backend::controlled_stream_was_cancelled(742));
    }
}

#[cfg(test)]
mod route_snapshot_lifecycle {
    ui_lang::include_app!("src/ui/route_snapshot_lifecycle.ice");

    fn output(
        task: iced::Task<__RouteSnapshotLifecycleMessage>,
    ) -> Option<__RouteSnapshotLifecycleMessage> {
        use iced::futures::StreamExt;

        let mut stream = iced_runtime::task::into_stream(task).expect("snapshot task stream");
        iced::futures::executor::block_on(async move {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    return Some(message);
                }
            }
            None
        })
    }

    #[test]
    fn future_routes_receive_launch_state_derived_param_and_local_values() {
        let (mut app, _) = RouteSnapshotLifecycle::__boot();
        let task = app.__update(__RouteSnapshotLifecycleMessage::StartFuture(
            501,
            "param launch".into(),
        ));
        let _ = app.__update(__RouteSnapshotLifecycleMessage::Change("changed".into()));

        crate::backend::complete_controlled_request(501, "future payload");
        let _ = app.__update(output(task).expect("future completion"));

        assert_eq!(app.future_state, "state launch");
        assert_eq!(app.future_derived, "derived launch");
        assert_eq!(app.future_param, "param launch");
        assert_eq!(app.future_local, "local launch");
        assert_eq!(app.future_payload, "future payload");
        assert_eq!(app.token, "changed");
        assert_eq!(app.derived_source, "derived changed");
    }

    #[test]
    fn task_routes_receive_launch_state_values() {
        let (mut app, _) = RouteSnapshotLifecycle::__boot();
        let task = app.__update(__RouteSnapshotLifecycleMessage::StartTask("launch".into()));
        let _ = app.__update(__RouteSnapshotLifecycleMessage::Change("changed".into()));

        let _ = app.__update(output(task).expect("task completion"));

        assert_eq!(app.task_state, "launch");
        assert_eq!(app.task_payload, 42);
        assert_eq!(app.token, "changed");
    }

    #[test]
    fn component_routes_snapshot_each_instance_state() {
        let (mut app, _) = RouteSnapshotLifecycle::__boot();
        let first_scope = "RouteSnapshotLifecycle/first";
        let second_scope = "RouteSnapshotLifecycle/second";
        let first = app.__update(__RouteSnapshotLifecycleMessage::__SnapshotHandleStart(
            first_scope.into(),
            601,
            "first launch".into(),
        ));
        let second = app.__update(__RouteSnapshotLifecycleMessage::__SnapshotHandleStart(
            second_scope.into(),
            602,
            "second launch".into(),
        ));
        let _ = app.__update(__RouteSnapshotLifecycleMessage::__SnapshotHandleChange(
            first_scope.into(),
            "first changed".into(),
        ));
        let _ = app.__update(__RouteSnapshotLifecycleMessage::__SnapshotHandleChange(
            second_scope.into(),
            "second changed".into(),
        ));

        crate::backend::complete_controlled_request(602, "second payload");
        let _ = app.__update(output(second).expect("second completion"));
        crate::backend::complete_controlled_request(601, "first payload");
        let _ = app.__update(output(first).expect("first completion"));

        assert_eq!(
            app.__ice_component_snapshot[first_scope].captured,
            "first launch"
        );
        assert_eq!(
            app.__ice_component_snapshot[first_scope].payload,
            "first payload"
        );
        assert_eq!(
            app.__ice_component_snapshot[second_scope].captured,
            "second launch"
        );
        assert_eq!(
            app.__ice_component_snapshot[second_scope].payload,
            "second payload"
        );
        assert_eq!(
            app.__ice_component_snapshot[first_scope].token,
            "first changed"
        );
        assert_eq!(
            app.__ice_component_snapshot[second_scope].token,
            "second changed"
        );
    }
}

#[cfg(test)]
mod task_stream {
    ui_lang::include_app!("src/ui/task_stream.ice");

    #[test]
    fn constructs_both_native_stream_units() {
        let (mut app, _) = TaskStream::__boot();
        assert_eq!(app.__update(__TaskStreamMessage::Start).units(), 3);
        assert_eq!(app.__subscription().units(), 7);
    }
}

#[cfg(test)]
mod task_sip {
    ui_lang::include_app!("src/ui/task_sip.ice");

    #[test]
    fn constructs_both_native_sipper_units() {
        let (mut app, _) = TaskSip::__boot();
        assert_eq!(app.__update(__TaskSipMessage::Start).units(), 3);
    }
}

#[cfg(test)]
mod task_flow {
    ui_lang::include_app!("src/ui/task_flow.ice");

    #[test]
    fn constructs_native_task_combinators() {
        let (mut app, _) = TaskFlow::__boot();
        assert_eq!(app.__update(__TaskFlowMessage::Start).units(), 9);
    }
}

#[cfg(test)]
mod task_map {
    ui_lang::include_app!("src/ui/task_map.ice");

    #[test]
    fn maps_success_values_and_preserves_errors() {
        use iced::futures::StreamExt;

        let (mut app, _) = TaskMap::__boot();
        let task = app.__update(__TaskMapMessage::Start);
        let mut stream = iced_runtime::task::into_stream(task).unwrap();
        let messages = iced::futures::executor::block_on(async move {
            let mut messages = Vec::new();
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    messages.push(message);
                }
            }
            messages
        });
        for message in messages {
            let _ = app.__update(message);
        }

        assert_eq!(app.mapped, 5);
        assert_eq!(app.mapped_optional, Some(2));
        assert_eq!(app.mapped_result, 8);
        assert_eq!(app.error, "task failed");
    }
}

#[cfg(test)]
mod theme_factory {
    #![deny(unreachable_code)]

    ui_lang::include_app!("src/ui/theme_factory.ice");

    #[test]
    fn constructs_app_and_nested_native_themes() {
        let (app, _) = NativeTheme::__boot();
        let theme = app.__theme();
        assert_eq!(theme.to_string(), "Native dark");
        assert!(theme.extended_palette().is_dark);
        assert_eq!(
            theme.extended_palette().primary.base.color,
            iced::Color::from_rgb8(0x7c, 0x3a, 0xed)
        );

        assert_eq!(
            crate::backend::native_theme(false).to_string(),
            "Native light"
        );
        let _ = app.__view();
    }
}

#[cfg(test)]
mod alternate_theme {
    ui_lang::include_app!("src/ui/alternate_theme.ice");

    #[test]
    fn constructs_an_alternate_theme_subtree() {
        let (app, _) = AlternateThemeApp::__boot();
        let (theme, _, text_color, background) = crate::backend::alternate_panel(true);
        let theme = theme.unwrap();
        assert_eq!(iced::theme::Base::name(&theme), "Alternate dark");
        assert_eq!(text_color.unwrap()(&theme), iced::Color::WHITE);
        assert_eq!(background.unwrap()(&theme), iced::Color::BLACK.into());

        let (theme, _, text_color, background) = crate::backend::alternate_panel(false);
        assert!(theme.is_none() && text_color.is_none() && background.is_none());
        let _ = app.__view();
    }

    #[test]
    fn renders_the_alternate_themer_through_the_headless_draw_path() {
        let (app, _) = AlternateThemeApp::__boot();
        let counts = super::super::draw_headlessly(
            app.__view(),
            &app.__theme(),
            iced::Size::new(320.0, 120.0),
        );
        assert!(counts.quads > 0 || counts.text > 0);
    }
}

#[cfg(test)]
mod native_overlay {
    ui_lang::include_app!("src/ui/native_overlay.ice");

    #[test]
    fn constructs_a_custom_indexed_overlay() {
        let (app, _) = NativeOverlay::__boot();
        let overlay = crate::backend::IndexedOverlay { index: 42.0 };
        assert_eq!(
            iced::advanced::Overlay::<(), iced::Theme, iced::Renderer>::index(&overlay),
            42.0
        );
        let _ = app.__view();
    }
}

#[cfg(test)]
mod timer {
    ui_lang::include_app!("src/ui/timer.ice");

    #[test]
    fn constructs_all_native_time_operations() {
        let (mut app, _) = TimerEvents::__boot();
        assert_eq!(app.__subscription().units(), 6);
        assert_eq!(app.__update(__TimerEventsMessage::Start).units(), 2);
    }
}

#[cfg(test)]
mod animation {
    ui_lang::include_app!("src/ui/animation.ice");

    #[test]
    fn drives_native_animations_only_while_active() {
        let (mut app, _) = NativeAnimation::__boot();
        assert_eq!(app.__subscription().units(), 2);

        let _ = app.__update(__NativeAnimationMessage::Start);
        assert!(app.expanded.value());
        assert_eq!(app.progress.value(), 1.0);
        assert_eq!(app.custom_motion.value().value, 1.0);
        assert_eq!(app.__subscription().units(), 3);
        let _ = app.__view();

        let _ = app.__update(__NativeAnimationMessage::Sample);
        assert!(app.maybe_progress.is_some());
        assert!(app.maybe_visibility.is_none());
        let _ = app.__update(__NativeAnimationMessage::Rewind(iced::time::Instant::now()));
        assert_eq!(app.progress.value(), 0.0);
        assert_eq!(
            app.__update(__NativeAnimationMessage::__AnimationFrame)
                .units(),
            0
        );
    }

    /// A row's fade belongs to the row: it starts when the instance first
    /// renders, keeps native frames alive on its own, and — because the stored
    /// animation is what later renders read — settles instead of restarting
    /// every pass. Re-evaluating the initializer per render would leave the
    /// frame subscription up forever.
    #[test]
    fn a_row_owns_its_fade_and_that_fade_ends() {
        let (mut app, _) = NativeAnimation::__boot();
        let quiet = app.__subscription().units();

        let _ = app.__update(__NativeAnimationMessage::Arrive);
        assert_eq!(
            app.__subscription().units(),
            quiet,
            "rows that have never rendered own no animation yet"
        );

        let _ = app.__view();
        assert_eq!(
            app.__subscription().units(),
            quiet + 1,
            "the first render of a row starts its fade and asks for native frames"
        );

        std::thread::sleep(std::time::Duration::from_millis(200));
        let _ = app.__view();
        std::thread::sleep(std::time::Duration::from_millis(1400));
        let _ = app.__view();
        assert_eq!(
            app.__subscription().units(),
            quiet,
            "the fade runs on one clock across renders and stops when it is over"
        );
    }
}

#[cfg(test)]
mod image_allocation {
    ui_lang::include_app!("src/ui/image_allocation.ice");

    #[test]
    fn constructs_native_allocation_and_preserves_exact_errors() {
        use iced::futures::StreamExt;

        let (mut app, _) = ImageAllocation::__boot();
        let task = app.__update(__ImageAllocationMessage::Allocate);
        assert_eq!(task.units(), 2);
        let mut stream = iced_runtime::task::into_stream(task).unwrap();
        let message = iced::futures::executor::block_on(async move {
            let mut sent_error = false;
            let mut saw_accessibility_snapshot = false;
            let mut routed_error = None;
            for _ in 0..3 {
                match stream.next().await.expect("batched task action") {
                    iced_runtime::Action::Image(iced_runtime::image::Action::Allocate(
                        _,
                        sender,
                    )) if !sent_error => {
                        sender
                            .send(Err(iced::widget::image::Error::Unsupported))
                            .unwrap();
                        sent_error = true;
                    }
                    iced_runtime::Action::Widget(_) if !saw_accessibility_snapshot => {
                        saw_accessibility_snapshot = true;
                    }
                    iced_runtime::Action::Output(message)
                        if sent_error && routed_error.is_none() =>
                    {
                        routed_error = Some(message);
                    }
                    _ => panic!("unexpected batched task action"),
                }
            }

            assert!(sent_error);
            assert!(saw_accessibility_snapshot);
            routed_error.expect("routed allocation error")
        });
        assert_eq!(
            app.__update(__ImageAllocationMessage::AllocateFlow).units(),
            2
        );
        let _ = app.__update(message);
        assert_eq!(app.error_kind, "unsupported");
        assert_eq!(app.error_message, "loading images is unsupported");
        let _ = app.__view();
    }
}

#[cfg(test)]
mod debug_timing {
    ui_lang::include_app!("src/ui/debug_timing.ice");

    #[test]
    fn owns_and_finishes_native_debug_spans() {
        let (mut app, _) = DebugTiming::__boot();
        assert!(app.timer.is_none());

        let _ = app.__update(__DebugTimingMessage::Begin);
        assert!(app.timer.is_some());
        let _ = app.__update(__DebugTimingMessage::Begin);
        assert!(app.timer.is_some());

        let _ = app.__update(__DebugTimingMessage::Finish);
        assert!(app.timer.is_none());
        let _ = app.__update(__DebugTimingMessage::Compute);
        assert_eq!(app.measured, 42);
        let _ = app.__view();
    }
}

#[cfg(test)]
mod canvas_events {
    ui_lang::include_app!("src/ui/canvas_events.ice");

    #[test]
    fn initializes() {
        let _ = CanvasEvents::__boot();
    }
}

#[cfg(test)]
mod daemon {
    ui_lang::include_app!("src/ui/daemon.ice");

    #[test]
    fn constructs_window_open_and_exit_tasks() {
        let (mut app, open) = BackgroundAgent::__boot();
        let window = iced::window::Id::unique();
        assert_eq!(open.units(), 1);
        assert_eq!(app.__title(window), "Background agent");
        assert_eq!(app.__theme(window), iced::Theme::Dark);
        assert_eq!(app.__scale_factor(window), 1.0);
        let _ = app.__view(window);
        assert_eq!(app.__update(__BackgroundAgentMessage::Quit).units(), 1);
    }
}
