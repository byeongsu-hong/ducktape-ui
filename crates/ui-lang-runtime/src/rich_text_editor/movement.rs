use super::document::{DocumentLayout, ordered_positions};
use iced::Point;
use iced::widget::text_editor::{Cursor, Motion, Position};
use std::cmp::Ordering;

pub(super) fn uses_rich_geometry(motion: Motion) -> bool {
    matches!(
        motion,
        Motion::Up | Motion::Down | Motion::Home | Motion::End | Motion::PageUp | Motion::PageDown
    )
}

pub(super) fn move_cursor(
    document: &DocumentLayout,
    preferred_x: &mut Option<f32>,
    viewport_height: f32,
    cursor: Cursor,
    motion: Motion,
    select: bool,
) -> Cursor {
    let anchor = select.then(|| cursor.selection.unwrap_or(cursor.position));
    let position = if let Some(selection) = cursor.selection
        && !select
        && matches!(
            motion,
            Motion::Up | Motion::Down | Motion::PageUp | Motion::PageDown
        ) {
        let (start, end) = ordered_positions(cursor.position, selection);
        if matches!(motion, Motion::Up | Motion::PageUp) {
            start
        } else {
            end
        }
    } else {
        rich_motion(
            document,
            preferred_x,
            viewport_height,
            cursor.position,
            motion,
        )
    };
    Cursor {
        position,
        selection: anchor.filter(|anchor| *anchor != position),
    }
}

fn rich_motion(
    document: &DocumentLayout,
    preferred_x: &mut Option<f32>,
    viewport_height: f32,
    position: Position,
    motion: Motion,
) -> Position {
    struct VisualRun {
        line: usize,
        top: f32,
        height: f32,
        start: usize,
        end: usize,
    }

    let caret = document.caret(position);
    let preferred_x_value = *preferred_x.get_or_insert(caret.x);
    let runs = document
        .lines
        .iter()
        .enumerate()
        .flat_map(|(line_index, line)| {
            line.paragraph
                .buffer()
                .layout_runs()
                .map(move |run| VisualRun {
                    line: line_index,
                    top: line.top + line.signature.line_padding.top + run.line_top,
                    height: run.line_height,
                    start: run.glyphs.first().map_or(0, |glyph| glyph.start),
                    end: run
                        .glyphs
                        .last()
                        .map_or(line.signature.text.len(), |glyph| glyph.end),
                })
        })
        .collect::<Vec<_>>();
    let caret_center = caret.y + caret.height / 2.0;
    let current = runs
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            let distance = |run: &VisualRun| {
                if caret_center < run.top {
                    run.top - caret_center
                } else if caret_center > run.top + run.height {
                    caret_center - run.top - run.height
                } else {
                    0.0
                }
            };
            distance(left)
                .partial_cmp(&distance(right))
                .unwrap_or(Ordering::Equal)
        })
        .map_or(0, |(index, _)| index);

    let target = match motion {
        Motion::Up => current.saturating_sub(1),
        Motion::Down => (current + 1).min(runs.len().saturating_sub(1)),
        Motion::PageUp => runs
            .iter()
            .rposition(|run| run.top <= caret.y - viewport_height)
            .unwrap_or(0),
        Motion::PageDown => runs
            .iter()
            .position(|run| run.top >= caret.y + viewport_height)
            .unwrap_or_else(|| runs.len().saturating_sub(1)),
        Motion::Home => {
            *preferred_x = None;
            return runs.get(current).map_or(position, |run| Position {
                line: run.line,
                column: run.start,
            });
        }
        Motion::End => {
            *preferred_x = None;
            return runs.get(current).map_or(position, |run| Position {
                line: run.line,
                column: run.end,
            });
        }
        _ => return position,
    };

    let Some(run) = runs.get(target) else {
        return position;
    };
    document.hit(Point::new(preferred_x_value, run.top + run.height / 2.0))
}
