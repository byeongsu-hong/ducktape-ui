# LogTimeline v1

`LogTimeline` is a retained product primitive for 100,000+ line build output,
agent traces, and service logs. Rows have one validated fixed height, unique
stable keys, and caller-owned payloads. It is not Ice Core syntax.

The runtime state composes `VirtualListState`; it does not implement another
row widget. Consequently selection, Up/Down/Home/End/Page keyboard navigation,
pointer handling, visible and mounted ranges, native scroll synchronization,
canonical headless selectors, and mounted-only AccessKit `List`/`ListItem`
semantics are the existing `VirtualList` contract. `LogTimelineState` adds only:

- tail-follow enabled by default;
- automatic pause when native scrolling, selection, keyboard navigation, or
  `scroll_to_key` leaves the live edge;
- a saturating unread count for rows appended while paused;
- explicit `ResumeTail`, which reaches the exact maximum offset and clears
  unread state;
- append-only history validation and an explicit `replace` boundary for log
  rotation, query changes, or clearing output.

```rust
let config = VirtualListConfig::new(24.0)?.overscan(3);
let mut state = LogTimelineState::new(VirtualListId::new("build-output"));
state.reconcile(&lines, |line| line.sequence, config)?;

let timeline = log_timeline(
    &state,
    &lines,
    config,
    "Build output",
    |line| line.sequence,
    |line| line.accessible_text.clone(),
    |_, line, selected| render_line(line, selected),
    Message::Log,
    &theme,
);
```

The reducer passes emitted list input back through `state.apply(event, config)`.
When `state.unread_count()` is nonzero, an application can show its own
notification or resume button; activating it applies
`LogTimelineEvent::ResumeTail`. Merely scrolling back to the bottom does not
silently resume, so a paused investigation is never changed by an append race.
`scroll_to_key(&key, config)` reveals a stable row without selecting it and
pauses following when it leaves the live edge.

`reconcile` accepts only an identical key prefix plus new suffix rows. Reorder,
removal, or replacement returns `HistoryChanged` without publishing partial
identity. `replace` makes an intentional discontinuity visible in code, resets
unread state, and resumes the new stream's tail. Duplicate keys are rejected in
both paths. State retains only keys and virtualization metadata; row payloads
and elements remain with the caller, and rendering invokes the row callback
only for visible plus overscan rows.

## LogTimeline versus MessageScroller

| Contract | LogTimeline | MessageScroller |
| --- | --- | --- |
| Primary data | Dense append-only logs | Conversational transcript |
| Row geometry | One fixed height | Measured variable height |
| Large collection | 100,000+ virtualized rows | Mounted transcript rows |
| Identity | Typed stable key | String message metadata/anchors |
| History changes | Append or explicit replace | Prepend/resize restoration |
| Tail behavior | Pause, unread count, explicit resume | Intent-aware following and jump control |
| Navigation | VirtualList selection and six list keys | Scroll commands and message alignment |

Do not use `LogTimeline` for chat bubbles, markdown messages, streaming blocks
whose height changes, new-turn peek anchors, or preserved prepend position; use
`MessageScroller`. Do not use `MessageScroller` to mount a 100,000-line fixed
log merely to obtain tail following; use `LogTimeline`.

The `ui-lang-components/log-timeline` feature enables `virtual-list` transitively and
therefore uses the renderer-only `ui-lang-runtime` boundary. It
selects no native platform backend, remains compatible with wasm, and inherits
the same bounded-height/no-vertical-scrolling-ancestor requirement documented
for `VirtualList`.
