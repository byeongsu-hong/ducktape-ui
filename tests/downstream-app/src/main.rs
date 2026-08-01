ui_lang::include_app!("src/ui/app.ice");

mod backend;

fn main() -> iced::Result {
    DownstreamConsumer::run()
}

#[cfg(test)]
mod tests {
    #[test]
    fn packaged_runtime_is_a_direct_dependency() {
        let id = ui_lang_runtime::StableId::new("downstream-consumer");
        assert_ne!(id.node_id().0, 0);

        use ducktape_ui::ui::theme::LIGHT;
        use ducktape_ui::ui::virtual_list::{
            VirtualListConfig, VirtualListEvent, VirtualListId, VirtualListState, virtual_list,
        };

        let items: Vec<u64> = (0..100_000).collect();
        let config = VirtualListConfig::new(20.0).unwrap().overscan(2);
        let mut state = VirtualListState::new(VirtualListId::new("packaged-virtual-list"));
        state
            .reconcile(&items, |item| *item, config)
            .expect("packaged keys are unique");
        state.apply(
            VirtualListEvent::ViewportChanged { height: 100.0 },
            &items,
            |item| *item,
            config,
        );
        assert!(state.scroll_to_item(50_000, items.len(), config));
        let inspection = state.inspect(items.len(), config);
        assert_eq!(inspection.logical_items, 100_000);
        assert!(inspection.mounted_rows <= 10);

        let _element: iced::Element<'_, ()> = virtual_list(
            &state,
            &items,
            config,
            "Packaged results",
            |item| *item,
            |item| format!("Item {item}"),
            |index, _, _| iced::widget::text(index).into(),
            |_| (),
            &LIGHT,
        );
    }
}
