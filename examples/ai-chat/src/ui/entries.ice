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
component Prompt(body:str) -> str
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
      col gap=4.0
        text body wrap=word @body
        row w=fill
          space w=fill h=1.0
          button "Copy" #copy @ghost_action -> emit(body)

// The model's account of what it was doing. Folded by default: it is context
// for the answer, not the answer.
//
// Whether it is open is a property of the row rather than of this component,
// because a settled row is drawn behind `lazy` — which redraws it only when
// the row itself changes, and cannot see state held anywhere else.
component Reasoning(row_id:i64, title:str, body:str, open:bool) -> i64
  row #root w=fill gap=11.0
    rule vertical thickness=2.0 color=border
    col w=fill gap=7.0
      // A summary is often nothing but its own bold heading. There is no fold
      // to offer then, and a toggle that opens an empty box is a lie about
      // there being more to read.
      if empty(body)
        row
          with
            w=fill
            gap=8.0
            align=center
          text "·" size=15.0 @text-muted
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
              text "▾" size=15.0 @text-muted
            if !open
              text "▸" size=15.0 @text-muted
            text title @field_label
        if open
          box w=fill pb=6.0
            text body wrap=word @caption

// A tool call: what it was, what it was given, and whether it is still going.
// The mark on the left is the state — a turn in progress and a turn that has
// finished must not look the same.
// Everything a turn did, under one line. It stays open while the turn runs —
// the only time watching it is worth anything — and closes once there is an
// answer to read instead.
component Work(row_id:i64, title:str, open:bool) -> i64
  row #root w=fill gap=11.0
    rule vertical thickness=2.0 color=border
    button #toggle -> emit(row_id)
      with
        label=title
        p=0.0
        @ghost_action
      row gap=8.0 align=center
        if open
          text "▾" size=15.0 @text-muted
        if !open
          text "▸" size=15.0 @text-muted
        text title @meta

// A tool call: what it was, and what it was given once asked. The mark on the
// left is the state — a call in progress and one that has finished must not
// look the same.
component ToolCall(row_id:i64, title:str, detail:str, status:str, open:bool) -> i64
  row #root w=fill gap=11.0
    rule vertical thickness=2.0 color=border
    col w=fill gap=3.0
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
          if status == "running"
            text "◌" size=12.5 @text-warning
          if status == "done"
            text "✓" size=12.5 @text-success
          if status == "failed"
            text "✕" size=12.5 @text-danger
          text title @field_label
          if !empty(detail) && open
            text "▾" size=14.0 @text-muted
          if !empty(detail) && !open
            text "▸" size=14.0 @text-muted
      if open && !empty(detail)
        box w=fill pb=4.0
          text detail wrap=word @machine

// The answer itself. `markdown_body` is a Rust adapter rather than the built-in
// widget because a parsed document cannot live in component state; it keeps one
// parse per row for the life of the window.
component Answer(row_id:i64, body:str, dark:bool) -> str
  col #root
    with
      w=fill
      pt=6.0
      gap=4.0
    extern markdown_body(row_id, body, 13.5, dark) #body -> emit(_)
    // iced draws non-editable text without selection, so an answer cannot be
    // dragged over and copied the way it could in a browser. Until the toolkit
    // grows that, this is how the text leaves the window.
    row w=fill
      space w=fill h=1.0
      button "Copy" #copy @ghost_action -> emit(body)

// What the turn cost, set quietly against the right edge.
component Usage(detail:str)
  row #root w=fill
    space w=fill h=1.0
    box
      with
        px=10.0
        py=5.0
        bg=accent
        r=7.0
      text detail
        with
          size=11.5
          font=code
          @text-muted

// A quiet dropdown for the header: it reads as a subtitle until it is used.
// The default pick styling puts a filled highlight behind the selection, which
// is far too loud for something sitting under the app's own name.
component Chip(options:[str], selected:str?) -> str
  pick options selected #root p=7.0 text-size=13.0 -> emit(_)
    active text=muted handle=muted bg=surface border=surface r=7.0
    hovered text=fg handle=fg bg=accent border=border r=7.0
    opened text=fg handle=fg bg=accent border=border r=7.0
    opened-hovered text=fg handle=fg bg=accent border=border r=7.0
    menu text=fg selected-text=fg selected-bg=accent bg=surface border=border border-w=1.0 r=10.0 shadow=shadow_popover shadow-y=6.0 shadow-blur=18.0
    handle dynamic
      closed code="▾" size=17.0
      open code="▴" size=17.0

// One chat that already happened, as a row to open it by.
component PastChat(chat:Chat, open:bool) -> str
  button #root -> emit(chat.path)
    with
      label=chat.title
      w=fill
      p=0.0
      @ghost_action
    box
      with
        w=fill
        px=10.0
        py=9.0
        r=8.0
      row w=fill gap=8.0
        // The chat being read is marked rather than filled in, so the list
        // stays one column of text with one thing standing out of it.
        if open
          rule vertical thickness=2.0 color=primary
        col w=fill gap=2.0
          text chat.title wrap=word @field_label
          row gap=7.0 align=center
            text chat.when @meta
            text chat.model @machine

// A row saying what was left out, so a truncated transcript says so rather
// than quietly beginning in the middle.
component Note(title:str)
  row #root w=fill gap=11.0
    rule vertical thickness=2.0 color=border
    text title @meta
