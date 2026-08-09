// One component per kind of transcript row.
//
// The transcript is a flat, ordered list of rows rather than a list of
// messages, because that is what a turn actually is: reasoning, then a search,
// then more reasoning, then the answer. Drawing it flat keeps the order the
// model produced, and keeps each row a small, separately drawable thing.
//
// Every row is a settled row. Nothing here is rebuilt while a reply is being
// written — the caller puts each one behind `lazy`, keyed on the row itself.

// What was asked, leaning to its own side of the column.
component Prompt(body:str)
  row #root w=fill gap=10.0
    space w=fill h=1.0
    box
      with
        max-w=520.0
        px=15.0
        py=11.0
        bg=muted_bg
        border=border
        border-w=1.0
        r=14.0
      text body wrap=word @body

// The model's account of what it was doing. Folded by default: it is context
// for the answer, not the answer.
//
// Whether it is open is a property of the row rather than of this component,
// because a settled row is drawn behind `lazy` — which redraws it only when
// the row itself changes, and cannot see state held anywhere else.
component Reasoning(row_id:i64, title:str, body:str, open:bool) -> i64
  col #root w=fill gap=7.0
    // A summary is often nothing but its own bold heading. There is no fold
    // to offer then, and a toggle that opens an empty box is a lie about
    // there being more to read.
    if empty(body)
      row
        with
          w=fill
          gap=8.0
          align=center
        text "·" size=11.0 @text-muted
        text title @field_label
    if !empty(body)
      button #toggle -> emit(row_id)
        with
          label=title
          p=0.0
          @ghost_action
        row
          with
            w=fill
            gap=8.0
            align=center
          if open
            text "▾" size=11.0 @text-muted
          if !open
            text "▸" size=11.0 @text-muted
          text title @field_label
      if open
        box #body
          with
            w=fill
            px=15.0
            py=12.0
            bg=muted_bg
            r=10.0
          text body wrap=word @caption

// A tool call: what it was, what it was given, and whether it is still going.
// The mark on the left is the state — a turn in progress and a turn that has
// finished must not look the same.
component ToolCall(title:str, detail:str, status:str)
  box #root
    with
      w=fill
      px=15.0
      py=12.0
      bg=muted_bg
      border=border
      border-w=1.0
      r=10.0
    row w=fill gap=11.0
      if status == "running"
        text "◌" size=12.0 @text-warning
      if status == "done"
        text "✓" size=12.0 @text-success
      if status == "failed"
        text "✕" size=12.0 @text-danger
      col w=fill gap=4.0
        text title @field_label
        if !empty(detail)
          text detail wrap=word @machine

// The answer itself. `markdown_body` is a Rust adapter rather than the built-in
// widget because a parsed document cannot live in component state; it keeps one
// parse per row for the life of the window.
component Answer(row_id:i64, body:str, dark:bool) -> str
  col #root w=fill pt=6.0
    extern markdown_body(row_id, body, 13.5, dark) #body -> emit(_)

// What the turn cost, set quietly against the right edge.
component Usage(detail:str)
  row #root w=fill
    space w=fill h=1.0
    Typography.Machine content=detail
