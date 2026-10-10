use super::*;

#[test]
fn steps_follow_the_painted_grid() {
    // 3 columns, 7 cells:  0 1 2 / 3 4 5 / 6
    let next = |at, step| next(Some(at), 7, 3, step);
    assert_eq!(next(4, Step::Up), 1);
    assert_eq!(next(4, Step::Down), 6);
    assert_eq!(next(1, Step::Down), 4);
    assert_eq!(next(1, Step::Up), 1);
    assert_eq!(next(6, Step::Down), 6);
    assert_eq!(next(6, Step::Up), 3);
    assert_eq!(next(2, Step::Right), 3);
    assert_eq!(next(3, Step::Left), 2);
    assert_eq!(next(0, Step::Left), 0);
    assert_eq!(next(6, Step::Right), 6);
}

#[test]
fn down_into_a_partial_row_reaches_its_last_cell() {
    // 0 1 2 / 3
    assert_eq!(next(Some(2), 4, 3, Step::Down), 3);
}

#[test]
fn first_step_selects_the_first_cell_and_one_column_is_a_list() {
    assert_eq!(next(None, 5, 3, Step::Down), 0);
    assert_eq!(next(None, 5, 3, Step::Left), 0);
    assert_eq!(next(Some(2), 5, 1, Step::Down), 3);
    assert_eq!(next(Some(2), 5, 0, Step::Up), 1);
}

#[test]
fn repeated_steps_stop_at_the_edge() {
    // 0 1 2 / 3 4 5 / 6
    assert_eq!(next_by(Some(0), 7, 3, Step::Down, 2), 6);
    assert_eq!(next_by(Some(0), 7, 3, Step::Down, 10), 6);
    assert_eq!(next_by(Some(5), 7, 3, Step::Left, 3), 2);
    assert_eq!(next_by(Some(5), 7, 3, Step::Up, usize::MAX), 2);
    assert_eq!(next_by(None, 7, 3, Step::Right, 1), 0);
}
