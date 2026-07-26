ui_lang::include_app!("src/ui/showcase.ice");

mod backend {
    pub fn sticky_nav_top(scroll_y: f64, viewport_width: f64) -> f64 {
        let origin = if viewport_width < 800.0 { 230.0 } else { 198.0 };
        (origin - scroll_y).max(10.0)
    }
}

fn main() -> iced::Result {
    Showcase::run()
}

#[cfg(test)]
mod tests {
    use super::Showcase;

    fn with_showcase_stack(test: fn()) {
        std::thread::Builder::new()
            .name("showcase-test".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(test)
            .expect("spawn showcase test")
            .join()
            .expect("showcase test thread");
    }

    #[test]
    fn showcase_runs_ice_behavior_cases() {
        with_showcase_stack(|| {
            iced_test::run(
                Showcase::__program(),
                concat!(env!("CARGO_MANIFEST_DIR"), "/tests/cases"),
            )
            .expect("showcase Ice behavior cases");
        });
    }

    #[test]
    fn showcase_records_stay_in_showcase() {
        let shared = include_str!("../../../crates/ui/src/ice/components.ice");
        let showcase = include_str!("ui/components.ice");

        for sample in [
            "민서",
            "acme-research",
            "127.0.0.1",
            "0x8c4f",
            "ducktape-core",
            "@builder",
        ] {
            assert!(!shared.contains(sample), "showcase value leaked: {sample}");
            assert!(
                showcase.contains(sample),
                "missing showcase value: {sample}"
            );
        }
    }

    #[test]
    fn showcase_owns_the_complete_reference_catalog() {
        let shared = include_str!("../../../crates/ui/src/ice/components.ice");
        let showcase = include_str!("ui/components.ice");
        let chrome = "PageHeader SectionNav HairlineDivider PageFooter";
        let primitives = "ButtonStateSwatch CodeLine GlassSample InkSwatch LangTag LayoutSpec LineSwatch LogLine MetaRow ModuleRow MotionSpec NavLink RadiusChip RuleNote SectionHeading SectionHeadingPlain ShadowSpec SpecRow StateCard StatusPill SurfaceSwatch TypeSpecRow";
        let icons = "Add Approvals ArrowRight BranchMini BranchSmall Chat Check ChevronRight Copy External File Folder Forge Gear Members Modules ModulesMini Node Search Settings Shield";
        let blocks = "AccentStateBlock AppShellBlock ApprovalCardBlock AvatarsBlock BadgesBlock BreadcrumbBlock ButtonsBlock CardsBlock CenteredFormBlock ComposerBlock DirectBlock DoBlock DontBlock ElevationBlock FeedbackBlock FileTreeBlock FullWidthCtaBlock GlassBlock InkBlock InputBlock InviteBlock KeyValueRowsBlock KeyboardBlock LabelPillsBlock LinesBlock LogEventInspectorBlock MemberRowBlock MentionAutocompleteBlock MenuTooltipBlock MessageRowBlock ModalBlock ModuleRowBlock MotionBlock NavRailListBlock PaneHeaderBlock PullRequestRowBlock QuorumDotsBlock RadiusBlock RepoCardBlock RepoTabsBlock SegmentedBlock SkeletonBlock SpacingFrameBlock StateMatrixBlock StatusPillBlock SurfacesBlock TabsBlock ThreadPanelBlock";
        let sections = "Section01MaterialColor Section02Typography Section03ShapeDepthMotion Section04Components Section05LayoutPatterns Section06Iconography Section07DataDisplay Section08ComposerOverlays Section09Voice Section10ForgeCode Section11Patterns Section12Rules";

        for name in chrome
            .split_whitespace()
            .chain(primitives.split_whitespace())
        {
            assert!(
                shared.contains(&format!("component {name}")),
                "missing shared reference family: {name}"
            );
        }
        for name in icons.split_whitespace() {
            assert!(
                shared.contains(&format!("component Icon.{name}(")),
                "missing shared reference icon: {name}"
            );
        }
        for name in blocks.split_whitespace().chain(sections.split_whitespace()) {
            let declaration = format!("component {name}()");
            let body = showcase
                .split_once(&declaration)
                .unwrap_or_else(|| panic!("missing showcase reference component: {name}"))
                .1
                .split("\ncomponent ")
                .next()
                .expect("component body");
            assert_ne!(
                body.trim(),
                "slot",
                "pass-through reference component: {name}"
            );
        }
    }

    #[test]
    fn showcase_matches_the_full_catalog_snapshots() {
        with_showcase_stack(|| {
            let program = Showcase::__program();
            let preset = iced::Program::presets(&program)
                .iter()
                .find(|preset| preset.name() == "test")
                .expect("test preset");
            let (state, _task) = preset.boot();
            let mut state = state;
            let window = iced::window::Id::unique();
            let mut simulator = iced_test::Simulator::with_size(
                iced::Program::settings(&program),
                iced::Size::new(1440.0, 900.0),
                iced::Program::view(&program, &state, window),
            );
            simulator.point_at(iced::Point::new(720.0, 450.0));

            for (index, name) in [
                "showcase-00",
                "showcase-01",
                "showcase-02",
                "showcase-03",
                "showcase-04",
                "showcase-05",
                "showcase-06",
                "showcase-07",
                "showcase-08",
                "showcase-09",
                "showcase-10",
            ]
            .into_iter()
            .enumerate()
            {
                if index == 1 {
                    drop(simulator);
                    state.catalog_y = 790.0;
                    simulator = iced_test::Simulator::with_size(
                        iced::Program::settings(&program),
                        iced::Size::new(1440.0, 900.0),
                        iced::Program::view(&program, &state, window),
                    );
                    simulator.point_at(iced::Point::new(720.0, 450.0));
                }
                if index > 0 {
                    let _ = simulator.simulate([iced::Event::Mouse(
                        iced::mouse::Event::WheelScrolled {
                            delta: iced::mouse::ScrollDelta::Pixels { x: 0.0, y: -790.0 },
                        },
                    )]);
                }

                let theme = iced::Program::theme(&program, &state, window).expect("showcase theme");
                let snapshot = simulator.snapshot(&theme).expect("headless screenshot");
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/snapshots")
                    .join(format!("{name}.png"));
                let matches = snapshot
                    .matches_image(path)
                    .expect("showcase snapshot comparison");

                assert!(matches, "showcase snapshot {name} changed");
            }

            drop(simulator);
            state.catalog_y = 0.0;
            let theme = iced::Program::theme(&program, &state, window).expect("showcase theme");
            let mut simulator = iced_test::Simulator::with_size(
                iced::Program::settings(&program),
                iced::Size::new(720.0, 900.0),
                iced::Program::view(&program, &state, window),
            );
            let snapshot = simulator.snapshot(&theme).expect("compact screenshot");
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/snapshots/showcase-compact-00.png");
            assert!(
                snapshot
                    .matches_image(path)
                    .expect("compact snapshot comparison"),
                "compact showcase snapshot changed"
            );
        });
    }
}
