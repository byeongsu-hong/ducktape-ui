#[cfg(test)]
pub type SliderNumber = f32;

#[cfg(test)]
pub type AppRenderer = iced::Renderer;

#[cfg(test)]
pub fn daemon_title(_: iced::window::Id) -> String {
    "Background agent".into()
}

#[cfg(test)]
pub fn daemon_theme(_: iced::window::Id) -> iced::Theme {
    iced::Theme::Dark
}

#[cfg(test)]
pub fn daemon_scale(_: iced::window::Id) -> f64 {
    1.0
}

#[cfg(test)]
pub fn keyboard_value(
    key: iced::keyboard::Key,
    _: iced::keyboard::key::Physical,
    _: iced::keyboard::Location,
    _: iced::keyboard::Modifiers,
) -> iced::keyboard::Key {
    key
}

#[cfg(test)]
pub fn pointer_click(
    click: iced::advanced::mouse::Click,
    _: iced::mouse::Cursor,
    _: iced::mouse::Button,
    _: iced::touch::Finger,
    _: iced::Point,
    _: iced::Rectangle,
) -> iced::advanced::mouse::Click {
    click
}

#[cfg(test)]
pub fn transformation_round_trip(
    value: iced::Transformation,
    _: iced::Vector,
    _: iced::Size,
) -> iced::Transformation {
    value
}

#[cfg(test)]
pub fn exact_rectangle() -> iced::Rectangle<u32> {
    iced::Rectangle {
        x: 1,
        y: 2,
        width: 3,
        height: 4,
    }
}

#[cfg(test)]
pub fn geometry_round_trip(
    _: iced::Point,
    _: iced::Point<u32>,
    _: iced::Vector,
    _: iced::Size,
    bounds: iced::Rectangle,
    _: Option<iced::Rectangle<u32>>,
) -> iced::Rectangle {
    bounds
}

#[cfg(test)]
pub fn unit_round_trip(
    _: iced::Pixels,
    padding: iced::Padding,
    _: iced::Degrees,
    _: iced::Radians,
) -> iced::Padding {
    padding
}

#[cfg(test)]
pub use crate::alignment::{alignment_round_trip, horizontal_round_trip, vertical_round_trip};

#[cfg(test)]
pub use crate::background_gradient::{
    background_round_trip, color_stop_round_trip, gradient_round_trip, linear_round_trip,
};

#[cfg(test)]
pub use crate::border_radius::{border_round_trip, radius_round_trip};

#[cfg(test)]
pub use crate::color::color_round_trip;

#[cfg(test)]
pub use crate::content_fit::content_fit_round_trip;

#[cfg(test)]
pub use crate::event_status::status_round_trip;

#[cfg(test)]
pub use crate::font_values::{
    family_round_trip, font_round_trip, stretch_round_trip, style_round_trip, weight_round_trip,
};

#[cfg(test)]
pub use crate::length::length_round_trip;

#[cfg(test)]
pub use crate::mouse_interaction::interaction_round_trip;

#[cfg(test)]
pub use crate::redraw_request::{redraw_now, redraw_round_trip};

#[cfg(test)]
pub use crate::rotation::rotation_round_trip;

#[cfg(test)]
pub use crate::scroll_delta::scroll_delta_round_trip;

#[cfg(test)]
pub use crate::shadow::shadow_round_trip;

#[cfg(test)]
pub use crate::theme_mode::theme_mode_round_trip;

/// Rows for the virtualized-column test: zero-padded so a test can name one by
/// value, and uniform so every row's height equals the column's estimate.
#[cfg(test)]
pub fn virtual_rows(count: i64) -> Vec<String> {
    (0..count.max(0))
        .map(|row| format!("row {row:03}"))
        .collect()
}

#[cfg(test)]
pub use crate::text_values::{
    text_alignment_round_trip, text_line_height_round_trip, text_shaping_round_trip,
    text_wrapping_round_trip,
};

#[cfg(test)]
pub use crate::window_id::window_id_round_trip;

#[cfg(test)]
pub use crate::window_screenshot::{
    screenshot_crop_region, screenshot_outside_region, screenshot_round_trip, screenshot_sample,
    screenshot_size, screenshot_zero_region,
};

#[cfg(test)]
pub use crate::window_values::{
    attention_round_trip, direction_round_trip, level_round_trip, mode_round_trip,
};

#[cfg(test)]
pub use crate::window_position::{position_round_trip, responsive_position};

#[cfg(test)]
pub fn elastic(value: f64) -> f64 {
    value
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub value: f64,
}

#[cfg(test)]
impl iced::animation::Float for Motion {
    fn float_value(&self) -> f32 {
        self.value as f32
    }
}

#[cfg(test)]
pub fn motion(value: f64) -> Motion {
    Motion { value }
}

#[cfg(test)]
pub fn native_theme(dark: bool) -> iced::Theme {
    let palette = if dark {
        iced::theme::Palette::DARK
    } else {
        iced::theme::Palette::LIGHT
    };
    iced::Theme::custom_with_fn(
        if dark { "Native dark" } else { "Native light" },
        palette,
        move |palette| {
            let mut extended = iced::theme::palette::Extended::generate(palette);
            extended.primary.base.color = if dark {
                iced::Color::from_rgb8(0x7c, 0x3a, 0xed)
            } else {
                iced::Color::from_rgb8(0x25, 0x63, 0xeb)
            };
            extended
        },
    )
}

#[cfg(test)]
#[derive(Clone)]
pub struct AlternateTheme {
    active: bool,
}

#[cfg(test)]
pub fn alternate_theme(active: bool) -> AlternateTheme {
    AlternateTheme { active }
}

#[cfg(test)]
pub async fn load_alternate_theme() -> AlternateTheme {
    alternate_theme(true)
}

#[cfg(test)]
impl iced::theme::Base for AlternateTheme {
    fn default(preference: iced::theme::Mode) -> Self {
        Self {
            active: preference == iced::theme::Mode::Dark,
        }
    }

    fn mode(&self) -> iced::theme::Mode {
        if self.active {
            iced::theme::Mode::Dark
        } else {
            iced::theme::Mode::Light
        }
    }

    fn base(&self) -> iced::theme::Style {
        iced::theme::Style {
            background_color: if self.active {
                iced::Color::BLACK
            } else {
                iced::Color::WHITE
            },
            text_color: if self.active {
                iced::Color::WHITE
            } else {
                iced::Color::BLACK
            },
        }
    }

    fn palette(&self) -> Option<iced::theme::Palette> {
        None
    }

    fn name(&self) -> &str {
        if self.active {
            "Alternate dark"
        } else {
            "Alternate light"
        }
    }
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
pub fn alternate_panel(
    active: bool,
) -> (
    Option<AlternateTheme>,
    iced::Element<'static, (), AlternateTheme>,
    Option<fn(&AlternateTheme) -> iced::Color>,
    Option<fn(&AlternateTheme) -> iced::Background>,
) {
    let content = iced::widget::Space::new().width(24).height(24).into();
    (
        active.then_some(AlternateTheme { active }),
        content,
        active.then_some(
            (|theme| iced::theme::Base::base(theme).text_color)
                as fn(&AlternateTheme) -> iced::Color,
        ),
        active.then_some(
            (|theme| iced::theme::Base::base(theme).background_color.into())
                as fn(&AlternateTheme) -> iced::Background,
        ),
    )
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkError {
    pub message: String,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
pub struct EditorCommand {
    pub save: bool,
}

#[cfg(test)]
#[derive(Debug)]
pub struct DemoHighlighter {
    token: String,
    line: usize,
}

#[cfg(test)]
impl iced::advanced::text::Highlighter for DemoHighlighter {
    type Settings = String;
    type Highlight = ();
    type Iterator<'a> = std::option::IntoIter<(std::ops::Range<usize>, ())>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            token: settings.clone(),
            line: 0,
        }
    }

    fn update(&mut self, settings: &Self::Settings) {
        self.token.clone_from(settings);
        self.line = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.line += 1;
        line.find(&self.token)
            .map(|start| (start..start + self.token.len(), ()))
            .into_iter()
    }

    fn current_line(&self) -> usize {
        self.line
    }
}

pub fn virtual_nums(count: i64) -> Vec<i64> {
    (0..count.max(0)).collect()
}

pub fn virtual_label(n: i64) -> String {
    format!("row {n}")
}

#[cfg(test)]
use crate::backend::Task;

#[cfg(test)]
pub fn seeded_tasks() -> Vec<Task> {
    vec![
        Task {
            id: 1,
            title: "Seeded one".into(),
            done: false,
        },
        Task {
            id: 2,
            title: "Seeded two".into(),
            done: false,
        },
    ]
}

#[cfg(test)]
pub fn retitled(mut tasks: Vec<Task>, id: i64, title: String) -> Vec<Task> {
    if let Some(task) = tasks.iter_mut().find(|task| task.id == id) {
        task.title = title;
    }
    tasks
}

#[cfg(test)]
pub fn toggled(mut tasks: Vec<Task>, id: i64) -> Vec<Task> {
    if let Some(task) = tasks.iter_mut().find(|task| task.id == id) {
        task.done = !task.done;
    }
    tasks
}

#[cfg(test)]
static HANDLER_MATCH_EVALUATIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
pub fn handler_match_expensive(values: Vec<String>) -> i64 {
    HANDLER_MATCH_EVALUATIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    values.len() as i64
}

#[cfg(test)]
pub fn reset_handler_match_evaluations() {
    HANDLER_MATCH_EVALUATIONS.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub fn handler_match_evaluations() -> usize {
    HANDLER_MATCH_EVALUATIONS.load(std::sync::atomic::Ordering::Relaxed)
}
