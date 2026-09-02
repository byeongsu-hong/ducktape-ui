use std::hint::black_box;

use ui_lang_core::SymbolKind;

mod common;

use common::clean_window_allocations;

#[test]
fn validating_component_names_does_not_allocate() {
    const VALIDATIONS: usize = 4_000;

    black_box(SymbolKind::Component.accepts(black_box("catalog::controls::Card.Header")));
    let mut accepted = 0;
    let stats = clean_window_allocations(0, || {
        accepted = (0..VALIDATIONS)
            .filter(|_| {
                black_box(
                    SymbolKind::Component.accepts(black_box("catalog::controls::Card.Header")),
                )
            })
            .count();
    });

    assert_eq!(accepted, VALIDATIONS);
    eprintln!(
        "{VALIDATIONS} component-name validations: {} allocations, {} bytes",
        stats.allocations, stats.bytes_allocated
    );
    // The previous segment-collecting path still fails at 4,000 allocations.
    assert_eq!(stats.allocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 0, "{stats:?}");
}
