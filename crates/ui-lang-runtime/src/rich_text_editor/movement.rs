use super::{State, ordered_positions};
use iced::Point;
use iced::advanced::text;
use iced::widget::text_editor::{Content, Cursor, Motion, Position};
use std::cmp::Ordering;
use unicode_segmentation::UnicodeSegmentation;

pub(super) fn uses_rich_geometry(motion: Motion) -> bool {
    matches!(
        motion,
        Motion::Up | Motion::Down | Motion::Home | Motion::End | Motion::PageUp | Motion::PageDown
    )
}

pub(super) fn move_cursor<H>(
    state: &mut State<H>,
    cursor: Cursor,
    motion: Motion,
    select: bool,
) -> Cursor
where
    H: text::Highlighter,
{
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
        rich_motion(state, cursor.position, motion)
    };
    Cursor {
        position,
        selection: anchor.filter(|anchor| *anchor != position),
    }
}

fn rich_motion<H>(state: &mut State<H>, position: Position, motion: Motion) -> Position
where
    H: text::Highlighter,
{
    struct VisualRun {
        line: usize,
        top: f32,
        height: f32,
        start: usize,
        end: usize,
    }

    let caret = state.document.caret(position);
    let preferred_x = *state.preferred_x.get_or_insert(caret.x);
    let runs = state
        .document
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
            .rposition(|run| run.top <= caret.y - state.viewport_height)
            .unwrap_or(0),
        Motion::PageDown => runs
            .iter()
            .position(|run| run.top >= caret.y + state.viewport_height)
            .unwrap_or_else(|| runs.len().saturating_sub(1)),
        Motion::Home => {
            state.preferred_x = None;
            return runs.get(current).map_or(position, |run| Position {
                line: run.line,
                column: run.start,
            });
        }
        Motion::End => {
            state.preferred_x = None;
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
    state
        .document
        .hit(Point::new(preferred_x, run.top + run.height / 2.0))
}

pub(super) fn select_word(content: &Content, position: Position) -> Cursor {
    let Some(line) = content.line(position.line) else {
        return Cursor {
            position,
            selection: None,
        };
    };
    let mut selected = None;
    for (start, word) in line.text.split_word_bound_indices() {
        let end = start + word.len();
        if start <= position.column && position.column < end
            || position.column == line.text.len() && end == line.text.len()
        {
            selected = Some(start..end);
            break;
        }
    }
    let Some(range) = selected else {
        return Cursor {
            position,
            selection: None,
        };
    };
    Cursor {
        position: Position {
            line: position.line,
            column: range.end,
        },
        selection: Some(Position {
            line: position.line,
            column: range.start,
        }),
    }
}

pub(super) fn select_line(content: &Content, position: Position) -> Cursor {
    let end = content
        .line(position.line)
        .map_or(0, |line| line.text.len());
    Cursor {
        position: Position {
            line: position.line,
            column: end,
        },
        selection: (end > 0).then_some(Position {
            line: position.line,
            column: 0,
        }),
    }
}
