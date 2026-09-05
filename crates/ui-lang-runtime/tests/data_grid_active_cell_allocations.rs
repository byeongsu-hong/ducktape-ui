use iced::{Element, Theme};

mod common;
use common::clean_window;
use ui_lang_runtime::{
    DataGridColumn, DataGridConfig, DataGridEvent, DataGridId, DataGridState, data_grid,
};

type Message = DataGridEvent<String, String>;
type Renderer = iced_test::renderer::Renderer;

#[test]
fn active_cell_moves_into_the_rendered_grid() {
    const FRAMES: usize = 256;
    const ALLOCATIONS: usize = 14_336;
    const ALLOCATED_BYTES: usize = 1_269_760;

    let config = DataGridConfig::new(20.0, 20.0).unwrap();
    let rows = [String::from("row-key-owned")];
    let columns = [DataGridColumn::new(
        String::from("column-key-owned"),
        "Column",
        100.0,
    )];
    let mut state = DataGridState::new(DataGridId::new("active-cell-allocation-contract"));
    state
        .reconcile(&rows, Clone::clone, &columns, config)
        .unwrap();
    state.apply(
        DataGridEvent::ViewportChanged {
            width: 100.0,
            height: 20.0,
        },
        config,
    );
    state.apply(
        DataGridEvent::FocusCell {
            row_index: 0,
            row: rows[0].clone(),
            column_index: 0,
            column: columns[0].key().clone(),
        },
        config,
    );

    let stats = clean_window((ALLOCATIONS, ALLOCATED_BYTES), || {
        for _ in 0..FRAMES {
            let element: Element<'_, Message, Theme, Renderer> = data_grid(
                &state,
                &rows,
                config,
                "Grid",
                Clone::clone,
                Clone::clone,
                |_, column| column.label().to_owned(),
                |_| None,
                |header| iced::widget::text(header.column.label()).into(),
                |_| iced::widget::space().into(),
                |event| event,
            );
            drop(std::hint::black_box(element));
        }
    });

    eprintln!(
        "{FRAMES} active-grid renders: allocations={} bytes={}",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, ALLOCATIONS);
    assert_eq!(stats.bytes_allocated, ALLOCATED_BYTES);
}
