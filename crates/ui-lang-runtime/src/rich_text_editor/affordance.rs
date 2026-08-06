//! Block affordances over the rich layout: the hover gutter ("+" and the
//! drag-dots handle) and the anchored dropdown menu (slash / block menus).
//!
//! Geometry lives here so `update`, `draw`, and `mouse_interaction` resolve
//! the exact same rectangles. Everything is in ABSOLUTE coordinates: callers
//! pass the shrunken `text_bounds` and the current scroll.

use iced::advanced::text::{self, Renderer as _, Text};
use iced::advanced::{Renderer as _, renderer};
use iced::{Color, Font, Pixels, Point, Rectangle, Size, alignment, border};

/// One of the two hover-gutter buttons beside a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GutterButton {
    /// The "+" — insert a block below this line.
    Plus,
    /// The dots handle — open the block menu for this line.
    Handle,
}

/// What the anchored menu reported back to the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEvent {
    /// Hover or arrow keys moved the highlighted row.
    Select(usize),
    /// A row was chosen — carries the item's `tag`.
    Pick(String),
    /// Escape or a press outside the panel.
    Dismiss,
}

/// One row of the anchored menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    /// The stable identifier handed back by [`MenuEvent::Pick`].
    pub tag: String,
    /// The visible row label.
    pub label: String,
}

/// Where the anchored menu hangs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAnchor {
    /// Below the caret — a slash menu.
    Caret,
    /// Below the first row of a source line — a block menu.
    Line(usize),
}

/// The anchored dropdown, fully described by the application each frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorMenu {
    /// Where the panel hangs.
    pub anchor: MenuAnchor,
    /// The rows, top to bottom.
    pub items: Vec<MenuItem>,
    /// The highlighted row.
    pub selected: usize,
}

pub(super) const MENU_WIDTH: f32 = 232.0;
pub(super) const MENU_ROW_HEIGHT: f32 = 27.0;
pub(super) const MENU_PADDING: f32 = 5.0;
const MENU_GAP: f32 = 4.0;
const MENU_LABEL_SIZE: f32 = 13.5;

const BUTTON_SIZE: f32 = 18.0;
const BUTTON_GAP: f32 = 2.0;
const GUTTER_INSET: f32 = 6.0;
/// The left padding a host must reserve for the gutter to fit.
pub const GUTTER_WIDTH: f32 = GUTTER_INSET + BUTTON_SIZE * 2.0 + BUTTON_GAP + 2.0;

/// The two gutter buttons beside a line whose first row is `row` (the
/// column-0 caret rectangle, already translated to absolute coordinates).
/// They sit right-aligned against the text edge, inside the host's left
/// padding, vertically centered on the first visual row.
pub(super) fn gutter_buttons(
    text_bounds: Rectangle,
    row: Rectangle,
) -> [(GutterButton, Rectangle); 2] {
    let center = row.y + row.height / 2.0;
    let top = center - BUTTON_SIZE / 2.0;
    let handle_x = text_bounds.x - GUTTER_INSET - BUTTON_SIZE;
    let plus_x = handle_x - BUTTON_GAP - BUTTON_SIZE;
    [
        (
            GutterButton::Plus,
            Rectangle::new(Point::new(plus_x, top), Size::new(BUTTON_SIZE, BUTTON_SIZE)),
        ),
        (
            GutterButton::Handle,
            Rectangle::new(
                Point::new(handle_x, top),
                Size::new(BUTTON_SIZE, BUTTON_SIZE),
            ),
        ),
    ]
}

/// The menu panel below `anchor`, flipped above it when the space under runs
/// out, clamped into `text_bounds` horizontally.
pub(super) fn menu_panel(
    anchor: Rectangle,
    item_count: usize,
    text_bounds: Rectangle,
) -> Rectangle {
    let height = MENU_ROW_HEIGHT * item_count as f32 + MENU_PADDING * 2.0;
    let below = anchor.y + anchor.height + MENU_GAP;
    let fits_below = below + height <= text_bounds.y + text_bounds.height;
    let y = match fits_below {
        true => below,
        false => anchor.y - MENU_GAP - height,
    };
    let max_x = text_bounds.x + text_bounds.width - MENU_WIDTH;
    let x = anchor.x.min(max_x).max(text_bounds.x);
    Rectangle::new(Point::new(x, y), Size::new(MENU_WIDTH, height))
}

/// The row index under `point`, if it is over one.
pub(super) fn menu_row_at(panel: Rectangle, item_count: usize, point: Point) -> Option<usize> {
    if !panel.contains(point) {
        return None;
    }
    let offset = point.y - panel.y - MENU_PADDING;
    if offset < 0.0 {
        return None;
    }
    let row = (offset / MENU_ROW_HEIGHT) as usize;
    (row < item_count).then_some(row)
}

/// A live handle drag: the grabbed source line, and the boundary the block
/// would land before once the pointer has actually moved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct GutterDrag {
    pub(super) from: usize,
    pub(super) boundary: Option<usize>,
    pub(super) moved: bool,
    pub(super) grab_y: f32,
}

/// Movement below this many pixels stays a click.
pub(super) const DRAG_THRESHOLD: f32 = 4.0;

/// The drop boundary nearest `y`, from `(boundary, absolute y)` candidates.
pub(super) fn snap_boundary(candidates: &[(usize, f32)], y: f32) -> Option<usize> {
    candidates
        .iter()
        .min_by(|left, right| (left.1 - y).abs().total_cmp(&(right.1 - y).abs()))
        .map(|(boundary, _)| *boundary)
}

/// The accent line marking where a dragged block would land.
pub(super) fn draw_drop_indicator(
    renderer: &mut iced::Renderer,
    text_bounds: Rectangle,
    y: f32,
    color: Color,
) {
    renderer.fill_quad(
        renderer::Quad {
            bounds: Rectangle::new(
                Point::new(text_bounds.x, y - 1.0),
                Size::new(text_bounds.width, 2.0),
            ),
            border: border::rounded(1.0),
            ..renderer::Quad::default()
        },
        color,
    );
}

pub(super) struct MenuColors {
    pub(super) panel: Color,
    pub(super) outline: Color,
    pub(super) selected: Color,
    pub(super) label: Color,
}

/// Paint the panel and its rows. The caller pushes the layer.
pub(super) fn draw_menu(
    renderer: &mut iced::Renderer,
    panel: Rectangle,
    items: &[MenuItem],
    selected: usize,
    font: Font,
    colors: &MenuColors,
) {
    renderer.fill_quad(
        renderer::Quad {
            bounds: panel,
            border: border::rounded(8).color(colors.outline).width(1.0),
            shadow: iced::Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.18),
                offset: iced::Vector::new(0.0, 3.0),
                blur_radius: 14.0,
            },
            ..renderer::Quad::default()
        },
        colors.panel,
    );
    for (index, item) in items.iter().enumerate() {
        let row = Rectangle::new(
            Point::new(
                panel.x + MENU_PADDING,
                panel.y + MENU_PADDING + MENU_ROW_HEIGHT * index as f32,
            ),
            Size::new(panel.width - MENU_PADDING * 2.0, MENU_ROW_HEIGHT),
        );
        if index == selected {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: row,
                    border: border::rounded(5),
                    ..renderer::Quad::default()
                },
                colors.selected,
            );
        }
        renderer.fill_text(
            Text {
                content: item.label.clone(),
                bounds: Size::new(row.width - 16.0, row.height),
                size: Pixels(MENU_LABEL_SIZE),
                line_height: text::LineHeight::default(),
                font,
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Center,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::None,
            },
            Point::new(row.x + 8.0, row.center_y()),
            colors.label,
            row,
        );
    }
}

/// Paint the "+" and the six-dot handle as bare quads — no font dependency,
/// so the affordance renders identically on every platform.
pub(super) fn draw_gutter(
    renderer: &mut iced::Renderer,
    buttons: &[(GutterButton, Rectangle); 2],
    color: Color,
) {
    for (button, bounds) in buttons {
        match button {
            GutterButton::Plus => {
                let stroke = 1.6;
                let arm = 10.0;
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(
                            Point::new(
                                bounds.center_x() - arm / 2.0,
                                bounds.center_y() - stroke / 2.0,
                            ),
                            Size::new(arm, stroke),
                        ),
                        ..renderer::Quad::default()
                    },
                    color,
                );
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(
                            Point::new(
                                bounds.center_x() - stroke / 2.0,
                                bounds.center_y() - arm / 2.0,
                            ),
                            Size::new(stroke, arm),
                        ),
                        ..renderer::Quad::default()
                    },
                    color,
                );
            }
            GutterButton::Handle => {
                let dot = 2.4;
                let step_x = 5.0;
                let step_y = 4.6;
                let left = bounds.center_x() - step_x / 2.0 - dot / 2.0;
                let top = bounds.center_y() - step_y - dot / 2.0;
                for row in 0..3 {
                    for column in 0..2 {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle::new(
                                    Point::new(
                                        left + step_x * column as f32,
                                        top + step_y * row as f32,
                                    ),
                                    Size::new(dot, dot),
                                ),
                                border: border::rounded(dot / 2.0),
                                ..renderer::Quad::default()
                            },
                            color,
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> Rectangle {
        Rectangle::new(Point::new(100.0, 50.0), Size::new(600.0, 400.0))
    }

    #[test]
    fn gutter_buttons_sit_inside_the_left_padding_centered_on_the_row() {
        let row = Rectangle::new(Point::new(100.0, 80.0), Size::new(1.0, 20.0));
        let [(_, plus), (_, handle)] = gutter_buttons(bounds(), row);
        // Right-aligned against the text edge, plus left of the handle.
        assert!(handle.x + handle.width <= 100.0);
        assert!(plus.x + plus.width <= handle.x);
        assert!(plus.x >= 100.0 - GUTTER_WIDTH);
        // Centered on the row.
        assert_eq!(plus.center_y(), 90.0);
        assert_eq!(handle.center_y(), 90.0);
    }

    #[test]
    fn the_menu_hangs_below_the_anchor_and_flips_when_space_runs_out() {
        let anchor = Rectangle::new(Point::new(120.0, 60.0), Size::new(1.0, 20.0));
        let panel = menu_panel(anchor, 4, bounds());
        assert_eq!(panel.y, 84.0);
        assert_eq!(panel.x, 120.0);
        let low = Rectangle::new(Point::new(120.0, 430.0), Size::new(1.0, 20.0));
        let flipped = menu_panel(low, 4, bounds());
        assert!(flipped.y + flipped.height <= 430.0);
        // Clamped so the panel never leaves the text area horizontally.
        let right = Rectangle::new(Point::new(690.0, 60.0), Size::new(1.0, 20.0));
        let clamped = menu_panel(right, 4, bounds());
        assert_eq!(clamped.x + clamped.width, 700.0);
    }

    #[test]
    fn the_drop_snaps_to_the_nearest_boundary() {
        let candidates = [(1, 100.0), (4, 160.0), (7, 240.0)];
        assert_eq!(snap_boundary(&candidates, 96.0), Some(1));
        assert_eq!(snap_boundary(&candidates, 131.0), Some(4));
        assert_eq!(snap_boundary(&candidates, 500.0), Some(7));
        assert_eq!(snap_boundary(&[], 100.0), None);
    }

    #[test]
    fn menu_rows_resolve_by_vertical_offset_within_the_panel() {
        let anchor = Rectangle::new(Point::new(120.0, 60.0), Size::new(1.0, 20.0));
        let panel = menu_panel(anchor, 3, bounds());
        let inside_second = Point::new(
            panel.x + 10.0,
            panel.y + MENU_PADDING + MENU_ROW_HEIGHT * 1.5,
        );
        assert_eq!(menu_row_at(panel, 3, inside_second), Some(1));
        assert_eq!(
            menu_row_at(panel, 3, Point::new(panel.x - 1.0, panel.y + 10.0)),
            None
        );
        let below_rows = Point::new(panel.x + 10.0, panel.y + panel.height + 1.0);
        assert_eq!(menu_row_at(panel, 3, below_rows), None);
    }
}
