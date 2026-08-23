ui_lang::include_app!("tests/cases/ui/lazy_state_revisions.ice");

/// The frame contract behind the `.ice` tests above: a `lazy` over a state
/// list is keyed by that field's revision, so the list is deep-cloned only
/// when the revision moved — never on an unchanged frame, never for a write
/// elsewhere, never for a write storing an equal list.
mod contract {
    use ui_lang_runtime::testing::{Config, Driver, Location, MouseButton};

    use super::{__LazyStateRevisionsMessage as Message, LazyStateRevisions};
    use crate::backend::entry_clones;

    fn here() -> Location {
        Location::new(
            "examples/showcase/src/lazy_state_revisions.rs",
            1,
            1,
            "state revision contract",
        )
    }

    // One test, not two: the clone tally is per thread, and both halves
    // clone the same `Entry` type on this thread.
    #[test]
    fn a_state_list_is_cloned_only_when_its_revision_moves() {
        let mut driver = Driver::new(
            LazyStateRevisions::__program(),
            Config::new("lazy_state_revisions"),
        );
        // The first frames materialize every lazy once; the contract is
        // about what follows.
        driver.redraw(here());
        driver.redraw(here());
        let _ = entry_clones();

        for _ in 0..5 {
            driver.redraw(here());
        }
        assert_eq!(entry_clones(), 0, "an unchanged frame deep-clones nothing");

        // App state.
        driver.dispatch(Message::TouchUnrelated, here());
        driver.redraw(here());
        assert_eq!(
            entry_clones(),
            0,
            "a write to another field rebuilds nothing"
        );

        driver.dispatch(Message::Restate, here());
        driver.redraw(here());
        // `seeded_entries` builds the list afresh: the handler clones
        // nothing and the compare-on-write sees an equal list.
        assert_eq!(
            entry_clones(),
            0,
            "a write storing an equal list rebuilds nothing"
        );

        driver.dispatch(Message::Append("Entry 4".into()), here());
        driver.redraw(here());
        assert_eq!(
            entry_clones(),
            4,
            "a write that changes the list rebuilds it: one clone per row"
        );

        // Component state. The instance comes into being on its first
        // write, seeded with revisions no other instance shares; until
        // then the memo read `0`, so that first write rebuilds once.
        let touch = "LazyStateRevisions/card/root/touch";
        let same = "LazyStateRevisions/card/root/same";
        let add = "LazyStateRevisions/card/root/add";
        driver.click_with(touch, MouseButton::Left, 1, here());
        driver.redraw(here());
        assert_eq!(
            entry_clones(),
            3,
            "the instance's first write seeds its revisions: one rebuild"
        );

        driver.click_with(touch, MouseButton::Left, 1, here());
        driver.redraw(here());
        assert_eq!(
            entry_clones(),
            0,
            "a write to another field rebuilds nothing"
        );

        driver.click_with(same, MouseButton::Left, 1, here());
        driver.redraw(here());
        assert_eq!(
            entry_clones(),
            0,
            "a write storing an equal list rebuilds nothing"
        );

        driver.click_with(add, MouseButton::Left, 1, here());
        driver.redraw(here());
        assert_eq!(
            entry_clones(),
            4,
            "a write that changes the list rebuilds it: one clone per row"
        );
    }
}
