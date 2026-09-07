//! Copied native tooltip options and concrete container style.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum TooltipPosition {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
    FollowCursor,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum TooltipPreset {
    #[default]
    Transparent,
    Rounded,
    Bordered,
    Dark,
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TooltipStyle {
    pub preset: TooltipPreset,
    pub background: Option<Rgba>,
    pub text: Option<Rgba>,
    pub border: Option<Border>,
    pub shadow_color: Option<Rgba>,
    pub shadow_x: Option<f32>,
    pub shadow_y: Option<f32>,
    pub shadow_blur: Option<f32>,
    pub pixel_snap: Option<bool>,
}

impl TooltipStyle {
    pub(super) fn sanitize(&mut self) {
        bound_color(&mut self.background);
        bound_color(&mut self.text);
        bound_color(&mut self.shadow_color);
        bound_border(&mut self.border);
        bound_optional(&mut self.shadow_blur);
        for value in [&mut self.shadow_x, &mut self.shadow_y]
            .into_iter()
            .flatten()
        {
            *value = if value.is_finite() {
                value.clamp(-MAX_PIXELS, MAX_PIXELS)
            } else {
                0.0
            };
        }
    }
}
