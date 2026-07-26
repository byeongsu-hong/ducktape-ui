use super::*;

#[path = "widgets/content.rs"]
mod content;
#[path = "widgets/controls.rs"]
mod controls;
#[path = "widgets/layout.rs"]
mod layout;

#[test]
fn rejects_removed_style_aliases_and_property_collisions() {
    let header = r#"app Demo
extern crate::backend
  box-style container_base()
  input-style input_base()
  button-style button_base()
  text-style text_base()
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
on pressed
view
"#;
    for (property, owner, utility, view) in [
        (
            "width",
            "w=",
            "w-full",
            "box w=fill @w-full\n    text \"x\"",
        ),
        (
            "height",
            "h=",
            "h-full",
            "box h=fill @h-full\n    text \"x\"",
        ),
        (
            "max-width",
            "max-w=",
            "max-w-md",
            "box max-w=320.0 @max-w-md\n    text \"x\"",
        ),
        ("padding", "p=", "p-2", "box p=8.0 @p-2\n    text \"x\""),
        (
            "background",
            "bg=",
            "bg-primary",
            "box bg=bg @bg-primary\n    text \"x\"",
        ),
        (
            "text color",
            "text=",
            "text-primary",
            "box text=fg @text-primary\n    text \"x\"",
        ),
        (
            "border width",
            "border-w=",
            "border",
            "box border-w=2.0 @border\n    text \"x\"",
        ),
        (
            "border color",
            "border=",
            "border-danger",
            "box border=primary @border border-danger\n    text \"x\"",
        ),
        (
            "radius",
            "r=",
            "rounded",
            "box r=4.0 @border rounded\n    text \"x\"",
        ),
        (
            "width",
            "w=",
            "w-full",
            "scroll w=fill @w-full\n    text \"x\"",
        ),
        (
            "spacing",
            "gap=",
            "gap-2",
            "col gap=8.0 @gap-2\n    text \"x\"",
        ),
        ("padding", "p=", "px-2", "row p=8.0 @px-2\n    text \"x\""),
        (
            "alignment",
            "align=",
            "items-center",
            "row align=center @items-center\n    text \"x\"",
        ),
        (
            "overflow",
            "clip=",
            "overflow-hidden",
            "box clip=true @overflow-hidden\n    text \"x\"",
        ),
        (
            "spacing",
            "gap=",
            "gap-2",
            "grid cols=1 gap=8.0 @gap-2\n    text \"x\"",
        ),
        (
            "width",
            "w=",
            "w-full",
            "stack w=100.0 @w-full\n    text \"x\"",
        ),
        (
            "height",
            "h=",
            "h-full",
            "stack h=100.0 @h-full\n    text \"x\"",
        ),
        (
            "text size",
            "size=",
            "text-sm",
            "text \"x\" size=16.0 @text-sm",
        ),
        (
            "text color",
            "color=",
            "text-primary",
            "rich-text color=fg @text-primary\n    span \"x\"",
        ),
        (
            "text size",
            "size=",
            "text-sm",
            "rich-text\n    span \"x\" size=16.0 @text-sm",
        ),
        (
            "width",
            "w=",
            "w-full",
            "input \"x\" <-> value w=fill @w-full",
        ),
        ("padding", "p=", "px-2", "input \"x\" <-> value p=8.0 @px-2"),
        (
            "active background",
            "active bg=",
            "bg-primary",
            "input \"x\" <-> value @bg-primary\n    active bg=bg",
        ),
        (
            "focused border color",
            "focused border=",
            "focus:border-danger",
            "input \"x\" <-> value @border focus:border-danger\n    focused border=primary",
        ),
        ("padding", "p=", "p-2", "button \"x\" p=8.0 @p-2 -> pressed"),
        (
            "hovered background",
            "hovered bg=",
            "hover:bg-danger",
            "button \"x\" @bg-primary hover:bg-danger -> pressed\n    hovered bg=bg",
        ),
        (
            "active text color",
            "active text=",
            "text-primary",
            "button \"x\" @text-primary -> pressed\n    active text=fg",
        ),
        (
            "pressed radius",
            "pressed r=",
            "rounded-lg",
            "button \"x\" @rounded-lg -> pressed\n    pressed r=4.0",
        ),
        (
            "background",
            "bg=",
            "bg-primary",
            "panes #work\n    pane files bg=bg @bg-primary\n      text \"x\"",
        ),
        (
            "background",
            "bg=",
            "bg-primary",
            "panes #work\n    pane files\n      title bg=bg @bg-primary\n        text \"title\"\n      text \"x\"",
        ),
    ] {
        let source = format!("{header}  {view}\n");
        let error = analyze(&source).unwrap_err();
        if matches!(error.code, "E041" | "E042") {
            continue;
        }
        assert_eq!(error.code, "E045", "{view}");
        assert!(error.message.contains(&format!("`{property}`")), "{view}");
        assert!(error.message.contains(&format!("`{owner}`")), "{view}");
        assert!(error.message.contains(&format!("`@{utility}`")), "{view}");
        assert!(
            error.hint.is_some_and(|hint| hint.contains(owner)),
            "{view}"
        );
    }

    for view in [
        "row w=fill @w-full\n    text \"x\"",
        "col max-w=320.0 @max-w-md\n    text \"x\"",
        "grid cols=1 w=320.0 @w-full\n    text \"x\"",
        "stack w=fill h=fill\n    text \"x\"",
        "stack @w-full h-full\n    text \"x\"",
        "box style=container_base() @bg-primary\n    text \"x\"",
        "input \"x\" <-> value style=input_base() @bg-primary",
        "button \"x\" style=button_base() @bg-primary -> pressed",
        "button \"x\" @disabled:opacity-50 -> pressed\n    disabled bg=bg",
        "text \"x\" font=mono style=text_base() @font-bold text-primary",
    ] {
        analyze(&format!("{header}  {view}\n")).unwrap();
    }

    let view = "box border=primary @border-danger\n    text \"x\"";
    let error = analyze(&format!("{header}  {view}\n")).unwrap_err();
    assert_eq!(error.code, "E044", "{view}");
}

#[test]
fn checks_accessibility_names_descriptions_and_icon_buttons() {
    let source = r#"app Accessible
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  name = ""
  checked = false
on press
on toggle(value)
view
  col
    input "Name" label=name description="Profile name" <-> name
    button label="Open help" description="Show help" -> press
      text "?"
    checkbox "Ready" label="Ready state" checked=checked -> toggle _
    image "photo.ppm" label="Portrait" description="Profile portrait"
"#;
    analyze(source).unwrap();

    let error = analyze(&source.replace("label=name", "label=checked")).unwrap_err();
    assert_eq!(error.code, "E101");
    assert!(error.message.contains("expected `str`"));

    let error = analyze(&source.replace("label=\"Open help\" ", "")).unwrap_err();
    assert_eq!(error.code, "E105");
    assert!(error.message.contains("child content"));

    let error = analyze(&source.replace(
        "label=\"Portrait\" description=\"Profile portrait\"",
        "description=\"Profile portrait\"",
    ))
    .unwrap_err();
    assert_eq!(error.code, "E105");
    assert!(error.message.contains("requires an accessibility"));
}
