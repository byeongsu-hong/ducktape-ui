#[cfg(test)]
mod frame_probe;
mod market;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    Candles::run()
}

#[cfg(test)]
mod perf {
    use super::*;

    /// Timing evidence for the live-beat design: what one full Elm cycle
    /// (tick message -> app update -> whole-view rebuild) costs, i.e. the
    /// work the chart's `.live()` beat avoids on every exchange tick.
    /// Run: `cargo test -p candles-example --release perf::bench_tick_cycle -- --ignored --nocapture`
    #[test]
    #[ignore = "timing evidence; run explicitly in release mode"]
    fn bench_tick_cycle() {
        use std::time::{Duration, Instant};

        if cfg!(debug_assertions) {
            eprintln!("bench_tick_cycle: skipped; run with --release");
            return;
        }
        let (mut app, _boot) = Candles::__boot();
        let tick = market::Tick {
            revision: 1,
            last: 50_000.0,
            up: true,
            connected: true,
        };
        let cycle = || __CandlesMessage::Tick(tick);
        let _ = app.__update(cycle());
        let _ = app.__view();

        let start = Instant::now();
        let mut iterations = 0u32;
        while start.elapsed() < Duration::from_millis(500) {
            let _ = app.__update(cycle());
            let _ = app.__view();
            iterations += 1;
        }
        println!(
            "one Elm tick cycle (update + view rebuild): {:?}",
            start.elapsed() / iterations.max(1)
        );
    }
}
