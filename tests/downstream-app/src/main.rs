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

        use ui_lang_components::ui::theme::LIGHT;
        use ui_lang_components::ui::virtual_list::{
            VirtualListConfig, VirtualListEvent, VirtualListId, VirtualListState, virtual_list,
        };

        let items: Vec<String> = (0..100_000)
            .map(|index| format!("item-{index}"))
            .collect();
        let config = VirtualListConfig::new(20.0).unwrap().overscan(2);
        let mut state = VirtualListState::new(VirtualListId::new("packaged-virtual-list"));
        state
            .reconcile(&items, Clone::clone, config)
            .expect("packaged keys are unique");
        let fork = state.fork("packaged-virtual-list/item/2");
        assert_ne!(state.id(), fork.id());
        assert_ne!(state.id().logical(), fork.id().logical());
        let selectors = [
            state.id().selector(),
            state.item_selector(&items[0]).unwrap(),
            fork.id().selector(),
            fork.item_selector(&items[0]).unwrap(),
        ];
        assert_eq!(
            selectors
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            selectors.len()
        );
        state.apply(
            VirtualListEvent::ViewportChanged { height: 100.0 },
            &items,
            Clone::clone,
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
            Clone::clone,
            |item| format!("Item {item}"),
            |index, _, _| iced::widget::text(index).into(),
            |_| (),
            &LIGHT,
        );
    }
}
