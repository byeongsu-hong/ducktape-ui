app LazyComponentState

use "themes/slate.ice"

// The issue-#592 shape: a keyed lazy whose value is the component's OWN
// state, with a second state read after it. The deferred `move` builder must
// not capture the call-site scope binding — the later read still needs it —
// and the value it reads when a key moves must be the live instance state,
// not the initializer fallback.
component Card()
  state
    count = 0
    label = "steady"
  on bump
    count = count + 1
  col #root gap=4.0
    lazy count by count as row
      text row #cached
    text label #label
    button "bump" #bump -> bump

view
  Card #card

test keyed_lazy_over_component_state_reads_the_live_state
  target cached = #card/root/cached
  target label = #card/root/label
  target bump = #card/root/bump
  expect text "0" within cached
  expect text "steady" within label
  // Bumping moves the key, so the lazy rebuilds — and the rebuilt row must
  // show the component instance's live count, not the initializer fallback.
  click bump
  expect text "1" within cached
  expect text "steady" within label
