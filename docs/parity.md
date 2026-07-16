# shadcn/ui parity

ducktape-ui targets the complete [official shadcn/ui component catalog](https://ui.shadcn.com/docs/components), adapted to iced's retained-mode architecture. Parity means the same user-facing job and state model, not a literal port of React or DOM APIs.

Status on 2026-07-16:

- **Shipped**: installable source exists, is compiled by this repository, and satisfies the current behavior contract.
- **Planned**: implementation is still required.
- **Foundation**: installable, compiled iced source is useful, but it does not yet satisfy the named shadcn behavior contract.

| shadcn/ui component | Status | iced implementation |
| --- | --- | --- |
| Accordion | Foundation | Controlled single/multiple disclosure and header-navigation reducer; activation can use `focus-control`, but the app still routes roving focus. |
| Alert | Shipped | Composable semantic container; visible text carries intent. |
| Alert Dialog | Planned | Controlled modal pattern with focus restoration and Escape handling. |
| Aspect Ratio | Shipped | Native responsive layout constrained to a caller-selected ratio. |
| Attachment | Shipped | Visible file name/metadata row with caller-owned controls. |
| Avatar | Shipped | Circular caller-owned content frame and text fallback without forcing image support. |
| Badge | Shipped | Six intents, two sizes, and optional redundant status marker. |
| Breadcrumb | Shipped | Caller-owned navigation items, current page, and separators. |
| Bubble | Shipped | Incoming/outgoing message bubble composition. |
| Button | Shipped | Six variants, four sizes, and disabled state. |
| Button Group | Shipped | Horizontal/vertical caller-owned control grouping and shared frame. |
| Calendar | Foundation | Validated date/month model and controlled six-week grid; day-grid roving focus and arrow handling remain app-scoped. |
| Card | Shipped | Composable card surface and header. |
| Carousel | Foundation | Controlled bounded/wrapping paging, orientations, active-slide clipping, and keyboard reducer; the app still scopes key events to carousel focus. |
| Chart | Planned | Canvas-based chart primitives and accessible textual companion data. |
| Checkbox | Shipped | Styled native checkbox; iced 0.14 currently limits it to pointer/touch. |
| Collapsible | Shipped | Controlled open/close reducer and caller-owned trigger/content composition. |
| Combobox | Shipped | Searchable selection built on iced's keyboard-capable native combo box. |
| Command | Planned | Searchable keyboard command model and result list. |
| Context Menu | Planned | Positioned overlay with focus and keyboard ownership. |
| Data Table | Shipped | Headless sorting, filtering-copy, and pagination state composed with table, input, checkbox, and pagination primitives. |
| Date Picker | Planned | Calendar and popover composition. |
| Dialog | Planned | Controlled modal with focus trap/restoration and dismissal rules. |
| Direction | Shipped | Explicit LTR/RTL alignment and reading-order helpers. |
| Drawer | Planned | Edge drawer with controlled state and focus handling. |
| Dropdown Menu | Planned | Menu overlay with roving focus and nested item state. |
| Empty | Shipped | Optional leading visual, title, and description. |
| Field | Shipped | Visible label with description or error copy around a native control. |
| Hover Card | Planned | Delayed hover/focus disclosure. |
| Input | Shipped | Default and invalid native text inputs. |
| Input Group | Shipped | Shared-border leading/input/trailing composition and borderless native input. |
| Input OTP | Shipped | Controlled grouped slots over one native focusable input with numeric/alphanumeric filtering, paste, disabled, and invalid states. |
| Item | Shipped | Reusable leading/content/trailing row composition. |
| Kbd | Shipped | Semantic keyboard-key visual. |
| Label | Shipped | Consistent visible control labels. |
| Marker | Shipped | Default, bordered, and separated labeled markers. |
| Menubar | Planned | Keyboard-operated top-level menu model. |
| Message | Shipped | Incoming/outgoing avatar, header, body, and actions composition. |
| Message Scroller | Shipped | Bottom-anchored transcript viewport; follow/unread state remains controlled. |
| Native Select | Foundation | Styled controlled native pick list; iced 0.14 limits opening and selection to pointer/touch. |
| Navigation Menu | Planned | Roving-focus navigation and disclosure panels. |
| Pagination | Shipped | Controlled previous/page/ellipsis/next composition. |
| Popover | Planned | Anchored overlay with dismissal and focus ownership. |
| Progress | Shipped | Four semantic visual progress variants; callers pair visible status text. |
| Radio Group | Shipped | Controlled exclusive selection with disabled-aware arrows/Home/End and stable focus helpers. |
| Resizable | Planned | Pointer and keyboard resizable panel group. |
| Scroll Area | Shipped | Styled native scrollable retaining sizing, direction, and callbacks. |
| Select | Planned | Keyboard-capable select overlay and controlled value. |
| Separator | Shipped | Horizontal and vertical semantic separators. |
| Sheet | Planned | Side sheet with modal/non-modal focus behavior. |
| Sidebar | Planned | Controlled collapsible navigation system and shortcut. |
| Skeleton | Shipped | Static, reduced-motion-safe loading placeholders with native sizing. |
| Slider | Shipped | Controlled single/range/multi-thumb values with pointer/touch drag, full keyboard steps, vertical, reversed/RTL, disabled, and invalid modes. |
| Sonner | Planned | Timed toast stack with pause, dismissal, and announcement model. |
| Spinner | Shipped | Controlled indeterminate frames with reduced-motion freeze. |
| Switch | Shipped | Controlled binary control with stable focus, pointer/touch, Enter/Space, two sizes, and disabled styling. |
| Table | Shipped | Native generic table plus header, cell, caption, and frame helpers. |
| Tabs | Shipped | Controlled panels with stable trigger focus, disabled-aware arrows/Home/End, automatic/manual activation, and horizontal/vertical default/line variants. |
| Textarea | Shipped | Styled, focusable native text editor with default/invalid states. |
| Toast | Planned | Controlled legacy toast stack. |
| Toggle | Shipped | Controlled two-state control with centered size geometry, default/outline styles, and complete activation behavior. |
| Toggle Group | Shipped | Single/multiple controlled toggles with configurable spacing/orientation and disabled-aware roving focus helpers. |
| Tooltip | Planned | Hover and focus trigger with nonessential content constraints. |
| Typography | Shipped | Theme-backed heading, prose, supporting, and inline-code roles. |

ducktape-ui also ships iced-specific `theme`, `surface`, `segmented-control`, and `focus-control` primitives. `focus-control` provides a stable focus ID, visible ring, and pointer, touch, Enter, and Space activation through iced's advanced widget API; the CLI enables the required `advanced` feature automatically. Compound-widget focus routing and semantic roles remain explicit limitations where iced does not provide them.
