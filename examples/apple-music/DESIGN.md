# Apple Music example design references

This example follows the current macOS Music UI where the language/runtime can
express it without platform-only assets.

## Reference project structure

```text
src/
├── main.rs                 Rust entry point; includes ui/app.ice
├── mock_api.rs             typed domain and data boundary
├── liquid_glass.rs         native shader boundary
└── ui/
    ├── app.ice             one app root, imports, and one view
    ├── theme.ice           semantic color tokens
    ├── state.ice           app state and deterministic test preset
    ├── components/         common, icons, sidebar, library, and player
    ├── handlers/app.ice    state transitions and native effects
    ├── extern/mock_api.ice typed Rust declarations
    └── tests/              app and component behavior contracts
```

The split follows Ice's source-graph model: one `app` and one `view`, with
graph-wide declarations imported by concern. It keeps display state and event
flow in Ice, domain conversion and async data in Rust, and tests beside the Ice
surface they exercise.

| Decision | Reference | Reason |
| --- | --- | --- |
| Playback and utility controls use centered 14–16 px vector glyphs. | [Apple Music on Mac lyrics view](https://support.apple.com/guide/music/musf438ffc97/mac), [Apple HIG: Icons](https://developer.apple.com/design/human-interface-guidelines/icons) | The Music screenshot uses fixed symbols. HIG requires consistent optical size, weight, and alignment, which font glyphs such as `Ⅱ` and `▶\|` cannot guarantee. |
| The selected sidebar row uses only the accent fill and accent text, with no border or shadow. | [Apple Music on Mac lyrics view](https://support.apple.com/guide/music/musf438ffc97/mac), [Apple HIG: Sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars) | Music identifies the current row through tint and color; an outline reads as keyboard focus and created the reported double-border effect. |
| Text uses bundled Geist Regular/Bold at 10 pt captions, 13 pt body, 18 pt section headings, 32–36 pt display headings, and 22 pt lyrics. Bold is limited to headings, featured titles, current-track metadata, and the active lyric. | [Apple HIG: Typography](https://developer.apple.com/design/human-interface-guidelines/typography), [Apple HIG: Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility) | HIG gives macOS a 10 pt minimum and 13 pt default, recommends few typefaces, and uses size, weight, and color to express hierarchy. Geist is the repository's bundled cross-platform face; Apple says not to embed its system font. |
| Lyrics open from the speech-bubble control in a right-side panel, track the playhead, and seek when a line is clicked. The active line alone gets the strongest color and weight. | [Apple: View lyrics in Music on Mac](https://support.apple.com/guide/music/musf438ffc97/mac), [Apple: See lyrics on Mac or PC](https://support.apple.com/en-us/108960) | These are the documented Music interactions and the visual hierarchy shown in Apple's current lyrics screenshot. The panel reuses the example's existing right-side queue surface instead of introducing another panel style. |
