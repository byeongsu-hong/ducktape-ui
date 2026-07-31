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
    }
}
