# shadcn/ui parity

ducktape-ui targets the complete [official shadcn/ui component catalog](https://ui.shadcn.com/docs/components), adapted to iced's retained-mode architecture. Parity means the same user-facing job and state model, not a literal port of React or DOM APIs.

Status on 2026-07-17:

**64 / 64 official components are Shipped.**

- **Shipped**: a feature-gated public module exists, is compiled by this repository, and satisfies the current behavior contract.
- **Planned**: implementation is still required.
- **Foundation**: a compiled iced module is useful, but it does not yet satisfy the named shadcn behavior contract.

| shadcn/ui component | Status | iced implementation |
| --- | --- | --- |
| Accordion | Shipped | Controlled single/multiple disclosure with stable header IDs, aligned copy/indicator/content, pointer/touch/Enter/Space activation, disabled-aware ArrowUp/Down/Home/End navigation, and direct focus tasks. |
| Alert | Shipped | Composable semantic container; visible text carries intent. |
| Alert Dialog | Shipped | Controlled high-friction modal with safe initial focus, wrapping Tab order, explicit cancel/action outcomes, Escape cancellation, and no backdrop dismissal. |
| Aspect Ratio | Shipped | Native responsive layout constrained to a caller-selected ratio. |
| Attachment | Shipped | Visible file name/metadata row with caller-owned controls. |
| Avatar | Shipped | Circular caller-owned content frame and text fallback without forcing image support. |
| Badge | Shipped | Six intents, two sizes, and optional redundant status marker. |
| Breadcrumb | Shipped | Caller-owned navigation items, current page, and separators. |
| Bubble | Shipped | Incoming/outgoing message bubble composition. |
| Button | Shipped | Six variants, four sizes, disabled state, visible focus, and pointer/touch/Enter/Space activation. |
| Button Group | Shipped | Horizontal/vertical caller-owned control grouping and shared frame. |
| Calendar | Shipped | Validated Gregorian model with controlled single/range/multiple selection, one roving day-grid focus stop, stable day IDs, pointer/touch/Enter/Space selection, disabled-aware arrows/Home/End/Page navigation and focus tasks, min/max/custom constraints, outside days, today, week numbers, month/year controls, and RTL. |
| Card | Shipped | Composable card surface and header. |
| Carousel | Shipped | Controlled bounded/wrapping horizontal or vertical viewport with stable focus, scoped Arrow/Home/End navigation, axis-locked pointer/touch swipe, configurable threshold, RTL-correct direction/control order, disabled boundary controls, and focusable numbered indicators. |
| Chart | Shipped | Keyed Canvas line/area/grouped-or-stacked bar/pie/donut charts with safe domains, axes/grids, controlled hit testing, semantic light/dark series colors, aligned legends, shadcn-style tooltip indicators, and explicit visible companion-table data. |
| Checkbox | Shipped | Styled controlled checkbox with visible focus and pointer/touch/Enter/Space activation. |
| Collapsible | Shipped | Controlled open/close reducer and caller-owned trigger/content composition. |
| Combobox | Shipped | Searchable selection built on iced's keyboard-capable native combo box. |
| Command | Shipped | Controlled grouped command palette with native editing/paste/IME, label/keyword filtering, disabled-aware navigation, active-row reveal, one roving result focus stop, Enter/pointer selection, shortcuts, empty state, and stable focus tasks. |
| Context Menu | Shipped | Exact right-click/touch point anchoring and re-anchoring with collision handling, dismissal/focus restoration, and the complete shared grouped/check/radio/submenu/shortcut keyboard menu contract. |
| Data Table | Shipped | Headless sorting, filtering-copy, and pagination state composed with table, input, checkbox, and pagination primitives. |
| Date Picker | Shipped | Controlled single/range trigger and collision-aware popover composed with Calendar, including placeholder/custom formats, constraints, completed-range close behavior, initial day focus, dismissal restoration, invalid/disabled states, and RTL. |
| Dialog | Shipped | Root-level controlled modal with inert underlay, wrapping Tab order, initial/restore focus tasks, Escape/backdrop dismissal, and explicit LTR/RTL copy/action alignment. |
| Direction | Shipped | Explicit LTR/RTL alignment and reading-order helpers. |
| Drawer | Shipped | Controlled draggable drawer on all four edges with mouse/touch distance and velocity thresholds, focus/dismissal rules, viewport caps, handle, reduced-motion snap behavior, and caller-owned state. |
| Dropdown Menu | Shipped | Controlled anchored trigger/content with focus restoration, collision placement, LTR/RTL, and shared groups, labels, separators, inset/disabled/check/radio/shortcut/submenu items plus complete keyboard navigation. |
| Empty | Shipped | Optional leading visual, title, and description. |
| Field | Shipped | Visible label with description or error copy around a native control. |
| Hover Card | Shipped | Delayed pointer/trigger/descendant-focus disclosure, collision placement, bounded metrics, gap-safe pointer transfer, and Escape dismissal. |
| Input | Shipped | Default and invalid native text inputs. |
| Input Group | Shipped | Shared-border leading/input/trailing composition and borderless native input. |
| Input OTP | Shipped | Controlled grouped slots over one native focusable input with numeric/alphanumeric filtering, paste, disabled, and invalid states. |
| Item | Shipped | Reusable leading/content/trailing row composition. |
| Kbd | Shipped | Semantic keyboard-key visual. |
| Label | Shipped | Consistent visible control labels. |
| Marker | Shipped | Default, bordered, and separated labeled markers. |
| Menubar | Shipped | Controlled top-level menu bar with one roving trigger focus stop, 36px trigger metrics, disabled-aware Left/Right switching, child Arrow/Home/End/Enter/Escape navigation, rich nested menu content, collision placement, and RTL. |
| Message | Shipped | Incoming/outgoing avatar, header, body, and actions composition. |
| Message Scroller | Shipped | Controlled stable-row transcript with initial start/end/last-anchor positioning, intent-aware live following, new-turn peek anchoring, prepend/resize restoration, start/end/message alignment commands, derived visibility/current-anchor/edge/unread state, keyboard scrolling, and a jump-to-latest control. |
| Native Select | Shipped | Styled controlled iced PickList preserving its native pointer/touch menu while adding a stable focus ID, ArrowUp/Down/Home/End selection, Enter/Space opening, Escape/Tab closing, disabled/invalid states, exact 36px and 12/20 text metrics, and explicit LTR/RTL label/chevron alignment. |
| Navigation Menu | Shipped | Controlled links and disclosure content with one roving trigger focus stop, stable active/open state, pointer/touch/Enter/Space, disabled-aware arrows/Home/End/Escape, content focus handoff, hover intent, collision-aware panels, responsive vertical mode, exact indicators, and RTL. |
| Pagination | Shipped | Controlled previous/page/ellipsis/next composition. |
| Popover | Shipped | Controlled anchored overlay with stable trigger/content IDs, focus handoff/restore tasks, outside/Escape dismissal, four sides, alignment/offsets, collision flip/clamp, viewport padding, and unclipped shadow geometry. |
| Progress | Shipped | Four semantic visual progress variants; callers pair visible status text. |
| Radio Group | Shipped | Controlled exclusive selection with one roving focus stop, disabled-aware arrows/Home/End, and stable focus helpers. |
| Resizable | Shipped | Controlled arbitrary panel groups with constrained shares, pointer/touch drag, full keyboard resizing, horizontal/vertical layouts, and optional grips. |
| Scroll Area | Shipped | Styled native scrollable retaining sizing, direction, and callbacks. |
| Select | Shipped | Controlled 36px grouped selection trigger with placeholder, labels, disabled options, active checkmarks, invalid state, collision overlay, full shared keyboard navigation, focus restoration, and RTL. |
| Separator | Shipped | Horizontal and vertical semantic separators. |
| Sheet | Shipped | Controlled modal/non-modal panel on all four edges with inert-underlay option, stable initial/restore focus, configurable dismissal, viewport-capped geometry, header/body/footer/close slots, and explicit LTR/RTL text/action alignment. |
| Sidebar | Shipped | Controlled expanded/collapsed plus mobile state, Ctrl/Cmd+B reducer, left/right icon/inset/floating variants, off-canvas/icon/none collapse modes, rail, responsive backdrop layout, sticky header/footer with scrolling content, groups, actions, badges, skeletons, submenus, collapsed tooltips, and explicit LTR/RTL metrics. |
| Skeleton | Shipped | Static, reduced-motion-safe loading placeholders with native sizing. |
| Slider | Shipped | Controlled single/range/multi-thumb values with pointer/touch drag, full keyboard steps, vertical, reversed/RTL, disabled, and invalid modes. |
| Sonner | Shipped | Controlled timed queue with six placements, visible/queued limits, hover/focus pause, actions, dismissal, reduced motion, and horizontal mouse swipe; iced exposes no live-region API. |
| Spinner | Shipped | Controlled indeterminate frames with reduced-motion freeze. |
| Switch | Shipped | Controlled binary control with stable focus, pointer/touch, Enter/Space, two sizes, and disabled styling. |
| Table | Shipped | Native generic table plus header, cell, caption, and frame helpers. |
| Tabs | Shipped | Controlled panels with one roving trigger focus stop, disabled-aware arrows/Home/End, automatic/manual activation, and horizontal/vertical default/line variants. |
| Textarea | Shipped | Styled, focusable native text editor with default/invalid states. |
| Toast | Shipped | Composable semantic legacy toast surface with aligned title/description, action, dismissal, six variants, and caller-owned lifetime. |
| Toggle | Shipped | Controlled two-state control with centered size geometry, default/outline styles, and complete activation behavior. |
| Toggle Group | Shipped | Single/multiple controlled toggles with configurable spacing/orientation and disabled-aware roving focus helpers. |
| Tooltip | Shipped | Noninteractive passive-content tooltip with keyboard focus plus hover, Escape dismissal, exact open/close delays, fixed text metrics/max width, collision placement, and unclipped shadow bounds. |
| Typography | Shipped | Theme-backed heading, prose, supporting, and inline-code roles. |

ducktape-ui also ships iced-specific `theme`, `surface`, `segmented-control`, `focus-control`, `modal`, and `menu` primitives. `focus-control` provides a stable focus ID and visible ring for either pointer/touch/Enter/Space activation or passive keyboard regions through iced's advanced widget API; the CLI enables the required `advanced` feature automatically. Compound-widget focus routing and semantic roles remain explicit limitations where iced does not provide them.
