use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Document {
    pub app: String,
    pub daemon: bool,
    pub settings: AppSettings,
    pub presets: Vec<Preset>,
    pub recipes: Vec<StyleRecipe>,
    pub structs: Vec<ExternStruct>,
    pub enums: Vec<UiEnum>,
    pub functions: Vec<ExternFn>,
    pub subscriptions: Vec<Subscription>,
    pub theme_contract: Option<ThemeContract>,
    pub palettes: Vec<Palette>,
    pub fonts: Vec<FontDecl>,
    pub states: Vec<State>,
    pub derived: Vec<Derived>,
    pub components: Vec<Component>,
    pub handlers: Vec<Handler>,
    pub tests: Vec<TestDecl>,
    pub view: ViewNode,
}

#[derive(Clone, Debug)]
pub struct Derived {
    pub name: String,
    pub value: Expr,
    pub ty: Type,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ThemeContract {
    pub name: String,
    pub tokens: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct UiEnum {
    pub name: String,
    pub variants: Vec<UiEnumVariant>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct UiEnumVariant {
    pub name: String,
    pub payload: Option<Type>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Palette {
    pub name: String,
    pub contract: String,
    pub colors: BTreeMap<String, String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct StyleRecipe {
    pub name: String,
    pub target: StyleRecipeTarget,
    pub base: Option<String>,
    pub utilities: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleRecipeTarget {
    Column,
    Row,
    Flex,
    Grid,
    Stack,
    Container,
    Text,
    Input,
    Button,
}

impl StyleRecipeTarget {
    pub fn source_name(self) -> &'static str {
        match self {
            Self::Column => "col",
            Self::Row => "row",
            Self::Flex => "flex",
            Self::Grid => "grid",
            Self::Stack => "stack",
            Self::Container => "box",
            Self::Text => "text",
            Self::Input => "input",
            Self::Button => "button",
        }
    }
}

impl Document {
    pub(crate) fn style_recipe(&self, name: &str) -> Option<&StyleRecipe> {
        self.recipes.iter().find(|recipe| recipe.name == name)
    }

    pub(crate) fn expand_styles(&self, styles: &[String]) -> Vec<String> {
        let mut expanded = Vec::new();
        for style in styles {
            if let Some(recipe) = self.style_recipe(style) {
                self.expand_recipe(recipe, &mut expanded);
            } else {
                expanded.push(crate::unqualified_name(style).to_owned());
            }
        }
        expanded
    }

    fn expand_recipe(&self, recipe: &StyleRecipe, expanded: &mut Vec<String>) {
        if let Some(base) = recipe
            .base
            .as_deref()
            .and_then(|name| self.style_recipe(name))
        {
            self.expand_recipe(base, expanded);
        }
        expanded.extend(recipe.utilities.iter().cloned());
    }
}

#[derive(Clone, Debug)]
pub struct Preset {
    pub name: String,
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AppSettings {
    pub title: Option<AppExpression>,
    pub theme: Option<AppExpression>,
    pub palette: Option<AppExpression>,
    pub background: Option<AppExpression>,
    pub text_color: Option<AppExpression>,
    pub id: Option<String>,
    pub executor: Option<String>,
    pub renderer: Option<String>,
    pub fonts: Vec<FontAsset>,
    pub default_text_size: Option<f64>,
    pub antialiasing: Option<bool>,
    pub vsync: Option<bool>,
    pub scale_factor: Option<AppExpression>,
    pub window: Option<WindowSettings>,
    pub windows: Vec<NamedWindow>,
    pub setting_spans: BTreeMap<String, Span>,
    pub span: Span,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            title: None,
            theme: None,
            palette: None,
            background: None,
            text_color: None,
            id: None,
            executor: None,
            renderer: None,
            fonts: Vec::new(),
            default_text_size: None,
            antialiasing: None,
            vsync: None,
            scale_factor: None,
            window: None,
            windows: Vec::new(),
            setting_spans: BTreeMap::new(),
            span: Span::line(1),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppExpression {
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedWindow {
    pub name: String,
    pub settings: WindowSettings,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontAsset {
    pub path: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowSettings {
    pub size: Option<(f64, f64)>,
    pub maximized: Option<bool>,
    pub fullscreen: Option<bool>,
    pub position: Option<WindowPosition>,
    pub min_size: Option<(f64, f64)>,
    pub max_size: Option<(f64, f64)>,
    pub visible: Option<bool>,
    pub resizable: Option<bool>,
    pub closeable: Option<bool>,
    pub minimizable: Option<bool>,
    pub decorations: Option<bool>,
    pub transparent: Option<bool>,
    pub blur: Option<bool>,
    pub level: Option<WindowLevel>,
    pub icon: Option<WindowIcon>,
    pub exit_on_close_request: Option<bool>,
    pub linux: Option<LinuxWindowSettings>,
    pub windows: Option<WindowsWindowSettings>,
    pub macos: Option<MacosWindowSettings>,
    pub wasm: Option<WasmWindowSettings>,
    pub setting_spans: BTreeMap<String, Span>,
    pub span: Span,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            size: None,
            maximized: None,
            fullscreen: None,
            position: None,
            min_size: None,
            max_size: None,
            visible: None,
            resizable: None,
            closeable: None,
            minimizable: None,
            decorations: None,
            transparent: None,
            blur: None,
            level: None,
            icon: None,
            exit_on_close_request: None,
            linux: None,
            windows: None,
            macos: None,
            wasm: None,
            setting_spans: BTreeMap::new(),
            span: Span::line(1),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinuxWindowSettings {
    pub application_id: Option<String>,
    pub override_redirect: Option<bool>,
    pub setting_spans: BTreeMap<String, Span>,
    pub span: Span,
}

impl Default for LinuxWindowSettings {
    fn default() -> Self {
        Self {
            application_id: None,
            override_redirect: None,
            setting_spans: BTreeMap::new(),
            span: Span::line(1),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowsWindowSettings {
    pub drag_and_drop: Option<bool>,
    pub skip_taskbar: Option<bool>,
    pub undecorated_shadow: Option<bool>,
    pub corner: Option<WindowCorner>,
    pub setting_spans: BTreeMap<String, Span>,
    pub span: Span,
}

impl Default for WindowsWindowSettings {
    fn default() -> Self {
        Self {
            drag_and_drop: None,
            skip_taskbar: None,
            undecorated_shadow: None,
            corner: None,
            setting_spans: BTreeMap::new(),
            span: Span::line(1),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MacosWindowSettings {
    pub title_hidden: Option<bool>,
    pub titlebar_transparent: Option<bool>,
    pub fullsize_content_view: Option<bool>,
    pub setting_spans: BTreeMap<String, Span>,
    pub span: Span,
}

impl Default for MacosWindowSettings {
    fn default() -> Self {
        Self {
            title_hidden: None,
            titlebar_transparent: None,
            fullsize_content_view: None,
            setting_spans: BTreeMap::new(),
            span: Span::line(1),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WasmWindowSettings {
    pub target: Option<Option<String>>,
    pub setting_spans: BTreeMap<String, Span>,
    pub span: Span,
}

impl Default for WasmWindowSettings {
    fn default() -> Self {
        Self {
            target: None,
            setting_spans: BTreeMap::new(),
            span: Span::line(1),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowCorner {
    Default,
    DoNotRound,
    Round,
    RoundSmall,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowIcon {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub byte_len: usize,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontDecl {
    pub name: String,
    pub family: FontFamily,
    pub weight: FontWeight,
    pub stretch: FontStretch,
    pub style: FontStyle,
    pub default: bool,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrCorrection {
    Low,
    Medium,
    Quartile,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrVersion {
    Normal(u8),
    Micro(u8),
}

#[derive(Clone, Debug)]
pub struct ExternStruct {
    pub name: String,
    pub rust_path: String,
    pub fields: Vec<(String, Type)>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ExternFn {
    pub kind: ExternKind,
    pub name: String,
    pub rust_path: String,
    pub params: Vec<(String, Type)>,
    pub borrowed: Vec<bool>,
    pub progress: Option<Type>,
    pub output: Type,
    pub error: Option<Type>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Subscription {
    pub source: SubscriptionSource,
    pub window_id: bool,
    pub context: Option<Expr>,
    pub filter: Option<String>,
    pub condition: Option<Expr>,
    pub status: Option<EventStatus>,
    pub route: Route,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum SubscriptionSource {
    Every { milliseconds: u64 },
    Repeat { function: String, milliseconds: u64 },
    Run { function: String, args: Vec<Expr> },
    Recipe { function: String, args: Vec<Expr> },
    Events { id: Expr, filter: String },
    Event { raw: bool },
    Extern { function: String, args: Vec<Expr> },
    InputMethod(InputMethodEvent),
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
    SystemTheme,
    Touch(TouchEvent),
    Window(WindowEvent),
}
