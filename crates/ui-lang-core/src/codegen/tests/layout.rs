use super::*;

#[test]
fn expands_semantic_recipes_before_explicit_overrides() {
    let source = r#"app Recipes
recipe page for col
  w-full h-full gap-6 p-6 bg-bg
recipe panel for box
  w-full p-5 bg-surface border border-border rounded-lg overflow-hidden
recipe title for text
  text-22px leading-tight font-bold text-fg
theme
  bg #101010
  fg #f0f0f0
  primary #336699
  danger #cc0000
  surface #202020
  border #404040
view
  col @page
    box p=24.0 @panel bg-danger
      text "Recipe" @title
"#;
    let generated = compile(source, "recipes.ice").unwrap();

    assert!(generated.contains(".spacing(24)"));
    assert!(generated.contains(".padding(::iced::Padding { top: 24.0"));
    assert!(
        generated.contains(".padding(::ui_lang_runtime::bounded_padding(24.0, 24.0, 24.0, 24.0))")
    );
    assert!(generated.contains(".size(22)"));
    assert!(generated.contains(".clip(true)"));
    assert!(generated.contains("LineHeight::Relative(1.2)"));
    assert!(generated.contains("weight: ::iced::font::Weight::Bold"));
    assert!(generated.contains("::iced::Color::from_rgba8(204, 0, 0, 1.000000)"));

    let error = compile(
        &source.replace("box p=24.0 @panel", "box p=24.0 @page"),
        "recipes.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E046");
    assert!(error.message.contains("targets `col`, not `box`"));

    let error = compile(
        &source.replace(
            "recipe panel for box\n  w-full p-5 bg-surface border border-border rounded-lg overflow-hidden",
            "recipe panel for box\n  w-full p-5 bg-surface border border-border rounded-lg overflow-hidden\nrecipe panel for box\n  p-4 bg-surface",
        ),
        "recipes.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E100");
    assert!(error.message.contains("duplicate recipe `panel`"));

    let error = compile(
        &source.replace(
            "recipe panel for box\n  w-full p-5 bg-surface border border-border rounded-lg overflow-hidden",
            "recipe panel for box\n  grid-cols-3",
        ),
        "recipes.ice",
    )
    .unwrap_err();
    assert_eq!(error.code, "E041");
    assert!(error.message.contains("unsupported utility `grid-cols-3`"));
}

#[test]
fn lowers_exact_pixel_recipe_utilities() {
    let source = r#"app ExactRecipes
recipe action for button
  px-16px py-11px rounded-9px bg-primary text-primary_fg
recipe caption for text
  text-12.5px text-fg
theme
  bg #101010
  fg #f0f0f0
  primary #336699
  primary_fg #ffffff
  danger #cc0000
on pressed
view
  col
    button "Save" @action -> pressed
    text "Caption" @caption
"#;
    let generated = compile(source, "exact-recipes.ice").unwrap();

    assert!(generated.contains("top: 11.0, right: 16.0, bottom: 11.0, left: 16.0"));
    assert!(generated.contains("__style.border.radius = 9.0.into()"));
    assert!(generated.contains(".size(12.5)"));
}

#[test]
fn lowers_monospace_and_exact_font_weight_recipe_utilities() {
    let source = r#"app FontRecipes
recipe meta for text
  text-11px font-mono font-medium text-fg
recipe label for text
  text-10px font-mono font-semibold text-fg
recipe strong for text
  font-bold text-fg
theme
  bg #101010
  fg #f0f0f0
  primary #336699
  danger #cc0000
view
  col
    text "Meta" @meta
    text "Label" @label
    text "Strong" @strong
"#;
    let generated = compile(source, "font-recipes.ice").unwrap();

    assert!(generated.contains("weight: ::iced::font::Weight::Medium, ..::iced::Font::MONOSPACE"));
    assert!(
        generated.contains("weight: ::iced::font::Weight::Semibold, ..::iced::Font::MONOSPACE")
    );
    assert!(generated.contains("weight: ::iced::font::Weight::Bold, ..::iced::Font::DEFAULT"));
}

#[test]
fn lowers_box_and_flex_sugar_to_native_layouts() {
    let source = r#"app Layouts
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
view
  box w=fill p=8.0 bg=bg
    flex dir=column gap=6.0
      text "Header"
      flex gap=4.0
        box w=fill(1)
          text "Left"
        box w=fill(2)
          text "Right"
"#;
    let generated = compile(source, "layouts.ice").unwrap();
    assert!(generated.contains("::iced::widget::container(__container_content)"));
    assert!(
        generated.contains(".direction(::ui_lang_runtime::FlexDirection::Column).gap(6.0 as f32)")
    );
    assert!(
        generated.contains(".direction(::ui_lang_runtime::FlexDirection::Row).gap(4.0 as f32)")
    );
    assert!(generated.contains(".width(::iced::Length::FillPortion(2))"));

    let error = compile(&source.replace("dir=column", "dir=diagonal"), "layouts.ice").unwrap_err();
    assert_eq!(error.code, "E074");
    assert!(
        error
            .message
            .contains("row, row-reverse, column, or column-reverse")
    );
}

#[test]
fn lowers_complete_css_flexbox() {
    let source = r#"app Flexbox
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
view
  flex dir=row-reverse wrap=wrap-reverse w=fill h=300.0 max-w=900.0 max-h=500.0 gap=8.0 gap-y=12.0 gap-x=16.0 justify=space-evenly items=baseline content=space-between p=4.0 clip=true
    box order=2 grow=1.0 shrink=0.5 basis=percent(40.0) self=flex-end m=auto
      text "First"
    box flex=2.0,1.0,120.0 mx=percent(5.0) mt=-2.0
      text "Second"
"#;
    let generated = compile(source, "flexbox.ice").unwrap();
    for expected in [
        "FlexDirection::RowReverse",
        "FlexWrap::WrapReverse",
        "JustifyContent::SpaceEvenly",
        "AlignItems::Baseline",
        "AlignContent::SpaceBetween",
        ".gap(8.0 as f32).row_gap(12.0 as f32).column_gap(16.0 as f32)",
        ".max_width(900.0 as f32).max_height(500.0 as f32).clip(true)",
        ".order(2 as i64).grow(1.0 as f32).shrink(0.5 as f32)",
        "FlexBasis::Percent((40.0 as f32) / 100.0)",
        ".align_self(::ui_lang_runtime::AlignItems::FlexEnd)",
        "top: ::ui_lang_runtime::FlexMargin::Auto",
        "FlexBasis::Fixed(120.0 as f32)",
        "FlexMargin::Percent((5.0 as f32) / 100.0)",
        "top: ::ui_lang_runtime::FlexMargin::Fixed(",
    ] {
        assert!(generated.contains(expected), "missing `{expected}`");
    }
    let shorthand = source.replace(
        "dir=row-reverse wrap=wrap-reverse",
        "flow=row-reverse,wrap-reverse",
    );
    assert!(
        compile(&shorthand, "flexbox.ice")
            .unwrap()
            .contains("FlexWrap::WrapReverse")
    );
}

#[test]
fn lowers_complete_flex_layouts_and_wrapping() {
    let source = r#"app Layouts
theme
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
    let generated = compile(source, "layouts.ice").unwrap();
    assert!(generated.contains(
        "::iced::widget::column(__children).spacing(::ui_lang_runtime::bounded_spacing(8.0, __child_count))"
    ));
    assert!(generated.contains("::ui_lang_runtime::bounded_padding(4.0, 5.0, 6.0, 7.0)"));
    assert!(generated.contains(".width(::iced::Fill).height(::iced::Shrink)"));
    assert!(generated.contains(".max_width(640.0 as f32)"));
    assert!(generated.contains(
            ".align_x(::iced::alignment::Horizontal::Center).clip(true).wrap().horizontal_spacing(::ui_lang_runtime::bounded_spacing(12.0, __child_count)).align_x(::iced::alignment::Vertical::Bottom)"
        ));
    assert!(generated.contains(".width(::iced::Length::FillPortion(2)).height(48.0 as f32)"));
    assert!(generated.contains(
            ".align_y(::iced::alignment::Vertical::Bottom).clip(false).wrap().vertical_spacing(::ui_lang_runtime::bounded_spacing(6.0, __child_count)).align_x(::iced::alignment::Horizontal::Left)"
        ));
}
#[test]
fn lowers_complete_container_layout() {
    let source = r#"app Boxed
extern crate::backend
  box-style dynamic_container(highlight:bool)
theme
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
    let generated = compile(source, "boxed.ice").unwrap();
    assert!(generated.contains("::iced::widget::container(__container_content)"));
    assert!(generated.contains(".id(::iced::widget::Id::from("));
    assert!(generated.contains(".width(::iced::Fill).height(80.0 as f32)"));
    assert!(generated.contains(".max_width(640.0 as f32).max_height(120.0 as f32)"));
    assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Center)"));
    assert!(generated.contains(".align_y(::iced::alignment::Vertical::Bottom)"));
    assert!(generated.contains(".clip(true)"));
    assert!(generated.contains("crate::backend::dynamic_container(__theme, self.highlight)"));
    assert!(generated.contains("fn __ui_lang_check_container_style_dynamic_container"));
    assert!(generated.contains("::iced::widget::container::Style"));
    assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
    assert!(generated.contains("__style.border.radius"));
    assert!(generated.contains("__style.shadow.blur_radius = 6.0 as f32"));
    assert!(generated.contains("__style.snap = true"));
    assert!(generated.contains("__style.border.width = 2.0 as f32;"));
}

#[test]
fn lowers_structured_overlays_to_native_overlay_widgets() {
    let source = r#"app Dialog
theme
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
    let generated = compile(source, "dialog.ice").unwrap();
    assert!(generated.contains("if self.shown"));
    assert!(generated.contains("::iced::widget::Stack::new()"));
    assert!(generated.contains("::iced::widget::float(__overlay_surface)"));
    assert!(generated.contains("::core::f32::EPSILON"));
    assert!(generated.contains("::iced::Color::from_rgba8(0, 0, 0, 0.600000)"));
    assert!(generated.contains(".on_press(__DialogMessage::Close)"));
    assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Center)"));
    assert!(generated.contains(".align_y(::iced::alignment::Vertical::Bottom)"));
    assert!(generated.contains("__DialogMessage::__ExternNoop"));
}

#[test]
fn lowers_plain_stacks_to_union_sizing_zstack() {
    // A plain `stack` must size to the bounding box of every layer so a small
    // first layer (e.g. a 1px focus helper) cannot collapse a later popover.
    // iced's native `Stack` sizes to the first layer only, so plain stacks lower
    // to the union-sizing `ui_lang_runtime::zstack` instead.
    let source = r#"app Popover
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
view
  stack
    box w=1.0 h=1.0 @bg-bg
      text ""
    box w=190.0 p=4.0 @bg-bg
      text "Menu"
"#;
    let generated = compile(source, "popover.ice").unwrap();
    assert!(generated.contains("::ui_lang_runtime::zstack(__children)"));
    // The union-sizing z-stack must not fall back to iced's first-layer Stack.
    assert!(!generated.contains("::iced::widget::stack(__children)"));

    // `under=N` still uses iced's native base-layer `Stack`/`push_under`, which
    // already picks the correct intrinsic base.
    let under = compile(
        &source.replace("  stack\n", "  stack under=1\n"),
        "popover.ice",
    )
    .unwrap();
    assert!(under.contains("::iced::widget::Stack::new()"));
    assert!(under.contains("push_under(__child)"));
    assert!(!under.contains("::ui_lang_runtime::zstack(__children)"));
}

#[test]
fn lowers_persistent_pane_grids() {
    let source = r#"app Workspace
theme
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
    let generated = compile(source, "workspace.ice").unwrap();
    assert!(generated.contains("__pane_work: ::iced::widget::pane_grid::State"));
    assert!(generated.contains("pane_grid::Configuration::Split"));
    assert!(generated.contains("pane_grid::Axis::Vertical"));
    assert!(generated.contains("Configuration::Pane(\"files\")"));
    assert!(generated.contains("::iced::widget::pane_grid(&self.__pane_work"));
    assert!(generated.contains(
        ".spacing(::ui_lang_runtime::bounded_table_metric(8.0, self.__pane_work.len()))"
    ));
    assert!(generated.contains(
        ".min_size(::ui_lang_runtime::bounded_table_metric(120.0, self.__pane_work.len()))"
    ));
    assert!(generated.contains(
        ".on_resize(::ui_lang_runtime::bounded_table_metric(6.0, self.__pane_work.len()), __WorkspaceMessage::__PaneWorkResize)"
    ));
    assert!(generated.contains(".on_drag(__WorkspaceMessage::__PaneWorkDrag)"));
    assert!(generated.contains("self.__pane_work.resize(__event.split, __event.ratio)"));
    assert!(generated.contains("self.__pane_work.drop(pane, target)"));
    assert!(generated.contains("__WorkspaceMessage::Clicked(__pane_name.to_owned())"));
}

#[test]
fn lowers_nested_pane_configuration_and_closed_templates() {
    let source = r#"app Workspace
theme
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
    let generated = compile(source, "workspace.ice").unwrap();
    assert_eq!(
        generated.matches("pane_grid::Configuration::Split").count(),
        2
    );
    assert!(generated.contains("pane_grid::Axis::Vertical"));
    assert!(generated.contains("pane_grid::Axis::Horizontal"));
    assert!(generated.contains("Configuration::Pane(\"terminal\")"));
    assert!(!generated.contains("Configuration::Pane(\"preview\")"));
    assert!(generated.contains("\"preview\" =>"));
    assert!(generated.contains(".split(::iced::widget::pane_grid::Axis::Horizontal"));
    assert!(generated.contains("__pane_work_splits: ::std::collections::BTreeMap"));
    assert!(generated.contains("Option::Some(\"workspace_root\")"));
    assert!(generated.contains("Option::Some(\"editor_stack\")"));
    assert!(generated.contains("self.__pane_work_splits.get(\"editor_stack\").copied()"));
    assert!(
        generated.contains("self.__pane_work.resize(__split, ((0.55) as f32).max(0.0).min(1.0))")
    );
}

#[test]
fn lowers_runtime_pane_templates() {
    let source = r#"app Workspace
extern crate::backend
  Task(id:i64, title:str)
theme
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
on clicked(name)
view
  panes #work click=clicked(_)
    pane files maximized=files_maximized
      col
        if files_maximized
          text "Maximized files"
    pane task in tasks by=task.id maximized=task_maximized
      scroll #body
        col
          if task_maximized
            text "Maximized task"
          text task.title
"#;
    let generated = compile(source, "workspace.ice").unwrap();
    assert!(generated.contains("enum __IcePaneWork"));
    assert!(generated.contains("Task(i64)"));
    assert!(generated.contains("State<__IcePaneWork>"));
    assert!(generated.contains("Configuration::Pane(__IcePaneWork::__Static(\"files\"))"));
    assert!(generated.contains("__IcePaneWork::Task(__pane_key)"));
    assert!(
        generated.contains("self.tasks.iter().find(|task| (*task).id == (*__pane_key).clone())")
    );
    assert!(generated.contains("__IcePaneWork::Task(self.selected)"));
    assert!(generated.contains("__pane.__name()"));
    assert!(generated.contains("__pane_maximized"));
    assert!(generated.contains("if __pane_maximized"));
    assert!(generated.contains("format!(\"{}/task({})\""));
}

#[test]
fn lowers_structured_pane_titles_and_dynamic_controls() {
    let source = r#"app Workspace
extern crate::backend
  panes-style dynamic_panes(active:bool)
theme
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
    let generated = compile(source, "workspace.ice").unwrap();
    assert!(generated.contains("__ui_lang_check_pane_grid_style_dynamic_panes"));
    assert!(generated.contains("crate::backend::dynamic_panes(__theme, self.active)"));
    assert!(generated.contains("pane_grid::Content::new(__pane_content).style"));
    assert!(generated.contains(".title_bar(::iced::widget::pane_grid::TitleBar::new"));
    assert!(generated.contains("::ui_lang_runtime::bounded_padding(6.0, 8.0, 4.0, 8.0)"));
    assert!(generated.contains("pane_grid::Controls::dynamic"));
    assert!(generated.contains("pane_grid::Controls::new"));
    assert!(generated.contains(".always_show_controls().style"));
    assert!(generated.contains("__BindFilter"));
    assert!(generated.contains("format!(\"{}/filter\""));
    assert!(generated.contains("let mut __style = crate::backend::dynamic_panes"));
    assert!(generated.contains("__style.hovered_region.background"));
    assert!(generated.contains("::iced::gradient::Linear::new(0.785 as f32)"));
    assert!(generated.contains(".add_stop(0.5 as f32"));
    assert!(generated.contains("__style.hovered_region.border.color"));
    assert!(generated.contains("__style.hovered_region.border.width = 2.0 as f32"));
    assert!(generated.contains("top_left: ((1.0) as f32).max(0.0).min(f32::MAX)"));
    assert!(generated.contains("top_right: ((2.0) as f32).max(0.0).min(f32::MAX)"));
    assert!(generated.contains("bottom_right: ((3.0) as f32).max(0.0).min(f32::MAX)"));
    assert!(generated.contains("bottom_left: ((4.0) as f32).max(0.0).min(f32::MAX)"));
    assert!(generated.contains("__style.hovered_split.color"));
    assert!(generated.contains("__style.hovered_split.width = 3.0 as f32"));
    assert!(generated.contains("__style.picked_split.color"));
    assert!(generated.contains("__style.picked_split.width = 4.0 as f32"));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    assert!(generated.contains("__style.shadow.color"));
    assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
    assert!(generated.contains("__style.shadow.offset.y = 2.0 as f32"));
    assert!(generated.contains("__style.shadow.blur_radius = 6.0 as f32"));
    assert!(generated.contains("__style.snap = true"));
    assert!(generated.contains("__style.snap = false"));
}

#[test]
fn lowers_pane_state_operations_and_queries() {
    let source = r#"app Workspace
theme
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
  pane #work drop editor files top
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
    let generated = compile(source, "workspace.ice").unwrap();
    assert!(generated.contains("self.__pane_work.maximize(__pane)"));
    assert!(generated.contains("self.__pane_work.restore()"));
    assert!(generated.contains("self.__pane_work.swap(__first, __second)"));
    assert!(generated.contains("move_to_edge(__pane, ::iced::widget::pane_grid::Edge::Left)"));
    assert!(generated.contains("layout().splits().next().copied()"));
    assert!(
        generated.contains("self.__pane_work.resize(__split, ((0.6) as f32).max(0.0).min(1.0))")
    );
    assert!(generated.contains("pane_grid::Target::Pane(__target"));
    assert!(generated.contains("pane_grid::Region::Edge"));
    assert!(generated.contains(".split(::iced::widget::pane_grid::Axis::Horizontal"));
    assert!(generated.contains("\"preview\""));
    assert!(generated.contains("self.__pane_work.close(__pane)"));
    assert!(generated.contains("self.__pane_work.maximized()"));
    assert!(generated.contains("pane_grid::Direction::Right"));
    assert!(generated.contains("::iced::Task::done(__WorkspaceMessage::Observed(value))"));
}

#[test]
fn lowers_list_literals_options_and_pick_lists() {
    let source = r#"app Selection
extern crate::backend
  pick-list-style dynamic_pick(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  choices = ["List", "Board"]
  selected:str? = none
on selected(next)
  selected = some(next)
on opened
on closed
view
  pick choices selected hint="Choose" w=fill menu-h=120.0 p=8.0 text-size=14.0 line-h=1.2 shape=advanced font=ui open=opened close=closed style=dynamic_pick(busy) menu-style=dynamic_menu(busy) -> selected _
    active text=fg placeholder=danger handle=primary bg=bg border=fg border-w=1.0 r=4.0
    hovered text=fg
    opened text=fg
    opened-hovered text=fg
    menu text=fg selected-text=bg selected-bg=primary bg=bg border=fg border-w=1.0 r=6.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0
    handle dynamic
      closed code="⌄" font=ui size=12.0 line-h=1.0 shape=basic
      open code="⌃" font=ui size=13.0 line-h=1.1 shape=advanced
"#;
    let generated = compile(source, "selection.ice").unwrap();
    assert!(
        generated.contains("pub(crate) selected: ::std::option::Option<::std::string::String>")
    );
    assert!(generated.contains("::std::vec![\"List\".to_owned(), \"Board\".to_owned()]"));
    assert!(generated.contains("::iced::widget::pick_list(__pick_options, self.selected.clone()"));
    assert!(generated.contains("let __pick_option_count = __pick_options.len()"));
    assert!(
        generated.contains(
            ".padding(::ui_lang_runtime::bounded_table_metric(8.0, __pick_option_count))"
        )
    );
    assert!(generated.contains(".on_open(__SelectionMessage::Opened)"));
    assert!(
        generated.contains(
            ".text_line_height(::iced::widget::text::LineHeight::Relative(((1.2) as f32).max(f32::EPSILON).min(f32::MAX)))"
        )
    );
    assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
    assert!(generated.contains("::iced::widget::pick_list::Handle::Dynamic"));
    assert!(
        generated.contains(
            "let mut __style = crate::backend::dynamic_pick(__theme, __status, self.busy);"
        )
    );
    assert!(generated.contains("match __status"));
    assert!(
        generated.contains("let mut __style = crate::backend::dynamic_menu(__theme, self.busy);")
    );
    assert!(generated.contains("fn __ui_lang_check_pick_list_style_dynamic_pick"));
    assert!(generated.contains("fn __ui_lang_check_menu_style_dynamic_menu"));
    assert!(generated.contains("Status::Opened { is_hovered: false }"));
    assert!(generated.contains("Status::Opened { is_hovered: true }"));
    assert!(generated.contains(".menu_style(move |__theme|"));
    assert!(generated.contains("__style.selected_background"));
    assert!(generated.contains("__style.shadow.blur_radius = 4.0 as f32"));
    assert!(generated.contains("self.selected = ::std::option::Option::Some(next);"));
    let defaults = compile(
        &source.replace(
            " style=dynamic_pick(busy) menu-style=dynamic_menu(busy)",
            "",
        ),
        "selection.ice",
    )
    .unwrap();
    assert!(defaults.contains("pick_list::default(__theme, __status)"));
    assert!(defaults.contains("menu::default(__theme)"));
}

#[test]
fn lowers_searchable_combo_boxes() {
    let source = r#"app Search
extern crate::backend
  input-style dynamic_input(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  modes:combo[str] = ["List", "Board"]
  selected:str? = none
  query = ""
on selected(next)
  selected = some(next)
on searched(next)
  query = next
on hovered(next)
on opened
on closed
on reset
  modes = ["Timeline"]
on add
  combo modes push "Calendar"
view
  combo modes selected "Search modes" w=fill menu-h=120.0 p=8.0 text-size=14.0 line-h=1.2 shape=advanced font=ui input=searched hover=hovered open=opened close=closed style=dynamic_input(busy) menu-style=dynamic_menu(busy) -> selected _
    active bg=bg border=fg border-w=1.0 r=4.0 icon=primary placeholder=danger value=fg selection=primary
    hovered bg=bg icon=fg placeholder=danger value=fg selection=primary
    focused bg=bg border=primary
    focused-hovered bg=bg border=fg
    disabled bg=bg value=danger
    menu text=fg selected-text=bg selected-bg=primary bg=bg border=fg border-w=1.0 r=6.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0
    icon code="⌕" font=ui size=12.0 gap=6.0 side=right
"#;
    let generated = compile(source, "search.ice").unwrap();
    assert!(
        generated
            .contains("pub(crate) modes: ::iced::widget::combo_box::State<::std::string::String>")
    );
    assert!(generated.contains(
            "::iced::widget::combo_box::State::new(::std::vec![\"List\".to_owned(), \"Board\".to_owned()])"
        ));
    assert!(generated.contains(
        "::iced::widget::combo_box(&self.modes, \"Search modes\", __combo_selection.as_ref()"
    ));
    assert!(generated.contains("let __combo_option_count = self.modes.options().len()"));
    assert!(
        generated.contains(
            ".padding(::ui_lang_runtime::bounded_table_metric(8.0, __combo_option_count))"
        )
    );
    assert!(generated.contains(".on_input(move |__value| __SearchMessage::Searched(__value))"));
    assert!(
        generated.contains(".on_option_hovered(move |__value| __SearchMessage::Hovered(__value))")
    );
    assert!(
        generated.contains(
            ".line_height(::iced::widget::text::LineHeight::Relative(((1.2) as f32).max(f32::EPSILON).min(f32::MAX)))"
        )
    );
    assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
    assert!(generated.contains("code_point: '⌕'"));
    assert!(generated.contains("Side::Right"));
    assert!(generated.contains(".input_style(move |__theme, __status|"));
    assert!(generated.contains("crate::backend::dynamic_input(__theme, __status, self.busy)"));
    assert!(generated.contains("crate::backend::dynamic_menu(__theme, self.busy)"));
    assert!(generated.contains("fn __ui_lang_check_input_style_dynamic_input"));
    assert!(generated.contains("fn __ui_lang_check_menu_style_dynamic_menu"));
    assert!(generated.contains("Status::Focused { is_hovered: true }"));
    assert!(generated.contains(".menu_style(move |__theme|"));
    assert!(generated.contains("__style.selected_background"));
    assert!(generated.contains(
        "self.modes = ::iced::widget::combo_box::State::new(::std::vec![\"Timeline\".to_owned()]);"
    ));
    assert!(generated.contains("self.modes.push(\"Calendar\".to_owned());"));
}

#[test]
fn lowers_structural_widgets_and_size_events() {
    let source = r#"app Structure
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  sensor_key = 0
  width = 0.0
  height = 0.0
on shown(w, h)
  width = w
  height = h
on resized(w, h)
  width = w
  height = h
on hidden
view
  col
    float scale=1.1 x=(viewport_x + viewport_width - original_x - original_width) y=(viewport_y + viewport_height - original_y - original_height) shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 r=8.0 r-tl=1.0 r-tr=2.0 r-br=3.0 r-bl=4.0
      text "Floating"
    pin w=fill h=80.0 x=12.0 y=8.0
      text "Pinned"
    sensor show=shown resize=resized hide=hidden key=sensor_key anticipate=32.0 delay=10
      text "Observed"
    responsive at=600.0 w=fill h=40.0
      text "Narrow"
      text "Wide"
    responsive size=(available_width, available_height) w=fill h=fill
      col
        if available_width < available_height
          text "Portrait"
        if available_width >= available_height
          text "Landscape"
"#;
    let generated = compile(source, "structure.ice").unwrap();
    assert!(generated.contains(
        "::iced::widget::float(__float_content).scale(((1.1) as f32).max(f32::EPSILON).min(f32::MAX))"
    ));
    assert!(generated.contains("translate(move |__original, __viewport|"));
    assert!(generated.contains(
            "(((__viewport.x as f64) + (__viewport.width as f64)) - (__original.x as f64)) - (__original.width as f64)"
        ));
    assert!(generated.contains(
            "(((__viewport.y as f64) + (__viewport.height as f64)) - (__original.y as f64)) - (__original.height as f64)"
        ));
    assert!(generated.contains("::iced::widget::float::Style::default()"));
    assert!(generated.contains("__style.shadow.color = ::iced::Color::from_rgba8"));
    assert!(generated.contains("__style.shadow.offset.x = 1.0 as f32"));
    assert!(generated.contains("__style.shadow.offset.y = 2.0 as f32"));
    assert!(generated.contains("__style.shadow.blur_radius = 4.0 as f32"));
    assert!(generated.contains("__style.shadow_border_radius = ::iced::border::Radius"));
    assert!(generated.contains("top_left: ((1.0) as f32).max(0.0).min(f32::MAX)"));
    assert!(generated.contains("top_right: ((2.0) as f32).max(0.0).min(f32::MAX)"));
    assert!(generated.contains("bottom_right: ((3.0) as f32).max(0.0).min(f32::MAX)"));
    assert!(generated.contains("bottom_left: ((4.0) as f32).max(0.0).min(f32::MAX)"));
    assert!(generated.contains("::iced::widget::pin(__pin_content).x(12.0 as f32)"));
    assert!(generated.contains(
            ".on_show(move |__size| __StructureMessage::Shown(__size.width as f64, __size.height as f64))"
        ));
    assert!(generated.contains(".key(self.sensor_key)"));
    assert!(generated.contains(".anticipate(((32.0) as f32).max(0.0).min(f32::MAX))"));
    assert!(
        generated
            .contains(".delay(::std::time::Duration::from_millis(u64::try_from(10).unwrap_or(0)))")
    );
    assert!(generated.contains("::iced::widget::responsive(move |__size|"));
    assert!(
        generated.contains("if __size.width < ((600.0) as f32).max(f32::EPSILON).min(f32::MAX)")
    );
    assert!(generated.contains("if ((__size.width as f64) < (__size.height as f64))"));
    assert!(generated.contains("if ((__size.width as f64) >= (__size.height as f64))"));
}

#[test]
fn lowers_configured_scrollables_and_viewport_events() {
    let source = r#"app Scrolling
extern crate::backend
  scroll-style dynamic_scroll(busy:bool)
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  absolute_x = 0.0
  absolute_y = 0.0
  relative_x = 0.0
  relative_y = 0.0
on scrolled(ax, ay, rx, ry)
  absolute_x = ax
  absolute_y = ay
  relative_x = rx
  relative_y = ry
on viewport(ax, ay, reversed_x, reversed_y, rx, ry, bx, by, bw, bh, cx, cy, cw, ch)
view
  col
    scroll #feed dir=both w=fill h=200.0 bar=hidden bar-w=8.0 bar-m=2.0 scroller-w=6.0 bar-gap=4.0 anchor-x=end anchor-y=start auto=true scroll=scrolled style=dynamic_scroll(busy)
      text "Absolute offsets"
    scroll dir=both w=fill h=200.0 viewport=viewport style=dynamic_scroll(busy)
      col
        text "Complete viewport"
      active x-disabled=false y-disabled=false
        box bg=bg text=fg border=primary border-w=1.0 r=4.0 r-tl=1.0 r-tr=2.0 r-br=3.0 r-bl=4.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 px-snap=true
        x-rail bg=bg border=primary border-w=1.0 r=2.0
        x-scroller bg=primary border=fg border-w=1.0 r=2.0
        y-rail bg=bg border=primary border-w=1.0 r=2.0
        y-scroller bg=primary border=fg border-w=1.0 r=2.0
        gap bg=bg
        auto bg=bg border=primary border-w=1.0 r=4.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 icon=fg
      hovered x-hovered=true y-hovered=false x-disabled=false y-disabled=false
        x-scroller bg=fg
      dragged x-dragged=false y-dragged=true x-disabled=false y-disabled=false
        y-scroller bg=danger
"#;
    let generated = compile(source, "scrolling.ice").unwrap();
    assert!(generated.contains("scrollable::Direction::Both"));
    assert!(generated.contains(
        "scrollable::Scrollbar::hidden().width(::ui_lang_runtime::bounded_table_metric(8.0, 2)).margin(::ui_lang_runtime::bounded_table_metric(2.0, 2)).scroller_width(::ui_lang_runtime::bounded_table_metric(6.0, 2)).spacing(::ui_lang_runtime::bounded_table_metric(4.0, 2))"
    ));
    assert!(generated.contains(".anchor_x(::iced::widget::scrollable::Anchor::End)"));
    assert!(generated.contains(".auto_scroll(true)"));
    assert!(generated.contains("crate::backend::dynamic_scroll(__theme, __status, self.busy)"));
    assert!(generated.contains(
            ".style(move |__theme, __status| crate::backend::dynamic_scroll(__theme, __status, self.busy))"
        ));
    assert!(generated.contains(
            "let mut __style = crate::backend::dynamic_scroll(__theme, __status, self.busy); match __status"
        ));
    assert!(generated.contains("fn __ui_lang_check_scroll_style_dynamic_scroll"));
    assert!(generated.contains("let __absolute = __viewport.absolute_offset()"));
    assert!(generated.contains(
            "__ScrollingMessage::Scrolled(__absolute.x as f64, __absolute.y as f64, __relative.x as f64, __relative.y as f64)"
        ));
    assert!(generated.contains("absolute_offset_reversed()"));
    assert!(generated.contains("let __bounds = __viewport.bounds()"));
    assert!(generated.contains("let __content_bounds = __viewport.content_bounds()"));
    assert!(generated.contains("scrollable::Status::Hovered"));
    assert!(generated.contains("__horizontal_interaction == true"));
    assert!(generated.contains("let __style = &mut __style.container"));
    assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
    assert!(generated.contains("__style.horizontal_rail.scroller.background"));
    assert!(generated.contains("__style.vertical_rail.scroller.background"));
    assert!(generated.contains("__style.gap = ::std::option::Option::Some"));
    assert!(generated.contains("let __style = &mut __style.auto_scroll"));
    assert!(generated.contains("__style.shadow.blur_radius = 4.0 as f32"));
    assert!(generated.contains("__style.auto_scroll.icon"));
    let default_scroll = compile(
        &source.replace(" style=dynamic_scroll(busy)", ""),
        "scrolling.ice",
    )
    .unwrap();
    assert!(default_scroll.contains("scrollable::default(__theme, __status)"));
}
