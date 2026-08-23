use crate::evidence::{
    CAPTURE_DIFF_ARTIFACT_KIND, CAPTURE_SCHEMA_VERSION, REVIEW_ARTIFACT_KIND, REVIEW_SCHEMA_VERSION,
};
use serde_json::{Value, json};
use ui_lang_template::trace::{
    ARTIFACT_KIND as TRACE_ARTIFACT_KIND, GENERATOR_VERSION as TRACE_GENERATOR_VERSION,
    SCHEMA_VERSION as TRACE_SCHEMA_VERSION,
};

pub use ui_lang_core::LANGUAGE_REVISION;
pub const ICED_VERSION: &str = "0.14.0";
pub const ICED_WIDGET_VERSION: &str = "0.14.2";
pub const UI_LANG_BUILD_VERSION: &str = "0.1.0";
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
        "theme contract",
        "declaration",
        "theme contract ${1:Name}\n  bg\n  fg\n  primary\n  danger\n  ${2:surface}",
    ),
    Completion::new(
        "palette",
        "declaration",
        "palette ${1:light} for ${2:Name}\n  bg ${3:#ffffff}\n  fg ${4:#111111}\n  primary ${5:#3366ff}\n  danger ${6:#cc3344}",
    ),
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
    Completion::new("secret", "declaration", "secret ${1:name}"),
    Completion::new(
        "enum",
        "declaration",
        "enum ${1:Name}\n  ${2:idle}\n  ${3:ready}(${4:str})",
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
    Completion::new(
        "lifetime",
        "component declaration",
        "lifetime ${1|retained,mounted|}",
    ),
    Completion::new("slot", "declaration", "slot ${1:Name}"),
    Completion::new("with", "node metadata", "with\n  ${1:property}=${2:value}"),
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
    Completion::new(
        "test theme",
        "test configuration",
        "theme ${1|light,dark,none|}",
    ),
    Completion::new("scale", "test configuration", "scale ${1:1.0}"),
    Completion::new("locale", "test configuration", "locale \"${1:en-US}\""),
    Completion::new(
        "platform",
        "test configuration",
        "platform ${1|linux,windows,macos,wasm|}",
    ),
    Completion::new(
        "reduced-motion",
        "test configuration",
        "reduced-motion ${1|true,false|}",
    ),
    Completion::new("mount", "test configuration", "mount\n  $0"),
    Completion::new("target", "test statement", "target ${1:name} = #${2:id}"),
    Completion::new(
        "click",
        "test interaction",
        "click ${1:target}${2|, left, right, middle, back, forward|}",
    ),
    Completion::new(
        "double-click",
        "test interaction",
        "double-click ${1:target}${2|, left, right, middle, back, forward|}",
    ),
    Completion::new(
        "click-at",
        "test interaction",
        "click-at ${1:x} ${2:y}${3|, left, right, middle, back, forward|}",
    ),
    Completion::new("leave", "test interaction", "leave"),
    Completion::new("move", "test interaction", "move ${1:target}"),
    Completion::new(
        "press",
        "test interaction",
        "press ${1:target}${2|, left, right, middle, back, forward|}",
    ),
    Completion::new(
        "release",
        "test interaction",
        "release${1|, left, right, middle, back, forward|}",
    ),
    Completion::new(
        "wheel",
        "test interaction",
        "wheel ${1|pixels,lines|} ${2:x} ${3:y}",
    ),
    Completion::new(
        "scroll-to",
        "test interaction",
        "scroll-to ${1:target} ${2:x} ${3:y}",
    ),
    Completion::new(
        "scroll-by",
        "test interaction",
        "scroll-by ${1:target} ${2:x} ${3:y}",
    ),
    Completion::new("snap", "test interaction", "snap ${1:target} ${2:x} ${3:y}"),
    Completion::new("snap-end", "test interaction", "snap-end ${1:target}"),
    Completion::new(
        "drag",
        "test interaction",
        "drag ${1:source} ${2:destination}",
    ),
    Completion::new("drop", "test interaction", "drop ${1:target}"),
    Completion::new("focus", "test interaction", "focus ${1:target}"),
    Completion::new("focus-next", "test interaction", "focus-next"),
    Completion::new("focus-previous", "test interaction", "focus-previous"),
    Completion::new("blur", "test interaction", "blur"),
    Completion::new(
        "window focus",
        "test interaction",
        "window ${1|focus,blur|}",
    ),
    Completion::new(
        "window move",
        "test interaction",
        "window move ${1:x} ${2:y}",
    ),
    Completion::new(
        "window resize",
        "test interaction",
        "window resize ${1:width} ${2:height}",
    ),
    Completion::new(
        "window rescale",
        "test interaction",
        "window rescale ${1:factor}",
    ),
    Completion::new(
        "window lifecycle",
        "test interaction",
        "window ${1|close-request,opened,closed,redraw|}",
    ),
    Completion::new("type", "test interaction", "type ${1:\"text\"}"),
    Completion::new("clear", "test interaction", "clear"),
    Completion::new("replace", "test interaction", "replace ${1:\"text\"}"),
    Completion::new("select", "test interaction", "select ${1:start} ${2:end}"),
    Completion::new("select-all", "test interaction", "select-all"),
    Completion::new("cursor", "test interaction", "cursor ${1|front,end,0|}"),
    Completion::new("composition", "test interaction", "composition ${1:start}"),
    Completion::new("key", "test interaction", "key ${1:enter}"),
    Completion::new(
        "key-down",
        "test interaction",
        "key-down ${1:enter}${2: modified=enter}${3: location=standard}${4: physical=enter}${5: text=\"x\"}${6: repeat=false}",
    ),
    Completion::new(
        "key-up",
        "test interaction",
        "key-up ${1:enter}${2: modified=enter}${3: location=standard}${4: physical=enter}",
    ),
    Completion::new(
        "modifiers",
        "test interaction",
        "modifiers ${1|shift,control,alt,logo|}",
    ),
    Completion::new("chord", "test interaction", "chord ${1:control} ${2:key}"),
    Completion::new("repeat", "test interaction", "repeat ${1:key} ${2:count}"),
    Completion::new("tap", "test interaction", "tap ${1:target}${2: 1}"),
    Completion::new(
        "touch",
        "test interaction",
        "touch ${1|down,move,up,cancel|} ${2:id} ${3:x} ${4:y}",
    ),
    Completion::new(
        "system-theme",
        "test interaction",
        "system-theme ${1|light,dark,none|}",
    ),
    Completion::new("file-hover", "test interaction", "file-hover ${1:\"path\"}"),
    Completion::new("file-drop", "test interaction", "file-drop ${1:\"path\"}"),
    Completion::new("file-leave", "test interaction", "file-leave"),
    Completion::new("wait", "test interaction", "wait ${1:50ms}"),
    Completion::new("advance", "test interaction", "advance ${1:16ms}"),
    Completion::new("idle", "test interaction", "idle"),
    Completion::new("capture", "test interaction", "capture ${1:name}"),
    Completion::new(
        "a11y",
        "test interaction",
        "a11y ${1|activate,focus|} ${2:target}",
    ),
    Completion::new(
        "dispatch",
        "test interaction",
        "dispatch ${1:handler}(${2})",
    ),
    Completion::new("expect", "test assertion", "expect ${1:condition}"),
    Completion::new(
        "expect a11y",
        "test assertion",
        "expect a11y ${1:target} ${2|role,name,value,checked,expanded,disabled,focused,action|} ${3:value}",
    ),
    Completion::new(
        "expect component",
        "test assertion",
        "expect component ${1:target}.${2:field} == ${3:value}",
    ),
    Completion::new("if", "control", "if ${1:condition}\n  $0"),
    Completion::new(
        "match",
        "control",
        "match ${1:value}\n  ${2:some(value)}\n    $0",
    ),
    Completion::new("some", "expression/pattern", "some(${1:value})"),
    Completion::new("none", "expression/pattern", "none"),
    Completion::new("ok", "expression/pattern", "ok(${1:value})"),
    Completion::new("err", "expression/pattern", "err(${1:error})"),
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
        "run every",
        "effect",
        "run every ${1:action}(${2}) -> ${3:succeeded} _ | ${4:failed} _",
    ),
    Completion::new(
        "run latest",
        "effect",
        "run latest lane=${1:request} ${2:action}(${3}) -> ${4:succeeded} _ | ${5:failed} _",
    ),
    Completion::new(
        "run replace",
        "effect",
        "run replace lane=${1:request} ${2:action}(${3}) -> ${4:succeeded} _ | ${5:failed} _",
    ),
    Completion::new(
        "stream every",
        "effect",
        "stream every ${1:source}(${2}) -> ${3:succeeded} _ | ${4:failed} _",
    ),
    Completion::new(
        "stream replace",
        "effect",
        "stream replace lane=${1:stream} ${2:source}(${3}) -> ${4:succeeded} _ | ${5:failed} _",
    ),
    Completion::new("invalidate", "effect", "invalidate lane=${1:request}"),
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
    let test_configuration = |syntax: &str| {
        details(
            &["test-configuration"],
            syntax,
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        )
    };
    let test_statement = |syntax: &str| {
        details(
            &["test"],
            syntax,
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        )
    };
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
        "theme contract" => details(
            &["document"],
            "theme contract <Name>\n  <token> ...",
            child_shape(4, None, "theme-token-name"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "palette" => details(
            &["document"],
            "palette <name> for <ThemeContract>\n  <token> <#RRGGBB|#RRGGBBAA> ...",
            child_shape(4, None, "palette-token-color"),
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
            "extern <rust-path>\n  [pure|sync|task|component] <name>(<param>:<type>, ...) -> <type>[ ! <error-type>] | extern <declared-component>(<argument>, ...) [#<id>] [-> <handler> [_]]",
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
        "secret" => details(
            &["document"],
            "secret <name>",
            child_shape(0, Some(0), "leaf"),
            json!({
                "required": true,
                "name": "name",
                "source": "runtime secret buffer",
                "condition": "one `input` binds it; expressions may only ask `empty` or `len`",
            }),
            no_route(),
            Vec::new(),
        ),
        "enum" => details(
            &["document"],
            "enum <Name>\n  <variant>\n  <variant>(<cloneable-type>)",
            child_shape(1, None, "enum-variant"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "component" => details(
            &["document"],
            "component <Name>([bind] <prop>:<type>[=<default-expression>], ...) [-> <default-output-type>]",
            child_shape(
                1,
                None,
                "component-lifetime|component-state|component-events|component-boot|component-handler|view-root",
            ),
            no_binding(),
            json!({ "requiredWhen": "a default output type is declared", "payload": "default component output" }),
            Vec::new(),
        ),
        "lifetime" => details(
            &["component"],
            "lifetime retained|mounted",
            leaf(),
            no_binding(),
            no_route(),
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
            "slot [<Name>[?]]",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "with" => details(
            &["view-node-metadata"],
            "with\n  <property>=<expression> | @<utility>",
            child_shape(1, None, "property|utility"),
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
        "test theme" => test_configuration("theme <light|dark|none>"),
        "scale" => test_configuration("scale <positive-number>"),
        "locale" => test_configuration("locale <non-empty-string>"),
        "platform" => test_configuration("platform <linux|windows|macos|wasm>"),
        "reduced-motion" => test_configuration("reduced-motion <true|false>"),
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
        "click" => test_statement("click <target> [<mouse-button>]"),
        "double-click" => test_statement("double-click <target> [<mouse-button>]"),
        "click-at" => test_statement("click-at <x> <y> [<mouse-button>]"),
        "leave" => test_statement("leave"),
        "move" => test_statement("move <target> | move <x> <y>"),
        "press" => test_statement("press <target> [<mouse-button>]"),
        "release" => test_statement("release [<mouse-button>]"),
        "wheel" => test_statement("wheel [pixels|lines] <x> <y>"),
        "scroll-to" => test_statement("scroll-to <target> <x> <y>"),
        "scroll-by" => test_statement("scroll-by <target> <x> <y>"),
        "snap" => test_statement("snap <target> <x-unit-offset> <y-unit-offset>"),
        "snap-end" => test_statement("snap-end <target>"),
        "drag" => test_statement("drag <source-target> <destination-target>"),
        "drop" => test_statement("drop <target>"),
        "focus" => test_statement("focus <target>"),
        "focus-next" => test_statement("focus-next"),
        "focus-previous" => test_statement("focus-previous"),
        "blur" => test_statement("blur"),
        "window focus" => test_statement("window focus|blur"),
        "window move" => test_statement("window move <x> <y>"),
        "window resize" => test_statement("window resize <width> <height>"),
        "window rescale" => test_statement("window rescale <positive-factor>"),
        "window lifecycle" => test_statement("window close-request|opened|closed|redraw"),
        "type" => test_statement("type <str-expression>"),
        "clear" => test_statement("clear"),
        "replace" => test_statement("replace <str-expression>"),
        "select" => test_statement("select <start-index> <end-index>"),
        "select-all" => test_statement("select-all"),
        "cursor" => test_statement("cursor <index>|front|end"),
        "composition" => test_statement(
            "composition start|cancel | composition update <str-expression> [<selection-start> <selection-end>] | composition commit <str-expression>",
        ),
        "key" => test_statement("key <kebab-key|exact-Iced-variant|string>"),
        "key-down" => test_statement(
            "key-down <key> [modified=<key>] [location=...] [physical=...] [text=\"<non-empty>\"] [repeat=true|false]",
        ),
        "key-up" => test_statement("key-up <key> [modified=<key>] [location=...] [physical=...]"),
        "modifiers" => test_statement("modifiers [shift] [control] [alt] [logo]"),
        "chord" => test_statement("chord [<modifier> ...] <key>"),
        "repeat" => test_statement("repeat <key> <positive-count-expression>"),
        "tap" => test_statement("tap <target> [<count-1..255>]"),
        "touch" => test_statement("touch down|move|up|cancel <id> <x> <y>"),
        "system-theme" => test_statement("system-theme <light|dark|none>"),
        "file-hover" => test_statement("file-hover <str-expression>"),
        "file-drop" => test_statement("file-drop <str-expression>"),
        "file-leave" => test_statement("file-leave"),
        "wait" => test_statement("wait <positive-duration>"),
        "advance" => test_statement("advance <positive-duration>"),
        "idle" => test_statement("idle"),
        "capture" => test_statement("capture <snake_case_name>"),
        "a11y" => test_statement("a11y activate|focus <target>"),
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
        "expect a11y" => test_statement(
            "expect a11y <target> role|name|value <str-expression> | checked|expanded|disabled|focused <bool-expression> | action <click|focus> [<bool-expression>]",
        ),
        "expect component" => test_statement(
            "expect component <component-scope-target>.<state-field> ==|!= <expression>",
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
            "match <expression>\n  <case-expression>|some(<binding>)|none|ok(<binding>)|err(<binding>)|<Enum>.<variant>[(<binding>)]|_\n    <view-node>...",
            child_shape(1, None, "match-arm"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "some" => details(
            &["expression", "match-arm"],
            "some(<expression>) | some(<payload-binding>)",
            leaf(),
            json!({ "requiredWhen": "used as a match pattern", "source": "option payload" }),
            no_route(),
            Vec::new(),
        ),
        "none" => details(
            &["expression", "match-arm"],
            "none",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "ok" => details(
            &["expression", "match-arm"],
            "ok(<expression>) | ok(<payload-binding>)",
            leaf(),
            json!({ "requiredWhen": "used as a match pattern", "source": "result output" }),
            no_route(),
            Vec::new(),
        ),
        "err" => details(
            &["expression", "match-arm"],
            "err(<expression>) | err(<payload-binding>)",
            leaf(),
            json!({ "requiredWhen": "used as a match pattern", "source": "result error" }),
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
                ("checked", "bool-expression", false),
                ("expanded", "bool-expression", false),
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
                "requiredOneOfProperties": ["press", "release", "double", "right-press", "right-release", "middle-press", "middle-release", "enter", "move", "scroll", "exit", "cursor"],
            }),
            properties(&[
                ("press", "route", false),
                ("release", "route", false),
                ("double", "route", false),
                ("right-press", "route", false),
                ("right-release", "route", false),
                ("middle-press", "route", false),
                ("middle-release", "route", false),
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
            json!({ "required": false, "properties": ["press", "release", "right-press", "right-release", "middle-press", "middle-release", "enter", "move", "scroll", "exit"] }),
            properties(&[
                ("w", "length", false),
                ("h", "length", false),
                ("cache", "expression", false),
                ("cache-group", "identifier", false),
                ("capture", "bool-expression", false),
                ("press", "payload-route(x,y)", false),
                ("release", "payload-route(x,y)", false),
                ("right-press", "payload-route(x,y)", false),
                ("right-release", "payload-route(x,y)", false),
                ("middle-press", "payload-route(x,y)", false),
                ("middle-release", "payload-route(x,y)", false),
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
        "run every" | "run latest" | "run replace" => {
            let (mode, syntax, lane_required) = match item.label {
                "run every" => (
                    "every",
                    "run every <extern-future>(<args>) -> <success-handler> _ [| <failure-handler> _]",
                    false,
                ),
                "run latest" => (
                    "latest",
                    "run latest lane=<qualified-identifier> <extern-future>(<args>) -> <success-handler> _ [| <failure-handler> _]",
                    true,
                ),
                "run replace" => (
                    "replace",
                    "run replace lane=<qualified-identifier> <extern-future>(<args>) -> <success-handler> _ [| <failure-handler> _]",
                    true,
                ),
                _ => unreachable!(),
            };
            details(
                &["handler-statement"],
                syntax,
                leaf(),
                no_binding(),
                json!({
                    "required": true,
                    "operator": "->",
                    "mode": mode,
                    "lane": {
                        "required": lane_required,
                        "forbidden": !lane_required,
                        "type": "static qualified identifier",
                        "sharedBy": "all members with the same fully qualified lane name and mode in one state owner"
                    },
                    "success": { "required": true, "payload": "extern output" },
                    "failure": {
                        "payload": "extern error",
                        "requiredWhen": "extern declaration has `! <error-type>`",
                        "forbiddenWhen": "extern declaration has no error type"
                    }
                }),
                Vec::new(),
            )
        }
        "stream every" | "stream replace" => {
            let (mode, syntax, lane_required) = match item.label {
                "stream every" => (
                    "every",
                    "stream every <extern-stream>(<args>) -> <success-handler> _ [| <failure-handler> _]",
                    false,
                ),
                "stream replace" => (
                    "replace",
                    "stream replace lane=<qualified-identifier> <extern-stream>(<args>) -> <success-handler> _ [| <failure-handler> _]",
                    true,
                ),
                _ => unreachable!(),
            };
            details(
                &["handler-statement"],
                syntax,
                leaf(),
                no_binding(),
                json!({
                    "required": true,
                    "operator": "->",
                    "mode": mode,
                    "lane": {
                        "required": lane_required,
                        "forbidden": !lane_required,
                        "type": "static qualified identifier",
                        "sharedBy": "stream replace starts with the same fully qualified lane name in one state owner"
                    },
                    "success": { "required": true, "payload": "each successful stream item" },
                    "failure": {
                        "payload": "each failed stream item; an error item does not end the stream",
                        "requiredWhen": "extern declaration has `! <error-type>`",
                        "forbiddenWhen": "extern declaration has no error type"
                    },
                    "latest": false,
                    "component": "stream replace only",
                    "abortableMember": false
                }),
                Vec::new(),
            )
        }
        "invalidate" => details(
            &["handler-statement"],
            "invalidate lane=<qualified-identifier>",
            leaf(),
            no_binding(),
            no_route(),
            properties(&[("lane", "static qualified identifier", true)]),
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
            &["route-payload", "match-arm"],
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
                "targets": ["text", "button (compact label only)"],
                "forms": ["text-xs", "text-sm", "text-base", "text-lg", "text-xl", "text-2xl", "leading-tight", "leading-snug", "leading-normal", "leading-relaxed", "font-mono", "font-medium", "font-semibold", "font-bold"],
            },
            "semantic": ["bg-TOKEN", "text-TOKEN", "border-TOKEN", "border", "border-2", "rounded-*", "state variants"],
            "rule": "utilities and recipes are target-specific; direct typed properties override recipe defaults but conflict with direct utilities that own the same field",
        },
    })
}

fn test_target_fields() -> Value {
    const FIELDS: &[(&str, &str)] = &[
        ("kind", "str"),
        ("value", "str"),
        ("visible", "bool"),
        ("x", "f64"),
        ("y", "f64"),
        ("width", "f64"),
        ("height", "f64"),
        ("left", "f64"),
        ("top", "f64"),
        ("right", "f64"),
        ("bottom", "f64"),
        ("center_x", "f64"),
        ("center_y", "f64"),
        ("visible_x", "f64"),
        ("visible_y", "f64"),
        ("visible_width", "f64"),
        ("visible_height", "f64"),
        ("content_x", "f64"),
        ("content_y", "f64"),
        ("content_width", "f64"),
        ("content_height", "f64"),
        ("scroll_x", "f64"),
        ("scroll_y", "f64"),
        ("translation_x", "f64"),
        ("translation_y", "f64"),
        ("background", "background"),
        ("border", "border"),
        ("shadow", "shadow"),
        ("text_color", "color"),
        ("text_size", "f64"),
        ("font", "font"),
        ("line_height", "text-line-height"),
        ("surface_count", "i64"),
        ("text_count", "i64"),
        ("image_count", "i64"),
        ("text_x", "f64"),
        ("text_y", "f64"),
        ("text_width", "f64"),
        ("text_height", "f64"),
        ("text_baseline", "f64"),
        ("image_x", "f64"),
        ("image_y", "f64"),
        ("image_width", "f64"),
        ("image_height", "f64"),
        ("pixel_aligned", "bool"),
        ("focused", "bool"),
        ("accessibility_role", "str"),
        ("accessibility_name", "str"),
        ("accessibility_description", "str"),
        ("accessibility_value", "str"),
        ("accessibility_checked", "bool"),
        ("accessibility_expanded", "bool"),
        ("accessibility_disabled", "bool"),
        ("accessibility_supports_activate", "bool"),
        ("accessibility_supports_focus", "bool"),
    ];
    Value::Array(
        FIELDS
            .iter()
            .map(|(name, ty)| {
                let mut field = json!({ "name": name, "type": ty });
                let object = field.as_object_mut().unwrap();
                if *name == "border" {
                    object.insert(
                        "members".into(),
                        json!({ "color": "color", "width": "f64", "radius": "radius" }),
                    );
                } else if *name == "text_baseline" {
                    object.insert(
                        "availability".into(),
                        json!(
                            "shaped retained text; unavailable for cached text without a shaped run"
                        ),
                    );
                }
                field
            })
            .collect(),
    )
}

fn capture_manifest_schema() -> Value {
    json!({
        "required": [
            "schema_version", "name", "png", "capture_source", "viewport", "physical_size",
            "scale_factor", "configured_theme", "resolved_theme", "system_theme",
            "locale", "platform", "reduced_motion", "window", "clock", "targets"
        ],
        "fields": {
            "schema_version": { "type": "integer", "const": CAPTURE_SCHEMA_VERSION },
            "name": { "type": "string" },
            "png": { "type": "string", "path": "sibling basename" },
            "capture_source": { "ref": "capture_source" },
            "viewport": { "ref": "logical_size" },
            "physical_size": { "ref": "physical_size" },
            "scale_factor": { "type": "number" },
            "configured_theme": {
                "type": ["string", "null"],
                "enum": [null, "none", "light", "dark"]
            },
            "resolved_theme": { "ref": "resolved_theme" },
            "system_theme": { "type": "string", "enum": ["none", "light", "dark"] },
            "locale": { "type": ["string", "null"] },
            "platform": { "type": "string", "enum": ["linux", "windows", "macos", "wasm"] },
            "reduced_motion": { "type": ["boolean", "null"] },
            "window": { "ref": "window" },
            "clock": { "ref": "clock" },
            "targets": {
                "type": "array",
                "items": { "ref": "target" },
                "excludesIdsWithFinalSegmentPrefix": "@"
            }
        },
        "definitions": {
            "source_origin": {
                "type": "object",
                "required": ["path", "line", "column"],
                "fields": {
                    "path": { "type": "string" },
                    "line": { "type": "integer" },
                    "column": { "type": "integer" }
                }
            },
            "capture_source": {
                "type": "object",
                "required": ["path", "line", "column", "statement"],
                "fields": {
                    "path": { "type": "string" },
                    "line": { "type": "integer" },
                    "column": { "type": "integer" },
                    "statement": { "type": "string" }
                }
            },
            "logical_size": {
                "type": "object",
                "required": ["width", "height"],
                "fields": { "width": { "type": "number" }, "height": { "type": "number" } }
            },
            "physical_size": {
                "type": "object",
                "required": ["width", "height"],
                "fields": { "width": { "type": "integer" }, "height": { "type": "integer" } },
                "maxPixelArea": 16777216
            },
            "resolved_theme": {
                "type": "object",
                "required": ["mode", "name"],
                "additionalProperties": false,
                "fields": {
                    "mode": { "type": "string", "enum": ["none", "light", "dark"] },
                    "name": { "type": "string" }
                }
            },
            "point": {
                "type": "object",
                "required": ["x", "y"],
                "fields": { "x": { "type": "number" }, "y": { "type": "number" } }
            },
            "rectangle": {
                "type": "object",
                "required": ["x", "y", "width", "height"],
                "fields": {
                    "x": { "type": "number" },
                    "y": { "type": "number" },
                    "width": { "type": "number" },
                    "height": { "type": "number" }
                }
            },
            "optional_rectangle": {
                "type": "object",
                "required": ["x", "y", "width", "height"],
                "fields": {
                    "x": { "type": ["number", "null"] },
                    "y": { "type": ["number", "null"] },
                    "width": { "type": ["number", "null"] },
                    "height": { "type": ["number", "null"] }
                }
            },
            "optional_vector": {
                "type": "object",
                "required": ["x", "y"],
                "fields": {
                    "x": { "type": ["number", "null"] },
                    "y": { "type": ["number", "null"] }
                }
            },
            "color": {
                "type": "object",
                "required": ["r", "g", "b", "a"],
                "fields": {
                    "r": { "type": "number" },
                    "g": { "type": "number" },
                    "b": { "type": "number" },
                    "a": { "type": "number" }
                }
            },
            "window": {
                "type": "object",
                "required": ["position", "focused"],
                "fields": {
                    "position": { "type": ["object", "null"], "ref": "point" },
                    "focused": { "type": "boolean" }
                }
            },
            "clock": {
                "type": "object",
                "required": ["supports_virtual_redraw_advance", "iced_timer_futures_are_virtual"],
                "fields": {
                    "supports_virtual_redraw_advance": { "type": "boolean", "const": true },
                    "iced_timer_futures_are_virtual": { "type": "boolean", "const": false }
                }
            },
            "target_geometry": {
                "type": "object",
                "required": [
                    "x", "y", "width", "height", "left", "top", "right", "bottom",
                    "center_x", "center_y", "pixel_aligned"
                ],
                "fields": {
                    "x": { "type": "number" },
                    "y": { "type": "number" },
                    "width": { "type": "number" },
                    "height": { "type": "number" },
                    "left": { "type": "number" },
                    "top": { "type": "number" },
                    "right": { "type": "number" },
                    "bottom": { "type": "number" },
                    "center_x": { "type": "number" },
                    "center_y": { "type": "number" },
                    "pixel_aligned": { "type": "boolean" }
                }
            },
            "visible_geometry": {
                "type": "object",
                "required": ["present", "x", "y", "width", "height"],
                "fields": {
                    "present": { "type": "boolean" },
                    "x": { "type": ["number", "null"] },
                    "y": { "type": ["number", "null"] },
                    "width": { "type": ["number", "null"] },
                    "height": { "type": ["number", "null"] }
                }
            },
            "accessibility": {
                "type": "object",
                "required": [
                    "role", "name", "description", "value", "checked", "expanded", "disabled",
                    "focused", "actions"
                ],
                "fields": {
                    "role": { "type": "string" },
                    "name": { "type": ["string", "null"] },
                    "description": { "type": ["string", "null"] },
                    "value": { "type": ["string", "null"] },
                    "checked": { "type": ["boolean", "null"] },
                    "expanded": { "type": ["boolean", "null"] },
                    "disabled": { "type": "boolean" },
                    "focused": { "type": "boolean" },
                    "actions": {
                        "type": "object",
                        "required": ["click", "focus"],
                        "fields": {
                            "click": { "type": "boolean" },
                            "focus": { "type": "boolean" }
                        }
                    }
                }
            },
            "gradient_stop": {
                "type": "object",
                "required": ["offset", "color"],
                "fields": {
                    "offset": { "type": "number" },
                    "color": { "ref": "color" }
                }
            },
            "background": {
                "oneOf": [
                    {
                        "type": "object",
                        "required": ["kind", "color"],
                        "fields": {
                            "kind": { "const": "color" },
                            "color": { "ref": "color" }
                        }
                    },
                    {
                        "type": "object",
                        "required": ["kind", "angle_radians", "stops"],
                        "fields": {
                            "kind": { "const": "linear-gradient" },
                            "angle_radians": { "type": "number" },
                            "stops": { "type": "array", "items": { "ref": "gradient_stop" } }
                        }
                    }
                ]
            },
            "radius": {
                "type": "object",
                "required": ["top_left", "top_right", "bottom_right", "bottom_left"],
                "fields": {
                    "top_left": { "type": "number" },
                    "top_right": { "type": "number" },
                    "bottom_right": { "type": "number" },
                    "bottom_left": { "type": "number" }
                }
            },
            "border": {
                "type": "object",
                "required": ["color", "width", "radius"],
                "fields": {
                    "color": { "ref": "color" },
                    "width": { "type": "number" },
                    "radius": { "ref": "radius" }
                }
            },
            "shadow": {
                "type": "object",
                "required": ["color", "offset_x", "offset_y", "blur_radius"],
                "fields": {
                    "color": { "ref": "color" },
                    "offset_x": { "type": "number" },
                    "offset_y": { "type": "number" },
                    "blur_radius": { "type": "number" }
                }
            },
            "surface": {
                "type": "object",
                "required": ["background", "border", "shadow"],
                "fields": {
                    "background": { "ref": "background" },
                    "border": { "ref": "border" },
                    "shadow": { "ref": "shadow" }
                }
            },
            "font_family": {
                "type": "object",
                "required": ["kind", "name"],
                "fields": {
                    "kind": { "type": "string", "enum": ["named", "generic"] },
                    "name": {
                        "type": "string",
                        "genericValues": ["serif", "sans-serif", "cursive", "fantasy", "monospace"]
                    }
                }
            },
            "font": {
                "type": "object",
                "required": ["family", "weight", "stretch", "style"],
                "fields": {
                    "family": { "ref": "font_family" },
                    "weight": {
                        "type": "string",
                        "enum": [
                            "thin", "extra-light", "light", "normal", "medium", "semibold",
                            "bold", "extra-bold", "black"
                        ]
                    },
                    "stretch": {
                        "type": "string",
                        "enum": [
                            "ultra-condensed", "extra-condensed", "condensed", "semi-condensed",
                            "normal", "semi-expanded", "expanded", "extra-expanded",
                            "ultra-expanded"
                        ]
                    },
                    "style": { "type": "string", "enum": ["normal", "italic", "oblique"] }
                }
            },
            "line_height": {
                "type": "object",
                "required": ["kind", "value"],
                "fields": {
                    "kind": { "type": "string", "enum": ["relative", "absolute"] },
                    "value": { "type": "number" }
                }
            },
            "text": {
                "type": "object",
                "required": ["content", "bounds", "color", "size", "font", "line_height", "baseline"],
                "fields": {
                    "content": { "type": ["string", "null"] },
                    "bounds": { "ref": "rectangle" },
                    "color": { "ref": "color" },
                    "size": { "type": ["number", "null"] },
                    "font": { "type": ["object", "null"], "ref": "font" },
                    "line_height": { "type": ["object", "null"], "ref": "line_height" },
                    "baseline": { "type": ["number", "null"] }
                }
            },
            "paint": {
                "type": "object",
                "required": ["available", "unavailable_reason", "surfaces", "texts", "images"],
                "fields": {
                    "available": { "type": "boolean" },
                    "unavailable_reason": { "type": ["string", "null"] },
                    "surfaces": { "type": "array", "items": { "ref": "surface" } },
                    "texts": { "type": "array", "items": { "ref": "text" } },
                    "images": { "type": "array", "items": { "ref": "rectangle" } }
                }
            },
            "target": {
                "type": "object",
                "required": [
                    "id", "kind", "source", "geometry", "visible", "content", "translation", "scroll",
                    "value", "focused", "accessibility", "paint"
                ],
                "fields": {
                    "id": { "type": "string" },
                    "kind": { "type": "string" },
                    "source": { "type": ["object", "null"], "ref": "source_origin" },
                    "geometry": { "ref": "target_geometry" },
                    "visible": { "ref": "visible_geometry" },
                    "content": { "ref": "optional_rectangle" },
                    "translation": { "ref": "optional_vector" },
                    "scroll": { "ref": "optional_vector" },
                    "value": { "type": ["string", "null"] },
                    "focused": { "type": "boolean" },
                    "accessibility": { "type": ["object", "null"], "ref": "accessibility" },
                    "paint": { "ref": "paint" }
                }
            }
        }
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
            "theme": {
                "syntax": "theme <light|dark|none>",
                "required": false,
                "maxOccurrences": 1,
                "effect": "replace the headless Program theme result with Theme::default(mode)",
                "applicationPaletteState": "unchanged; use preset or dispatch",
            },
            "scale": { "syntax": "scale <positive-number>", "required": false, "maxOccurrences": 1, "effect": "override generated program scale factor" },
            "locale": { "syntax": "locale <non-empty-string>", "required": false, "maxOccurrences": 1, "effect": "pinned test metadata and driver context" },
            "platform": { "syntax": "platform <linux|windows|macos|wasm>", "required": false, "maxOccurrences": 1, "effect": "pinned platform-sensitive driver context" },
            "reducedMotion": { "syntax": "reduced-motion <true|false>", "required": false, "maxOccurrences": 1, "effect": "pinned test metadata and driver context" },
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
            "pointerButtons": ["left", "right", "middle", "back", "forward"],
            "click": "click <target> [<button>]",
            "doubleClick": "double-click <target> [<button>]",
            "clickAt": "click-at <x> <y> [<button>]",
            "leave": "leave",
            "move": ["move <target>", "move <x> <y>"],
            "press": "press <target> [<button>]",
            "release": "release [<button>]",
            "wheel": "wheel [pixels|lines] <x> <y>",
            "scrollTo": "scroll-to <target> <x> <y>",
            "scrollBy": "scroll-by <target> <x> <y>",
            "snap": "snap <target> <x-unit-offset> <y-unit-offset>",
            "snapEnd": "snap-end <target>",
            "drag": "drag <source-target> <destination-target>",
            "drop": "drop <target>",
            "focus": ["focus <target>", "focus-next", "focus-previous", "blur"],
            "windowFocus": ["window focus", "window blur"],
            "type": "type <str-expression>",
            "text": ["clear", "replace <str-expression>", "select <start> <end>", "select-all", "cursor <index|front|end>"],
            "composition": ["composition start", "composition update <str-expression> [<selection-start> <selection-end>]", "composition commit <str-expression>", "composition cancel"],
            "key": "key <kebab-key|exact-Iced-variant|non-empty-string>",
            "keyDown": "key-down <key> [modified=<key>] [location=...] [physical=...] [text=\"<non-empty>\"] [repeat=true|false]",
            "keyUp": "key-up <key> [modified=<key>] [location=...] [physical=...]",
            "modifiers": "modifiers [shift] [control] [alt] [logo]",
            "chord": "chord [<modifier> ...] <key>",
            "repeat": {
                "syntax": "repeat <key> <positive-count-expression>",
                "countMeaning": "total activations: one initial non-repeat key-down, count - 1 repeat key-down events, then one key-up"
            },
            "touch": ["tap <target> [<count>]", "touch down|move|up|cancel <id> <x> <y>"],
            "window": ["window move <x> <y>", "window resize <width> <height>", "window rescale <factor>", "window close-request", "window opened", "window closed", "window redraw"],
            "systemTheme": "system-theme <light|dark|none>",
            "files": ["file-hover <str-expression>", "file-drop <str-expression>", "file-leave"],
            "settling": ["idle", "wait <positive-duration>", "advance <positive-duration>"],
            "capture": "capture <snake_case_name>",
            "accessibility": ["a11y activate <target>", "a11y focus <target>"],
            "dispatch": "dispatch <handler> | dispatch <handler>(<argument>, ...)",
        },
        "assertions": {
            "boolean": "expect <bool-expression>",
            "approximate": { "syntax": "expect <numeric-expression> ~= <numeric-expression>", "absoluteTolerance": 0.001 },
            "presence": ["expect exists <target>", "expect missing <target>"],
            "text": ["expect text <str-expression> [within <target>]", "expect no text <str-expression> [within <target>]"],
            "componentState": ["expect component <component-scope-target>.<state-field> == <expression>", "expect component <component-scope-target>.<state-field> != <expression>"],
            "accessibility": {
                "text": "expect a11y <target> role|name|value <str-expression>",
                "boolean": "expect a11y <target> checked|expanded|disabled|focused <bool-expression>",
                "action": "expect a11y <target> action <click|focus> [<bool-expression>]"
            },
        },
        "targetFields": test_target_fields(),
        "execution": {
            "driver": "::ui_lang_runtime::testing::Driver",
            "actionBoundary": "semantic, raw-event-independent ::ui_lang_runtime::testing::Action through Driver::perform_action(Action, Location)",
            "generatedApplicationMessagePublic": false,
            "state": "fresh per test",
            "uiCache": "persistent across rerenders within one current window; reset by a task-issued window open",
            "startupSystemThemeRustConfig": "Config::system_theme; independent from the render theme override",
            "settling": "widget messages, updates, real tasks, and recursively emitted messages drain after each executable step",
            "externs": "real pure, sync, and task Rust extern implementations",
            "subscriptions": "active under the generated program contract",
            "wait": "bounded real elapsed time followed by settling",
            "advance": "deterministic redraw timestamp plus RedrawRequested; arbitrary iced::time futures remain real",
            "capture": {
                "scope": "current window",
                "memory": "RGBA bytes, physical dimensions, scale factor, PNG path, and metadata path returned to runtime callers",
                "files": ["<capture-name>.png", "<capture-name>.json"],
                "defaultDirectory": "target/ice-test-artifacts/<sanitized-test-name>/",
                "environmentRoot": "ICE_TEST_ARTIFACT_DIR",
                "runtimeExactDirectory": "Config::artifact_dir",
                "maxPhysicalPixels": 16777216,
                "metadata": capture_manifest_schema(),
            },
            "mockLayer": false,
        },
        "inspection": {
            "layout": "post-layout selector candidate bounds in logical pixels",
            "paint": "structured tiny-skia draw commands from a real redraw",
            "surfaceMatch": "one quad whose bounds equal the target",
            "textMatch": "one visible text primitive within the target",
            "primitiveCounts": ["surface_count", "text_count", "image_count"],
            "pixelGoldenComparison": false,
            "ambiguousMatch": "runtime failure; never guessed",
            "customRendererPaint": false,
            "interactionTrace": {
                "commands": [
                    "cargo ice inspect ROOT.ice --test NAME --trace [--warmup N] [--repeat N]",
                    "cargo ice inspect ROOT.ice --fuzz interactions --seed N --steps N [--confirm N]",
                    "cargo ice inspect ROOT.ice --replay TRACE.json [--confirm N]"
                ],
                "artifactKind": TRACE_ARTIFACT_KIND,
                "schemaVersion": TRACE_SCHEMA_VERSION,
                "generatorVersion": TRACE_GENERATOR_VERSION,
                "buildProfile": "release",
                "phases": ["action", "view", "ui_build_layout", "event_dispatch", "program_update", "widget_operation", "task_settle"],
                "unavailablePhases": ["draw"],
                "rawSamples": true,
                "summaries": ["p50", "p95", "p99", "max", "60hz deadline misses", "120hz deadline misses"],
                "findingKinds": ["panic", "timeout", "assertion", "latency"],
                "latencyPolicies": ["--deadline-ms", "--max-to-median"],
                "confirmation": "exact semantic sequence on fresh boots before publishing a finding",
                "reduction": "confirmed generated findings only; every accepted result is strictly smaller and preserves the stable fingerprint",
                "evidence": "worst-state PNG and capture-v2 manifest produced outside measured intervals",
                "provenance": ["action source", "rendered target source"],
                "promotion": "translate the reduced semantic sequence into an ordinary first-class Ice test with domain assertions",
                "secondDsl": false,
                "secondDriver": false,
            },
            "reviewBundle": {
                "command": "cargo ice review ROOT.ice [options]",
                "artifactKind": REVIEW_ARTIFACT_KIND,
                "schemaVersion": REVIEW_SCHEMA_VERSION,
                "captureDiffArtifactKind": CAPTURE_DIFF_ARTIFACT_KIND,
                "formats": ["report.json", "report.html", "diagnostics.json", "test logs", "capture PNG/JSON", "optional trace JSON", "diff PNG/JSON"],
                "testSelection": "all declared first-class Ice tests or repeated --test NAME",
                "baselineIdentity": "stable test-name/capture-name manifest key",
                "baselineReport": "exact-schema successful ice_review_bundle with typed unique capture entries",
                "selectedBaselineScope": "filter keys before resolving, reading, or checking manifest paths",
                "failurePublication": "every failure after opening output publishes the current run ID; preserve a detailed current-run failure",
                "failurePolicy": ["test failure", "changed capture", "new capture", "removed capture", "invalid requested trace", "unreadable evidence"],
                "summaries": ["semantic diagnostics", "AccessKit role/name/action inventory", "source-mapped structured changes", "optional interaction tail/finding"],
            },
        },
        "nonGoals": ["DOM", "CSS selectors", "synthetic component bounds", "component-local state access", "DSL mocks", "general virtual clock", "built-in pixel-golden comparison", "multi-window orchestration"],
    })
}

pub fn document() -> Value {
    let constructs = COMPLETIONS.iter().map(construct_schema).collect::<Vec<_>>();

    json!({
        "schemaVersion": 1,
        "language": {
            "name": "Ice",
            "revision": LANGUAGE_REVISION,
            "status": "preview",
            "stability": "implemented candidate",
            "fileExtension": ".ice",
            "encoding": "UTF-8",
            "indent": "two spaces",
            "treeSyntax": "indentation",
        },
        "backend": {
            "iced": ICED_VERSION,
            "iced_widget": ICED_WIDGET_VERSION,
            "build": {
                "package": "ui-lang-build",
                "version": UI_LANG_BUILD_VERSION,
                "phase": "Cargo build script",
                "sourceDirectoryApi": "ui_lang_build::compile_dir",
                "output": "OUT_DIR/ui-lang-generated",
                "outputFileName": "lowercase full SHA-256 of normalized manifest-relative Ice root plus .rs",
                "manifest": "OUT_DIR/ui-lang-generated/manifest.json",
                "manifestSchemaVersion": 2,
                "publication": "directory-locked transaction; synced atomic outputs, manifest committed last",
                "cacheRecovery": "missing, invalid, incomplete, or digest-mismatched cache is fully regenerated",
                "includeMacro": "ui_lang::include_app!",
                "procMacroWritesFiles": false,
            },
            "runtime": {
                "package": "ui-lang-runtime",
                "version": UI_LANG_RUNTIME_VERSION,
                "generatedRustPath": "::ui_lang_runtime",
                "publicApi": [
                    "accessible", "dynamic_themer", "navigation", "snapshot", "Bridge", "Role", "StableId",
                    "testing::Location", "testing::ThemeMode", "testing::Platform", "testing::MouseButton",
                    "testing::WheelDelta", "testing::Modifiers", "testing::Key", "testing::KeyLocation",
                    "testing::KeyMetadata", "testing::CompositionPhase", "testing::TouchPhase",
                    "testing::Capture", "testing::AccessibilityAction", "testing::AccessibilityProperty", "testing::Action",
                    "testing::Config", "testing::Driver", "testing::Target",
                    "testing::step",
                ],
                "testing": {
                    "module": "::ui_lang_runtime::testing",
                    "publicApi": [
                        "Location", "ThemeMode", "Platform", "MouseButton", "WheelDelta",
                        "Modifiers", "Key", "KeyLocation", "KeyMetadata", "CompositionPhase",
                        "TouchPhase", "Capture", "AccessibilityAction", "AccessibilityProperty", "Action", "Config",
                        "Driver", "Target", "step"
                    ],
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
                "source": "one process-local ui_lang_core::AnalysisDb for file URIs; ui_lang_core::analyze for non-file buffers",
                "inMemory": true,
                "rootBufferOverlay": true,
                "diskImports": true,
                "importedBufferOverlays": true,
                "diskFallbackOnClose": true,
                "ownership": "app roots own reports; reports are aggregated by diagnostic URI; fragments are not analyzed as standalone apps",
                "scope": "all open app roots and their overlaid import graphs",
                "reanalyze": "reverse imports invalidate affected open roots; failed roots remain dirty for dependency recovery; unrelated reports are retained without loading their graphs",
                "incrementalMetrics": ["filesLoaded", "bytesLoaded", "filesHashed", "bytesHashed", "filesScanned", "rootsChecked", "rootsReused", "symbolsIndexed", "loadElapsed", "checkElapsed", "codegenRoots", "codegenElapsed"],
                "severities": ["error", "warning"],
                "warnings": {
                    "W001": "component unreachable from every open app root and test mount",
                    "W002": "reachable state has no reader",
                    "W003": "reachable state has no writer",
                    "W004": "unconditional immediate handler routing cycle",
                    "W005": "handler unreachable from runtime and test roots",
                    "W006": "future, task, query, stream, or progress routing cycle",
                    "W007": "unfiltered raw event redraw feedback risk",
                    "W008": "position-based identity for a repeated stateful component",
                    "W009": "retained component state under dynamic identities",
                    "W010": "workspace Ice source outside every root import graph; cargo ice only",
                    "W011": "unused reachable derived value, handler parameter, or local",
                    "W012": "constant no-op statement or dead/redundant view gate",
                    "W013": "statement unreachable after an unconditional return",
                    "W014": "duplicate subscription delivery",
                    "W015": "component mounted without the public ID scope its widget targets need",
                },
                "generatedRustSourceMap": "ui-lang-build writes marked generated Rust below Cargo OUT_DIR; generated items suppress backend-only warnings; cargo ice check and clippy consume Cargo JSON and map nested generated error provenance regions to root or imported Ice syntax; the LSP ice.lint workspace command publishes mapped error-level Clippy and rustc diagnostics; test and compat run that check before the normal test runner",
            },
            "formatting": {
                "supported": true,
                "source": "ui_lang_core::format_fragment",
                "wholeDocument": true,
            },
            "completion": {
                "supported": true,
                "source": "core constructs plus checked component and extern contracts",
                "contextAware": true,
                "contexts": ["top-level", "handler", "view", "node-metadata", "component-call", "component-events", "typed-match-arm", "palette-value", "status", "theme-contract", "test"],
            },
            "hover": {
                "supported": true,
                "symbols": ["component", "recipe"],
                "recipeExpansion": "base-first utilities",
            },
            "signatureHelp": {
                "supported": true,
                "symbols": ["component"],
                "contract": ["read/bind/default props", "output", "named events", "slots"],
            },
            "codeAction": {
                "supported": true,
                "edits": "workspace edits with no server command round-trip",
                "actions": [
                    "component binding syntax",
                    "missing named event routes",
                    "handler skeleton",
                    "fallible extern error route",
                    "child-content button label",
                    "long node with block",
                    "repeated inline utilities to recipe",
                    "direct app handler to named component event",
                    "all missing explicit typed match arms",
                    "unambiguous import alias qualification",
                ],
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
            "candidateRevision": LANGUAGE_REVISION,
            "frozen": false,
            "generative": true,
            "deliveryLanes": {
                "every": {
                    "future": "run every delivers every Future completion",
                    "stream": "stream every delivers every item from every independently started stream",
                    "ownsLane": false,
                    "compilerOwnedHandle": false,
                    "memory": "repeatedly starting a stream that does not terminate intentionally keeps every producer and its captures alive",
                    "safeStreamCompletionDefault": "stream replace lane=<qualified-function-name>"
                },
                "name": "a static qualified identifier; each checked state owner has a finite set of named delivery lanes",
                "qualification": "unaliased app and preset fragments remain in the root namespace and may share root lanes; an aliased component qualifies its internal lane names, but those lanes remain owned by each component instance",
                "sharing": "the same fully qualified lane name joins members across handlers; one owner cannot mix Future and stream effects or latest and replace delivery modes for a lane",
                "storage": "fixed per state owner by the source-declared lanes; component-owner count follows retained/mounted lifetime; a Future replace lane releases its current abort handle when its matching terminal completion is accepted, the next replacement starts, the lane is invalidated, or its owner drops, while a stream replace lane retains its handle across items and releases it only after natural stream termination, the next replacement, invalidation, or owner drop",
                "owner": {
                    "app": "the top-level application state",
                    "daemon": "the daemon state shared across all of its windows",
                    "component": "one component instance; equal fully qualified lane names in different instances are independent"
                },
                "latest": {
                    "effects": ["Future"],
                    "delivery": "only the current generation may route success or failure",
                    "cancelsStaleWork": false,
                    "memory": "stale futures and their captures remain live until they finish or their backend drops them"
                },
                "replace": {
                    "effects": ["Future", "stream"],
                    "delivery": "only the current generation may route a Future completion or stream item",
                    "abortsPriorTask": true,
                    "rollback": false,
                    "memory": "one handle and generation are retained per declared lane and owner; aborting drops work still owned by the task, but cannot undo prior effects, stop detached or blocking backend work, or retract messages already queued by the runtime",
                    "outerAbort": "an outer abort can suppress a Future replacement completion before update; its one fixed current handle then remains until replacement, invalidation, or owner drop",
                    "streamTerminal": "a private terminal envelope clears the handle only after natural stream termination; stream items never clear it"
                },
                "invalidate": {
                    "syntax": "invalidate lane=<qualified-identifier>",
                    "target": "an existing latest Future or replace Future/stream lane in the same state owner; forward references are allowed and invalidation never declares a lane",
                    "scope": "the app/daemon/preset owner or the current component instance",
                    "position": "a direct handler statement, never a parallel, sequential, or abortable task member",
                    "delivery": "advance the generation so every earlier Future completion or stream item is stale",
                    "latest": "does not cancel the in-flight Future",
                    "replace": "advances the generation, then aborts and releases the current replacement handle so already queued old messages are stale",
                    "task": false
                }
            },
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
                "emission": "emit(<event>[, <value>|_ ...]) inside the component view",
                "routing": "events block with exactly one caller-scoped route per declared event",
                "forwarding": "forward block with explicit same-name, same-signature events",
                "closedComponents": true,
                "defaultEventShorthand": "component Name(...) -> Type paired with call-site -> route",
            },
            "componentLifecycle": {
                "default": "retained",
                "mounted": "state, delivery-lane generations, and replace handles are dropped when the scope leaves its rendered root",
                "unmountEffects": false,
            },
            "derivedValues": {
                "syntax": "derived\n  <name> = <pure-expression>",
                "model": {
                    "kind": "pure read-only computation",
                    "evaluation": "observable evaluation cardinality is not guaranteed; the compiler may coalesce equivalent safe reads within one eager view build",
                    "signal": false,
                    "persistentCache": false,
                    "runtimeDependencyGraph": false,
                    "handlerMaintainedMirror": false,
                    "retainedAcrossFrames": false,
                },
            },
            "documentPrelude": {
                "syntax": "app <Name>\ntheme contract <Contract>\n  bg\n  fg\n  primary\n  danger\npalette <name> for <Contract>\n  bg <color>\n  fg <color>\n  primary <color>\n  danger <color>",
                "requiredDeclarations": ["app", "theme contract", "palette", "view"],
                "themeContract": {
                    "required": true,
                    "syntax": "theme contract <Contract>",
                    "tokens": [
                        { "name": "bg", "type": "color", "required": true },
                        { "name": "fg", "type": "color", "required": true },
                        { "name": "primary", "type": "color", "required": true },
                        { "name": "danger", "type": "color", "required": true },
                    ],
                    "additionalTokens": true,
                },
                "palettes": {
                    "required": true,
                    "min": 1,
                    "syntax": "palette <name> for <Contract>",
                    "complete": true,
                    "unknownTokens": false,
                    "duplicateNames": false,
                    "valueType": "color",
                    "selection": "app setting `palette <str-expression>`; defaults to the first declared palette",
                },
            },
            "externFunctions": {
                "pure": {
                    "syntax": "pure <name>(<param>:<type>, ...) -> <type>",
                    "rustAbi": "fn(...) -> Output",
                    "trustedRustContract": {
                        "sameArgumentsSameResult": true,
                        "sideEffectFree": true,
                        "compilerInspectsBody": false,
                    },
                    "allowedContexts": "every checked expression context",
                },
                "sync": {
                    "syntax": "sync <name>(<param>:<type>, ...) -> <type>",
                    "rustAbi": "fn(...) -> Output",
                    "purpose": ["immediate effect", "environment read", "retained identity"],
                    "allowedContexts": [
                        "top-level app state initializer",
                        "immediately evaluated app handler expression",
                        "immediately evaluated component handler expression",
                        "immediately evaluated preset handler expression",
                        "handler task argument, including nested task statements",
                    ],
                    "componentStateInitializer": false,
                    "reason": "component rendering may initialize local state again",
                    "runTaskCompletionRouteExpression": {
                        "statements": ["run every/latest/replace Future", "task statement, including built-in tasks"],
                        "explicitValues": ["state", "derived value", "handler parameter", "handler let local", "pure expression"],
                        "evaluation": "each explicit success and failure expression becomes an owned snapshot when the statement launches",
                        "valueType": "ordinary cloneable Ice data",
                        "branches": "both success and failure snapshots materialize at launch even though only the delivered branch routes",
                        "payloadPlaceholder": "_ is supplied by the delivered completion and is not snapshotted",
                        "syncExtern": false,
                        "externKinds": ["pure"],
                        "recomputationUnsafeBuiltin": false,
                        "syncPattern": "evaluate sync once in a preceding handler let and route that local",
                        "runtimeValuePattern": "evaluate a recomputation-unsafe builtin once in a preceding handler let and route that local",
                        "unchangedFamilies": ["stream", "sip", "flow", "native query"],
                        "memory": {
                            "ownership": "one snapshot set per in-flight task",
                            "release": "task completion, drop, or replace abort",
                            "latest": "a stale latest Future retains its snapshot set until it finishes",
                            "multiOutputTask": "retain one original snapshot set and clone values into each delivered message",
                            "globalMap": false,
                            "accumulatesPerCompletion": false,
                        },
                    },
                },
                "errorType": false,
                "derived": {
                    "externKinds": ["pure"],
                },
                "recomputationUnsafeBuiltins": {
                    "term": "recomputation-unsafe builtin",
                    "definition": "runtime reads or fresh retained identities forbidden where Ice requires stable recomputation",
                    "names": [
                        "window_id.unique",
                        "aborted",
                        "debug.time_with",
                        "image.upgrade",
                        "encoded",
                        "rgba",
                        "animation.animating without explicit instant",
                        "animation.interpolate without explicit instant",
                        "animation.remaining without explicit instant",
                        "animation.project without explicit instant",
                    ],
                    "imageConstructorQualification": "encoded and rgba mean the unqualified built-in image constructors; declared pure/sync externs with those names take precedence",
                    "forbiddenContexts": [
                        "derived",
                        "component prop default",
                        "component state initializer",
                        "direct run/task completion route expression",
                    ],
                    "allowedContexts": [
                        "top-level app state initializer",
                        "handler expressions other than direct run/task completion route expressions",
                        "view",
                    ],
                },
                "componentRecomputedInitializers": {
                    "contexts": ["component prop default", "component state initializer"],
                    "pureExtern": true,
                    "syncExtern": false,
                    "recomputationUnsafeBuiltins": false,
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
                "ui-enum": "non-generic, non-recursive cloneable variants declared with enum",
            },
            "constructs": constructs,
            "components": {
                "prop": "<name>:<type>[=<default-expression>]",
                "defaults": {
                    "optional": true,
                    "expression": "pure checked expression closed over no app state, component state, or parameters; pure extern calls allowed; sync extern calls forbidden",
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

fn completion_item(item: &Completion) -> Value {
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
}

pub fn completion_items() -> Vec<Value> {
    COMPLETIONS.iter().map(completion_item).collect()
}

pub fn completion_items_for(categories: &[&str]) -> Vec<Value> {
    let matches = |item: &&Completion| {
        item.category
            .split('/')
            .any(|category| categories.contains(&category))
    };
    let mut items = Vec::with_capacity(COMPLETIONS.iter().filter(&matches).count());
    items.extend(COMPLETIONS.iter().filter(matches).map(completion_item));
    items
}

#[cfg(test)]
mod tests {
    use super::{
        ACCESSKIT_WINDOWS_VERSION, CAPTURE_SCHEMA_VERSION, COMPLETIONS, ICED_VERSION,
        ICED_WIDGET_VERSION, UI_LANG_BUILD_VERSION, UI_LANG_RUNTIME_VERSION, completion_items,
        completion_items_for, document,
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
        assert_eq!(schema["backend"]["build"]["version"], UI_LANG_BUILD_VERSION);
        assert_eq!(
            schema["backend"]["build"]["output"],
            "OUT_DIR/ui-lang-generated"
        );
        assert_eq!(
            schema["backend"]["build"]["manifest"],
            "OUT_DIR/ui-lang-generated/manifest.json"
        );
        assert_eq!(schema["backend"]["build"]["manifestSchemaVersion"], 2);
        assert_eq!(
            schema["backend"]["build"]["publication"],
            "directory-locked transaction; synced atomic outputs, manifest committed last"
        );
        assert_eq!(
            schema["backend"]["build"]["cacheRecovery"],
            "missing, invalid, incomplete, or digest-mismatched cache is fully regenerated"
        );
        assert_eq!(schema["backend"]["build"]["procMacroWritesFiles"], false);
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
    #[ignore = "allocation contract; run alone with --test-threads=1"]
    fn performance_contract_filtered_completions_use_exact_storage() {
        const REQUESTS: usize = 256;
        const MAX_BLOCKS: u64 = 193_280;
        const MAX_BYTES: u64 = 14_419_456;
        const CATEGORIES: &[&str] = &[
            "test configuration",
            "test statement",
            "test interaction",
            "test assertion",
            "operator",
        ];

        let expected = completion_items_for(CATEGORIES).len();
        let _profiler = dhat::Profiler::builder().testing().build();
        for _ in 0..REQUESTS {
            std::hint::black_box(completion_items_for(std::hint::black_box(CATEGORIES)));
        }
        let heap = dhat::HeapStats::get();

        eprintln!(
            "{REQUESTS} filtered completion requests ({expected} items): {} heap blocks / {} bytes",
            heap.total_blocks, heap.total_bytes
        );
        assert!(
            heap.total_blocks <= MAX_BLOCKS,
            "filtered completions allocated too many blocks: {heap:?}"
        );
        assert!(
            heap.total_bytes <= MAX_BYTES,
            "filtered completions allocated too many bytes: {heap:?}"
        );
    }

    #[test]
    fn completes_ui_enums_and_sum_type_patterns() {
        let completions = completion_items();
        for label in ["enum", "match", "some", "none", "ok", "err"] {
            assert!(
                completions
                    .iter()
                    .any(|completion| completion["label"] == label),
                "missing `{label}` completion"
            );
        }
        assert_eq!(document()["language"]["revision"], "2.0");
    }

    #[test]
    fn delivery_lane_schema_and_completions_are_canonical() {
        let schema = document();
        let lanes = &schema["core"]["deliveryLanes"];
        assert_eq!(
            lanes["every"]["stream"],
            "stream every delivers every item from every independently started stream"
        );
        assert_eq!(lanes["every"]["compilerOwnedHandle"], false);
        assert_eq!(
            lanes["every"]["safeStreamCompletionDefault"],
            "stream replace lane=<qualified-function-name>"
        );
        assert!(lanes.get("ordinary").is_none());
        assert_eq!(
            lanes["owner"]["daemon"],
            "the daemon state shared across all of its windows"
        );
        assert_eq!(lanes["latest"]["cancelsStaleWork"], false);
        assert_eq!(lanes["latest"]["effects"], json!(["Future"]));
        assert_eq!(lanes["replace"]["rollback"], false);
        assert_eq!(lanes["replace"]["effects"], json!(["Future", "stream"]));
        assert_eq!(
            lanes["replace"]["outerAbort"],
            "an outer abort can suppress a Future replacement completion before update; its one fixed current handle then remains until replacement, invalidation, or owner drop"
        );
        assert_eq!(lanes["invalidate"]["task"], false);
        assert_eq!(
            schema["core"]["componentLifecycle"]["mounted"],
            "state, delivery-lane generations, and replace handles are dropped when the scope leaves its rendered root"
        );
        assert_eq!(
            lanes["invalidate"]["scope"],
            "the app/daemon/preset owner or the current component instance"
        );
        assert_eq!(
            lanes["invalidate"]["target"],
            "an existing latest Future or replace Future/stream lane in the same state owner; forward references are allowed and invalidation never declares a lane"
        );
        let constructs = schema["core"]["constructs"].as_array().unwrap();
        let construct = |label| {
            constructs
                .iter()
                .find(|construct| construct["label"] == label)
                .unwrap_or_else(|| panic!("missing `{label}` construct"))
        };
        assert_eq!(construct("run every")["route"]["mode"], "every");
        assert_eq!(construct("run every")["route"]["lane"]["forbidden"], true);
        assert!(
            constructs
                .iter()
                .all(|construct| construct["label"] != "run")
        );
        assert_eq!(construct("run latest")["route"]["lane"]["required"], true);
        assert_eq!(
            construct("run latest")["route"]["lane"]["type"],
            "static qualified identifier"
        );
        assert_eq!(
            construct("invalidate")["syntax"],
            "invalidate lane=<qualified-identifier>"
        );
        assert_eq!(
            construct("invalidate")["properties"],
            json!([{
                "name": "lane",
                "type": "static qualified identifier",
                "required": true
            }])
        );
        assert_eq!(construct("stream every")["route"]["mode"], "every");
        assert_eq!(
            construct("stream every")["route"]["lane"]["forbidden"],
            true
        );
        assert_eq!(
            construct("stream replace")["route"]["lane"]["required"],
            true
        );
        assert!(
            constructs
                .iter()
                .all(|construct| construct["label"] != "stream")
        );
        assert!(
            constructs
                .iter()
                .all(|construct| construct["label"] != "stream latest")
        );

        let completions = completion_items();
        let completion = |label| {
            completions
                .iter()
                .find(|completion| completion["label"] == label)
                .unwrap_or_else(|| panic!("missing `{label}` completion"))
        };
        assert_eq!(
            completion("run every")["insertText"],
            "run every ${1:action}(${2}) -> ${3:succeeded} _ | ${4:failed} _"
        );
        assert!(
            completions
                .iter()
                .all(|completion| completion["label"] != "run")
        );
        assert_eq!(
            completion("run latest")["insertText"],
            "run latest lane=${1:request} ${2:action}(${3}) -> ${4:succeeded} _ | ${5:failed} _"
        );
        assert_eq!(
            completion("run replace")["insertText"],
            "run replace lane=${1:request} ${2:action}(${3}) -> ${4:succeeded} _ | ${5:failed} _"
        );
        assert_eq!(
            completion("stream every")["insertText"],
            "stream every ${1:source}(${2}) -> ${3:succeeded} _ | ${4:failed} _"
        );
        assert_eq!(
            completion("stream replace")["insertText"],
            "stream replace lane=${1:stream} ${2:source}(${3}) -> ${4:succeeded} _ | ${5:failed} _"
        );
        assert!(
            completions
                .iter()
                .all(|completion| completion["label"] != "stream")
        );
        assert!(
            completions
                .iter()
                .all(|completion| completion["label"] != "stream latest")
        );
        assert_eq!(
            completion("invalidate")["insertText"],
            "invalidate lane=${1:request}"
        );
    }

    #[test]
    fn generative_core_matches_the_contract_boundary() {
        const CORE_CONTRACT: &[&str] = &[
            "app",
            "use",
            "theme contract",
            "palette",
            "recipe",
            "state",
            "derived",
            "secret",
            "enum",
            "component",
            "emits",
            "events",
            "lifetime",
            "slot",
            "with",
            "on",
            "let",
            "view",
            "test",
            "preset",
            "viewport",
            "timeout",
            "test theme",
            "scale",
            "locale",
            "platform",
            "reduced-motion",
            "mount",
            "target",
            "click",
            "double-click",
            "click-at",
            "leave",
            "move",
            "press",
            "release",
            "wheel",
            "scroll-to",
            "scroll-by",
            "snap",
            "snap-end",
            "drag",
            "drop",
            "focus",
            "focus-next",
            "focus-previous",
            "blur",
            "window focus",
            "window move",
            "window resize",
            "window rescale",
            "window lifecycle",
            "type",
            "clear",
            "replace",
            "select",
            "select-all",
            "cursor",
            "composition",
            "key",
            "key-down",
            "key-up",
            "modifiers",
            "chord",
            "repeat",
            "tap",
            "touch",
            "system-theme",
            "file-hover",
            "file-drop",
            "file-leave",
            "wait",
            "advance",
            "idle",
            "capture",
            "a11y",
            "dispatch",
            "expect",
            "expect a11y",
            "expect component",
            "if",
            "match",
            "some",
            "none",
            "ok",
            "err",
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
            "run every",
            "run latest",
            "run replace",
            "stream every",
            "stream replace",
            "invalidate",
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
            find("run every")["route"]["failure"]["requiredWhen"],
            "extern declaration has `! <error-type>`"
        );
        assert_eq!(
            find("run every")["route"]["failure"]["forbiddenWhen"],
            "extern declaration has no error type"
        );
        assert!(
            find("extern")["syntax"]
                .as_str()
                .unwrap()
                .contains("<type>")
        );
        assert_eq!(
            schema["core"]["externFunctions"]["pure"]["trustedRustContract"]["sameArgumentsSameResult"],
            true
        );
        assert_eq!(
            schema["core"]["externFunctions"]["sync"]["componentStateInitializer"],
            false
        );
        let sync = &schema["core"]["externFunctions"]["sync"];
        assert!(sync.get("asyncCompletionRouteExpression").is_none());
        assert_eq!(
            schema["core"]["externFunctions"]["recomputationUnsafeBuiltins"]["term"],
            "recomputation-unsafe builtin"
        );
        assert!(
            schema["core"]["externFunctions"]["recomputationUnsafeBuiltins"]["names"]
                .as_array()
                .unwrap()
                .contains(&json!("encoded"))
        );
        assert!(
            schema["core"]["externFunctions"]["recomputationUnsafeBuiltins"]["names"]
                .as_array()
                .unwrap()
                .contains(&json!("rgba"))
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
        assert_eq!(
            styles["utilities"]["text"]["targets"],
            serde_json::json!(["text", "button (compact label only)"])
        );
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
        let completion = |label: &str| {
            constructs
                .iter()
                .find(|construct| construct["label"] == label)
                .unwrap_or_else(|| panic!("missing `{label}` completion"))
        };
        assert_eq!(
            completion("cursor")["insertText"],
            "cursor ${1|front,end,0|}"
        );
        assert_eq!(
            completion("composition")["insertText"],
            "composition ${1:start}"
        );
        assert_eq!(
            completion("key-down")["insertText"],
            "key-down ${1:enter}${2: modified=enter}${3: location=standard}${4: physical=enter}${5: text=\"x\"}${6: repeat=false}"
        );

        let contract = &schema["core"]["testMode"];
        assert_eq!(contract["cargoCommand"], "cargo ice test");
        assert_eq!(
            schema["backend"]["runtime"]["testing"]["module"],
            "::ui_lang_runtime::testing"
        );
        assert_eq!(contract["configuration"]["mount"]["children"]["max"], 1);
        assert_eq!(
            contract["configuration"]["timeout"]["syntax"],
            "timeout <positive-integer><ms|s>"
        );
        assert_eq!(contract["configuration"]["theme"]["maxOccurrences"], 1);
        assert_eq!(
            contract["configuration"]["theme"]["effect"],
            "replace the headless Program theme result with Theme::default(mode)"
        );
        assert_eq!(
            contract["configuration"]["theme"]["applicationPaletteState"],
            "unchanged; use preset or dispatch"
        );
        assert_eq!(contract["configuration"]["scale"]["maxOccurrences"], 1);
        assert_eq!(
            contract["configuration"]["platform"]["syntax"],
            "platform <linux|windows|macos|wasm>"
        );
        assert_eq!(
            contract["interactions"]["dispatch"],
            "dispatch <handler> | dispatch <handler>(<argument>, ...)"
        );
        assert_eq!(
            contract["interactions"]["scrollTo"],
            "scroll-to <target> <x> <y>"
        );
        assert!(
            contract["interactions"]["keyDown"]
                .as_str()
                .unwrap()
                .contains("text=\"<non-empty>\"")
        );
        assert_eq!(
            contract["assertions"]["accessibility"]["action"],
            "expect a11y <target> action <click|focus> [<bool-expression>]"
        );
        assert_eq!(
            contract["assertions"]["approximate"]["absoluteTolerance"],
            0.001
        );
        assert_eq!(
            contract["execution"]["capture"]["environmentRoot"],
            "ICE_TEST_ARTIFACT_DIR"
        );
        assert_eq!(
            contract["execution"]["capture"]["runtimeExactDirectory"],
            "Config::artifact_dir"
        );
        assert_eq!(
            contract["interactions"]["repeat"]["countMeaning"],
            "total activations: one initial non-repeat key-down, count - 1 repeat key-down events, then one key-up"
        );
        assert!(
            contract["execution"]["actionBoundary"]
                .as_str()
                .unwrap()
                .contains("raw-event-independent")
        );
        assert!(
            contract["execution"]["actionBoundary"]
                .as_str()
                .unwrap()
                .contains("Driver::perform_action")
        );
        assert_eq!(
            contract["execution"]["generatedApplicationMessagePublic"],
            false
        );
        let manifest = &contract["execution"]["capture"]["metadata"];
        let definitions = &manifest["definitions"];
        let required = manifest["required"].as_array().unwrap();
        assert!(required.contains(&json!("configured_theme")));
        assert!(required.contains(&json!("resolved_theme")));
        assert!(required.contains(&json!("capture_source")));
        assert!(!required.contains(&json!("theme")));
        assert_eq!(
            manifest["fields"]["schema_version"]["const"],
            CAPTURE_SCHEMA_VERSION
        );
        assert_eq!(manifest["fields"]["png"]["path"], "sibling basename");
        assert_eq!(
            manifest["fields"]["capture_source"]["ref"],
            "capture_source"
        );
        assert_eq!(
            definitions["capture_source"]["required"],
            json!(["path", "line", "column", "statement"])
        );
        assert!(manifest["fields"]["theme"].is_null());
        assert_eq!(
            manifest["fields"]["configured_theme"]["type"],
            json!(["string", "null"])
        );
        assert_eq!(
            manifest["fields"]["resolved_theme"]["ref"],
            "resolved_theme"
        );
        assert_eq!(definitions["resolved_theme"]["additionalProperties"], false);
        assert_eq!(
            definitions["resolved_theme"]["fields"]["mode"]["enum"],
            json!(["none", "light", "dark"])
        );
        assert_eq!(
            definitions["resolved_theme"]["fields"]["name"]["type"],
            "string"
        );
        assert_eq!(definitions["physical_size"]["maxPixelArea"], 16_777_216);
        assert_eq!(
            contract["execution"]["capture"]["maxPhysicalPixels"],
            16_777_216
        );
        assert_eq!(manifest["fields"]["targets"]["items"]["ref"], "target");
        assert!(
            definitions["target"]["required"]
                .as_array()
                .unwrap()
                .contains(&json!("source"))
        );
        assert_eq!(
            definitions["target"]["fields"]["source"]["ref"],
            "source_origin"
        );
        assert_eq!(
            manifest["fields"]["targets"]["excludesIdsWithFinalSegmentPrefix"],
            "@"
        );
        assert_eq!(
            definitions["clock"]["fields"]["supports_virtual_redraw_advance"]["const"],
            true
        );
        assert_eq!(
            definitions["clock"]["fields"]["iced_timer_futures_are_virtual"]["const"],
            false
        );
        assert!(definitions["clock"]["fields"]["redraw_time_is_virtual"].is_null());
        assert_eq!(
            definitions["target"]["fields"]["geometry"]["ref"],
            "target_geometry"
        );
        assert_eq!(
            definitions["target_geometry"]["fields"]["pixel_aligned"]["type"],
            "boolean"
        );
        assert_eq!(
            definitions["accessibility"]["fields"]["actions"]["fields"]["click"]["type"],
            "boolean"
        );
        assert_eq!(
            definitions["paint"]["fields"]["surfaces"]["items"]["ref"],
            "surface"
        );
        assert_eq!(
            definitions["paint"]["fields"]["texts"]["items"]["ref"],
            "text"
        );
        let review = &contract["inspection"]["reviewBundle"];
        assert_eq!(review["artifactKind"], "ice_review_bundle");
        assert_eq!(review["schemaVersion"], 2);
        assert_eq!(review["captureDiffArtifactKind"], "ice_capture_diff");
        let trace = &contract["inspection"]["interactionTrace"];
        assert_eq!(trace["artifactKind"], "ice_interaction_trace");
        assert_eq!(trace["schemaVersion"], 1);
        assert_eq!(trace["generatorVersion"], 1);
        assert_eq!(trace["buildProfile"], "release");
        assert_eq!(trace["secondDsl"], false);
        assert_eq!(trace["secondDriver"], false);
        assert!(
            review["selectedBaselineScope"]
                .as_str()
                .unwrap()
                .contains("before resolving")
        );
        assert_eq!(
            definitions["paint"]["fields"]["images"]["items"]["ref"],
            "rectangle"
        );
        assert_eq!(
            definitions["background"]["oneOf"][1]["fields"]["kind"]["const"],
            "linear-gradient"
        );
        assert_eq!(
            definitions["background"]["oneOf"][1]["fields"]["stops"]["items"]["ref"],
            "gradient_stop"
        );
        assert_eq!(definitions["text"]["fields"]["font"]["ref"], "font");
        assert_eq!(
            definitions["font"]["fields"]["family"]["ref"],
            "font_family"
        );
        assert!(
            schema["backend"]["runtime"]["testing"]["publicApi"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item == "AccessibilityProperty")
        );
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
                "surface_count",
                "text_count",
                "image_count",
                "text_x",
                "text_y",
                "text_width",
                "text_height",
                "text_baseline",
                "image_x",
                "image_y",
                "image_width",
                "image_height",
                "pixel_aligned",
                "focused",
                "accessibility_role",
                "accessibility_name",
                "accessibility_description",
                "accessibility_value",
                "accessibility_checked",
                "accessibility_expanded",
                "accessibility_disabled",
                "accessibility_supports_activate",
                "accessibility_supports_focus",
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
        assert_eq!(contract["inspection"]["pixelGoldenComparison"], false);
        assert_eq!(
            contract["inspection"]["reviewBundle"]["command"],
            "cargo ice review ROOT.ice [options]"
        );
        assert_eq!(
            contract["inspection"]["reviewBundle"]["baselineIdentity"],
            "stable test-name/capture-name manifest key"
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
        let tokens = schema["core"]["documentPrelude"]["themeContract"]["tokens"]
            .as_array()
            .unwrap();
        assert_eq!(
            tokens
                .iter()
                .map(|token| token["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["bg", "fg", "primary", "danger"]
        );
        assert_eq!(
            schema["core"]["documentPrelude"]["palettes"]["selection"],
            "app setting `palette <str-expression>`; defaults to the first declared palette"
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
