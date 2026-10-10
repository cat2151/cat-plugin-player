//! Keyboard movement over the painted grid: cells run left→right, then the next row.
use eframe::egui::{InputState, Key, Modifiers};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Step {
    Left,
    Right,
    Up,
    Down,
}

const PAGE: usize = 10;

const DIGITS: [Key; 10] = [
    Key::Num0,
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
];

/// A step and how many times it repeats before any vim-style count is applied.
pub(super) fn consume_step(input: &mut InputState) -> Option<(Step, usize)> {
    [
        (Step::Left, Key::ArrowLeft, Key::H),
        (Step::Right, Key::ArrowRight, Key::L),
        (Step::Up, Key::ArrowUp, Key::K),
        (Step::Down, Key::ArrowDown, Key::J),
    ]
    .into_iter()
    .find(|&(_, arrow, letter)| {
        input.consume_key(Modifiers::NONE, arrow) | input.consume_key(Modifiers::NONE, letter)
    })
    .map(|(step, ..)| (step, 1))
    .or_else(|| {
        [(Step::Up, Key::PageUp), (Step::Down, Key::PageDown)]
            .into_iter()
            .find(|&(_, key)| input.consume_key(Modifiers::NONE, key))
            .map(|(step, _)| (step, PAGE))
    })
}

/// Appends a typed digit to the pending count; a leading 0 is not a count.
pub(super) fn consume_count_digit(input: &mut InputState, count: Option<usize>) -> Option<usize> {
    let digit = DIGITS
        .iter()
        .position(|&key| input.consume_key(Modifiers::NONE, key))?;
    match (count, digit) {
        (None, 0) => None,
        (count, digit) => Some(
            count
                .unwrap_or_default()
                .saturating_mul(10)
                .saturating_add(digit),
        ),
    }
}

/// The cell reached from `at`; with nothing selected yet, every step starts at the first cell.
pub(super) fn next(at: Option<usize>, len: usize, columns: usize, step: Step) -> usize {
    let Some(at) = at else {
        return 0;
    };
    let columns = columns.max(1);
    let last = len - 1;
    match step {
        Step::Left => at.saturating_sub(1),
        Step::Right => (at + 1).min(last),
        Step::Up => at.checked_sub(columns).unwrap_or(at),
        // Below a cell of the full row above the partial last row lies empty space;
        // the nearest cell is then the last one.
        Step::Down if at / columns < last / columns => (at + columns).min(last),
        Step::Down => at,
    }
}

/// `times` steps stop early at an edge; `len` steps always reach it.
pub(super) fn next_by(
    at: Option<usize>,
    len: usize,
    columns: usize,
    step: Step,
    times: usize,
) -> usize {
    let first = next(at, len, columns, step);
    (1..times.min(len)).fold(first, |at, _| next(Some(at), len, columns, step))
}

#[cfg(test)]
mod tests;
