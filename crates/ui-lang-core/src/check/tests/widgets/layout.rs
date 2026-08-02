use super::*;

#[test]
fn checks_css_flexbox_container_and_items() {
    let source = r#"app Flexbox
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
view
  flex dir=column-reverse wrap=wrap-reverse w=fill h=fill gap=8.0 gap-y=10.0 gap-x=12.0 justify=space-between items=stretch content=space-around
    box order=-1 grow=2.0 shrink=0.5 basis=percent(40.0) self=baseline ml=auto mr=-4.0
      text "Flexible"
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("grow=2.0", "grow=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("grow"));

    let error = analyze(&source.replace("order=-1", "order=true")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("space-between", "sideways")).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("content alignment"));
}

#[test]
fn checks_complete_flex_layout_options() {
    let source = r#"app Layouts
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
view
  col w=fill h=shrink gap=8.0 p=1.0 px=2.0 py=3.0 pt=4.0 pr=5.0 pb=6.0 pl=7.0 max-w=640.0 align=center clip=true wrap wrap-gap=12.0 wrap-align=end
    row w=fill(2) h=48.0 gap=4.0 p=2.0 align=end clip=false wrap wrap-gap=6.0 wrap-align=start
      text "One"
      text "Two"
"#;
    analyze(source).unwrap();

    let bad_metric = source.replace("gap=8.0", "gap=-1.0");
    let error = analyze(&bad_metric).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("column metric"));

    let missing_wrap = source.replace("clip=true wrap wrap-gap", "clip=true wrap-gap");
    let error = analyze(&missing_wrap).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("require `wrap`"));

    let wrong_property = source.replace("row w=", "row max-w=100.0 w=");
    let error = analyze(&wrong_property).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("unknown layout property"));
}

#[test]
fn checks_complete_container_layout() {
    let source = r#"app Boxed
extern crate::backend
  box-style dynamic_container(highlight:bool)
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  highlight = false
view
  box #card style=dynamic_container(highlight) w=fill h=80.0 max-w=640.0 max-h=120.0 align-x=center align-y=end clip=true p=8.0 pl=12.0 bg=linear(1.57, bg@0.0, primary/25@1.0) text=fg border=primary border-w=2.0 r=4.0 r-tl=1.0 r-tr=2.0 r-br=3.0 r-bl=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=6.0 px-snap=true
    text "Card"
"#;
    analyze(source).unwrap();

    let bad_metric = source.replace("max-h=120.0", "max-h=-1.0");
    let error = analyze(&bad_metric).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("box metric"));

    let bad_clip = source.replace("clip=true", "clip=1");
    let error = analyze(&bad_clip).unwrap_err();
    assert_eq!(error.code, "E101");

    let bad_style = source.replace("shadow-blur=6.0", "shadow-blur=-1.0");
    let error = analyze(&bad_style).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("surface style metric"));

    let error = analyze(&source.replace("style=dynamic_container(highlight)", "style=rounded"))
        .unwrap_err();
    assert_eq!(error.code, "E184");
    assert!(error.message.contains("box style must be"));

    let error = analyze(&source.replace(
        "dynamic_container(highlight)",
        "missing_container(highlight)",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("box style"));

    let error = analyze(&source.replace("dynamic_container(highlight)", "dynamic_container(1.0)"))
        .unwrap_err();
    assert_eq!(error.code, "E101");

    let unknown = source.replace("clip=true", "opaque=true");
    let error = analyze(&unknown).unwrap_err();
    assert_eq!(error.code, "E184");
    assert!(error.message.contains("unknown box property"));
}

#[test]
fn checks_dashed_borders() {
    let source = r#"app Boxed
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
view
  box #draft p=8.0 border=fg border-w=1.0 border-dash=(4.0, 3.0) r=6.0
    text "Draft"
"#;
    analyze(source).unwrap();

    // The stroke draws the border color, so it needs one to draw.
    let error = analyze(&source.replace("border=fg ", "")).unwrap_err();
    assert_eq!(error.code, "E176");
    assert!(error.message.contains("dashed border needs `border=`"));

    let error =
        analyze(&source.replace("border-dash=(4.0, 3.0)", "border-dash=(4.0, -3.0)")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("border dash segment"));

    let error = analyze(&source.replace("border-dash=(4.0, 3.0)", "border-dash=()")).unwrap_err();
    assert_eq!(error.code, "E184");
    assert!(error.message.contains("at least one segment"));

    let error =
        analyze(&source.replace("border-dash=(4.0, 3.0)", "border-dash=(0.0, 0.0)")).unwrap_err();
    assert_eq!(error.code, "E176");
    assert!(error.message.contains("at least one positive segment"));
}

#[test]
fn checks_structured_overlays() {
    let source = r#"app Dialog
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  shown = true
on close
  shown = false
view
  overlay when=shown dismiss=close backdrop=black/60 p=24.0 align-x=center align-y=end
    content
      text "Page"
    layer
      box w=320.0 p=16.0 bg=bg r=10.0
        text "Dialog"
"#;
    analyze(source).unwrap();

    let wrong_condition = source.replace("when=shown", "when=1");
    let error = analyze(&wrong_condition).unwrap_err();
    assert_eq!(error.code, "E101");

    let bad_padding = source.replace("p=24.0", "p=-1.0");
    let error = analyze(&bad_padding).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("overlay padding"));

    let bad_color = source.replace("black/60", "missing/60");
    let error = analyze(&bad_color).unwrap_err();
    assert_eq!(error.code, "E185");
    assert!(error.message.contains("backdrop color"));

    let unnamed_section = source.replace("    content\n", "    page\n");
    let error = analyze(&unnamed_section).unwrap_err();
    assert_eq!(error.code, "E185");
    assert!(error.message.contains("`content` then `layer`"));
}

#[test]
fn checks_persistent_pane_grids() {
    let source = r#"app Workspace
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
on clicked(name)
view
  panes #work w=fill h=fill gap=8.0 min-size=120.0 resize=6.0 drag click=clicked(_)
    split vertical ratio=0.7
      pane files
        text "Files"
      pane editor
        text "Editor"
"#;
    analyze(source).unwrap();

    let bad_ratio = source.replace("ratio=0.7", "ratio=2.0");
    let error = analyze(&bad_ratio).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("ratio"));

    let bad_metric = source.replace("min-size=120.0", "min-size=-1.0");
    let error = analyze(&bad_metric).unwrap_err();
    assert_eq!(error.code, "E128");

    let bad_panes = source.replace("pane editor", "panel editor");
    let error = analyze(&bad_panes).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("pane configuration"));
}

#[test]
fn checks_nested_pane_configurations_and_closed_templates() {
    let source = r#"app Workspace
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
on open_preview
  pane #work split editor preview horizontal ratio=0.4
on resize_editor_stack
  pane #work resize editor_stack 0.55
view
  panes #work w=fill h=fill
    split workspace_root vertical ratio=0.7
      pane files
        text "Files"
      split editor_stack horizontal ratio=0.6
        pane editor
          text "Editor"
        pane terminal
          text "Terminal"
    pane preview closed
      text "Preview"
"#;
    let document = analyze(source).unwrap();
    let ViewNode::PaneGrid {
        configuration,
        panes,
        ..
    } = &document.source_document().view
    else {
        panic!("panes view")
    };
    assert_eq!(panes.len(), 4);
    assert!(matches!(configuration, PaneConfiguration::Split { .. }));

    let error = analyze(&source.replace("ratio=0.6", "ratio=1.1")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("ratio"));

    let error = analyze(&source.replace("pane terminal", "pane editor")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("duplicate pane `editor`"));

    let error = analyze(&source.replace("pane preview closed", "pane preview hidden")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("pane name closed"));

    let error = analyze(&source.replace("resize editor_stack", "resize missing")).unwrap_err();
    assert_eq!(error.code, "E188");
    assert!(error.message.contains("has no split `missing`"));

    let error = analyze(&source.replace("editor_stack horizontal", "workspace_root horizontal"))
        .unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(
        error
            .message
            .contains("duplicate pane split `workspace_root`")
    );
}

#[test]
fn checks_runtime_pane_templates_and_keys() {
    let source = r#"app Workspace
extern crate::backend
  Task(id:i64, title:str)
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  tasks:[Task] = []
  selected = 7
on open_task
  pane #work split files task(selected) horizontal
on close_task
  pane #work close task(selected)
view
  panes #work
    pane files maximized=files_maximized
      col
        if files_maximized
          text "Maximized files"
    pane task in tasks by=task.id maximized=task_maximized
      col
        if task_maximized
          text "Maximized task"
        text task.title
"#;
    let document = analyze(source).unwrap();
    let ViewNode::PaneGrid { templates, .. } = &document.source_document().view else {
        panic!("panes view")
    };
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].item, "task");
    assert_eq!(templates[0].items, "tasks");

    let error = analyze(&source.replace("task(selected)", "task(\"wrong\")")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replacen("task(selected)", "missing(selected)", 1)).unwrap_err();
    assert_eq!(error.code, "E188");
    assert!(error.message.contains("no dynamic pane template `missing`"));

    let error = analyze(&source.replace("by=task.id", "by=task")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("dynamic pane keys"));

    let error = analyze(&source.replace("maximized=task_maximized", "maximized=task")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("must differ from its template item"));

    let error = analyze(&source.replace("in tasks", "in selected")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("requires list state `selected`"));
}

#[test]
fn checks_structured_pane_titles_and_controls() {
    let source = r#"app Workspace
extern crate::backend
  panes-style dynamic_panes(active:bool)
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  filter = ""
  active = true
on close
view
  panes #work style=dynamic_panes(active)
    style
      hovered-region bg=linear(0.785, primary/25@0.0, bg@0.5, danger@1.0) border=fg border-w=2.0 r=4.0 r-tl=1.0 r-tr=2.0 r-br=3.0 r-bl=4.0
      hovered-split color=primary w=3.0
      picked-split color=danger w=4.0
    split vertical
      pane files bg=linear(1.57, bg@0.0, primary/25@1.0) text=fg border=primary border-w=2.0 r=4.0 r-tl=1.0 r-tr=2.0 r-br=3.0 r-bl=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=6.0 px-snap=true
        title p=4.0 px=8.0 pt=6.0 always-controls bg=primary/50 text=fg border=danger border-w=1.0 r=3.0 shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 px-snap=false
          text "Files"
        controls
          button "Close" -> close
        compact
          button "×" -> close
        input "Filter" #filter <-> filter
      pane editor
        title
          text "Editor"
        controls
          button "Close" -> close
        text "Editor body"
"#;
    analyze(source).unwrap();

    let error =
        analyze(&source.replace("style=dynamic_panes(active)", "style=missing_panes(active)"))
            .unwrap_err();
    assert_eq!(error.code, "E130");
    assert!(error.message.contains("unknown extern panes style"));

    let error = analyze(&source.replace("pt=6.0", "pt=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("pane title padding"));

    let error = analyze(&source.replace(
        "        controls\n          button \"Close\" -> close\n",
        "",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(
        error
            .message
            .contains("compact controls require a `controls`")
    );

    let error = analyze(&source.replace(
        "        input \"Filter\" #filter <-> filter",
        "        content\n          input \"Filter\" #filter <-> filter",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E064");
    assert!(error.message.contains("unknown view node `content`"));

    let error = analyze(&source.replace(
        "px-snap=true\n        title",
        "px-snap=true @p-4\n        title",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E042");
    assert!(error.message.contains("has no effect on `pane`"));

    let error = analyze(&source.replace("primary/25@0.0", "missing@0.0")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("unknown panes background color"));

    let error = analyze(&source.replace("danger@1.0", "danger@1.1")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("gradient stop"));

    let error = analyze(&source.replace("danger@1.0", "danger")).unwrap_err();
    assert_eq!(error.code, "E189");
    assert!(error.message.contains("color@offset"));

    let error = analyze(&source.replace(
            "linear(0.785, primary/25@0.0, bg@0.5, danger@1.0)",
            "linear(0.785, primary@0.0, primary@0.1, primary@0.2, primary@0.3, primary@0.4, primary@0.5, primary@0.6, primary@0.7, primary@1.0)",
        ))
        .unwrap_err();
    assert_eq!(error.code, "E189");
    assert!(error.message.contains("at most 8 color stops"));

    let error = analyze(&source.replace("shadow-blur=6.0", "shadow-blur=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("surface style metric"));

    let error = analyze(&source.replace("px-snap=true", "px-snap=1.0")).unwrap_err();
    assert_eq!(error.code, "E101");

    let error = analyze(&source.replace("w=3.0", "w=-1.0")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("panes style metric"));

    let error = analyze(&source.replace("hovered-split color", "active-split color")).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(
        error
            .message
            .contains("hovered-region, hovered-split, or picked-split")
    );
}

#[test]
fn checks_pane_state_operations_and_queries() {
    let source = r#"app Workspace
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
on arrange
  pane #work maximize editor
  pane #work restore
  pane #work swap files editor
  pane #work move editor left
  pane #work resize 0.6
  pane #work drop editor files center
  pane #work split editor preview horizontal ratio=0.4
  pane #work close editor
on inspect
  pane #work maximized -> observed _
on inspect_neighbor
  pane #work adjacent files right -> observed _
on observed(name)
view
  panes #work
    split vertical
      pane files
        text "Files"
      pane editor
        text "Editor"
    pane preview closed
      text "Preview"
"#;
    let document = analyze(source).unwrap();
    assert_eq!(
        document.source_document().handlers[3].params[0]
            .ty
            .display(),
        "str?"
    );

    let error = analyze(&source.replace("#work maximize", "#missing maximize")).unwrap_err();
    assert_eq!(error.code, "E188");
    assert!(error.message.contains("unknown panes"));

    let error = analyze(&source.replace("maximize editor", "maximize missing")).unwrap_err();
    assert_eq!(error.code, "E188");
    assert!(error.message.contains("has no pane `missing`"));

    let error = analyze(&source.replace("swap files editor", "swap files files")).unwrap_err();
    assert_eq!(error.code, "E188");
    assert!(error.message.contains("different panes"));

    let error = analyze(&source.replace("resize 0.6", "resize 1.1")).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("pane split ratio"));

    let error =
        analyze(&source.replace("pane #work maximized -> observed _", "pane #work maximized"))
            .unwrap_err();
    assert_eq!(error.code, "E188");
    assert!(error.message.contains("query requires a route"));

    let duplicate = r#"app Workspace
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
component Frame()
  row
    slot left
    slot right
view
  Frame
    left:
      panes #work
        split vertical
          pane a
            text "A"
          pane b
            text "B"
    right:
      panes #work
        split horizontal
          pane c
            text "C"
          pane d
            text "D"
"#;
    let error = analyze(duplicate).unwrap_err();
    assert_eq!(error.code, "E187");
    assert!(error.message.contains("duplicate persistent panes"));
}

#[test]
fn checks_complete_grid_sizing() {
    let source = r#"app Layouts
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
view
  col
    grid cols=2 w=640.0 gap=12.0 h=aspect(16.0,9.0)
      text "Fixed"
    grid max-cell=240.0 h=fill(2)
      text "Maximum"
    grid min-cell=240.0
      text "Minimum"
"#;
    analyze(source).unwrap();

    let conflicting = source.replace("cols=2", "cols=2 min-cell=240.0");
    let error = analyze(&conflicting).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("mutually exclusive"));

    let zero_cell = source.replace("min-cell=240.0", "min-cell=0.0");
    let error = analyze(&zero_cell).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("grid minimum cell width"));

    let fixed_height = source.replace("grid min-cell=240.0", "grid min-cell=240.0 h=shrink");
    let error = analyze(&fixed_height).unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(error.message.contains("natural row height"));

    let zero_aspect = source.replace("aspect(16.0,9.0)", "aspect(16.0,0.0)");
    let error = analyze(&zero_aspect).unwrap_err();
    assert_eq!(error.code, "E128");
    assert!(error.message.contains("grid aspect height"));
}
