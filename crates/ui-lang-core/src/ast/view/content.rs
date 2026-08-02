use super::*;

#[derive(Clone, Debug, Default)]
pub struct FloatStyleOptions {
    pub shadow_color: Option<String>,
    pub shadow_x: Option<Expr>,
    pub shadow_y: Option<Expr>,
    pub shadow_blur: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
}

pub(crate) fn float_expression_roots<'a>(
    scale: &'a Expr,
    x: &'a Expr,
    y: &'a Expr,
    style: &'a FloatStyleOptions,
) -> Vec<&'a Expr> {
    let mut roots = vec![scale, x, y];
    roots.extend(
        [
            &style.shadow_x,
            &style.shadow_y,
            &style.shadow_blur,
            &style.radius,
            &style.radius_top_left,
            &style.radius_top_right,
            &style.radius_bottom_right,
            &style.radius_bottom_left,
        ]
        .into_iter()
        .flatten(),
    );
    roots
}

pub(crate) fn float_semantic_key(style: &FloatStyleOptions) -> String {
    format!(
        "float|shadow={:?}|fields={:?}",
        style.shadow_color,
        [
            style.shadow_x.is_some(),
            style.shadow_y.is_some(),
            style.shadow_blur.is_some(),
            style.radius.is_some(),
            style.radius_top_left.is_some(),
            style.radius_top_right.is_some(),
            style.radius_bottom_right.is_some(),
            style.radius_bottom_left.is_some(),
        ]
    )
}

pub(crate) fn pin_expression_roots<'a>(
    width: &'a Option<LengthValue>,
    height: &'a Option<LengthValue>,
    x: &'a Expr,
    y: &'a Expr,
) -> Vec<&'a Expr> {
    let mut roots = vec![x, y];
    for length in [width, height].into_iter().flatten() {
        if let LengthValue::Fixed(expression) = length {
            roots.push(expression);
        }
    }
    roots
}

pub(crate) fn pin_semantic_key(
    width: &Option<LengthValue>,
    height: &Option<LengthValue>,
) -> String {
    fn length_key(length: &Option<LengthValue>) -> String {
        match length {
            None => "none".into(),
            Some(LengthValue::Fill) => "fill".into(),
            Some(LengthValue::FillPortion(portion)) => format!("fill-portion:{portion}"),
            Some(LengthValue::Shrink) => "shrink".into(),
            Some(LengthValue::Fixed(_)) => "fixed".into(),
        }
    }

    format!(
        "pin|width={}|height={}",
        length_key(width),
        length_key(height)
    )
}

#[derive(Clone, Debug, Default)]
pub struct MarkdownOptions {
    pub text_size: Option<Expr>,
    pub h1_size: Option<Expr>,
    pub h2_size: Option<Expr>,
    pub h3_size: Option<Expr>,
    pub h4_size: Option<Expr>,
    pub h5_size: Option<Expr>,
    pub h6_size: Option<Expr>,
    pub code_size: Option<Expr>,
    pub spacing: Option<Expr>,
    pub viewer: Option<ExternCall>,
    pub style: MarkdownStyleOptions,
}

#[derive(Clone, Debug)]
pub struct ExternCall {
    pub function: String,
    pub args: Vec<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct MarkdownStyleOptions {
    pub font: Option<FontPreset>,
    pub inline_code_background: Option<BackgroundValue>,
    pub inline_code_color: Option<String>,
    pub inline_code_font: Option<FontPreset>,
    pub code_block_font: Option<FontPreset>,
    pub link_color: Option<String>,
    pub inline_code_padding: PaddingOptions,
    pub inline_code_border_color: Option<String>,
    pub inline_code_border_width: Option<Expr>,
    pub inline_code_radius: Option<Expr>,
    pub inline_code_radius_top_left: Option<Expr>,
    pub inline_code_radius_top_right: Option<Expr>,
    pub inline_code_radius_bottom_right: Option<Expr>,
    pub inline_code_radius_bottom_left: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct TextEditorOptions {
    pub placeholder: Option<String>,
    pub width: Option<Expr>,
    pub height: Option<LengthValue>,
    pub min_height: Option<Expr>,
    pub max_height: Option<Expr>,
    pub size: Option<Expr>,
    pub line_height: Option<TextLineHeight>,
    pub padding: Option<Expr>,
    pub wrapping: Option<TextWrapping>,
    pub font: Option<FontPreset>,
    pub highlight: Option<String>,
    pub highlight_theme: Option<HighlightTheme>,
    pub highlighter: Option<ExternCall>,
    pub key_binding: Option<ExternCall>,
    pub key_binding_route: Option<Route>,
    pub action: Option<ExternCall>,
    pub custom_style: Option<ExternCall>,
    pub style: Box<TextInputStyleSet>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HighlightTheme {
    SolarizedDark,
    Base16Mocha,
    Base16Ocean,
    Base16Eighties,
    InspiredGithub,
}

#[derive(Clone, Debug, Default)]
pub struct TableOptions {
    pub width: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub padding_x: Option<Expr>,
    pub padding_y: Option<Expr>,
    pub separator: Option<Expr>,
    pub separator_x: Option<Expr>,
    pub separator_y: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct TableColumn {
    pub width: Option<LengthValue>,
    pub align_x: Option<InputAlignment>,
    pub align_y: Option<VerticalAlignment>,
    pub header: ViewNode,
    pub cell: ViewNode,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ThemePreset {
    Default,
    App,
    BuiltIn(String),
    Factory(ExternCall),
}

pub(crate) const BUILT_IN_THEMES: &[&str] = &[
    "light",
    "dark",
    "dracula",
    "nord",
    "solarized-light",
    "solarized-dark",
    "gruvbox-light",
    "gruvbox-dark",
    "catppuccin-latte",
    "catppuccin-frappe",
    "catppuccin-macchiato",
    "catppuccin-mocha",
    "tokyo-night",
    "tokyo-night-storm",
    "tokyo-night-light",
    "kanagawa-wave",
    "kanagawa-dragon",
    "kanagawa-lotus",
    "moonfly",
    "nightfly",
    "oxocarbon",
    "ferra",
];

#[derive(Clone, Debug)]
pub enum ResponsiveContent {
    Breakpoint {
        breakpoint: Expr,
        narrow: Box<ViewNode>,
        wide: Box<ViewNode>,
    },
    Size {
        width: String,
        height: String,
        content: Box<ViewNode>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct AccessibilityOptions {
    pub label: Option<Expr>,
    pub description: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct InputOptions {
    pub accessibility: AccessibilityOptions,
    pub secure: Option<Expr>,
    pub change: Option<Route>,
    pub submit: Option<Route>,
    pub paste: Option<Route>,
    pub width: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub text_size: Option<Expr>,
    pub line_height: Option<Expr>,
    pub align: Option<InputAlignment>,
    pub font: Option<FontPreset>,
    pub icon: Option<TextInputIcon>,
    pub custom_style: Option<ExternCall>,
    pub style: Box<TextInputStyleSet>,
}

#[derive(Clone, Debug, Default)]
pub struct TextOptions {
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub size: Option<Expr>,
    pub line_height: Option<TextLineHeight>,
    pub font: Option<FontPreset>,
    pub align_x: Option<TextAlignment>,
    pub align_y: Option<VerticalAlignment>,
    pub shaping: Option<TextShaping>,
    pub wrapping: Option<TextWrapping>,
    pub tracking: Option<f64>,
    pub custom_style: Option<ExternCall>,
}

#[derive(Clone, Debug)]
pub enum TextLineHeight {
    Relative(Expr),
    Absolute(Expr),
}

#[derive(Clone, Debug)]
pub struct RichSpan {
    pub value: Expr,
    pub options: RichSpanOptions,
    pub styles: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct RichSpanOptions {
    pub size: Option<Expr>,
    pub line_height: Option<TextLineHeight>,
    pub font: Option<FontPreset>,
    pub color: Option<String>,
    pub link: Option<Expr>,
    pub background: Option<BackgroundValue>,
    pub border: Option<String>,
    pub border_width: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub padding: PaddingOptions,
    pub underline: Option<Expr>,
    pub strikethrough: Option<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct ButtonOptions {
    pub accessibility: AccessibilityOptions,
    pub width: Option<LengthValue>,
    pub height: Option<LengthValue>,
    pub padding: Option<Expr>,
    pub clip: Option<Expr>,
    pub style: Box<ButtonStyleSet>,
}

#[derive(Clone, Debug, Default)]
pub struct ButtonStyleSet {
    pub preset: ButtonStylePreset,
    pub custom: Option<ExternCall>,
    pub active: Option<ButtonStatusStyle>,
    pub hovered: Option<ButtonStatusStyle>,
    pub pressed: Option<ButtonStatusStyle>,
    pub disabled: Option<ButtonStatusStyle>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonStylePreset {
    #[default]
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Text,
    Background,
    Subtle,
}

#[derive(Clone, Debug)]
pub struct ButtonStatusStyle {
    pub options: ContainerStyleOptions,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputAlignment {
    Left,
    Center,
    Right,
}

impl std::str::FromStr for InputAlignment {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontPreset {
    Default,
    Monospace,
    Named(String),
}

#[derive(Clone, Debug, Default)]
pub struct BoolControlOptions {
    pub accessibility: AccessibilityOptions,
    pub size: Option<Expr>,
    pub width: Option<LengthValue>,
    pub spacing: Option<Expr>,
    pub text_size: Option<Expr>,
    pub line_height: Option<Expr>,
    pub shaping: Option<TextShaping>,
    pub wrapping: Option<TextWrapping>,
    pub font: Option<FontPreset>,
    pub alignment: Option<TextAlignment>,
    pub icon: Option<char>,
    pub icon_size: Option<Expr>,
    pub icon_line_height: Option<Expr>,
    pub icon_shaping: Option<TextShaping>,
}

#[derive(Clone, Debug, Default)]
pub struct CheckboxStyleSet {
    pub preset: CheckboxStylePreset,
    pub custom: Option<ExternCall>,
    pub active_checked: Option<CheckboxStatusStyle>,
    pub active_unchecked: Option<CheckboxStatusStyle>,
    pub hovered_checked: Option<CheckboxStatusStyle>,
    pub hovered_unchecked: Option<CheckboxStatusStyle>,
    pub disabled_checked: Option<CheckboxStatusStyle>,
    pub disabled_unchecked: Option<CheckboxStatusStyle>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckboxStylePreset {
    #[default]
    Primary,
    Secondary,
    Success,
    Danger,
}

#[derive(Clone, Debug, Default)]
pub struct CheckboxStatusStyle {
    pub background: Option<BackgroundValue>,
    pub icon_color: Option<String>,
    pub text_color: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<Expr>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub span: Option<Span>,
}

#[derive(Clone, Debug, Default)]
pub struct TogglerStyleSet {
    pub custom: Option<ExternCall>,
    pub active_checked: Option<TogglerStatusStyle>,
    pub active_unchecked: Option<TogglerStatusStyle>,
    pub hovered_checked: Option<TogglerStatusStyle>,
    pub hovered_unchecked: Option<TogglerStatusStyle>,
    pub disabled_checked: Option<TogglerStatusStyle>,
    pub disabled_unchecked: Option<TogglerStatusStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct TogglerStatusStyle {
    pub background: Option<BackgroundValue>,
    pub background_border_color: Option<String>,
    pub background_border_width: Option<Expr>,
    pub foreground: Option<BackgroundValue>,
    pub foreground_border_color: Option<String>,
    pub foreground_border_width: Option<Expr>,
    pub text_color: Option<String>,
    pub radius: Option<Expr>,
    pub radius_top_left: Option<Expr>,
    pub radius_top_right: Option<Expr>,
    pub radius_bottom_right: Option<Expr>,
    pub radius_bottom_left: Option<Expr>,
    pub padding_ratio: Option<Expr>,
    pub span: Option<Span>,
}

#[derive(Clone, Debug, Default)]
pub struct RadioStyleSet {
    pub custom: Option<ExternCall>,
    pub active_selected: Option<RadioStatusStyle>,
    pub active_unselected: Option<RadioStatusStyle>,
    pub hovered_selected: Option<RadioStatusStyle>,
    pub hovered_unselected: Option<RadioStatusStyle>,
}

#[derive(Clone, Debug, Default)]
pub struct RadioStatusStyle {
    pub background: Option<BackgroundValue>,
    pub dot_color: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<Expr>,
    pub text_color: Option<String>,
    pub span: Option<Span>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextShaping {
    Auto,
    Basic,
    Advanced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextWrapping {
    None,
    Word,
    Glyph,
    WordOrGlyph,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlignment {
    Default,
    Left,
    Center,
    Right,
    Justified,
}

impl std::str::FromStr for TextAlignment {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "default" => Ok(Self::Default),
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            "justified" => Ok(Self::Justified),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

impl std::str::FromStr for VerticalAlignment {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "top" => Ok(Self::Top),
            "center" => Ok(Self::Center),
            "bottom" => Ok(Self::Bottom),
            _ => Err(()),
        }
    }
}
