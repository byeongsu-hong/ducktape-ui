use super::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RecipeId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct StyleUseId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct StyleTargetId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct StyleVariantId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ThemeContractId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ThemeTokenId {
    pub(crate) contract: ThemeContractId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedStyleTargetKind {
    Column,
    Row,
    Scroll,
    Flex,
    Grid,
    Stack,
    Container,
    Text,
    Input,
    Button,
    PaneContent,
    PaneTitle,
}

impl ResolvedStyleTargetKind {
    fn id(self) -> StyleTargetId {
        StyleTargetId(self as u32)
    }
}

impl From<StyleRecipeTarget> for ResolvedStyleTargetKind {
    fn from(value: StyleRecipeTarget) -> Self {
        match value {
            StyleRecipeTarget::Column => Self::Column,
            StyleRecipeTarget::Row => Self::Row,
            StyleRecipeTarget::Flex => Self::Flex,
            StyleRecipeTarget::Grid => Self::Grid,
            StyleRecipeTarget::Stack => Self::Stack,
            StyleRecipeTarget::Container => Self::Container,
            StyleRecipeTarget::Text => Self::Text,
            StyleRecipeTarget::Input => Self::Input,
            StyleRecipeTarget::Button => Self::Button,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedStyleVariantKind {
    Base,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}

impl ResolvedStyleVariantKind {
    fn id(self) -> StyleVariantId {
        StyleVariantId(self as u32)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedThemeColorBase {
    White,
    Black,
    Transparent,
    Token(ThemeTokenId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedThemeColor {
    pub(crate) base: ResolvedThemeColorBase,
    pub(crate) opacity: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedStyleProperty {
    WidthFill,
    HeightFill,
    MaxWidth,
    Padding,
    Gap,
    ItemsCenter,
    SelfCenter,
    Clip,
    TextSize,
    TextLineHeight,
    FontMonospace,
    FontWeight,
    TextColor,
    Background,
    BorderColor,
    BorderWidth,
    Radius,
    Opacity,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ResolvedUtilityValue {
    Flag,
    Pixels(u16),
    Padding([Option<u16>; 4]),
    TextSize(f32),
    LineHeight(f32),
    FontWeight(ResolvedStyleFontWeight),
    Color(ResolvedThemeColor),
    Opacity(f32),
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedUtility {
    pub(crate) variant: StyleVariantId,
    pub(crate) property: ResolvedStyleProperty,
    pub(crate) value: ResolvedUtilityValue,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedRecipe {
    pub(crate) id: RecipeId,
    pub(crate) name: String,
    pub(crate) target: StyleTargetId,
    pub(crate) base: Option<RecipeId>,
    pub(crate) declared_utilities: Vec<ResolvedUtility>,
    pub(crate) style: ResolvedStyle,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ResolvedStyleFontWeight {
    Medium,
    Semibold,
    Bold,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedStyle {
    set_properties: u64,
    pub(crate) width_fill: bool,
    pub(crate) height_fill: bool,
    pub(crate) max_width: Option<u16>,
    pub(crate) padding: [u16; 4],
    pub(crate) gap: Option<u16>,
    pub(crate) items_center: bool,
    pub(crate) self_center: bool,
    pub(crate) clip: bool,
    pub(crate) text_size: Option<f32>,
    pub(crate) text_line_height: Option<f32>,
    pub(crate) font_monospace: bool,
    pub(crate) font_weight: Option<ResolvedStyleFontWeight>,
    pub(crate) text_color: Option<ResolvedThemeColor>,
    pub(crate) background: Option<ResolvedThemeColor>,
    pub(crate) hover_background: Option<ResolvedThemeColor>,
    pub(crate) pressed_background: Option<ResolvedThemeColor>,
    pub(crate) disabled_background: Option<ResolvedThemeColor>,
    pub(crate) disabled_text_color: Option<ResolvedThemeColor>,
    pub(crate) border_color: Option<ResolvedThemeColor>,
    pub(crate) focus_border_color: Option<ResolvedThemeColor>,
    pub(crate) border_width: u16,
    pub(crate) radius: u16,
    pub(crate) disabled_opacity: Option<f32>,
}

impl ResolvedStyle {
    fn apply(&mut self, utility: &ResolvedUtility) {
        use ResolvedStyleProperty as Property;
        use ResolvedStyleVariantKind as Variant;
        let variant = match utility.variant.0 {
            0 => Variant::Base,
            1 => Variant::Hovered,
            2 => Variant::Pressed,
            3 => Variant::Focused,
            4 => Variant::Disabled,
            _ => unreachable!("style variants are closed during lowering"),
        };
        match (variant, utility.property, &utility.value) {
            (Variant::Base, Property::WidthFill, _) => self.width_fill = true,
            (Variant::Base, Property::HeightFill, _) => self.height_fill = true,
            (Variant::Base, Property::MaxWidth, ResolvedUtilityValue::Pixels(value)) => {
                self.max_width = Some(*value)
            }
            (Variant::Base, Property::Padding, ResolvedUtilityValue::Padding(value)) => {
                for (slot, value) in self.padding.iter_mut().zip(value) {
                    if let Some(value) = value {
                        *slot = *value;
                    }
                }
            }
            (Variant::Base, Property::Gap, ResolvedUtilityValue::Pixels(value)) => {
                self.gap = Some(*value)
            }
            (Variant::Base, Property::ItemsCenter, _) => self.items_center = true,
            (Variant::Base, Property::SelfCenter, _) => self.self_center = true,
            (Variant::Base, Property::Clip, _) => self.clip = true,
            (Variant::Base, Property::TextSize, ResolvedUtilityValue::TextSize(value)) => {
                self.text_size = Some(*value)
            }
            (Variant::Base, Property::TextLineHeight, ResolvedUtilityValue::LineHeight(value)) => {
                self.text_line_height = Some(*value)
            }
            (Variant::Base, Property::FontMonospace, _) => self.font_monospace = true,
            (Variant::Base, Property::FontWeight, ResolvedUtilityValue::FontWeight(value)) => {
                self.font_weight = Some(*value)
            }
            (Variant::Base, Property::TextColor, ResolvedUtilityValue::Color(value)) => {
                self.text_color = Some(value.clone())
            }
            (Variant::Base, Property::Background, ResolvedUtilityValue::Color(value)) => {
                self.background = Some(value.clone())
            }
            (Variant::Hovered, Property::Background, ResolvedUtilityValue::Color(value)) => {
                self.hover_background = Some(value.clone())
            }
            (Variant::Pressed, Property::Background, ResolvedUtilityValue::Color(value)) => {
                self.pressed_background = Some(value.clone())
            }
            (Variant::Disabled, Property::Background, ResolvedUtilityValue::Color(value)) => {
                self.disabled_background = Some(value.clone())
            }
            (Variant::Disabled, Property::TextColor, ResolvedUtilityValue::Color(value)) => {
                self.disabled_text_color = Some(value.clone())
            }
            (Variant::Base, Property::BorderColor, ResolvedUtilityValue::Color(value)) => {
                self.border_color = Some(value.clone())
            }
            (Variant::Focused, Property::BorderColor, ResolvedUtilityValue::Color(value)) => {
                self.focus_border_color = Some(value.clone())
            }
            (Variant::Base, Property::BorderWidth, ResolvedUtilityValue::Pixels(value)) => {
                self.border_width = *value
            }
            (Variant::Base, Property::Radius, ResolvedUtilityValue::Pixels(value)) => {
                self.radius = *value
            }
            (Variant::Disabled, Property::Opacity, ResolvedUtilityValue::Opacity(value)) => {
                self.disabled_opacity = Some(*value)
            }
            _ => unreachable!("checker-approved utility has a canonical style assignment"),
        }
        self.set_properties |= utility_property_mask(utility);
    }

    fn overlay(&mut self, other: &Self) {
        macro_rules! copy {
            ($bit:expr, $field:ident) => {
                if other.set_properties & (1 << $bit) != 0 {
                    self.$field = other.$field.clone();
                }
            };
        }
        copy!(0, width_fill);
        copy!(1, height_fill);
        copy!(2, max_width);
        for (bit, index) in [(3, 0), (4, 1), (5, 2), (6, 3)] {
            if other.set_properties & (1 << bit) != 0 {
                self.padding[index] = other.padding[index];
            }
        }
        copy!(7, gap);
        copy!(8, items_center);
        copy!(9, self_center);
        copy!(10, clip);
        copy!(11, text_size);
        copy!(12, text_line_height);
        copy!(13, font_monospace);
        copy!(14, font_weight);
        copy!(15, text_color);
        copy!(16, background);
        copy!(17, hover_background);
        copy!(18, pressed_background);
        copy!(19, disabled_background);
        copy!(20, disabled_text_color);
        copy!(21, border_color);
        copy!(22, focus_border_color);
        copy!(23, border_width);
        copy!(24, radius);
        copy!(25, disabled_opacity);
        self.set_properties |= other.set_properties;
    }
}

fn utility_property_mask(utility: &ResolvedUtility) -> u64 {
    use ResolvedStyleProperty as Property;
    use ResolvedStyleVariantKind as Variant;
    let variant = match utility.variant.0 {
        0 => Variant::Base,
        1 => Variant::Hovered,
        2 => Variant::Pressed,
        3 => Variant::Focused,
        4 => Variant::Disabled,
        _ => unreachable!("style variants are closed during lowering"),
    };
    let bit = match (variant, utility.property) {
        (Variant::Base, Property::WidthFill) => 0,
        (Variant::Base, Property::HeightFill) => 1,
        (Variant::Base, Property::MaxWidth) => 2,
        (Variant::Base, Property::Gap) => 7,
        (Variant::Base, Property::ItemsCenter) => 8,
        (Variant::Base, Property::SelfCenter) => 9,
        (Variant::Base, Property::Clip) => 10,
        (Variant::Base, Property::TextSize) => 11,
        (Variant::Base, Property::TextLineHeight) => 12,
        (Variant::Base, Property::FontMonospace) => 13,
        (Variant::Base, Property::FontWeight) => 14,
        (Variant::Base, Property::TextColor) => 15,
        (Variant::Base, Property::Background) => 16,
        (Variant::Hovered, Property::Background) => 17,
        (Variant::Pressed, Property::Background) => 18,
        (Variant::Disabled, Property::Background) => 19,
        (Variant::Disabled, Property::TextColor) => 20,
        (Variant::Base, Property::BorderColor) => 21,
        (Variant::Focused, Property::BorderColor) => 22,
        (Variant::Base, Property::BorderWidth) => 23,
        (Variant::Base, Property::Radius) => 24,
        (Variant::Disabled, Property::Opacity) => 25,
        (Variant::Base, Property::Padding) => {
            let ResolvedUtilityValue::Padding(values) = &utility.value else {
                unreachable!("padding property has a padding value")
            };
            return values
                .iter()
                .enumerate()
                .filter(|(_, value)| value.is_some())
                .fold(0, |mask, (index, _)| mask | (1 << (3 + index)));
        }
        _ => unreachable!("checker-approved utility has a canonical property mask"),
    };
    1 << bit
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedStyleUse {
    pub(crate) id: StyleUseId,
    pub(crate) target: StyleTargetId,
    pub(crate) recipes: Vec<RecipeId>,
    pub(crate) utilities: Vec<ResolvedUtility>,
    pub(crate) style: ResolvedStyle,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedThemeToken {
    pub(crate) id: ThemeTokenId,
    pub(crate) name: String,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedThemeContract {
    pub(crate) id: ThemeContractId,
    pub(crate) name: String,
    pub(crate) tokens: Vec<ResolvedThemeToken>,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedPaletteColor {
    pub(crate) token: ThemeTokenId,
    pub(crate) rgba: [u8; 4],
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedPalette {
    pub(crate) id: PaletteId,
    pub(crate) name: String,
    pub(crate) contract: ThemeContractId,
    pub(crate) colors: Vec<ResolvedPaletteColor>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedPaletteSelection {
    Static(PaletteId),
    Dynamic(ResolvedAppExpression),
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedThemeFactory {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<Expr>,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedAppThemeFactory {
    pub(crate) function: ExternFnId,
    pub(crate) arguments: Vec<ResolvedAppExpression>,
    pub(crate) origin: OriginId,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedAppThemeSelection {
    App,
    Default,
    BuiltIn(String),
    Dynamic(ResolvedAppExpression),
    Factory(ResolvedAppThemeFactory),
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedThemePreset {
    Default,
    App,
    BuiltIn(String),
    Factory(ResolvedThemeFactory),
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedBackground {
    Color(ResolvedThemeColor),
    Linear {
        angle: Expr,
        stops: Vec<(ResolvedThemeColor, Expr)>,
    },
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedNestedTheme {
    pub(crate) preset: ResolvedThemePreset,
    pub(crate) text: Option<ResolvedThemeColor>,
    pub(crate) background: Option<ResolvedBackground>,
    pub(crate) origin: OriginId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct ResolvedThemeProgram {
    pub(crate) contract: ResolvedThemeContract,
    pub(crate) palettes: Vec<ResolvedPalette>,
    pub(crate) active_palette: ResolvedPaletteSelection,
    pub(crate) active_palette_origin: OriginId,
    pub(crate) app_theme: ResolvedAppThemeSelection,
    pub(crate) app_theme_origin: OriginId,
    pub(crate) native_tokens: ResolvedNativeThemeTokens,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ResolvedNativeThemeTokens {
    pub(crate) background: ThemeTokenId,
    pub(crate) text: ThemeTokenId,
    pub(crate) primary: ThemeTokenId,
    pub(crate) danger: ThemeTokenId,
}

#[derive(Debug)]
pub(super) struct StyleProgram {
    pub(super) theme: ResolvedThemeProgram,
    #[allow(dead_code)]
    pub(super) recipes: Vec<ResolvedRecipe>,
    pub(super) style_uses: Vec<ResolvedStyleUse>,
    style_uses_by_site: HashMap<CallSite, StyleUseId>,
    nested_themes: Vec<ResolvedNestedTheme>,
    nested_themes_by_site: HashMap<CallSite, usize>,
}

#[derive(Debug, Default)]
pub(super) struct StyleProgramBuilder {
    theme: Option<ResolvedThemeProgram>,
    recipes: Vec<ResolvedRecipe>,
    recipe_ids: HashMap<String, RecipeId>,
    token_ids: HashMap<String, ThemeTokenId>,
    palette_ids: HashMap<String, PaletteId>,
    theme_factory_ids: HashMap<String, ExternFnId>,
    style_uses: Vec<ResolvedStyleUse>,
    style_uses_by_site: HashMap<CallSite, StyleUseId>,
    nested_themes: Vec<ResolvedNestedTheme>,
    nested_themes_by_site: HashMap<CallSite, usize>,
}

impl StyleProgramBuilder {
    pub(super) fn finish(self) -> Option<StyleProgram> {
        Some(StyleProgram {
            theme: self.theme?,
            recipes: self.recipes,
            style_uses: self.style_uses,
            style_uses_by_site: self.style_uses_by_site,
            nested_themes: self.nested_themes,
            nested_themes_by_site: self.nested_themes_by_site,
        })
    }
}

impl StyleProgram {
    pub(super) fn style_use(&self, span: &Span) -> Result<&ResolvedStyleUse, Error> {
        let site = CallSite {
            line: span.line,
            column: span.column,
        };
        let id = self.style_uses_by_site.get(&site).ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "style use reached code generation without normalized style facts",
            )
        })?;
        self.style_uses.get(id.0 as usize).ok_or_else(|| {
            Error::new(
                "E196",
                span,
                "style use references an invalid normalized style ID",
            )
        })
    }

    pub(super) fn nested_theme(&self, span: &Span) -> Result<&ResolvedNestedTheme, Error> {
        let site = CallSite {
            line: span.line,
            column: span.column,
        };
        self.nested_themes_by_site
            .get(&site)
            .and_then(|index| self.nested_themes.get(*index))
            .ok_or_else(|| {
                Error::new(
                    "E196",
                    span,
                    "nested theme reached code generation without normalized theme facts",
                )
            })
    }
}

impl Lowerer {
    pub(super) fn lower_style_program(&mut self) -> Result<(), Error> {
        self.index_theme_factories()?;
        self.lower_theme_declarations()?;
        self.lower_recipes()?;
        Ok(())
    }

    fn index_theme_factories(&mut self) -> Result<(), Error> {
        for (index, function) in self.document.functions.iter().enumerate() {
            if function.kind != ExternKind::Theme {
                continue;
            }
            let id = self.declarations.extern_fn(index).id;
            if self
                .styles
                .theme_factory_ids
                .insert(function.name.clone(), id)
                .is_some()
            {
                return Err(self.invariant(
                    &function.span,
                    format!("duplicate checked theme factory `{}`", function.name),
                ));
            }
        }
        Ok(())
    }

    fn lower_theme_declarations(&mut self) -> Result<(), Error> {
        let source = self.document.theme_contract.clone().ok_or_else(|| {
            self.invariant(&Span::line(1), "checked document has no theme contract")
        })?;
        let contract_origin = self.push_origin(&source.span, None);
        let contract_id = ThemeContractId(0);
        let tokens = source
            .tokens
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let id = ThemeTokenId {
                    contract: contract_id,
                    index: index as u32,
                };
                self.styles.token_ids.insert(name.clone(), id);
                ResolvedThemeToken {
                    id,
                    name: name.clone(),
                    origin: self.push_origin(&source.span, Some(contract_origin)),
                }
            })
            .collect();
        let contract = ResolvedThemeContract {
            id: contract_id,
            name: source.name.clone(),
            tokens,
            origin: contract_origin,
        };
        let source_palettes = self.document.palettes.clone();
        for (index, palette) in source_palettes.iter().enumerate() {
            let id = self.declarations.palette(index).id;
            self.styles.palette_ids.insert(palette.name.clone(), id);
        }
        let mut palettes = Vec::with_capacity(source_palettes.len());
        for (index, palette) in source_palettes.iter().enumerate() {
            let declaration = self.declarations.palette(index);
            let id = declaration.id;
            let origin = declaration.origin;
            if palette.contract != source.name {
                return Err(self.invariant(
                    &palette.span,
                    format!(
                        "palette `{}` does not reference checked contract `{}`",
                        palette.name, source.name
                    ),
                ));
            }
            let colors = contract
                .tokens
                .iter()
                .map(|token| {
                    let value = palette.colors.get(&token.name).ok_or_else(|| {
                        self.invariant(
                            &palette.span,
                            format!(
                                "palette `{}` is missing checked token `{}`",
                                palette.name, token.name
                            ),
                        )
                    })?;
                    Ok(ResolvedPaletteColor {
                        token: token.id,
                        rgba: parse_palette_color(value).ok_or_else(|| {
                            self.invariant(
                                &palette.span,
                                format!(
                                    "palette `{}` contains invalid checked color `{value}`",
                                    palette.name
                                ),
                            )
                        })?,
                        origin: self.push_origin(&palette.span, Some(origin)),
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            palettes.push(ResolvedPalette {
                id,
                name: palette.name.clone(),
                contract: contract_id,
                colors,
                origin,
            });
        }
        let default_palette = palettes
            .first()
            .map(|palette| palette.id)
            .ok_or_else(|| self.invariant(&source.span, "checked theme has no palette"))?;
        let checked_settings = &self
            .facts
            .app_settings()
            .ok_or_else(|| {
                self.invariant(
                    &self.document.settings.span,
                    "application settings are missing their authoritative checked snapshot",
                )
            })?
            .source;
        let active_palette_origin = checked_settings
            .palette
            .as_ref()
            .and_then(|_| {
                self.declarations
                    .app_setting_expression(AppSettingExprId::Palette)
                    .map(|declaration| declaration.origin)
            })
            .unwrap_or_else(|| self.declarations.app_settings().origin);
        let active_palette = match checked_settings.palette.as_ref() {
            Some(setting) => {
                let expression =
                    self.checked_app_setting_expression(AppSettingExprId::Palette, &setting.span)?;
                let checked = self
                    .facts
                    .expression(self.facts.expression_use(expression.expression).root);
                match &checked.kind {
                    crate::check::CheckedExprKind::Path {
                        root: crate::check::CheckedPathRoot::Palette(id),
                        projections,
                    } if projections.is_empty() => ResolvedPaletteSelection::Static(*id),
                    _ => ResolvedPaletteSelection::Dynamic(expression),
                }
            }
            None => ResolvedPaletteSelection::Static(default_palette),
        };
        let app_theme_origin = checked_settings
            .theme
            .as_ref()
            .and_then(|_| {
                self.declarations
                    .app_setting_expression(AppSettingExprId::Theme)
                    .map(|declaration| declaration.origin)
            })
            .unwrap_or_else(|| self.declarations.app_settings().origin);
        let app_theme = self.lower_app_theme_selection()?;
        let native_token = |name: &str| {
            self.styles.token_ids.get(name).copied().ok_or_else(|| {
                self.invariant(
                    &source.span,
                    format!("checked theme contract is missing native token `{name}`"),
                )
            })
        };
        self.styles.theme = Some(ResolvedThemeProgram {
            contract,
            palettes,
            active_palette,
            active_palette_origin,
            app_theme,
            app_theme_origin,
            native_tokens: ResolvedNativeThemeTokens {
                background: native_token("bg")?,
                text: native_token("fg")?,
                primary: native_token("primary")?,
                danger: native_token("danger")?,
            },
        });
        Ok(())
    }

    fn lower_app_theme_selection(&mut self) -> Result<ResolvedAppThemeSelection, Error> {
        let checked_settings = self
            .facts
            .app_settings()
            .ok_or_else(|| {
                self.invariant(
                    &self.document.settings.span,
                    "application settings are missing their authoritative checked snapshot",
                )
            })?
            .source
            .clone();
        let Some(setting) = checked_settings.theme.as_ref() else {
            return Ok(ResolvedAppThemeSelection::App);
        };
        let span = setting.span.clone();
        if let Some(factory) = self.facts.app_theme_factory() {
            let function = factory.function;
            let argument_count = factory.arguments;
            let origin = self
                .declarations
                .app_setting_expression(AppSettingExprId::Theme)
                .ok_or_else(|| self.invariant(&span, "app theme factory has no origin"))?
                .origin;
            let arguments = (0..argument_count)
                .map(|index| {
                    self.checked_app_setting_expression(
                        AppSettingExprId::ThemeFactoryArgument(index),
                        &span,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(ResolvedAppThemeSelection::Factory(
                ResolvedAppThemeFactory {
                    function,
                    arguments,
                    origin,
                },
            ));
        }
        let expression = self.checked_app_setting_expression(AppSettingExprId::Theme, &span)?;
        let checked = self
            .facts
            .expression(self.facts.expression_use(expression.expression).root);
        match &checked.kind {
            crate::check::CheckedExprKind::Str(name) if name == "app" => {
                Ok(ResolvedAppThemeSelection::App)
            }
            crate::check::CheckedExprKind::Str(name) if name == "default" => {
                Ok(ResolvedAppThemeSelection::Default)
            }
            crate::check::CheckedExprKind::Str(name)
                if BUILT_IN_THEMES.contains(&name.as_str()) =>
            {
                Ok(ResolvedAppThemeSelection::BuiltIn(name.clone()))
            }
            _ => Ok(ResolvedAppThemeSelection::Dynamic(expression)),
        }
    }

    fn theme_factory_id(&self, name: &str) -> Option<ExternFnId> {
        self.styles.theme_factory_ids.get(name).copied()
    }

    fn lower_recipes(&mut self) -> Result<(), Error> {
        let source = self.document.recipes.clone();
        for (index, recipe) in source.iter().enumerate() {
            if self
                .styles
                .recipe_ids
                .insert(recipe.name.clone(), RecipeId(index as u32))
                .is_some()
            {
                return Err(self.invariant(
                    &recipe.span,
                    format!("duplicate checked recipe `{}`", recipe.name),
                ));
            }
        }
        let recipe_origins = source
            .iter()
            .map(|recipe| self.push_origin(&recipe.span, None))
            .collect::<Vec<_>>();
        let mut flattened = vec![None; source.len()];
        let mut declared = vec![None; source.len()];
        let mut visiting = HashSet::new();
        for index in 0..source.len() {
            self.flatten_recipe(
                index,
                &source,
                &recipe_origins,
                &mut flattened,
                &mut declared,
                &mut visiting,
            )?;
        }
        self.styles.recipes = source
            .iter()
            .enumerate()
            .map(|(index, recipe)| {
                Ok(ResolvedRecipe {
                    id: RecipeId(index as u32),
                    name: recipe.name.clone(),
                    target: ResolvedStyleTargetKind::from(recipe.target).id(),
                    base: recipe
                        .base
                        .as_ref()
                        .map(|name| {
                            self.styles.recipe_ids.get(name).copied().ok_or_else(|| {
                                self.invariant(
                                    &recipe.span,
                                    format!("unknown checked recipe base `{name}`"),
                                )
                            })
                        })
                        .transpose()?,
                    declared_utilities: declared[index].clone().ok_or_else(|| {
                        self.invariant(&recipe.span, "checked recipe body was not normalized")
                    })?,
                    style: flattened[index].clone().ok_or_else(|| {
                        self.invariant(&recipe.span, "checked recipe was not flattened")
                    })?,
                    origin: recipe_origins[index],
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(())
    }

    fn flatten_recipe(
        &mut self,
        index: usize,
        source: &[StyleRecipe],
        origins: &[OriginId],
        flattened: &mut [Option<ResolvedStyle>],
        declared: &mut [Option<Vec<ResolvedUtility>>],
        visiting: &mut HashSet<usize>,
    ) -> Result<ResolvedStyle, Error> {
        if let Some(style) = &flattened[index] {
            return Ok(style.clone());
        }
        if !visiting.insert(index) {
            return Err(self.invariant(
                &source[index].span,
                format!("checked recipe cycle includes `{}`", source[index].name),
            ));
        }
        let recipe = &source[index];
        let origin = origins[index];
        let mut style = if let Some(base) = &recipe.base {
            let id = self.styles.recipe_ids.get(base).copied().ok_or_else(|| {
                self.invariant(
                    &recipe.span,
                    format!("unknown checked recipe base `{base}`"),
                )
            })?;
            if source[id.0 as usize].target != recipe.target {
                return Err(self.invariant(
                    &recipe.span,
                    format!(
                        "recipe `{}` targets `{}` but its checked base `{base}` targets `{}`",
                        recipe.name,
                        recipe.target.source_name(),
                        source[id.0 as usize].target.source_name()
                    ),
                ));
            }
            self.flatten_recipe(
                id.0 as usize,
                source,
                origins,
                flattened,
                declared,
                visiting,
            )?
        } else {
            ResolvedStyle::default()
        };
        let mut own = Vec::with_capacity(recipe.utilities.len());
        for utility in &recipe.utilities {
            let utility = self.resolve_utility(utility, origin, &recipe.span)?;
            style.apply(&utility);
            own.push(utility);
        }
        visiting.remove(&index);
        declared[index] = Some(own);
        flattened[index] = Some(style.clone());
        Ok(style)
    }

    fn resolve_utility(
        &self,
        source: &str,
        origin: OriginId,
        span: &Span,
    ) -> Result<ResolvedUtility, Error> {
        let (variant, utility) = match source.split_once(':') {
            Some(("hover", utility)) => (ResolvedStyleVariantKind::Hovered, utility),
            Some(("pressed", utility)) => (ResolvedStyleVariantKind::Pressed, utility),
            Some(("focus", utility)) => (ResolvedStyleVariantKind::Focused, utility),
            Some(("disabled", utility)) => (ResolvedStyleVariantKind::Disabled, utility),
            Some((variant, _)) => {
                return Err(self.invariant(
                    span,
                    format!("unknown checked utility variant `{variant}` in `{source}`"),
                ));
            }
            None => (ResolvedStyleVariantKind::Base, source),
        };
        let pair = match utility {
            "w-full" => (ResolvedStyleProperty::WidthFill, ResolvedUtilityValue::Flag),
            "h-full" => (
                ResolvedStyleProperty::HeightFill,
                ResolvedUtilityValue::Flag,
            ),
            "max-w-sm" => (
                ResolvedStyleProperty::MaxWidth,
                ResolvedUtilityValue::Pixels(384),
            ),
            "max-w-md" => (
                ResolvedStyleProperty::MaxWidth,
                ResolvedUtilityValue::Pixels(448),
            ),
            "max-w-lg" => (
                ResolvedStyleProperty::MaxWidth,
                ResolvedUtilityValue::Pixels(512),
            ),
            "max-w-xl" => (
                ResolvedStyleProperty::MaxWidth,
                ResolvedUtilityValue::Pixels(576),
            ),
            "max-w-2xl" => (
                ResolvedStyleProperty::MaxWidth,
                ResolvedUtilityValue::Pixels(672),
            ),
            "items-center" => (
                ResolvedStyleProperty::ItemsCenter,
                ResolvedUtilityValue::Flag,
            ),
            "self-center" => (
                ResolvedStyleProperty::SelfCenter,
                ResolvedUtilityValue::Flag,
            ),
            "overflow-hidden" => (ResolvedStyleProperty::Clip, ResolvedUtilityValue::Flag),
            "text-xs" => (
                ResolvedStyleProperty::TextSize,
                ResolvedUtilityValue::TextSize(12.0),
            ),
            "text-sm" => (
                ResolvedStyleProperty::TextSize,
                ResolvedUtilityValue::TextSize(14.0),
            ),
            "text-base" => (
                ResolvedStyleProperty::TextSize,
                ResolvedUtilityValue::TextSize(16.0),
            ),
            "text-lg" => (
                ResolvedStyleProperty::TextSize,
                ResolvedUtilityValue::TextSize(18.0),
            ),
            "text-xl" => (
                ResolvedStyleProperty::TextSize,
                ResolvedUtilityValue::TextSize(20.0),
            ),
            "text-2xl" => (
                ResolvedStyleProperty::TextSize,
                ResolvedUtilityValue::TextSize(24.0),
            ),
            "leading-tight" => (
                ResolvedStyleProperty::TextLineHeight,
                ResolvedUtilityValue::LineHeight(1.2),
            ),
            "leading-snug" => (
                ResolvedStyleProperty::TextLineHeight,
                ResolvedUtilityValue::LineHeight(1.35),
            ),
            "leading-normal" => (
                ResolvedStyleProperty::TextLineHeight,
                ResolvedUtilityValue::LineHeight(1.5),
            ),
            "leading-relaxed" => (
                ResolvedStyleProperty::TextLineHeight,
                ResolvedUtilityValue::LineHeight(1.65),
            ),
            "font-mono" => (
                ResolvedStyleProperty::FontMonospace,
                ResolvedUtilityValue::Flag,
            ),
            "font-medium" => (
                ResolvedStyleProperty::FontWeight,
                ResolvedUtilityValue::FontWeight(ResolvedStyleFontWeight::Medium),
            ),
            "font-semibold" => (
                ResolvedStyleProperty::FontWeight,
                ResolvedUtilityValue::FontWeight(ResolvedStyleFontWeight::Semibold),
            ),
            "font-bold" => (
                ResolvedStyleProperty::FontWeight,
                ResolvedUtilityValue::FontWeight(ResolvedStyleFontWeight::Bold),
            ),
            "border" => (
                ResolvedStyleProperty::BorderWidth,
                ResolvedUtilityValue::Pixels(1),
            ),
            "border-2" => (
                ResolvedStyleProperty::BorderWidth,
                ResolvedUtilityValue::Pixels(2),
            ),
            "rounded-sm" => (
                ResolvedStyleProperty::Radius,
                ResolvedUtilityValue::Pixels(2),
            ),
            "rounded" | "rounded-md" => (
                ResolvedStyleProperty::Radius,
                ResolvedUtilityValue::Pixels(6),
            ),
            "rounded-lg" => (
                ResolvedStyleProperty::Radius,
                ResolvedUtilityValue::Pixels(10),
            ),
            "rounded-full" => (
                ResolvedStyleProperty::Radius,
                ResolvedUtilityValue::Pixels(999),
            ),
            value if value.starts_with("text-") && exact_text_size(&value[5..]).is_some() => (
                ResolvedStyleProperty::TextSize,
                ResolvedUtilityValue::TextSize(exact_text_size(&value[5..]).unwrap()),
            ),
            value if value.starts_with("rounded-") => {
                let pixels = exact_radius(&value[8..]).ok_or_else(|| {
                    self.invariant(span, format!("invalid checked radius utility `{source}`"))
                })?;
                (
                    ResolvedStyleProperty::Radius,
                    ResolvedUtilityValue::Pixels(pixels),
                )
            }
            value if value.starts_with("gap-") => {
                let pixels = spacing(&value[4..]).ok_or_else(|| {
                    self.invariant(span, format!("invalid checked gap utility `{source}`"))
                })?;
                (
                    ResolvedStyleProperty::Gap,
                    ResolvedUtilityValue::Pixels(pixels),
                )
            }
            value if value.starts_with("p-") => {
                let value = spacing(&value[2..]).ok_or_else(|| {
                    self.invariant(span, format!("invalid checked padding utility `{source}`"))
                })?;
                (
                    ResolvedStyleProperty::Padding,
                    ResolvedUtilityValue::Padding([Some(value); 4]),
                )
            }
            value if value.starts_with("px-") => {
                let value = spacing(&value[3..]).ok_or_else(|| {
                    self.invariant(span, format!("invalid checked padding utility `{source}`"))
                })?;
                (
                    ResolvedStyleProperty::Padding,
                    ResolvedUtilityValue::Padding([None, Some(value), None, Some(value)]),
                )
            }
            value if value.starts_with("py-") => {
                let value = spacing(&value[3..]).ok_or_else(|| {
                    self.invariant(span, format!("invalid checked padding utility `{source}`"))
                })?;
                (
                    ResolvedStyleProperty::Padding,
                    ResolvedUtilityValue::Padding([Some(value), None, Some(value), None]),
                )
            }
            value if value.starts_with("bg-") => (
                ResolvedStyleProperty::Background,
                ResolvedUtilityValue::Color(self.resolve_theme_color(&value[3..], span)?),
            ),
            value if value.starts_with("text-") => (
                ResolvedStyleProperty::TextColor,
                ResolvedUtilityValue::Color(self.resolve_theme_color(&value[5..], span)?),
            ),
            value if value.starts_with("border-") => (
                ResolvedStyleProperty::BorderColor,
                ResolvedUtilityValue::Color(self.resolve_theme_color(&value[7..], span)?),
            ),
            value if value.starts_with("opacity-") => {
                let opacity = value[8..].parse::<f32>().ok().ok_or_else(|| {
                    self.invariant(span, format!("invalid checked opacity utility `{source}`"))
                })? / 100.0;
                (
                    ResolvedStyleProperty::Opacity,
                    ResolvedUtilityValue::Opacity(opacity),
                )
            }
            _ => return Err(self.invariant(span, format!("unknown checked utility `{source}`"))),
        };
        Ok(ResolvedUtility {
            variant: variant.id(),
            property: pair.0,
            value: pair.1,
            origin,
        })
    }

    fn resolve_theme_color(&self, value: &str, span: &Span) -> Result<ResolvedThemeColor, Error> {
        let (name, opacity) = value
            .split_once('/')
            .map_or((value, None), |(name, value)| (name, Some(value)));
        let base = match name {
            "white" => ResolvedThemeColorBase::White,
            "black" => ResolvedThemeColorBase::Black,
            "transparent" => ResolvedThemeColorBase::Transparent,
            name => {
                ResolvedThemeColorBase::Token(self.styles.token_ids.get(name).copied().ok_or_else(
                    || self.invariant(span, format!("unknown checked theme token `{name}`")),
                )?)
            }
        };
        let opacity = opacity
            .map(|value| {
                value
                    .parse::<u8>()
                    .ok()
                    .filter(|value| *value <= 100)
                    .ok_or_else(|| {
                        self.invariant(span, format!("invalid checked color opacity `{value}`"))
                    })
            })
            .transpose()?;
        Ok(ResolvedThemeColor { base, opacity })
    }

    fn lower_style_use(
        &mut self,
        styles: &[String],
        target: ResolvedStyleTargetKind,
        span: &Span,
    ) -> Result<(), Error> {
        let origin = self.push_origin(span, None);
        let mut recipes = Vec::new();
        let mut utilities = Vec::new();
        let mut style = ResolvedStyle::default();
        for source in styles {
            if let Some(id) = self.styles.recipe_ids.get(source).copied() {
                let recipe = &self.styles.recipes[id.0 as usize];
                if recipe.target != target.id() {
                    return Err(self.invariant(
                        span,
                        format!("checked recipe `{source}` has an incompatible target"),
                    ));
                }
                recipes.push(id);
                style.overlay(&recipe.style);
            } else {
                let utility =
                    self.resolve_utility(crate::unqualified_name(source), origin, span)?;
                style.apply(&utility);
                utilities.push(utility);
            }
        }
        let id = StyleUseId(self.styles.style_uses.len() as u32);
        let site = CallSite {
            line: span.line,
            column: span.column,
        };
        if self.styles.style_uses_by_site.insert(site, id).is_some() {
            return Err(self.invariant(span, "style use source identity is not unique"));
        }
        self.styles.style_uses.push(ResolvedStyleUse {
            id,
            target: target.id(),
            recipes,
            utilities,
            style,
            origin,
        });
        Ok(())
    }

    pub(super) fn lower_view_style(&mut self, node: &ViewNode) -> Result<(), Error> {
        match node {
            ViewNode::Layout {
                kind,
                options,
                styles,
                span,
                ..
            } => {
                let target = if options.flexbox.is_some() {
                    ResolvedStyleTargetKind::Flex
                } else {
                    match kind {
                        Layout::Column => ResolvedStyleTargetKind::Column,
                        Layout::Row => ResolvedStyleTargetKind::Row,
                        Layout::Grid => ResolvedStyleTargetKind::Grid,
                        Layout::Stack => ResolvedStyleTargetKind::Stack,
                        Layout::Scroll => ResolvedStyleTargetKind::Scroll,
                    }
                };
                self.lower_style_use(styles, target, span)?;
            }
            ViewNode::Container { styles, span, .. } => {
                self.lower_style_use(styles, ResolvedStyleTargetKind::Container, span)?;
            }
            ViewNode::Text { styles, span, .. } => {
                self.lower_style_use(styles, ResolvedStyleTargetKind::Text, span)?;
            }
            ViewNode::RichText {
                styles,
                spans,
                span,
                ..
            } => {
                self.lower_style_use(styles, ResolvedStyleTargetKind::Text, span)?;
                for item in spans {
                    self.lower_style_use(&item.styles, ResolvedStyleTargetKind::Text, &item.span)?;
                }
            }
            ViewNode::Input { styles, span, .. } => {
                self.lower_style_use(styles, ResolvedStyleTargetKind::Input, span)?;
            }
            ViewNode::Button { styles, span, .. } => {
                self.lower_style_use(styles, ResolvedStyleTargetKind::Button, span)?;
            }
            ViewNode::PaneGrid {
                panes, templates, ..
            } => {
                for pane in panes
                    .iter()
                    .chain(templates.iter().map(|template| &template.pane))
                {
                    self.lower_style_use(
                        &pane.styles,
                        ResolvedStyleTargetKind::PaneContent,
                        &pane.span,
                    )?;
                    if let Some(title) = &pane.title {
                        self.lower_style_use(
                            &title.styles,
                            ResolvedStyleTargetKind::PaneTitle,
                            &title.span,
                        )?;
                    }
                }
            }
            ViewNode::Theme {
                preset,
                text,
                background,
                span,
                ..
            } => self.lower_nested_theme(preset, text, background, span)?,
            _ => {}
        }
        Ok(())
    }

    pub(super) fn lower_nested_theme(
        &mut self,
        preset: &ThemePreset,
        text: &Option<String>,
        background: &Option<BackgroundValue>,
        span: &Span,
    ) -> Result<(), Error> {
        let origin = self.push_origin(span, None);
        let preset = match preset {
            ThemePreset::Default => ResolvedThemePreset::Default,
            ThemePreset::App => ResolvedThemePreset::App,
            ThemePreset::BuiltIn(name) => ResolvedThemePreset::BuiltIn(name.clone()),
            ThemePreset::Factory(factory) => ResolvedThemePreset::Factory(ResolvedThemeFactory {
                function: self.theme_factory_id(&factory.function).ok_or_else(|| {
                    self.invariant(
                        span,
                        format!("unknown checked theme factory `{}`", factory.function),
                    )
                })?,
                arguments: factory.args.clone(),
                origin,
            }),
        };
        let text = text
            .as_ref()
            .map(|value| self.resolve_theme_color(value, span))
            .transpose()?;
        let background = background
            .as_ref()
            .map(|value| self.resolve_background(value, span))
            .transpose()?;
        let index = self.styles.nested_themes.len();
        let site = CallSite {
            line: span.line,
            column: span.column,
        };
        if self
            .styles
            .nested_themes_by_site
            .insert(site, index)
            .is_some()
        {
            return Err(self.invariant(span, "nested theme source identity is not unique"));
        }
        self.styles.nested_themes.push(ResolvedNestedTheme {
            preset,
            text,
            background,
            origin,
        });
        Ok(())
    }

    fn resolve_background(
        &self,
        value: &BackgroundValue,
        span: &Span,
    ) -> Result<ResolvedBackground, Error> {
        Ok(match value {
            BackgroundValue::Color(color) => {
                ResolvedBackground::Color(self.resolve_theme_color(color, span)?)
            }
            BackgroundValue::Linear { angle, stops } => ResolvedBackground::Linear {
                angle: angle.clone(),
                stops: stops
                    .iter()
                    .map(|stop| {
                        Ok((
                            self.resolve_theme_color(&stop.color, span)?,
                            stop.offset.clone(),
                        ))
                    })
                    .collect::<Result<Vec<_>, Error>>()?,
            },
        })
    }
}

fn spacing(value: &str) -> Option<u16> {
    match value {
        "0" => Some(0),
        "1" => Some(4),
        "2" => Some(8),
        "3" => Some(12),
        "4" => Some(16),
        "5" => Some(20),
        "6" => Some(24),
        "8" => Some(32),
        "10" => Some(40),
        "12" => Some(48),
        "16" => Some(64),
        "20" => Some(80),
        "24" => Some(96),
        value => value.strip_suffix("px")?.parse().ok(),
    }
}

fn exact_radius(value: &str) -> Option<u16> {
    value
        .strip_suffix("px")?
        .parse()
        .ok()
        .filter(|value| *value > 0)
}

fn exact_text_size(value: &str) -> Option<f32> {
    value
        .strip_suffix("px")?
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn parse_palette_color(value: &str) -> Option<[u8; 4]> {
    let value = value.strip_prefix('#')?;
    if !matches!(value.len(), 6 | 8) {
        return None;
    }
    let channel = |start| u8::from_str_radix(&value[start..start + 2], 16).ok();
    Some([
        channel(0)?,
        channel(2)?,
        channel(4)?,
        if value.len() == 8 { channel(6)? } else { 255 },
    ])
}
