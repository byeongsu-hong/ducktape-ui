ui_lang::include_app!("tests/cases/ui/lazy_extra_deps.ice");

/// The count behind the first-class test: `counted_title` runs once per row
/// that is actually rebuilt, so an unchanged redraw must leave it at zero, one
/// retitled task must rebuild one row, and a moved extra must rebuild all of
/// them. This is the trading terminal's `fills_stay_memoized` contract with the
/// extra dependency added.
#[test]
fn extra_dependency_keeps_unchanged_rows_memoized() {
    use std::cell::Cell;

    use ui_lang_runtime::testing::{Config, Driver, Location};

    use crate::backend::COUNTED_TITLES;

    let here = Location::new(
        "examples/showcase/src/lazy_extra_deps.rs",
        1,
        1,
        "extra deps",
    );
    // Mounting already builds the view once, so the cold count starts here.
    COUNTED_TITLES.with(Cell::take);
    let mut driver = Driver::new(
        LazyExtraDeps::__program(),
        Config::new("lazy_extra_deps").viewport(320.0, 240.0),
    );
    let rows = driver.state_mut().tasks.len();
    driver.redraw(here);
    let cold = COUNTED_TITLES.with(Cell::take);
    assert!(
        cold > 0 && cold.is_multiple_of(rows),
        "a cold redraw builds every row a whole number of times: {cold} for {rows} rows"
    );

    driver.redraw(here);
    assert_eq!(
        COUNTED_TITLES.with(Cell::take),
        0,
        "a redraw of {rows} unchanged rows must rebuild none of them"
    );

    driver.state_mut().tasks[0].title.push('!');
    driver.redraw(here);
    assert_eq!(
        COUNTED_TITLES.with(Cell::take),
        1,
        "retitling one task must rebuild that row and no other"
    );

    driver.state_mut().locale = Locale::Ko;
    driver.redraw(here);
    assert_eq!(
        COUNTED_TITLES.with(Cell::take),
        rows,
        "moving the extra must rebuild every row that lists it"
    );
}
