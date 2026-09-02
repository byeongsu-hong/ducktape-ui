//! The per-app frame probe, derived from the app's own tree.
//!
//! A hand-written probe names its targets as string constants and its actions
//! by hand, and both rot the moment the `.ice` file moves a node — silently,
//! because a probe is `#[ignore]`d until someone runs it. This asks the running
//! app instead: every identified target it has, driven by whatever that target
//! says it supports, one fresh app per target so no earlier click decides a
//! later number.
//!
//! What stays hand-written is what cannot be derived: the app's own scenarios —
//! a 1MiB paste, a scroll through 100k rows, the message that only matters
//! under a particular state.

use super::*;

/// The interaction a probe runs against one target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// A left click: the handler behind the target runs.
    Click,
    /// A focus and one keystroke.
    Type,
    /// A wheel scroll of 40 logical pixels.
    Scroll,
    /// The pointer over it — the cheapest interaction that still rebuilds the
    /// interface, which is what makes it the floor every other row is read
    /// against.
    Hover,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::Type => "type",
            Self::Scroll => "scroll",
            Self::Hover => "hover",
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.pad(self.label())
    }
}

/// One identified target, and what one interaction with it cost.
#[derive(Clone, Debug)]
pub struct Interaction {
    /// The target path, as the live tree spells it.
    pub target: String,
    /// The widget kind the action was chosen from.
    pub kind: String,
    /// What was done to it.
    pub action: Action,
    /// `(p50, p95)` microseconds of the interaction itself.
    pub action_us: (u64, u64),
    /// `(p50, p95)` microseconds of the redraw that follows it.
    pub frame_us: (u64, u64),
    /// Rounds actually measured. Fewer than asked for means the target stopped
    /// existing and a rebuilt app did not bring it back.
    pub rounds: usize,
}

/// A target the probe measured nothing for, and why.
#[derive(Clone, Debug)]
pub struct Skipped {
    /// The target path, as the census spelled it.
    pub target: String,
    /// `asked for` (the skip list), `hidden`, `no longer in the tree`, or
    /// `no round completed`.
    pub reason: &'static str,
}

/// What [`measure_interactions`] found.
#[derive(Clone, Debug)]
pub struct Interactions {
    /// Timed rounds per target.
    pub rounds: usize,
    /// `"debug"` or `"release"`; debug microseconds are ratios, not budgets.
    pub build_profile: &'static str,
    /// One row per measured target, most expensive interaction first.
    pub rows: Vec<Interaction>,
    /// Targets no interaction ran against.
    pub skipped: Vec<Skipped>,
}

impl Interactions {
    /// The row for one target, by path.
    pub fn target(&self, path: &str) -> Option<&Interaction> {
        self.rows.iter().find(|row| row.target == path)
    }
}

impl fmt::Display for Interactions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A target path is as long as the app's tree is deep, so the columns
        // are sized to what is in this report rather than to a guess.
        let width = |column: &dyn Fn(&Interaction) -> usize, header: usize| {
            self.rows
                .iter()
                .map(column)
                .chain([header])
                .max()
                .unwrap_or(header)
        };
        let target = width(&|row| row.target.len(), 6);
        let kind = width(&|row| row.kind.len(), 4);

        writeln!(
            formatter,
            "{} targets, {} rounds each, {} build",
            self.rows.len(),
            self.rounds,
            self.build_profile
        )?;
        writeln!(
            formatter,
            "{:<target$}  {:<kind$}  {:<6}  {:>3}  {:>16}  {:>16}",
            "target", "kind", "action", "n", "action p50/p95", "frame p50/p95"
        )?;
        for row in &self.rows {
            writeln!(
                formatter,
                "{:<target$}  {:<kind$}  {:<6}  {:>3}  {:>7}/{:<7} {:>7}/{:<7}",
                row.target,
                row.kind,
                row.action,
                row.rounds,
                row.action_us.0,
                row.action_us.1,
                row.frame_us.0,
                row.frame_us.1
            )?;
        }
        for skipped in &self.skipped {
            writeln!(
                formatter,
                "{:<target$}  skipped: {}",
                skipped.target, skipped.reason
            )?;
        }
        Ok(())
    }
}

/// The interaction a target affords, read from the target rather than from its
/// widget kind: an Ice view wraps most nodes in an accessible one, so the kind
/// of an identified node is often the wrapper's, while what it *supports* is
/// its own.
fn action_for(target: &Target) -> Action {
    let scrolls = target
        .content
        .is_some_and(|content| content.height > target.bounds.height + 1.0);
    let writes = target.kind == "text_input" || target.kind == "text_editor";
    let activates = target
        .accessibility
        .as_ref()
        .is_some_and(|accessibility| accessibility.supports_activate && !accessibility.disabled);

    if writes {
        Action::Type
    } else if scrolls {
        Action::Scroll
    } else if activates {
        Action::Click
    } else {
        Action::Hover
    }
}

/// Times one interaction per identified target of the app `app` builds.
///
/// `app` is called once to ask the app what it has, once per target to measure
/// it, and again whenever an interaction takes its own target off the screen,
/// so what a row reports is that interaction from the state the closure hands
/// back — not from whatever the previous row's click left behind. An app that
/// needs a window opened, a preset applied or a screen selected does it there.
///
/// It *drives the app*: a click runs the handler the button routes to, with the
/// real extern behind it. Pass the ids whose handlers should not run in `skip`.
pub fn measure_interactions<P>(
    app: impl Fn() -> Driver<P>,
    rounds: usize,
    skip: &[&str],
    source: Location,
) -> Interactions
where
    P: Program + 'static,
    P::Renderer: 'static,
    P::Message: Clone,
{
    assert!(rounds > 0, "a probe needs at least one round");

    // One driver to ask what there is, then one per target to measure it.
    let mut census = warm(app(), source);
    let targets = census.known_ids();
    // A daemon's target path carries the window it was built for, and window
    // ids are handed out per process — so every driver after the census has a
    // different one, and the census's paths name a window that driver does not
    // have. Rows keep the census spelling; lookups use the driver's own.
    let census_window = window_segment(census.window());
    drop(census);

    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    for target in targets {
        if skip.contains(&target.as_str()) {
            skipped.push(Skipped {
                target,
                reason: "asked for",
            });
            continue;
        }

        let mut driver = warm(app(), source);
        let mut path = retarget(&target, &census_window, driver.window());
        let Some(inspected) = driver.inspect(&path, false, source) else {
            skipped.push(Skipped {
                target,
                reason: "no longer in the tree",
            });
            continue;
        };
        if !inspected.visible() {
            skipped.push(Skipped {
                target,
                reason: "hidden",
            });
            continue;
        }
        let action = action_for(&inspected);

        let mut action_us = Vec::with_capacity(rounds);
        let mut frame_us = Vec::with_capacity(rounds);
        for _ in 0..rounds {
            // An interaction can take its own target off the screen — a
            // button that opens a bar over itself, a row that navigates. The
            // app is rebuilt when that happens, so the next round measures
            // the same thing this one did rather than panicking on a target
            // the previous round removed.
            if driver.inspect(&path, false, source).is_none() {
                driver = warm(app(), source);
                path = retarget(&target, &census_window, driver.window());
                if driver.inspect(&path, false, source).is_none() {
                    break;
                }
            }
            let started = Instant::now();
            match action {
                Action::Click => driver.click_with(&path, MouseButton::Left, 1, source),
                Action::Type => {
                    driver.focus(&path, source);
                    driver.typewrite("a", source);
                }
                Action::Scroll => driver.scroll_by(&path, 0.0, 40.0, source),
                Action::Hover => driver.move_to(&path, source),
            }
            action_us.push(micros(started.elapsed()));
            let phases = driver.redraw_phases(source);
            frame_us.push(micros(phases.view + phases.layout + phases.update));
        }

        if action_us.is_empty() {
            skipped.push(Skipped {
                target,
                reason: "no round completed",
            });
            continue;
        }
        rows.push(Interaction {
            target,
            kind: inspected.kind(),
            action,
            rounds: action_us.len(),
            action_us: percentiles(action_us),
            frame_us: percentiles(frame_us),
        });
    }

    rows.sort_by(|left, right| {
        right
            .frame_us
            .0
            .cmp(&left.frame_us.0)
            .then_with(|| left.target.cmp(&right.target))
    });

    Interactions {
        rounds,
        build_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        rows,
        skipped,
    }
}

/// A settled driver: boot tasks applied and the same warmup
/// [`Driver::measure_frames`] runs, so a row reports a settled app rather than
/// its first frame.
fn warm<P>(mut driver: Driver<P>, source: Location) -> Driver<P>
where
    P: Program + 'static,
    P::Renderer: 'static,
    P::Message: Clone,
{
    const WARMUP: usize = 8;

    driver.settle(Some(source));
    for _ in 0..WARMUP {
        driver.redraw(source);
    }
    driver
}

/// The path segment a window id contributes to a daemon's target paths.
fn window_segment(window: window::Id) -> String {
    format!("{window:?}")
}

/// `target` with the census window's segment replaced by this driver's.
fn retarget(target: &str, census_window: &str, window: window::Id) -> String {
    match target.contains(census_window) {
        true => target.replace(census_window, &window_segment(window)),
        false => target.to_owned(),
    }
}

fn micros(phase: Duration) -> u64 {
    u64::try_from(phase.as_micros()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_worst_row_is_the_one_the_report_leads_with() {
        let row = |target: &str, frame: u64| Interaction {
            target: target.to_owned(),
            kind: "button".to_owned(),
            action: Action::Click,
            action_us: (1, 2),
            frame_us: (frame, frame),
            rounds: 4,
        };
        let report = Interactions {
            rounds: 4,
            build_profile: "debug",
            rows: vec![row("App/slow", 900), row("App/fast", 10)],
            skipped: Vec::new(),
        };
        assert_eq!(
            report.rows.first().map(|row| row.target.as_str()),
            Some("App/slow")
        );
        assert_eq!(
            report.target("App/fast").map(|row| row.frame_us.0),
            Some(10)
        );
        assert!(report.to_string().contains("App/slow"));
    }
}
