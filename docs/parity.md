# shadcn/ui parity

ducktape-ui targets the complete [official shadcn/ui component catalog](https://ui.shadcn.com/docs/components), adapted to iced's retained-mode architecture. Parity means the same user-facing job and state model, not a literal port of React or DOM APIs.

Status on 2026-07-16:

- **Shipped**: installable source exists in the registry and is compiled by this repository.
- **Planned**: implementation is still required.
- **Foundation**: a useful iced primitive exists, but it does not yet satisfy the named shadcn behavior contract.

| shadcn/ui component | Status | iced implementation |
| --- | --- | --- |
| Accordion | Planned | Controlled disclosure group with keyboard navigation. |
| Alert | Shipped | Composable semantic container; visible text carries intent. |
| Alert Dialog | Planned | Controlled modal pattern with focus restoration and Escape handling. |
| Aspect Ratio | Shipped | Native responsive layout constrained to a caller-selected ratio. |
| Attachment | Planned | File attachment composition and state model. |
| Avatar | Shipped | Circular caller-owned content frame and text fallback without forcing image support. |
| Badge | Shipped | Six intents, two sizes, and optional redundant status marker. |
| Breadcrumb | Shipped | Caller-owned navigation items, current page, and separators. |
| Bubble | Planned | Message bubble composition. |
| Button | Shipped | Six variants, four sizes, and disabled state. |
| Button Group | Planned | Joined button layout with edge-aware styling. |
| Calendar | Planned | Keyboard-navigable month grid and selection model. |
| Card | Shipped | Composable card surface and header. |
| Carousel | Planned | Controlled paging, keyboard commands, and viewport clipping. |
| Chart | Planned | Canvas-based chart primitives and accessible textual companion data. |
| Checkbox | Shipped | Styled native checkbox; iced 0.14 currently limits it to pointer/touch. |
| Collapsible | Planned | Controlled disclosure primitive. |
| Combobox | Planned | Filtered selection built on iced's keyboard-capable combo box. |
| Command | Planned | Searchable keyboard command model and result list. |
| Context Menu | Planned | Positioned overlay with focus and keyboard ownership. |
| Data Table | Planned | A table composition guide plus sorting/filtering/pagination state helpers. |
| Date Picker | Planned | Calendar and popover composition. |
| Dialog | Planned | Controlled modal with focus trap/restoration and dismissal rules. |
| Direction | Planned | LTR/RTL layout and text-direction configuration. |
| Drawer | Planned | Edge drawer with controlled state and focus handling. |
| Dropdown Menu | Planned | Menu overlay with roving focus and nested item state. |
| Empty | Shipped | Optional leading visual, title, and description. |
| Field | Shipped | Visible label with description or error copy around a native control. |
| Hover Card | Planned | Delayed hover/focus disclosure. |
| Input | Shipped | Default and invalid native text inputs. |
| Input Group | Planned | Joined leading/trailing controls and input layout. |
| Input OTP | Planned | Multi-cell input state and paste/keyboard behavior. |
| Item | Shipped | Reusable leading/content/trailing row composition. |
| Kbd | Shipped | Semantic keyboard-key visual. |
| Label | Shipped | Consistent visible control labels. |
| Marker | Planned | Map/content marker visual and state variants. |
| Menubar | Planned | Keyboard-operated top-level menu model. |
| Message | Planned | Structured chat message composition. |
| Message Scroller | Planned | Follow-bottom and unread-message scrolling state. |
| Native Select | Planned | Styled native pick list with documented iced interaction limits. |
| Navigation Menu | Planned | Roving-focus navigation and disclosure panels. |
| Pagination | Shipped | Controlled previous/page/ellipsis/next composition. |
| Popover | Planned | Anchored overlay with dismissal and focus ownership. |
| Progress | Shipped | Four semantic visual progress variants; callers pair visible status text. |
| Radio Group | Planned | Single-selection group with roving focus and arrow keys. |
| Resizable | Planned | Pointer and keyboard resizable panel group. |
| Scroll Area | Planned | Styled native scrollable and scrollbar policy. |
| Select | Planned | Keyboard-capable select overlay and controlled value. |
| Separator | Shipped | Horizontal and vertical semantic separators. |
| Sheet | Planned | Side sheet with modal/non-modal focus behavior. |
| Sidebar | Planned | Controlled collapsible navigation system and shortcut. |
| Skeleton | Shipped | Static, reduced-motion-safe loading placeholders with native sizing. |
| Slider | Planned | Focusable range control with full keyboard operation. |
| Sonner | Planned | Timed toast stack with pause, dismissal, and announcement model. |
| Spinner | Planned | Reduced-motion-aware indeterminate progress visual. |
| Switch | Planned | Focusable binary control with keyboard operation. |
| Table | Planned | Composable headers, rows, cells, caption, and footer. |
| Tabs | Foundation | `segmented-control` is shipped; real Tabs still needs focus and arrow-key behavior. |
| Textarea | Shipped | Styled, focusable native text editor with default/invalid states. |
| Toast | Planned | Controlled legacy toast stack. |
| Toggle | Planned | Pressed-state button with keyboard operation. |
| Toggle Group | Planned | Single/multiple controlled toggles with roving focus. |
| Tooltip | Planned | Hover and focus trigger with nonessential content constraints. |
| Typography | Shipped | Theme-backed heading, prose, supporting, and inline-code roles. |

ducktape-ui also ships iced-specific `theme`, `surface`, and `segmented-control` primitives. Interactive components that iced 0.14 cannot operate correctly from the keyboard will use source-owned custom widgets instead of receiving a misleading visual-only port.
