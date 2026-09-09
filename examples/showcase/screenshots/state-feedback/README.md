# State feedback captures

All captures use native tiny-skia, light theme, scale 1, en-US, Linux and
reduced motion, with each app's bundled fonts. The standalone feedback app
loads Geist regular/bold; the other app roots declare their own fonts.

| Capture | Root/preset and viewport | Verified observation |
| --- | --- | --- |
| [Compact notices](compact-notices.png) | `tests/cases/ui/state_feedback.ice`, default,360×640 | Four empty descriptions reserve no row; Create stays reachable. |
| [Created project](created-project.png) | same root, Create action,360×640 | Creation opens a name field with keyboard focus; native typing persists Research. |
| [Centered copy](centered-empty-copy.png) | same root, `long_copy`,360×720 | Every painted title/description line is centered. |
| [Long notice](long-notice.png) | same root, mounted Alert,320×360 | Unbroken title/description wrap within the surface. |
| [Editing during save](editing-during-save.png) | Markdown editor, `pending_save`,760×520 | Native typing remains usable; Saving is present and Saved absent. |
| [Pending search](pending-search.png) | Music, `search_pending`,1180×760 | Draft cloud remains editable while submitted nova labels the request. |

The pending-save preset represents in-flight work; separate owner tests execute
real scratch-file writes, defer their actual completion, and verify failure and
retry. The native music test follows this frame by submitting the edited query
through Enter and asserting the resulting title, results and cleared loading.
The cover-image overflow found in that result capture is fixed at the shared
software renderer. The [content workspace evidence](../content-workspaces/README.md)
covers raster/SVG clipping, rounded covers and a bounded editor beneath media.
This directory is not evidence for platform or Tree-host behavior.

A 60-frame debug inspection of the same pending music root/preset at 1180×760
reported view p50/p95 472/782µs, layout 1003/1599µs, update 114/181µs,
revision memo 720 hits/0 misses, lazy memo 0/0. This is an idle pending-state
measurement, not a release/network throughput budget.
