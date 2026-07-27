use serde_json::{Value, json};

pub const LANGUAGE_REVISION: &str = "1.70";
pub const ICED_VERSION: &str = "0.14.0";
pub const ICED_WIDGET_VERSION: &str = "0.14.2";
pub const UI_LANG_RUNTIME_VERSION: &str = "0.1.0";
pub const ACCESSKIT_VERSION: &str = "0.24.1";
pub const ACCESSKIT_UNIX_VERSION: &str = "0.22.1";
pub const ACCESSKIT_WINDOWS_VERSION: &str = "0.32.0";

#[derive(Clone, Copy)]
struct Completion {
    label: &'static str,
    category: &'static str,
    insert_text: &'static str,
}

impl Completion {
    const fn new(label: &'static str, category: &'static str, insert_text: &'static str) -> Self {
        Self {
            label,
            category,
            insert_text,
        }
    }
}

const COMPLETIONS: &[Completion] = &[
    Completion::new("app", "declaration", "app ${1:Name}\n  $0"),
    Completion::new("use", "declaration", "use \"${1:path}.ice\""),
    Completion::new(
        "recipe",
        "declaration",
        "recipe ${1:panel} for ${2:box}\n  @${3:w-full p-5}",
    ),
    Completion::new(
        "extern",
        "declaration/widget",
        "extern ${1|crate::backend,name() #id|}",
    ),
    Completion::new("state", "declaration", "state\n  ${1:name} = ${2:value}"),
    Completion::new(
        "derived",
        "declaration",
        "derived\n  ${1:name} = ${2:expression}",
    ),
    Completion::new(
        "component",
        "declaration",
        "component ${1:Name}(${2})\n  $0",
    ),
    Completion::new(
        "emits",
        "component declaration",
        "emits\n  ${1:event}(${2:type})",
    ),
    Completion::new(
        "events",
        "component call",
        "events\n  ${1:event} -> ${2:handler} $0",
    ),
    Completion::new("slot", "declaration", "slot ${1:Name}"),
    Completion::new("on", "declaration", "on ${1:event}\n  $0"),
    Completion::new("let", "statement", "let ${1:name} = ${2:expression}"),
    Completion::new("view", "declaration", "view\n  $0"),
    Completion::new("test", "declaration", "test ${1:name}\n  $0"),
    Completion::new(
        "preset",
        "declaration/test configuration",
        "preset ${1:name}",
    ),
    Completion::new(
        "viewport",
        "test configuration",
        "viewport ${1:800} ${2:600}",
    ),
    Completion::new("timeout", "test configuration", "timeout ${1:2s}"),
    Completion::new("mount", "test configuration", "mount\n  $0"),
    Completion::new("target", "test statement", "target ${1:name} = #${2:id}"),
    Completion::new("click", "test interaction", "click ${1:target}"),
    Completion::new("hover", "test interaction", "hover ${1:target}"),
    Completion::new("press", "test interaction", "press ${1:target}"),
    Completion::new("release", "test interaction", "release"),
    Completion::new("type", "test interaction", "type ${1:\"text\"}"),
    Completion::new("key", "test interaction", "key ${1:enter}"),
    Completion::new("resize", "test interaction", "resize ${1:800} ${2:600}"),
    Completion::new(
        "dispatch",
        "test interaction",
        "dispatch ${1:handler}(${2})",
    ),
    Completion::new("expect", "test assertion", "expect ${1:condition}"),
    Completion::new("if", "control", "if ${1:condition}\n  $0"),
    Completion::new("match", "control", "match ${1:value}\n  ${2:case}\n    $0"),
    Completion::new("for", "control", "for ${1:item} in ${2:items}\n  $0"),
    Completion::new(
        "keyed",
        "control",
        "keyed ${1:item} in ${2:items} by=${3:item.id} #${4:id}\n  $0",
    ),
    Completion::new(
        "lazy",
        "control",
        "lazy ${1:dependency} as ${2:value} #${3:id}\n  $0",
    ),
    Completion::new("row", "layout", "row #${1:id}\n  $0"),
    Completion::new("col", "layout", "col #${1:id}\n  $0"),
    Completion::new("flex", "layout", "flex #${1:id} w=fill\n  $0"),
    Completion::new("grid", "layout", "grid #${1:id} cols=${2:3}\n  $0"),
    Completion::new("stack", "layout", "stack #${1:id}\n  $0"),
    Completion::new("scroll", "layout", "scroll #${1:id}\n  $0"),
    Completion::new("box", "layout", "box #${1:id}\n  $0"),
    Completion::new(
        "overlay",
        "layout",
        "overlay #${1:id} when=${2:visible}\n  content\n    ${3:text \"Content\"}\n  layer\n    $0",
    ),
    Completion::new(
        "panes",
        "layout",
        "panes #${1:id}\n  pane ${2:main}\n    $0",
    ),
    Completion::new("text", "widget", "text ${1:\"Text\"} #${2:id}"),
    Completion::new(
        "rich-text",
        "widget",
        "rich-text #${1:id}\n  span ${2:\"Text\"}",
    ),
    Completion::new(
        "input",
        "widget",
        "input \"${1:Label}\" #${2:id} <-> ${3:state}",
    ),
    Completion::new(
        "button",
        "widget",
        "button \"${1:Label}\" #${2:id} -> ${3:handler}",
    ),
    Completion::new(
        "checkbox",
        "widget",
        "checkbox ${1:label} #${2:id} checked=${3:value} -> ${4:handler} _",
    ),
    Completion::new(
        "toggler",
        "widget",
        "toggler ${1:label} #${2:id} checked=${3:value} -> ${4:handler} _",
    ),
    Completion::new(
        "slider",
        "widget",
        "slider ${1:value} #${2:id} min=${3:0.0} max=${4:100.0} -> ${5:handler} _",
    ),
    Completion::new("progress", "widget", "progress ${1:value} #${2:id}"),
    Completion::new(
        "radio",
        "widget",
        "radio ${1:label} #${2:id} value=${3:value} selected=${4:condition} -> ${5:handler} _",
    ),
    Completion::new(
        "pick",
        "widget",
        "pick ${1:options} ${2:selected} #${3:id} -> ${4:handler} _",
    ),
    Completion::new(
        "combo",
        "widget",
        "combo ${1:state} ${2:selected} \"${3:Placeholder}\" #${4:id} -> ${5:handler} _",
    ),
    Completion::new("rule", "widget", "rule ${1:horizontal} #${2:id}"),
    Completion::new("qr", "widget", "qr ${1:data} #${2:id}"),
    Completion::new("space", "widget", "space #${1:id}"),
    Completion::new(
        "markdown",
        "widget",
        "markdown ${1:content} #${2:id} -> ${3:open_link} _",
    ),
    Completion::new("editor", "widget", "editor #${1:id} <-> ${2:state}"),
    Completion::new(
        "table",
        "widget",
        "table ${1:item} in ${2:rows} #${3:id}\n  col\n    header\n      ${4:text \"Header\"}\n    cell\n      $0",
    ),
    Completion::new("themer", "widget", "themer ${1:name}(${2}) #${3:id}"),
    Completion::new("shader", "widget", "shader ${1:name}(${2}) #${3:id}"),
    Completion::new("image", "widget", "image ${1:handle} #${2:id}"),
    Completion::new("svg", "widget", "svg ${1:handle} #${2:id}"),
    Completion::new("viewer", "widget", "viewer ${1:handle} #${2:id}"),
    Completion::new(
        "tooltip",
        "widget",
        "tooltip #${1:id}\n  ${2:text \"Content\"}\n  $0",
    ),
    Completion::new("mouse", "widget", "mouse #${1:id} press=${2:handler}\n  $0"),
    Completion::new(
        "resize-handle",
        "widget",
        "resize-handle #${1:id} drag=${2:handler}\n  $0",
    ),
    Completion::new(
        "canvas",
        "widget",
        "canvas #${1:id} w=${2:fill} h=${3:240.0}\n  $0",
    ),
    Completion::new("theme", "widget", "theme #${1:id} ${2:default}\n  $0"),
    Completion::new("float", "widget", "float #${1:id}\n  $0"),
    Completion::new("pin", "widget", "pin #${1:id}\n  $0"),
    Completion::new(
        "sensor",
        "widget",
        "sensor #${1:id} resize=${2:handler}\n  $0",
    ),
    Completion::new(
        "responsive",
        "widget",
        "responsive #${1:id} size=(${2:width}, ${3:height})\n  $0",
    ),
    Completion::new(
        "run",
        "effect",
        "run ${1:action}(${2}) -> ${3:succeeded} _ | ${4:failed} _",
    ),
    Completion::new("<->", "operator", "<-> ${1:state}"),
    Completion::new("->", "operator", "-> ${1:handler}"),
    Completion::new("~=", "operator", "~= ${1:expected}"),
    Completion::new("_", "operator", "_"),
    Completion::new("#id", "operator", "#${1:id}"),
];

fn property(name: &str, value_type: &str, required: bool) -> Value {
    json!({ "name": name, "type": value_type, "required": required })
}

fn properties(items: &[(&str, &str, bool)]) -> Vec<Value> {
    items
        .iter()
        .map(|(name, value_type, required)| property(name, value_type, *required))
        .collect()
}

fn padding_properties() -> Vec<Value> {
    properties(&[
        ("p", "number", false),
        ("px", "number", false),
        ("py", "number", false),
        ("pt", "number", false),
        ("pr", "number", false),
        ("pb", "number", false),
        ("pl", "number", false),
    ])
}

fn surface_properties() -> Vec<Value> {
    properties(&[
        ("bg", "background", false),
        ("text", "color-token", false),
        ("border", "color-token", false),
        ("border-w", "number", false),
        ("r", "number", false),
        ("r-tl", "number", false),
        ("r-tr", "number", false),
        ("r-br", "number", false),
        ("r-bl", "number", false),
        ("shadow", "color-token", false),
        ("shadow-x", "number", false),
        ("shadow-y", "number", false),
        ("shadow-blur", "number", false),
        ("px-snap", "bool-expression", false),
    ])
}

fn flex_properties(column: bool) -> Vec<Value> {
    let mut output = properties(&[
        ("w", "length", false),
        ("h", "length", false),
        ("clip", "bool-expression", false),
        ("gap", "number", false),
        ("align", "enum(start|center|end)", false),
        ("wrap", "flag", false),
        ("wrap-gap", "number", false),
        ("wrap-align", "enum(start|center|end)", false),
    ]);
    output.extend(padding_properties());
    if column {
        output.push(property("max-w", "number", false));
    }
    output
}

fn css_flex_properties() -> Vec<Value> {
    let mut output = properties(&[
        ("dir", "enum(row|row-reverse|column|column-reverse)", false),
        ("flow", "dir,nowrap|wrap|wrap-reverse", false),
        ("wrap", "enum(nowrap|wrap|wrap-reverse)", false),
        (
            "justify",
            "enum(start|end|flex-start|flex-end|center|stretch|space-between|space-around|space-evenly)",
            false,
        ),
        (
            "items",
            "enum(start|end|flex-start|flex-end|center|baseline|stretch)",
            false,
        ),
        (
            "content",
            "enum(start|end|flex-start|flex-end|center|stretch|space-between|space-around|space-evenly)",
            false,
        ),
        ("gap", "number", false),
        ("gap-y", "number", false),
        ("gap-x", "number", false),
        ("w", "length", false),
        ("h", "length", false),
        ("max-w", "number", false),
        ("max-h", "number", false),
        ("clip", "bool-expression", false),
    ]);
    output.extend(padding_properties());
    output
}

fn keyed_properties() -> Vec<Value> {
    let mut output = properties(&[
        ("w", "length", false),
        ("h", "length", false),
        ("gap", "number", false),
        ("max-w", "number", false),
        ("align", "enum(start|center|end)", false),
    ]);
    output.extend(padding_properties());
    output
}

fn container_properties() -> Vec<Value> {
    let mut output = properties(&[
        ("w", "length", false),
        ("h", "length", false),
        ("max-w", "number", false),
        ("max-h", "number", false),
        ("align-x", "enum(start|center|end)", false),
        ("align-y", "enum(start|center|end)", false),
        ("clip", "bool-expression", false),
        ("order", "integer-expression", false),
        ("grow", "number", false),
        ("shrink", "number", false),
        ("basis", "auto|content|number|percent(number)", false),
        ("flex", "none|auto|initial|grow[,shrink[,basis]]", false),
        (
            "self",
            "enum(auto|start|end|flex-start|flex-end|center|baseline|stretch)",
            false,
        ),
        ("m", "auto|number|percent(number)", false),
        ("mx", "auto|number|percent(number)", false),
        ("my", "auto|number|percent(number)", false),
        ("mt", "auto|number|percent(number)", false),
        ("mr", "auto|number|percent(number)", false),
        ("mb", "auto|number|percent(number)", false),
        ("ml", "auto|number|percent(number)", false),
        ("style", "extern-call", false),
    ]);
    output.extend(padding_properties());
    output.extend(surface_properties());
    output
}

fn text_properties() -> Vec<Value> {
    properties(&[
        ("w", "length", false),
        ("h", "length", false),
        ("size", "number", false),
        ("line-h", "number", false),
        ("line-h-px", "number", false),
        ("font", "font", false),
        (
            "align-x",
            "enum(default|left|center|right|justified)",
            false,
        ),
        ("align-y", "enum(top|center|bottom)", false),
        ("shape", "enum(auto|basic|advanced)", false),
        ("wrap", "enum(none|word|glyph|word-or-glyph)", false),
        ("style", "extern-call", false),
    ])
}

fn bool_control_properties() -> Vec<Value> {
    properties(&[
        ("size", "number", false),
        ("w", "length", false),
        ("gap", "number", false),
        ("text-size", "number", false),
        ("line-h", "number", false),
        ("shape", "enum(auto|basic|advanced)", false),
        ("wrap", "enum(none|word|glyph|word-or-glyph)", false),
        ("font", "font", false),
        ("style", "extern-call", false),
    ])
}

fn selection_properties() -> Vec<Value> {
    properties(&[
        ("w", "length", false),
        ("menu-h", "length", false),
        ("p", "number", false),
        ("text-size", "number", false),
        ("line-h", "number", false),
        ("shape", "enum(auto|basic|advanced)", false),
        ("font", "font", false),
        ("open", "route", false),
        ("close", "route", false),
        ("style", "extern-call", false),
        ("menu-style", "extern-call", false),
    ])
}

fn media_properties(kind: &str) -> Vec<Value> {
    let mut output = properties(&[
        ("label", "str-expression", false),
        ("w", "length", false),
        ("h", "length", false),
        (
            "fit",
            "enum(contain|cover|fill|none|scale-down)|expression",
            false,
        ),
    ]);
    output.insert(
        1,
        json!({
            "name": "description",
            "type": "str-expression",
            "required": false,
            "forbiddenWhen": "label is absent",
        }),
    );
    output.extend(match kind {
        "svg" => properties(&[
            ("rotate", "rotation", false),
            ("opacity", "number", false),
            ("memory", "flag", false),
            ("color", "color-token", false),
            ("hover", "color-token|none", false),
            ("style", "extern-call", false),
        ]),
        "viewer" => properties(&[
            ("filter", "enum(linear|nearest)", false),
            ("p", "number", false),
            ("min-scale", "number", false),
            ("max-scale", "number", false),
            ("scale-step", "number", false),
        ]),
        _ => unreachable!("known media kind"),
    });
    output
}

fn child_shape(min: usize, max: Option<usize>, role: &str) -> Value {
    json!({ "min": min, "max": max, "role": role })
}

fn details(
    contexts: &[&str],
    syntax: &str,
    children: Value,
    binding: Value,
    route: Value,
    properties: Vec<Value>,
) -> Value {
    json!({
        "contexts": contexts,
        "syntax": syntax,
        "children": children,
        "binding": binding,
        "route": route,
        "properties": properties,
    })
}

fn construct_schema(item: &Completion) -> Value {
    let leaf = || child_shape(0, Some(0), "none");
    let no_binding = || Value::Null;
    let no_route = || Value::Null;
    let shape = match item.label {
        "app" => details(
            &["document"],
            "app <Name>",
            child_shape(0, None, "app-setting"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "use" => details(
            &["document"],
            "use \"<relative-path>.ice\"",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "recipe" => details(
            &["document"],
            "recipe <name> for <col|row|flex|grid|stack|box|text|input|button>",
            child_shape(1, None, "utility-line"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "extern" => details(
            &["document", "view"],
            "extern <rust-path>\n  [sync|task|component] <name>(<param>:<type>, ...) -> <type>[ ! <error-type>] | extern <declared-component>(<argument>, ...) [#<id>] [-> <handler> [_]]",
            json!({
                "min": 0,
                "max": null,
                "role": "typed-extern-signature|none",
                "condition": "document declarations own signatures; view calls are leaves",
            }),
            no_binding(),
            json!({ "required": false, "payload": "declared extern component message" }),
            Vec::new(),
        ),
        "state" => details(
            &["document", "component"],
            "state\n  <name>[:<type>] = <expression>",
            child_shape(0, None, "state-entry"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "derived" => details(
            &["document"],
            "derived\n  <name> = <pure-expression>",
            child_shape(1, None, "derived-entry"),
            json!({ "required": true, "name": "name", "source": "read-only app value" }),
            no_route(),
            Vec::new(),
        ),
        "component" => details(
            &["document"],
            "component <Name>([bind] <prop>:<type>[=<default-expression>], ...) [-> <default-output-type>]",
            child_shape(
                1,
                None,
                "component-state|component-events|component-handler|view-root",
            ),
            no_binding(),
            json!({ "requiredWhen": "a default output type is declared", "payload": "default component output" }),
            Vec::new(),
        ),
        "emits" => details(
            &["component"],
            "emits\n  <event>[(<payload-type>, ...)]",
            child_shape(1, None, "named-event-signature"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "events" => details(
            &["component-call"],
            "events\n  <event> -> <handler> [<argument>|_ ...]",
            child_shape(1, None, "named-event-route"),
            no_binding(),
            json!({ "required": true, "payload": "declared ordered event payloads", "scope": "component caller" }),
            Vec::new(),
        ),
        "slot" => details(
            &["component-view"],
            "slot [<Name>]",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "on" => details(
            &["document", "component"],
            "on <handler>[(<payload>, ...)]",
            child_shape(0, None, "statement"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "let" => details(
            &["handler-statement"],
            "let <name> = <expression>",
            leaf(),
            json!({ "required": true, "name": "name", "source": "immutable handler-local value" }),
            no_route(),
            Vec::new(),
        ),
        "view" => details(
            &["document"],
            "view",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "test" => details(
            &["document"],
            "test <snake_case_name>",
            child_shape(0, None, "test-configuration|test-statement"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "preset" => details(
            &["document", "test-configuration"],
            "preset <name> [document declarations may add state and boot sections]",
            json!({
                "min": 0,
                "max": 2,
                "role": "preset-state|preset-boot",
                "condition": "children are available only for a document preset declaration",
            }),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "viewport" => details(
            &["test-configuration"],
            "viewport <width> <height>",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "timeout" => details(
            &["test-configuration"],
            "timeout <duration>",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "mount" => details(
            &["test-configuration"],
            "mount",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "target" => details(
            &["test"],
            "target <name> = #<scoped-id> | <earlier-alias>/<descendant-id>",
            leaf(),
            json!({ "required": true, "name": "name", "source": "widget-selector" }),
            no_route(),
            Vec::new(),
        ),
        "click" => details(
            &["test"],
            "click <target>",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "hover" => details(
            &["test"],
            "hover <target>",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "press" => details(
            &["test"],
            "press <target>",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "release" => details(
            &["test"],
            "release",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "type" => details(
            &["test"],
            "type <str-expression>",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "key" => details(
            &["test"],
            "key <enter|escape|tab|backspace>",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "resize" => details(
            &["test"],
            "resize <width> <height>",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "dispatch" => details(
            &["test"],
            "dispatch <handler>[(<argument>, ...)]",
            leaf(),
            no_binding(),
            json!({ "required": true, "payload": "checked handler arguments" }),
            Vec::new(),
        ),
        "expect" => details(
            &["test"],
            "expect <bool-expression> | expect exists|missing <target> | expect [no] text <str-expression> [within <target>]",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "if" => details(
            &["view"],
            "if <bool-expression>",
            child_shape(0, None, "view-node"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "match" => details(
            &["view"],
            "match <expression>\n  <case-expression>|_\n    <view-node>...",
            child_shape(1, None, "match-arm"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "for" => details(
            &["view"],
            "for <item> in <list-expression>",
            child_shape(0, None, "view-template"),
            json!({ "required": true, "name": "item", "source": "list-expression" }),
            no_route(),
            Vec::new(),
        ),
        "keyed" => details(
            &["view"],
            "keyed <item> in <list-expression> by=<key-expression> [#<id>] [<property>=<value> ...]",
            child_shape(1, Some(1), "view-template"),
            json!({ "required": true, "name": "item", "source": "list-expression" }),
            no_route(),
            keyed_properties(),
        ),
        "lazy" => details(
            &["view"],
            "lazy <dependency-expression> as <name> [#<id>]",
            child_shape(1, Some(1), "view-root"),
            json!({ "required": true, "name": "name", "source": "dependency-expression" }),
            no_route(),
            Vec::new(),
        ),
        "row" => details(
            &["view"],
            "row [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(0, None, "view-node"),
            no_binding(),
            no_route(),
            flex_properties(false),
        ),
        "col" => details(
            &["view"],
            "col [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(0, None, "view-node"),
            no_binding(),
            no_route(),
            flex_properties(true),
        ),
        "flex" => details(
            &["view"],
            "flex [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(0, None, "view-node"),
            no_binding(),
            no_route(),
            css_flex_properties(),
        ),
        "grid" => details(
            &["view"],
            "grid [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(0, None, "view-node"),
            no_binding(),
            no_route(),
            properties(&[
                ("cols", "number", false),
                ("min-cell", "number", false),
                ("max-cell", "number", false),
                ("w", "number", false),
                ("h", "length|aspect(number,number)", false),
                ("gap", "number", false),
            ]),
        ),
        "stack" => details(
            &["view"],
            "stack [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(0, None, "view-node"),
            no_binding(),
            no_route(),
            properties(&[
                ("w", "length", false),
                ("h", "length", false),
                ("clip", "bool-expression", false),
                ("under", "u16", false),
            ]),
        ),
        "scroll" => details(
            &["view"],
            "scroll [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            json!({
                "min": 1,
                "max": null,
                "role": "view-root|scroll-status",
                "condition": "exactly one view root beside any active, hovered, or dragged status children",
            }),
            no_binding(),
            no_route(),
            properties(&[
                ("dir", "enum(vertical|horizontal|both)", false),
                ("w", "length", false),
                ("h", "length", false),
                ("bar", "enum(visible|hidden)", false),
                ("bar-w", "number", false),
                ("bar-m", "number", false),
                ("scroller-w", "number", false),
                ("bar-gap", "number", false),
                ("anchor-x", "enum(start|end)", false),
                ("anchor-y", "enum(start|end)", false),
                ("auto", "bool-expression", false),
                ("scroll", "payload-route(x,y,dx,dy)", false),
                ("viewport", "payload-route(bounds...)", false),
                ("style", "extern-call", false),
            ]),
        ),
        "box" => details(
            &["view"],
            "box [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            no_route(),
            container_properties(),
        ),
        "overlay" => details(
            &["view"],
            "overlay [#<id>] when=<bool-expression> [dismiss=<route>] [backdrop=<color-token>] [p=<number>] [align-x=<start|center|end>] [align-y=<start|center|end>]",
            child_shape(2, Some(2), "content-section|layer-section"),
            no_binding(),
            json!({ "required": false, "property": "dismiss", "payload": "unit" }),
            properties(&[
                ("when", "bool-expression", true),
                ("dismiss", "route", false),
                ("backdrop", "color-token", false),
                ("p", "number", false),
                ("align-x", "enum(start|center|end)", false),
                ("align-y", "enum(start|center|end)", false),
            ]),
        ),
        "panes" => details(
            &["view"],
            "panes #<id> [<property>=<value> ...]",
            json!({
                "min": 1,
                "max": null,
                "role": "style|pane-configuration|closed-pane|pane-template",
                "condition": "an optional style may appear first; one initial pane or split configuration is always required",
            }),
            no_binding(),
            json!({ "required": false, "property": "click", "payload": "pane name" }),
            properties(&[
                ("w", "length", false),
                ("h", "length", false),
                ("gap", "number", false),
                ("min-size", "number", false),
                ("resize", "number", false),
                ("drag", "flag", false),
                ("click", "route", false),
                ("style", "extern-call", false),
            ]),
        ),
        "text" => details(
            &["view"],
            "text <expression> [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            leaf(),
            no_binding(),
            no_route(),
            text_properties(),
        ),
        "rich-text" => {
            let mut rich_text = text_properties();
            rich_text.retain(|property| property["name"] != "shape");
            rich_text.push(property("color", "color-token", false));
            details(
                &["view"],
                "rich-text [#<id>] [<property>=<value> ...] [@<semantic-utility> ...] [-> <handler> [_]]",
                child_shape(0, None, "span"),
                no_binding(),
                json!({
                    "requiredWhen": "any span has link=",
                    "forbiddenWhen": "no span has link=",
                    "payload": "clicked link value",
                }),
                rich_text,
            )
        }
        "input" => details(
            &["view"],
            "input \"<label>\" [#<id>] <-> <state> [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(0, None, "optional-status-extension"),
            json!({ "required": true, "operator": "<->", "target": "state-identifier" }),
            no_route(),
            properties(&[
                ("label", "str-expression", false),
                ("description", "str-expression", false),
                ("hint", "string", false),
                ("disabled", "bool-expression", false),
                ("secure", "bool-expression", false),
                ("change", "payload-route(text)", false),
                ("submit", "route", false),
                ("paste", "payload-route(text)", false),
                ("w", "length", false),
                ("p", "number", false),
                ("text-size", "number", false),
                ("line-h", "number", false),
                ("align", "enum(left|center|right)", false),
                ("font", "font", false),
                ("style", "extern-call", false),
            ]),
        ),
        "button" => {
            let mut button = properties(&[
                ("description", "str-expression", false),
                ("disabled", "bool-expression", false),
                ("w", "length", false),
                ("h", "length", false),
                ("p", "number", false),
                ("clip", "bool-expression", false),
                ("style", "button-preset|extern-call", false),
            ]);
            button.insert(
                0,
                json!({
                    "name": "label",
                    "type": "str-expression",
                    "required": false,
                    "requiredWhen": "button uses child content instead of a string label",
                }),
            );
            details(
                &["view"],
                "button [\"<label>\"] [#<id>] [<property>=<value> ...] [@<semantic-utility> ...] -> <handler> [_]",
                json!({
                    "min": 0,
                    "max": null,
                    "role": "view-root|button-status",
                    "condition": "at most one view root; exactly one when the string label is omitted; any active, hovered, pressed, or disabled status children may follow",
                }),
                no_binding(),
                json!({ "required": true, "operator": "->", "payload": "unit" }),
                button,
            )
        }
        "checkbox" => details(
            &["view"],
            "checkbox <label-expression> [#<id>] checked=<bool-expression> [<property>=<value> ...] -> <handler> _",
            child_shape(0, None, "optional-status-extension"),
            no_binding(),
            json!({ "required": true, "operator": "->", "payload": "bool", "placeholder": "_" }),
            properties(&[
                ("label", "str-expression", false),
                ("description", "str-expression", false),
                ("checked", "bool-expression", true),
                ("disabled", "bool-expression", false),
                ("size", "number", false),
                ("w", "length", false),
                ("gap", "number", false),
                ("text-size", "number", false),
                ("line-h", "number", false),
                ("shape", "enum(auto|basic|advanced)", false),
                ("wrap", "enum(none|word|glyph|word-or-glyph)", false),
                ("font", "font", false),
                ("icon", "one-character-string", false),
                ("icon-size", "number", false),
                ("icon-line-h", "number", false),
                ("icon-shape", "enum(auto|basic|advanced)", false),
                ("style", "checkbox-preset|extern-call", false),
            ]),
        ),
        "toggler" => {
            let mut toggler = bool_control_properties();
            toggler.extend(properties(&[
                ("checked", "bool-expression", true),
                ("disabled", "bool-expression", false),
                ("align", "enum(default|left|center|right|justified)", false),
            ]));
            details(
                &["view"],
                "toggler <label-expression> [#<id>] checked=<bool-expression> [<property>=<value> ...] [@<semantic-utility> ...] -> <handler> _",
                child_shape(0, None, "optional-status-extension"),
                no_binding(),
                json!({ "required": true, "operator": "->", "payload": "bool", "placeholder": "_" }),
                toggler,
            )
        }
        "slider" => details(
            &["view"],
            "slider <value-expression> [#<id>] min=<expression> max=<expression> [<property>=<value> ...] [@<semantic-utility> ...] -> <handler> _",
            child_shape(0, None, "active|hovered|dragged-style"),
            no_binding(),
            json!({ "required": true, "operator": "->", "payload": "slider value", "placeholder": "_" }),
            properties(&[
                ("min", "number|extern-number", true),
                ("max", "number|extern-number", true),
                ("step", "number|extern-number", false),
                ("default", "number|extern-number", false),
                ("shift-step", "number|extern-number", false),
                ("w", "length", false),
                ("h", "length", false),
                ("vertical", "flag", false),
                ("release", "route", false),
                ("style", "extern-call", false),
            ]),
        ),
        "progress" => details(
            &["view"],
            "progress <value-expression> [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            leaf(),
            no_binding(),
            no_route(),
            properties(&[
                ("min", "number", false),
                ("max", "number", false),
                ("length", "length", false),
                ("girth", "length", false),
                ("vertical", "flag", false),
                ("style", "progress-preset|extern-call", false),
                ("bg", "background", false),
                ("bar", "background", false),
                ("border", "color-token", false),
                ("border-w", "number", false),
                ("r", "number", false),
                ("r-tl", "number", false),
                ("r-tr", "number", false),
                ("r-br", "number", false),
                ("r-bl", "number", false),
            ]),
        ),
        "radio" => {
            let mut radio = bool_control_properties();
            radio.extend(properties(&[
                ("value", "expression", true),
                ("selected", "bool-expression", true),
            ]));
            details(
                &["view"],
                "radio <label-expression> [#<id>] value=<expression> selected=<bool-expression> [<property>=<value> ...] [@<semantic-utility> ...] -> <handler> _",
                child_shape(0, None, "active|hovered selected|unselected style"),
                no_binding(),
                json!({ "required": true, "operator": "->", "payload": "radio value", "placeholder": "_" }),
                radio,
            )
        }
        "pick" => {
            let mut pick = selection_properties();
            pick.push(property("hint", "expression", false));
            details(
                &["view"],
                "pick <options-expression> <selected-expression> [#<id>] [<property>=<value> ...] -> <handler> _",
                child_shape(0, None, "field-status|menu|handle"),
                no_binding(),
                json!({ "required": true, "operator": "->", "payload": "selected value", "placeholder": "_" }),
                pick,
            )
        }
        "combo" => {
            let mut combo = selection_properties();
            combo.extend(properties(&[
                ("input", "payload-route(str)", false),
                ("hover", "payload-route(option)", false),
            ]));
            details(
                &["view"],
                "combo <state> <selected-expression> <placeholder-string> [#<id>] [<property>=<value> ...] -> <handler> _",
                child_shape(0, None, "input-status|menu|icon"),
                json!({ "required": true, "name": "state", "source": "combo state" }),
                json!({ "required": true, "operator": "->", "payload": "selected value", "placeholder": "_" }),
                combo,
            )
        }
        "rule" => details(
            &["view"],
            "rule <horizontal|vertical> [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            leaf(),
            no_binding(),
            no_route(),
            properties(&[
                ("thickness", "number", false),
                ("style", "enum(default|weak)", false),
                ("fill", "full|percent(number)|pad(u16[,u16])", false),
                ("color", "color-token", false),
                ("r", "number", false),
                ("r-tl", "number", false),
                ("r-tr", "number", false),
                ("r-br", "number", false),
                ("r-bl", "number", false),
                ("snap", "bool-expression", false),
            ]),
        ),
        "qr" => details(
            &["view"],
            "qr <declared-data-name> [#<id>] [<property>=<value> ...]",
            leaf(),
            no_binding(),
            no_route(),
            properties(&[
                ("cell-size", "number", false),
                ("size", "number", false),
                ("cell", "color-token", false),
                ("bg", "color-token", false),
            ]),
        ),
        "space" => details(
            &["view"],
            "space [#<id>] [w=<length>] [h=<length>] [@<semantic-utility> ...]",
            leaf(),
            no_binding(),
            no_route(),
            properties(&[("w", "length", false), ("h", "length", false)]),
        ),
        "markdown" => details(
            &["view"],
            "markdown <content-state> [#<id>] [<property>=<value> ...] -> <handler> _",
            child_shape(0, Some(1), "style"),
            no_binding(),
            json!({ "required": true, "operator": "->", "payload": "clicked URI", "placeholder": "_" }),
            properties(&[
                ("text-size", "number", false),
                ("h1-size", "number", false),
                ("h2-size", "number", false),
                ("h3-size", "number", false),
                ("h4-size", "number", false),
                ("h5-size", "number", false),
                ("h6-size", "number", false),
                ("code-size", "number", false),
                ("gap", "number", false),
                ("viewer", "extern-call", false),
            ]),
        ),
        "editor" => details(
            &["view"],
            "editor [#<id>] <-> <state> [<property>=<value> ...] [-> <handler> _]",
            child_shape(0, None, "input-status"),
            json!({ "required": true, "operator": "<->", "target": "editor state" }),
            json!({
                "requiredWhen": "key-binding is present",
                "forbiddenWhen": "key-binding is absent",
                "payload": "custom key binding message",
            }),
            properties(&[
                ("hint", "string", false),
                ("w", "number", false),
                ("h", "length", false),
                ("min-h", "number", false),
                ("max-h", "number", false),
                ("size", "number", false),
                ("line-h", "number", false),
                ("line-h-px", "number", false),
                ("p", "number", false),
                ("wrap", "enum(none|word|glyph|word-or-glyph)", false),
                ("font", "font", false),
                ("highlight", "string", false),
                ("highlight-theme", "highlight-theme", false),
                ("highlighter", "extern-call", false),
                ("key-binding", "extern-call", false),
                ("style", "extern-call", false),
                ("disabled", "bool-expression", false),
            ]),
        ),
        "table" => details(
            &["view"],
            "table <item> in <rows-expression> [#<id>] [<property>=<value> ...]",
            child_shape(1, None, "table-column"),
            json!({ "required": true, "name": "item", "source": "rows-expression" }),
            no_route(),
            properties(&[
                ("w", "length", false),
                ("p", "number", false),
                ("px", "number", false),
                ("py", "number", false),
                ("sep", "number", false),
                ("sep-x", "number", false),
                ("sep-y", "number", false),
            ]),
        ),
        "themer" => details(
            &["view"],
            "themer <declared-component>(<argument>, ...) [#<id>] [-> <handler> [_]]",
            leaf(),
            no_binding(),
            json!({ "required": false, "payload": "declared themer message" }),
            Vec::new(),
        ),
        "shader" => details(
            &["view"],
            "shader <declared-component>(<argument>, ...) [#<id>] [w=<length>] [h=<length>] [-> <handler> [_]]",
            leaf(),
            no_binding(),
            json!({ "required": false, "payload": "declared shader message" }),
            properties(&[("w", "length", false), ("h", "length", false)]),
        ),
        "image" => {
            let mut image = properties(&[
                ("label", "str-expression", false),
                ("w", "length", false),
                ("h", "length", false),
                (
                    "fit",
                    "enum(contain|cover|fill|none|scale-down)|expression",
                    false,
                ),
                ("rotate", "rotation", false),
                ("opacity", "number", false),
                ("filter", "enum(linear|nearest)", false),
                ("scale", "number", false),
                ("expand", "bool-expression", false),
                ("r", "number", false),
                ("r-tl", "number", false),
                ("r-tr", "number", false),
                ("r-br", "number", false),
                ("r-bl", "number", false),
                ("crop", "tuple(i64,i64,i64,i64)", false),
            ]);
            image.insert(
                1,
                json!({
                    "name": "description",
                    "type": "str-expression",
                    "required": false,
                    "forbiddenWhen": "label is absent",
                }),
            );
            details(
                &["view"],
                "image <source-expression> [#<id>] [<property>=<value> ...]",
                leaf(),
                no_binding(),
                no_route(),
                image,
            )
        }
        "svg" | "viewer" => details(
            &["view"],
            &format!(
                "{} <source-expression> [#<id>] [<property>=<value> ...]",
                item.label
            ),
            leaf(),
            no_binding(),
            no_route(),
            media_properties(item.label),
        ),
        "tooltip" => {
            let mut tooltip = properties(&[
                ("position", "enum(top|bottom|left|right|cursor)", false),
                ("gap", "number", false),
                ("p", "number", false),
                ("delay", "i64", false),
                ("snap", "bool-expression", false),
                ("style", "tooltip-preset|extern-call", false),
            ]);
            tooltip.extend(surface_properties());
            details(
                &["view"],
                "tooltip [#<id>] [<property>=<value> ...]",
                child_shape(2, Some(2), "content|tip"),
                no_binding(),
                no_route(),
                tooltip,
            )
        }
        "mouse" => details(
            &["view"],
            "mouse [#<id>] <event-or-cursor-property> [<property>=<value> ...]",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            json!({
                "required": false,
                "requiredOneOfProperties": ["press", "release", "double", "right_press", "right_release", "middle_press", "middle_release", "enter", "move", "scroll", "exit", "cursor"],
            }),
            properties(&[
                ("press", "route", false),
                ("release", "route", false),
                ("double", "route", false),
                ("right_press", "route", false),
                ("right_release", "route", false),
                ("middle_press", "route", false),
                ("middle_release", "route", false),
                ("enter", "route", false),
                ("move", "payload-route(x,y)", false),
                ("scroll", "payload-route(x,y,pixels)", false),
                ("exit", "route", false),
                ("cursor", "mouse-interaction|expression", false),
            ]),
        ),
        "resize-handle" => details(
            &["view"],
            "resize-handle [#<id>] drag=<payload-route(dx,dy)> [<property>=<value> ...]",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            json!({ "required": true, "property": "drag", "payload": ["dx", "dy"] }),
            properties(&[
                ("drag", "payload-route(dx,dy)", true),
                ("press", "route", false),
                ("release", "route", false),
                ("cursor", "mouse-interaction", false),
            ]),
        ),
        "canvas" => details(
            &["view"],
            "canvas [#<id>] [<property>=<value> ...]",
            child_shape(0, None, "state|canvas-command|canvas-event"),
            no_binding(),
            json!({ "required": false, "properties": ["press", "release", "right_press", "right_release", "middle_press", "middle_release", "enter", "move", "scroll", "exit"] }),
            properties(&[
                ("w", "length", false),
                ("h", "length", false),
                ("cache", "expression", false),
                ("cache-group", "identifier", false),
                ("capture", "bool-expression", false),
                ("press", "payload-route(x,y)", false),
                ("release", "payload-route(x,y)", false),
                ("right_press", "payload-route(x,y)", false),
                ("right_release", "payload-route(x,y)", false),
                ("middle_press", "payload-route(x,y)", false),
                ("middle_release", "payload-route(x,y)", false),
                ("enter", "route", false),
                ("move", "payload-route(x,y)", false),
                ("scroll", "payload-route(x,y,pixels)", false),
                ("exit", "route", false),
                ("cursor", "mouse-interaction|expression", false),
                ("cursor-outside", "bool-expression", false),
            ]),
        ),
        "theme" => details(
            &["view"],
            "theme [#<id>] [<default|app|built-in-theme|extern-call>] [fg=<color-token>] [bg=<background>]",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            no_route(),
            properties(&[("fg", "color-token", false), ("bg", "background", false)]),
        ),
        "float" => {
            let mut float = properties(&[
                ("scale", "number", false),
                ("x", "number", false),
                ("y", "number", false),
            ]);
            float.extend(properties(&[
                ("shadow", "color-token", false),
                ("shadow-x", "number", false),
                ("shadow-y", "number", false),
                ("shadow-blur", "number", false),
                ("r", "number", false),
                ("r-tl", "number", false),
                ("r-tr", "number", false),
                ("r-br", "number", false),
                ("r-bl", "number", false),
            ]));
            details(
                &["view"],
                "float [#<id>] [<property>=<value> ...]",
                child_shape(1, Some(1), "view-root"),
                no_binding(),
                no_route(),
                float,
            )
        }
        "pin" => details(
            &["view"],
            "pin [#<id>] [w=<length>] [h=<length>] [x=<number>] [y=<number>]",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            no_route(),
            properties(&[
                ("w", "length", false),
                ("h", "length", false),
                ("x", "number", false),
                ("y", "number", false),
            ]),
        ),
        "sensor" => details(
            &["view"],
            "sensor [#<id>] <show|resize|hide>=<route> [<property>=<value> ...]",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            json!({ "required": true, "oneOfProperties": ["show", "resize", "hide"] }),
            properties(&[
                ("show", "payload-route(width,height)", false),
                ("resize", "payload-route(width,height)", false),
                ("hide", "route", false),
                ("key", "expression", false),
                ("anticipate", "number", false),
                ("delay", "i64", false),
            ]),
        ),
        "responsive" => details(
            &["view"],
            "responsive [#<id>] (at=<number> | size=(<width-name>, <height-name>)) [w=<length>] [h=<length>]",
            json!({
                "min": 1,
                "max": 2,
                "role": "view-root",
                "condition": "at= requires narrow then wide; size= requires one child",
            }),
            json!({ "requiredWhen": "size= is used", "names": ["width", "height"], "source": "responsive bounds" }),
            no_route(),
            properties(&[
                ("at", "number", false),
                ("size", "tuple(identifier,identifier)", false),
                ("w", "length", false),
                ("h", "length", false),
            ]),
        ),
        "run" => details(
            &["handler-statement"],
            "run <extern-future>(<args>) -> <success-handler> _ [| <failure-handler> _]",
            leaf(),
            no_binding(),
            json!({
                "required": true,
                "operator": "->",
                "success": { "required": true, "payload": "extern output" },
                "failure": {
                    "payload": "extern error",
                    "requiredWhen": "extern declaration has `! <error-type>`",
                    "forbiddenWhen": "extern declaration has no error type"
                }
            }),
            Vec::new(),
        ),
        "<->" => details(
            &["binding-position"],
            "<-> <state-identifier>",
            leaf(),
            json!({ "required": true, "operator": "<->", "target": "state-identifier" }),
            no_route(),
            Vec::new(),
        ),
        "->" => details(
            &["route-position"],
            "-> <handler> [<payload-expression>]",
            leaf(),
            no_binding(),
            json!({ "required": true, "operator": "->", "payload": "expression|_" }),
            Vec::new(),
        ),
        "~=" => details(
            &["test-expectation"],
            "<numeric-expression> ~= <numeric-expression>",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "_" => details(
            &["route-payload"],
            "_",
            leaf(),
            no_binding(),
            json!({ "placeholder": true, "meaning": "forward emitted payload" }),
            Vec::new(),
        ),
        "#id" => details(
            &["view-node-id", "test-target"],
            "#<scoped-id> | #<scoped-id>(<key-expression>)",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        _ => unreachable!("every completion is an Ice Core construct"),
    };
    let mut object = json!({
        "label": item.label,
        "category": item.category,
        "insertText": item.insert_text,
        "canonical": true,
    });
    object
        .as_object_mut()
        .expect("construct schema is an object")
        .extend(
            shape
                .as_object()
                .expect("construct details are an object")
                .clone(),
        );
    object
}

fn style_contract() -> Value {
    json!({
        "utilitySyntax": "forms omit the leading `@` marker",
        "recipes": {
            "declaration": "recipe <name> for <target> [extends <base>]",
            "use": "@<name>",
            "targets": ["col", "row", "flex", "grid", "stack", "box", "text", "input", "button"],
            "expansion": "the optional same-target base expands first, then child utility tokens expand in place",
            "precedence": "later utilities win; direct typed properties override recipe defaults",
            "composition": { "bases": 1, "sameTarget": true, "cycles": false },
        },
        "statusCascade": {
            "base": "active fields apply to every native interaction status",
            "checked": "checked/selected statuses inherit their matching active checked/unchecked or selected/unselected fields",
            "compound": {
                "focused-hovered": ["active", "focused", "focused-hovered"],
                "opened-hovered": ["active", "opened", "opened-hovered"],
            },
            "precedence": "later, more-specific fields override inherited fields",
        },
        "patternNotation": {
            "N": "unsigned integer multiplied by four pixels",
            "TOKEN": "checked semantic theme token",
        },
        "utilities": {
            "size": [
                {
                    "targets": ["row", "col", "flex", "grid", "stack", "box"],
                    "forms": ["w-full", "h-full", "max-w-sm", "max-w-md", "max-w-lg", "max-w-xl", "max-w-2xl"],
                },
                {
                    "targets": ["row", "col", "grid", "stack", "box"],
                    "forms": ["self-center"],
                },
            ],
            "inputSize": { "targets": ["input"], "forms": ["w-full"] },
            "spacing": {
                "targets": ["row", "col", "flex", "grid", "stack", "box", "input", "button"],
                "patterns": ["p-N", "px-N", "py-N"],
            },
            "gap": {
                "targets": ["row", "col", "flex", "grid", "stack"],
                "patterns": ["gap-N"],
            },
            "alignment": { "targets": ["row", "col", "flex"], "forms": ["items-center"] },
            "overflow": { "targets": ["row", "col", "flex", "grid", "stack", "box"], "forms": ["overflow-hidden"] },
            "text": {
                "targets": ["text"],
                "forms": ["text-xs", "text-sm", "text-base", "text-lg", "text-xl", "text-2xl", "leading-tight", "leading-snug", "leading-normal", "leading-relaxed", "font-mono", "font-medium", "font-semibold", "font-bold"],
            },
            "semantic": ["bg-TOKEN", "text-TOKEN", "border-TOKEN", "border", "border-2", "rounded-*", "state variants"],
            "rule": "utilities and recipes are target-specific; direct typed properties override recipe defaults but conflict with direct utilities that own the same field",
        },
    })
}

fn test_contract() -> Value {
    json!({
        "declaration": { "syntax": "test <snake_case_name>", "name": "graph-global and unique" },
        "cargoCommand": "cargo ice test",
        "generatedHarness": "ordinary cfg(test) Rust tests",
        "configuration": {
            "ordering": "configuration and target aliases may be mixed, but both precede executable statements",
            "preset": { "syntax": "preset <name>", "required": false, "maxOccurrences": 1, "default": "normal application boot" },
            "viewport": { "syntax": "viewport <positive-number> <positive-number>", "required": false, "maxOccurrences": 1, "units": "logical pixels" },
            "timeout": { "syntax": "timeout <positive-integer><ms|s>", "required": false, "maxOccurrences": 1, "default": "2s" },
            "mount": { "syntax": "mount", "required": false, "maxOccurrences": 1, "children": { "min": 1, "max": 1, "role": "view-root" }, "default": "complete app view" },
        },
        "targets": {
            "declaration": "target <name> = #<scoped-id> | <earlier-alias>/<descendant-id>",
            "aliasNames": "unique within one test",
            "references": ["alias identifier", "direct #scoped-id"],
            "aliases": "selectors resolved again after every rerender",
            "relative": "an earlier alias may prefix a descendant path; it expands to the same checked selector as the corresponding absolute path",
            "componentCallIds": "scopes, not rendered nodes; select an identified rendered descendant",
            "nonRenderedNodes": ["if", "for", "slot"],
            "dynamicKeys": "checked with the normal widget-target key rules",
            "directIdNodes": [
                "row", "col", "flex", "grid", "stack", "scroll", "box", "overlay", "panes", "text", "rich-text", "input", "button", "checkbox", "toggler", "slider", "progress", "radio", "pick", "combo", "rule", "qr", "space", "keyed", "lazy", "markdown", "editor", "table", "extern", "themer", "shader", "image", "svg", "viewer", "tooltip", "mouse", "resize-handle", "canvas", "theme", "float", "pin", "sensor", "responsive"
            ],
        },
        "interactions": {
            "click": "click <target>",
            "hover": "hover <target>",
            "press": "press <target>",
            "release": "release",
            "type": "type <str-expression>",
            "key": "key <enter|escape|tab|backspace>",
            "resize": "resize <number-expression> <number-expression>",
            "dispatch": "dispatch <handler> | dispatch <handler>(<argument>, ...)",
        },
        "assertions": {
            "boolean": "expect <bool-expression>",
            "approximate": { "syntax": "expect <numeric-expression> ~= <numeric-expression>", "absoluteTolerance": 0.001 },
            "presence": ["expect exists <target>", "expect missing <target>"],
            "text": ["expect text <str-expression> [within <target>]", "expect no text <str-expression> [within <target>]"],
        },
        "targetFields": [
            { "name": "kind", "type": "str" },
            { "name": "value", "type": "str" },
            { "name": "visible", "type": "bool" },
            { "name": "x", "type": "f64" },
            { "name": "y", "type": "f64" },
            { "name": "width", "type": "f64" },
            { "name": "height", "type": "f64" },
            { "name": "left", "type": "f64" },
            { "name": "top", "type": "f64" },
            { "name": "right", "type": "f64" },
            { "name": "bottom", "type": "f64" },
            { "name": "center_x", "type": "f64" },
            { "name": "center_y", "type": "f64" },
            { "name": "visible_x", "type": "f64" },
            { "name": "visible_y", "type": "f64" },
            { "name": "visible_width", "type": "f64" },
            { "name": "visible_height", "type": "f64" },
            { "name": "content_x", "type": "f64" },
            { "name": "content_y", "type": "f64" },
            { "name": "content_width", "type": "f64" },
            { "name": "content_height", "type": "f64" },
            { "name": "scroll_x", "type": "f64" },
            { "name": "scroll_y", "type": "f64" },
            { "name": "translation_x", "type": "f64" },
            { "name": "translation_y", "type": "f64" },
            { "name": "background", "type": "background" },
            { "name": "border", "type": "border", "members": { "color": "color", "width": "f64", "radius": "radius" } },
            { "name": "shadow", "type": "shadow" },
            { "name": "text_color", "type": "color" },
            { "name": "text_size", "type": "f64" },
            { "name": "font", "type": "font" },
            { "name": "line_height", "type": "text-line-height" },
        ],
        "execution": {
            "driver": "::ui_lang_runtime::testing::Driver",
            "state": "fresh per test",
            "uiCache": "persistent across rerenders within one test",
            "settling": "widget messages, updates, real tasks, and recursively emitted messages drain after each executable step",
            "externs": "real sync and task Rust extern implementations",
            "subscriptions": "active under the generated program contract",
            "mockLayer": false,
        },
        "inspection": {
            "layout": "post-layout selector candidate bounds in logical pixels",
            "paint": "structured tiny-skia draw commands from a real redraw",
            "surfaceMatch": "one quad whose bounds equal the target",
            "textMatch": "one visible text primitive within the target",
            "ambiguousMatch": "runtime failure; never guessed",
            "customRendererPaint": false,
        },
        "legacyIcedTestIceSyntax": false,
        "nonGoals": ["DOM", "CSS selectors", "synthetic component bounds", "component-local state access", "DSL mocks", "virtual time", "pixel snapshots", "multi-window orchestration"],
    })
}

pub fn document() -> Value {
    let constructs = COMPLETIONS.iter().map(construct_schema).collect::<Vec<_>>();

    json!({
        "schemaVersion": 1,
        "language": {
            "name": "Ice",
            "revision": LANGUAGE_REVISION,
            "fileExtension": ".ice",
            "encoding": "UTF-8",
            "indent": "two spaces",
            "treeSyntax": "indentation",
        },
        "backend": {
            "iced": ICED_VERSION,
            "iced_widget": ICED_WIDGET_VERSION,
            "runtime": {
                "package": "ui-lang-runtime",
                "version": UI_LANG_RUNTIME_VERSION,
                "generatedRustPath": "::ui_lang_runtime",
                "publicApi": [
                    "accessible", "navigation", "snapshot", "Bridge", "Role", "StableId",
                    "testing::Location", "testing::Config", "testing::Driver", "testing::Target",
                    "testing::step",
                ],
                "testing": {
                    "module": "::ui_lang_runtime::testing",
                    "publicApi": ["Location", "Config", "Driver", "Target", "step"],
                    "legacyIcedTestIceApi": false,
                },
                "accesskit": ACCESSKIT_VERSION,
                "accesskit_unix": ACCESSKIT_UNIX_VERSION,
                "accesskit_unixTarget": "linux",
                "accesskit_windows": ACCESSKIT_WINDOWS_VERSION,
                "accesskit_windowsTarget": "windows",
            },
            "compatibilityCommand": "cargo ice compat",
        },
        "lsp": {
            "transport": "stdio Content-Length framing",
            "diagnostics": {
                "supported": true,
                "source": "ui_lang_core::analyze_file_with_overlays for existing file URIs; ui_lang_core::analyze otherwise",
                "inMemory": true,
                "rootBufferOverlay": true,
                "diskImports": true,
                "importedBufferOverlays": true,
                "diskFallbackOnClose": true,
                "ownership": "app roots own reports; reports are aggregated by diagnostic URI; fragments are not analyzed as standalone apps",
                "scope": "all open app roots and their overlaid import graphs",
                "reanalyze": "all open app roots after any open, change, or close",
            },
            "formatting": {
                "supported": true,
                "source": "ui_lang_core::format_fragment",
                "wholeDocument": true,
            },
            "completion": {
                "supported": true,
                "source": "core.constructs",
                "contextAware": false,
            },
            "definition": {
                "supported": true,
                "symbols": ["component", "handler", "recipe", "test-target"],
                "componentLocalHandlers": false,
                "testTargetScope": "one test declaration; alias names may repeat in other tests",
                "crossFile": true,
                "source": "checked reference spans and imported source origins",
            },
            "rename": {
                "supported": true,
                "prepare": true,
                "symbols": ["component", "handler", "recipe", "test-target"],
                "componentLocalHandlers": false,
                "testTargetScope": "definition, references, and collision checks stay inside one test declaration",
                "componentRule": "plain names and compound-family roots; a root rename cascades to dotted descendants",
                "definitionOnly": ["dotted component descendants", "mount handler"],
                "completeReferencesOnly": true,
                "declarationCollisionCheck": true,
                "allWorkspaceAppRootsMustCheck": true,
                "workspaceRootRequiredForImportedSymbols": true,
                "openBufferOverlays": true,
            },
        },
        "core": {
            "frozenAt": LANGUAGE_REVISION,
            "generative": true,
            "componentProps": {
                "read": {
                    "declaration": "<name>:<type>",
                    "argument": "<name>=<expression>",
                    "writable": false,
                },
                "bind": {
                    "declaration": "bind <name>:<type>",
                    "argument": "<name><-><state>",
                    "writable": true,
                    "sources": ["app state", "component-local state", "another bind prop"],
                    "directPathOnly": true,
                },
            },
            "componentEvents": {
                "declaration": "emits block with unique zero-or-more typed ordered payloads",
                "emission": "emit <event> [<value>|_ ...] inside the component view",
                "routing": "events block with exactly one caller-scoped route per declared event",
                "closedComponents": true,
                "defaultEventShorthand": "component Name(...) -> Type paired with call-site -> route",
            },
            "documentPrelude": {
                "syntax": "app <Name>\ntheme\n  bg <color>\n  fg <color>\n  primary <color>\n  danger <color>",
                "requiredDeclarations": ["app", "theme", "view"],
                "theme": {
                    "required": true,
                    "syntax": "theme",
                    "tokens": [
                        { "name": "bg", "type": "color", "required": true },
                        { "name": "fg", "type": "color", "required": true },
                        { "name": "primary", "type": "color", "required": true },
                        { "name": "danger", "type": "color", "required": true },
                    ],
                    "additionalTokens": true,
                },
            },
            "types": {
                "expression": "statically checked Ice expression",
                "bool-expression": "expression of bool",
                "str-expression": "expression of str",
                "number": "expression checked as a numeric value",
                "f64": "checked 64-bit floating-point value",
                "duration": "positive number followed by ms or s",
                "color": "#RRGGBB or #RRGGBBAA",
                "length": ["fill", "shrink", "fill(<u16>)", "<number-expression>"],
                "route": "<handler> [<payload-expression>|_]",
                "extern-call": "declared typed extern function call",
                "color-token": "declared theme token or checked color form",
                "background": "color token, color literal, or typed gradient",
                "font": ["default", "mono", "<declared-font>"],
                "text-line-height": "native relative or absolute line-height value",
                "border": "typed border with color, width, and radius fields",
                "radius": "typed per-corner radius",
                "shadow": "typed color, offset, and blur shadow",
                "test-target": "checked runtime selector available only inside test declarations",
            },
            "constructs": constructs,
            "components": {
                "prop": "<name>:<type>[=<default-expression>]",
                "defaults": {
                    "optional": true,
                    "expression": "pure checked expression closed over no app state, component state, or parameters",
                    "mutableValues": false,
                    "requiredAfterDefault": true,
                    "callRule": "a missing named argument uses its declared default"
                }
            },
            "style": style_contract(),
            "testMode": test_contract(),
        },
    })
}

pub fn completion_items() -> Vec<Value> {
    COMPLETIONS
        .iter()
        .map(|item| {
            let kind = match item.category {
                "operator" => 24,
                "layout" | "widget" => 15,
                _ => 14,
            };
            json!({
                "label": item.label,
                "kind": kind,
                "detail": format!("Ice Core {}", item.category),
                "insertText": item.insert_text,
                "insertTextFormat": 2,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        ACCESSKIT_WINDOWS_VERSION, COMPLETIONS, ICED_VERSION, ICED_WIDGET_VERSION,
        UI_LANG_RUNTIME_VERSION, completion_items, document,
    };
    use serde_json::json;
    use std::collections::BTreeSet;

    #[test]
    fn schema_drives_completion_and_records_capability_gaps() {
        let schema = document();
        let constructs = schema["core"]["constructs"].as_array().unwrap();
        let completions = completion_items();

        assert_eq!(schema["backend"]["iced"], ICED_VERSION);
        assert_eq!(schema["backend"]["iced_widget"], ICED_WIDGET_VERSION);
        assert_eq!(
            schema["backend"]["runtime"]["version"],
            UI_LANG_RUNTIME_VERSION
        );
        assert_eq!(
            schema["backend"]["runtime"]["accesskit_windows"],
            ACCESSKIT_WINDOWS_VERSION
        );
        assert_eq!(
            schema["backend"]["runtime"]["accesskit_windowsTarget"],
            "windows"
        );
        assert_eq!(constructs.len(), COMPLETIONS.len());
        assert_eq!(completions.len(), COMPLETIONS.len());
        for (construct, completion) in constructs.iter().zip(&completions) {
            assert_eq!(construct["label"], completion["label"]);
            assert_eq!(construct["insertText"], completion["insertText"]);
            assert_eq!(completion["insertTextFormat"], 2);
        }
        assert_eq!(schema["lsp"]["definition"]["supported"], true);
        assert_eq!(schema["lsp"]["definition"]["componentLocalHandlers"], false);
        assert!(
            schema["lsp"]["definition"]["symbols"]
                .as_array()
                .unwrap()
                .contains(&json!("test-target"))
        );
        assert_eq!(schema["lsp"]["rename"]["supported"], true);
        assert_eq!(schema["lsp"]["rename"]["completeReferencesOnly"], true);
        assert_eq!(
            schema["lsp"]["rename"]["definitionOnly"],
            json!(["dotted component descendants", "mount handler"])
        );
    }

    #[test]
    fn generative_core_matches_the_contract_boundary() {
        const CORE_CONTRACT: &[&str] = &[
            "app",
            "use",
            "recipe",
            "state",
            "derived",
            "component",
            "emits",
            "events",
            "slot",
            "on",
            "let",
            "view",
            "test",
            "preset",
            "viewport",
            "timeout",
            "mount",
            "target",
            "click",
            "hover",
            "press",
            "release",
            "type",
            "key",
            "resize",
            "dispatch",
            "expect",
            "if",
            "match",
            "for",
            "keyed",
            "lazy",
            "row",
            "col",
            "flex",
            "grid",
            "stack",
            "scroll",
            "box",
            "overlay",
            "panes",
            "text",
            "rich-text",
            "input",
            "button",
            "checkbox",
            "toggler",
            "slider",
            "progress",
            "radio",
            "pick",
            "combo",
            "rule",
            "qr",
            "space",
            "markdown",
            "editor",
            "table",
            "themer",
            "shader",
            "image",
            "svg",
            "viewer",
            "tooltip",
            "mouse",
            "resize-handle",
            "canvas",
            "theme",
            "float",
            "pin",
            "sensor",
            "responsive",
            "<->",
            "->",
            "~=",
            "_",
            "#id",
            "extern",
            "run",
        ];
        let schema = document();
        let constructs = schema["core"]["constructs"].as_array().unwrap();
        let actual = constructs
            .iter()
            .map(|construct| construct["label"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        let expected = CORE_CONTRACT.iter().copied().collect::<BTreeSet<_>>();

        assert_eq!(schema["core"]["generative"], true);
        assert_eq!(schema["core"]["componentProps"]["bind"]["writable"], true);
        assert_eq!(actual, expected);
        for construct in constructs {
            assert!(!construct["contexts"].as_array().unwrap().is_empty());
            assert!(!construct["syntax"].as_str().unwrap().is_empty());
            assert!(construct["children"].is_object());
            for property in construct["properties"].as_array().unwrap() {
                assert!(property["name"].is_string());
                assert!(property["type"].is_string());
                assert!(property["required"].is_boolean());
            }
        }
        let find = |label| {
            constructs
                .iter()
                .find(|construct| construct["label"] == label)
                .unwrap()
        };
        for label in [
            "row", "col", "stack", "scroll", "box", "text", "input", "button", "checkbox", "image",
        ] {
            assert!(!find(label)["properties"].as_array().unwrap().is_empty());
        }
        assert_eq!(find("view")["children"]["min"], 1);
        assert_eq!(find("recipe")["children"]["min"], 1);
        assert_eq!(find("scroll")["children"]["max"], json!(null));
        assert!(
            find("scroll")["children"]["condition"]
                .as_str()
                .unwrap()
                .contains("exactly one view root")
        );
        assert_eq!(find("input")["binding"]["operator"], "<->");
        assert!(
            find("input")["insertText"]
                .as_str()
                .unwrap()
                .contains("${1:Label}")
        );
        assert!(
            find("input")["syntax"]
                .as_str()
                .unwrap()
                .contains("\"<label>\"")
        );
        assert_eq!(find("button")["route"]["required"], true);
        assert_eq!(
            find("run")["route"]["failure"]["requiredWhen"],
            "extern declaration has `! <error-type>`"
        );
        assert_eq!(
            find("run")["route"]["failure"]["forbiddenWhen"],
            "extern declaration has no error type"
        );
        assert!(
            find("extern")["syntax"]
                .as_str()
                .unwrap()
                .contains("<type>")
        );
    }

    #[test]
    fn style_utilities_are_target_scoped() {
        let schema = document();
        let styles = &schema["core"]["style"];
        assert_eq!(
            styles["recipes"]["targets"],
            serde_json::json!([
                "col", "row", "flex", "grid", "stack", "box", "text", "input", "button"
            ])
        );
        assert_eq!(styles["recipes"]["composition"]["bases"], 1);
        assert_eq!(schema["core"]["components"]["defaults"]["optional"], true);
        assert_eq!(
            schema["core"]["components"]["defaults"]["requiredAfterDefault"],
            true
        );
        assert!(
            styles["utilities"]["rule"]
                .as_str()
                .unwrap()
                .contains("target-specific")
        );
    }

    #[test]
    fn test_mode_schema_covers_configuration_actions_assertions_and_inspection() {
        let schema = document();
        let constructs = schema["core"]["constructs"].as_array().unwrap();
        let labels = constructs
            .iter()
            .map(|construct| construct["label"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        for label in [
            "test", "preset", "viewport", "timeout", "mount", "target", "click", "hover", "press",
            "release", "type", "key", "resize", "dispatch", "expect", "~=",
        ] {
            assert!(labels.contains(label), "{label}");
        }

        let contract = &schema["core"]["testMode"];
        assert_eq!(contract["cargoCommand"], "cargo ice test");
        assert_eq!(
            schema["backend"]["runtime"]["testing"]["module"],
            "::ui_lang_runtime::testing"
        );
        assert_eq!(
            schema["backend"]["runtime"]["testing"]["legacyIcedTestIceApi"],
            false
        );
        assert_eq!(contract["configuration"]["mount"]["children"]["max"], 1);
        assert_eq!(
            contract["configuration"]["timeout"]["syntax"],
            "timeout <positive-integer><ms|s>"
        );
        assert_eq!(
            contract["interactions"]["dispatch"],
            "dispatch <handler> | dispatch <handler>(<argument>, ...)"
        );
        assert_eq!(
            contract["assertions"]["approximate"]["absoluteTolerance"],
            0.001
        );
        assert_eq!(contract["legacyIcedTestIceSyntax"], false);
        assert_eq!(
            contract["targets"]["directIdNodes"],
            json!([
                "row",
                "col",
                "flex",
                "grid",
                "stack",
                "scroll",
                "box",
                "overlay",
                "panes",
                "text",
                "rich-text",
                "input",
                "button",
                "checkbox",
                "toggler",
                "slider",
                "progress",
                "radio",
                "pick",
                "combo",
                "rule",
                "qr",
                "space",
                "keyed",
                "lazy",
                "markdown",
                "editor",
                "table",
                "extern",
                "themer",
                "shader",
                "image",
                "svg",
                "viewer",
                "tooltip",
                "mouse",
                "resize-handle",
                "canvas",
                "theme",
                "float",
                "pin",
                "sensor",
                "responsive"
            ])
        );
        assert_eq!(
            contract["targets"]["nonRenderedNodes"],
            json!(["if", "for", "slot"])
        );
        assert!(
            contract["targets"]["componentCallIds"]
                .as_str()
                .unwrap()
                .contains("scopes, not rendered nodes")
        );
        let fields = contract["targetFields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|field| field["name"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            fields,
            [
                "kind",
                "value",
                "visible",
                "x",
                "y",
                "width",
                "height",
                "left",
                "top",
                "right",
                "bottom",
                "center_x",
                "center_y",
                "visible_x",
                "visible_y",
                "visible_width",
                "visible_height",
                "content_x",
                "content_y",
                "content_width",
                "content_height",
                "scroll_x",
                "scroll_y",
                "translation_x",
                "translation_y",
                "background",
                "border",
                "shadow",
                "text_color",
                "text_size",
                "font",
                "line_height",
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            contract["targetFields"]
                .as_array()
                .unwrap()
                .iter()
                .find(|field| field["name"] == "line_height")
                .unwrap()["type"],
            "text-line-height"
        );
    }

    #[test]
    fn direct_id_nodes_are_canonical_constructs_and_completions() {
        let schema = document();
        let constructs = schema["core"]["constructs"].as_array().unwrap();
        let completions = completion_items();
        let targets = &schema["core"]["testMode"]["targets"];
        let direct = targets["directIdNodes"].as_array().unwrap();

        for label in direct.iter().map(|label| label.as_str().unwrap()) {
            let matching = constructs
                .iter()
                .filter(|construct| construct["label"] == label)
                .collect::<Vec<_>>();
            assert_eq!(
                matching.len(),
                1,
                "{label} must have one canonical construct"
            );
            let construct = matching[0];
            assert_eq!(construct["canonical"], true, "{label}");
            assert!(
                construct["contexts"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("view")),
                "{label} must be a view construct"
            );
            let syntax = construct["syntax"].as_str().unwrap();
            if label == "panes" {
                assert!(syntax.contains("panes #<id>"), "{label}: {syntax}");
                assert!(!syntax.contains("panes [#<id>]"), "{label}: {syntax}");
            } else {
                assert!(syntax.contains("[#<id>]"), "{label}: {syntax}");
            }
            assert_eq!(
                completions
                    .iter()
                    .filter(|completion| completion["label"] == label)
                    .count(),
                1,
                "{label} must have one completion"
            );
        }

        for label in targets["nonRenderedNodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|label| label.as_str().unwrap())
        {
            assert!(!direct.contains(&json!(label)), "{label}");
            let construct = constructs
                .iter()
                .find(|construct| construct["label"] == label)
                .unwrap();
            assert!(
                !construct["syntax"].as_str().unwrap().contains("#<id>"),
                "{label} must remain non-rendered"
            );
        }
    }

    #[test]
    fn prelude_and_accessibility_schema_match_accepted_source() {
        let schema = document();
        let tokens = schema["core"]["documentPrelude"]["theme"]["tokens"]
            .as_array()
            .unwrap();
        assert_eq!(
            tokens
                .iter()
                .map(|token| token["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["bg", "fg", "primary", "danger"]
        );

        let constructs = schema["core"]["constructs"].as_array().unwrap();
        let find = |label| {
            constructs
                .iter()
                .find(|construct| construct["label"] == label)
                .unwrap()
        };
        for label in ["input", "button", "checkbox", "image"] {
            let names = find(label)["properties"]
                .as_array()
                .unwrap()
                .iter()
                .map(|property| property["name"].as_str().unwrap())
                .collect::<BTreeSet<_>>();
            assert!(names.contains("label"), "{label}");
            assert!(names.contains("description"), "{label}");
        }
        let button_label = find("button")["properties"]
            .as_array()
            .unwrap()
            .iter()
            .find(|property| property["name"] == "label")
            .unwrap();
        assert_eq!(
            button_label["requiredWhen"],
            "button uses child content instead of a string label"
        );
        let image_description = find("image")["properties"]
            .as_array()
            .unwrap()
            .iter()
            .find(|property| property["name"] == "description")
            .unwrap();
        assert_eq!(image_description["forbiddenWhen"], "label is absent");

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
    input "Name" label="Full name" description="Profile name" <-> name
    button label="Open help" description="Show help" -> press
      text "?"
    checkbox "Ready" label="Ready state" description="Current readiness" checked=checked -> toggle _
    image "photo.ppm" label="Portrait" description="Profile portrait"
"#;
        ui_lang_core::analyze(source).unwrap();
        let error = ui_lang_core::analyze(&source.replace("label=\"Open help\" ", "")).unwrap_err();
        assert_eq!(error.code, "E105");
        assert!(error.message.contains("child content"));
        let error = ui_lang_core::analyze(&source.replace("label=\"Portrait\" ", "")).unwrap_err();
        assert_eq!(error.code, "E105");
        assert!(
            error
                .message
                .contains("requires an accessibility `label=...`")
        );
    }
}
