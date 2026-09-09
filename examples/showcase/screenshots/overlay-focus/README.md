# Overlay focus captures

Native tiny-skia evidence from public component fixtures, without a live window.
All captures use scale 1, en-US, Linux and reduced motion.

| Capture | Fixture and viewport | State |
| --- | --- | --- |
| [Long dialog](long-dialog.png) | `long_custom_dialog`, 320×300, light/default native font | Initial safe action remains visible below scrolling copy. |
| [Focused body](focused-body.png) | `dialog_body_focus`, 320×300, light/default native font | Tab reveals the custom body action at the end of the scroll area. |
| [Wrapped alert](wrapped-alert.png) | `custom_alert_focus`, 360×260, light/default native font | Safe and destructive custom actions wrap into separate rows. |
| [Corner search](search-corner.png) | `search_top_left`, 240×240, light/default native font | Last enabled variable-height result remains visible; search input retains focus. |
| [Selected ComboBox](combobox-selected.png) | `combobox_keyboard_selection_keeps_accessible_focus`, 360×260, Showcase test preset/app fonts | Selection persists after reopening and Escape; typing subsequently proves retained focus. |

Tests assert geometry, selection and focus before capture. The searchable
popover test also covers the other three corners. Full generated PNG/JSON pairs
are produced under `examples/showcase/target/ice-test-artifacts/<fixture>/`.
These captures do not establish platform accessibility or Tree-host parity.
