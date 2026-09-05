use iced::{Element, Theme};

mod common;
use common::clean_window;
use ui_lang_runtime::{
    VirtualListConfig, VirtualListEvent, VirtualListId, VirtualListState, virtual_list,
};

fn config() -> VirtualListConfig {
    VirtualListConfig::new(20.0).unwrap().overscan(2)
}

#[test]
#[ignore = "virtual-list allocation contract run explicitly in CI"]
fn performance_contract_string_key_render_moves_mounted_keys() {
    const SAMPLES: usize = 256;
    const ALLOCATIONS_PER_RENDER: usize = 46;
    const BYTES_PER_RENDER: usize = 5_088;

    let items = (0..16)
        .map(|index| format!("row-key-{index:02}"))
        .collect::<Vec<_>>();
    let mut state = VirtualListState::new(VirtualListId::new("key-allocation-contract"));
    state.reconcile(&items, Clone::clone, config()).unwrap();
    state.apply(
        VirtualListEvent::ViewportChanged { height: 60.0 },
        &items,
        Clone::clone,
        config(),
    );
    assert_eq!(state.mounted_range(items.len(), config()).len(), 5);

    let build = || -> Element<'_, (), Theme, iced_test::renderer::Renderer> {
        virtual_list(
            &state,
            &items,
            config(),
            "",
            Clone::clone,
            |_| String::new(),
            |_, item, _| iced::widget::text(item.as_str()).into(),
            |_| (),
        )
    };
    drop(build());

    let stats = clean_window(
        (SAMPLES * ALLOCATIONS_PER_RENDER, SAMPLES * BYTES_PER_RENDER),
        || {
            for _ in 0..SAMPLES {
                drop(std::hint::black_box(build()));
            }
        },
    );

    eprintln!(
        "256 string-key virtual-list renders: allocations={} bytes={}",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, SAMPLES * ALLOCATIONS_PER_RENDER);
    assert_eq!(stats.bytes_allocated, SAMPLES * BYTES_PER_RENDER);
}
