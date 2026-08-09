mod codex;
mod render;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    AiChat::run()
}

/// What one streamed token costs against a transcript already on screen.
///
/// This is the claim the screen is built around. A settled row sits behind a
/// `lazy` boundary keyed on the row itself, and its Markdown is parsed once and
/// kept — so a token landing in the reply still being written should not make
/// the rest of the transcript pay for it.
///
/// The measurement is a full Elm cycle: `update` with one token, then a whole
/// view rebuild. It is compared against the same cycle with every row changed,
/// which is what the transcript would cost per token if nothing were held.
///
/// Run: `cargo test -p ai-chat-example --release perf -- --ignored --nocapture`
#[cfg(test)]
mod perf {
    use super::*;
    use crate::codex::Chunk;
    use std::time::{Duration, Instant};

    /// A settled transcript of `turns` finished exchanges.
    fn transcript(turns: usize, dark: bool) -> Vec<codex::Entry> {
        let one = codex::sample_entries(false);
        let mut rows = Vec::with_capacity(turns * one.len());
        for turn in 0..turns {
            for (index, row) in one.iter().enumerate() {
                rows.push(codex::Entry {
                    // Ids must stay distinct: they key both the `keyed` list
                    // and the parsed-Markdown cache.
                    id: -((turn * one.len() + index + 1) as i64),
                    dark,
                    ..row.clone()
                });
            }
        }
        rows
    }

    /// A fixed run rather than a time budget: the live reply grows with every
    /// token, so a longer run would measure a longer document instead of a
    /// longer transcript.
    const TOKENS: u32 = 200;

    fn measure(mut cycle: impl FnMut()) -> Duration {
        // Warm the parse cache and the first build, so what follows is the
        // steady state rather than arriving at it.
        for _ in 0..20 {
            cycle();
        }
        let start = Instant::now();
        for _ in 0..TOKENS {
            cycle();
        }
        start.elapsed() / TOKENS
    }

    fn booted(turns: usize) -> AiChat {
        let (mut app, _boot) = AiChat::__boot();
        let _ = app.__update(__AiChatMessage::Rows(transcript(turns, false)));
        app
    }

    /// One token arriving: only the live reply changes.
    fn streaming(turns: usize) -> Duration {
        let mut app = booted(turns);
        measure(move || {
            let _ = app.__update(__AiChatMessage::Streamed(Chunk {
                answer: "token ".to_owned(),
                thinking: String::new(),
                status: "Responding".to_owned(),
            }));
            let _ = app.__view();
        })
    }

    /// Every settled row changing: the same cycle with nothing reusable.
    fn rebuilding(turns: usize) -> Duration {
        let mut app = booted(turns);
        let mut dark = false;
        measure(move || {
            dark = !dark;
            let _ = app.__update(__AiChatMessage::Rows(transcript(turns, dark)));
            let _ = app.__view();
        })
    }

    /// What 138 extra rows add to one cycle, in each mode. Taking the
    /// difference cancels everything the two modes do not share.
    fn marginal(mode: fn(usize) -> Duration) -> Duration {
        mode(24).saturating_sub(mode(1))
    }

    #[test]
    #[ignore = "timing evidence; run explicitly in release mode"]
    fn a_streamed_token_does_not_pay_for_the_settled_transcript() {
        if cfg!(debug_assertions) {
            eprintln!("skipped: run with --release");
            return;
        }
        for turns in [1, 8, 24] {
            eprintln!(
                "{turns:>3} turns ({:>3} rows)  token {:>10?}   all rows changed {:>10?}",
                turns * 6,
                streaming(turns),
                rebuilding(turns),
            );
        }

        let held = marginal(streaming);
        let rebuilt = marginal(rebuilding);
        eprintln!("138 extra rows cost  {held:?} per token held, {rebuilt:?} rebuilt");

        // The assertion is on the property that holds with room to spare: the
        // per-token cost grows far slower than the transcript. 24x the rows for
        // well under 5x the cost. The held-vs-rebuilt figures above are printed
        // rather than asserted because the two are close enough that timing
        // noise decides the comparison; the margin that matters is this one.
        let (short, long) = (streaming(1), streaming(24));
        assert!(
            long < short * 5,
            "per-token cost tracked the transcript: {short:?} -> {long:?}"
        );
    }
}
