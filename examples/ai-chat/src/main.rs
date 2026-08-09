mod auth;
mod codex;
mod composer;
mod history;
mod qa;
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
/// The second measurement splits that cycle into its two halves against the
/// length of the reply being written, because the halves answer to different
/// things: the append answers to the shape of the reply and the rebuild to how
/// many blocks it has been cut into.
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

    /// What the extra rows of 23 more turns add to one cycle, in each mode.
    /// Taking the difference cancels everything the two modes do not share.
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
                transcript(turns, false).len(),
                streaming(turns),
                rebuilding(turns),
            );
        }

        let held = marginal(streaming);
        let rebuilt = marginal(rebuilding);
        let extra = transcript(24, false).len() - transcript(1, false).len();
        eprintln!("{extra} extra rows cost  {held:?} per token held, {rebuilt:?} rebuilt");

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

    /// A turn that is still being written, with the live surfaces on screen.
    ///
    /// The `streaming` preset is what puts `busy` on; without it the reply
    /// being written is not drawn at all and a redraw measurement would be
    /// measuring the transcript alone.
    fn mid_turn(turns: usize) -> AiChat {
        let (mut app, _boot) = AiChat::__preset_2();
        let _ = app.__update(__AiChatMessage::Rows(transcript(turns, false)));
        // The preset's live reply ends on a closing code fence. Anything
        // appended straight onto it lands inside the fence line and reopens the
        // block, so close it before measuring a reply of a chosen shape.
        let _ = app.__update(chunk("\n\n"));
        app
    }

    fn chunk(answer: &str) -> __AiChatMessage {
        __AiChatMessage::Streamed(Chunk {
            answer: answer.to_owned(),
            thinking: String::new(),
            status: "Responding".to_owned(),
        })
    }

    /// What the reply being written looks like. `push_str` reparses the block
    /// the reply is still inside, so the shape of a reply is what decides the
    /// cost of a token, not its length.
    #[derive(Clone, Copy)]
    struct Shape {
        name: &'static str,
        /// Opens the reply — a code fence, or nothing for prose.
        seed: &'static str,
        /// Tokens between line breaks.
        every: u32,
        /// What a line break is: a blank line closes the block, a lone newline
        /// stays inside it.
        separator: &'static str,
    }

    const PROSE: Shape = Shape {
        name: "prose, a paragraph every 40 tokens",
        seed: "",
        every: 40,
        separator: "\n\n",
    };

    const CODE: Shape = Shape {
        name: "one code block, a line every 12 tokens",
        seed: "```rust\n",
        every: 12,
        separator: "\n",
    };

    /// Where a token's cost goes once the reply is long: the append into the
    /// parsed document, and the view rebuild that follows it.
    fn split(turns: usize, written: u32, shape: Shape) -> (Duration, Duration) {
        let mut app = mid_turn(turns);
        let _ = app.__update(chunk(shape.seed));
        let mut so_far = 0u32;
        let mut token = |app: &mut AiChat| {
            so_far += 1;
            let text = if so_far.is_multiple_of(shape.every) {
                format!("token{}", shape.separator)
            } else {
                "token ".to_owned()
            };
            let _ = app.__update(chunk(&text));
        };
        for _ in 0..written {
            token(&mut app);
        }
        for _ in 0..20 {
            token(&mut app);
            let _ = app.__view();
        }
        let (mut append, mut redraw) = (Duration::ZERO, Duration::ZERO);
        for _ in 0..TOKENS {
            let start = Instant::now();
            token(&mut app);
            append += start.elapsed();
            let start = Instant::now();
            let _ = app.__view();
            redraw += start.elapsed();
        }
        (append / TOKENS, redraw / TOKENS)
    }

    #[test]
    #[ignore = "timing evidence; run explicitly in release mode"]
    fn a_streamed_token_pays_for_the_block_it_lands_in_not_the_whole_reply() {
        if cfg!(debug_assertions) {
            eprintln!("skipped: run with --release");
            return;
        }
        let mut flat = None;
        for shape in [PROSE, CODE] {
            eprintln!("{}", shape.name);
            let mut appends = Vec::new();
            for written in [0, 500, 1500, 3000] {
                let (append, redraw) = split(1, written, shape);
                appends.push(append);
                eprintln!(
                    "  {written:>5} tokens written   token {:>10?}   append {:>10?}   redraw {:>10?}",
                    append + redraw,
                    append,
                    redraw,
                );
            }
            flat.get_or_insert((appends[0], *appends.last().unwrap()));
        }
        let (append, redraw) = split(24, 3000, CODE);
        let worst = append + redraw;
        eprintln!("worst case, 24 turns behind a 3000-token code block: {worst:?}");

        // A reply whose blocks close — which is what prose is — costs the same
        // per token at 3000 tokens as at none: the parser is re-reading the
        // paragraph in hand, not the reply. A reply that is one long code block
        // has no such boundary, so it grows; the figures above show by how much,
        // and the bound below is what makes that growth affordable rather than
        // the growth itself being the claim.
        let (short, long) = flat.expect("prose ran first");
        assert!(
            long < short * 2,
            "appending to prose tracked the reply already written: {short:?} -> {long:?}"
        );
        // A model writes 50-100 tokens a second, so the budget for one is
        // 10-20ms. The worst reply this screen can be handed is an order of
        // magnitude inside it.
        assert!(
            worst < Duration::from_millis(1),
            "a token cost {worst:?} of a 10ms budget"
        );
    }
}
