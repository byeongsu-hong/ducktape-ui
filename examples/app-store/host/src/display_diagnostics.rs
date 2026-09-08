//! Host observations and advisory producer reports retain separate provenance.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct FrameReports {
    pub(super) local: wire::SanitizeReport,
    pub(super) upstream: wire::SanitizeReport,
}
impl FrameReports {
    pub(super) fn inherit(&mut self, held: Self) {
        self.local.merge(held.local);
        self.upstream.merge(held.upstream);
    }
}
#[derive(Default)]
pub(super) struct DisplayDiagnostics {
    seen: FrameReports,
}
impl DisplayDiagnostics {
    pub(super) fn observe(&mut self, reports: FrameReports) -> [Option<&'static str>; 2] {
        let mut origins = [None, None];
        if reports.local.display_text_truncated && !self.seen.local.display_text_truncated {
            origins[0] = Some("host");
        }
        if reports.upstream.display_text_truncated && !self.seen.upstream.display_text_truncated {
            origins[1] = Some("producer-reported");
        }
        self.seen.inherit(reports);
        origins
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shape_observes_loss_independently_of_an_untrusted_report() {
        let reported = wire::SanitizeReport {
            display_text_truncated: true,
        };
        let frame = wire::Frame {
            upstream_sanitization: reported,
            ..Default::default()
        };
        let (_, reports) = shape(&wire::encode(&frame)).unwrap();
        assert_eq!(reports.upstream, reported);
        assert_eq!(reports.local, wire::SanitizeReport::default());
        let frame = wire::Frame {
            root: Some(wire::Node::Text {
                key: "display".into(),
                content: "x".repeat(wire::MAX_STRING_BYTES + 1),
                size: None,
                color: None,
                font: Default::default(),
                width: None,
                align_x: None,
                options: Default::default(),
            }),
            ..Default::default()
        };
        let (_, reports) = shape(&wire::encode(&frame)).unwrap();
        assert_eq!(reports.local, reported);
        assert_eq!(reports.upstream, wire::SanitizeReport::default());
    }

    #[test]
    fn reports_are_deduplicated_by_origin_for_one_installation() {
        let reported = wire::SanitizeReport {
            display_text_truncated: true,
        };
        let local = FrameReports {
            local: reported,
            ..Default::default()
        };
        let upstream = FrameReports {
            upstream: reported,
            ..Default::default()
        };
        let mut active = DisplayDiagnostics::default();
        assert_eq!(active.observe(local), [Some("host"), None]);
        assert_eq!(active.observe(local), [None, None]);
        assert_eq!(active.observe(FrameReports::default()), [None, None]);
        assert_eq!(
            active.observe(local),
            [None, None],
            "a healthy frame doesn't create a new generation"
        );
        assert_eq!(active.observe(upstream), [None, Some("producer-reported")]);
        assert_eq!(active.observe(upstream), [None, None]);
        let mut next_installation = DisplayDiagnostics::default();
        assert_eq!(next_installation.observe(local), [Some("host"), None]);
    }
}

#[cfg(test)]
mod actual_tests {
    use super::*;

    fn button(node: &wire::Node, label: &str) -> Option<u32> {
        if let wire::Node::Button {
            content: wire::ButtonContent::Label(text),
            on_press,
            ..
        } = node
            && text == label
        {
            return *on_press;
        }
        node.children()
            .iter()
            .find_map(|child| button(child, label))
    }
    fn press(guest: &mut Guest, label: &str) {
        let handler = button(guest.frame.root.as_ref().unwrap(), label).expect("fixture route");
        guest.pending.push(wire::Event::Message(handler));
        guest.tick();
        assert!(guest.fault.is_none(), "{:?}", guest.fault);
    }
    #[test]
    #[ignore = "requires current native and Wasm text-budget fixtures"]
    fn display_budget_native_and_wasm_report_patch_loss_and_preserve_reload_report() {
        for native in [false, true] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(if native {
                "../target/text-budget-native"
            } else {
                "../target/text-budget-fixture"
            });
            let entries = crate::catalog::scan_dir(&path);
            assert_eq!(entries.len(), 1, "build current text-budget fixture");
            let entry = entries[0].clone();
            let mut guest = Guest::load(&entry).unwrap();
            guest.tick();
            assert!(guest.fault.is_none(), "{:?}", guest.fault);
            assert_eq!(
                guest.frame_reports,
                FrameReports::default(),
                "within-budget control"
            );
            assert_eq!(guest.display_diagnostics.seen, FrameReports::default());
            let old_generation = guest.generation;
            press(&mut guest, "Grow");
            assert!(guest.patched > 0, "actual guest used a patch frame");
            assert!(
                guest.frame_reports.local.display_text_truncated,
                "applied timeline tail loss must be observed by host"
            );
            assert!(
                !guest.frame_reports.upstream.display_text_truncated,
                "guest supplied no diagnostic flag"
            );
            assert!(guest.display_diagnostics.seen.local.display_text_truncated);
            assert_eq!(
                guest.display_diagnostics.observe(guest.frame_reports),
                [None, None],
                "production already emitted one host warning"
            );
            guest.tick();
            assert!(
                guest.frame_reports.local.display_text_truncated,
                "unchanged tree retains loss history"
            );
            press(&mut guest, "Shrink");
            assert!(
                guest.frame_reports.local.display_text_truncated,
                "patch removal cannot certify previously lost bytes restored"
            );
            guest.pending.push(wire::Event::Resync);
            guest.tick();
            assert!(guest.fault.is_none(), "{:?}", guest.fault);
            assert_eq!(
                guest.frame_reports,
                FrameReports::default(),
                "a fresh, untruncated full tree clears held-tree loss"
            );
            assert!(guest.display_diagnostics.seen.local.display_text_truncated);
            press(&mut guest, "Grow");
            let running = Running {
                id: entry.id.clone(),
                name: entry.name.clone(),
                surface: Surface(Arc::new(Mutex::new(guest))),
                window: iced::window::Id::unique(),
            };
            let reload =
                iced::futures::executor::block_on(prepare_reload(entry, vec![running.clone()], 1));
            reload::finish_reload(std::slice::from_ref(&running), 1, reload).unwrap();
            let installed = running.surface.0.lock().unwrap();
            assert_ne!(installed.generation, old_generation);
            assert!(
                installed.frame_reports.local.display_text_truncated,
                "candidate's already-sanitized first full frame carries its local report into installation"
            );
            assert!(
                installed
                    .display_diagnostics
                    .seen
                    .local
                    .display_text_truncated,
                "new successful installation emitted its own warning"
            );
            assert!(!installed.frame_reports.upstream.display_text_truncated);
        }
    }
}
