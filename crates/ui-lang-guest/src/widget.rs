//! Existing Ice widget statements use the mounted host's operation channel.
use crate::{host, wire};
use iced_runtime::futures::MaybeSend;

/// Execute a mutation inside this guest's mounted widget tree.
pub fn perform<M: MaybeSend + 'static>(command: wire::WidgetCommand) -> iced::Task<M> {
    iced::Task::future(async move {
        if let Err(error) = host::request("host.widget", &wire::encode(&command)).await {
            host::log(format!("host.widget: {error}"));
        }
    })
    .discard()
}

/// Query focus after the host has mounted the requesting frame.
pub fn is_focused(target: String) -> iced::Task<bool> {
    iced::Task::future(async move {
        let command = wire::WidgetCommand::Focused { target };
        match host::request("host.widget", &wire::encode(&command))
            .await
            .and_then(|bytes| wire::decode(&bytes))
        {
            Ok(focused) => focused,
            Err(error) => {
                host::log(format!("host.widget focused: {error}"));
                false
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{App, Driver};

    struct WidgetApp(bool);
    impl App for WidgetApp {
        type Message = bool;
        fn boot() -> (Self, iced::Task<bool>) {
            (
                Self(false),
                perform(wire::WidgetCommand::Focus {
                    target: "App/draft".into(),
                })
                .chain(is_focused("App/draft".into())),
            )
        }
        fn view(&self) -> wire::Node {
            wire::Node::empty()
        }
        fn update(&mut self, focused: bool) -> iced::Task<bool> {
            self.0 = focused;
            iced::Task::none()
        }
        fn subscription(&self) -> iced::Subscription<bool> {
            iced::Subscription::none()
        }
    }

    #[test]
    fn widget_tasks_wait_for_mutations_before_querying_focus() {
        let mut driver = Driver::<WidgetApp>::new();
        let frame = driver.tick(vec![]);
        let [focus] = frame.requests.as_slice() else {
            panic!("one focus request: {:?}", frame.requests)
        };
        assert_eq!(focus.kind, "host.widget");
        assert_eq!(
            wire::decode::<wire::WidgetCommand>(&focus.payload).unwrap(),
            wire::WidgetCommand::Focus {
                target: "App/draft".into()
            }
        );
        assert!(
            driver.tick(vec![]).requests.is_empty(),
            "query must wait for focus acknowledgement"
        );
        let frame = driver.tick(vec![wire::Event::Response {
            id: focus.id,
            result: Ok(wire::encode(&())),
            done: true,
        }]);
        let [query] = frame.requests.as_slice() else {
            panic!("one focused query: {:?}", frame.requests)
        };
        assert_eq!(
            wire::decode::<wire::WidgetCommand>(&query.payload).unwrap(),
            wire::WidgetCommand::Focused {
                target: "App/draft".into()
            }
        );
        driver.tick(vec![wire::Event::Response {
            id: query.id,
            result: Ok(wire::encode(&true)),
            done: true,
        }]);
        assert!(
            driver.app.0,
            "host focus reply must reach the guest handler"
        );
    }
}
