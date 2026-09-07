//! Adapts Iced clipboard Tasks to the guest's existing capability channel.
use crate::{host, wire};
use iced_runtime::clipboard::Action;
use iced_runtime::core::clipboard::Kind;
use iced_runtime::futures::futures::future::{Either, select};

pub(crate) fn run<M: iced_runtime::futures::MaybeSend + 'static>(action: Action) -> iced::Task<M> {
    let target = |kind| match kind {
        Kind::Standard => wire::ClipboardTarget::Standard,
        Kind::Primary => wire::ClipboardTarget::Primary,
    };
    match action {
        Action::Read {
            target: kind,
            mut channel,
        } => {
            if channel.is_canceled() {
                return iced::Task::none();
            }
            let response = host::request("clipboard.read", &wire::encode(&target(kind)));
            iced::Task::future(async move {
                let answer = match select(response, channel.cancellation()).await {
                    Either::Left((answer, _)) => answer,
                    Either::Right(_) => return,
                };
                let mut value: Option<String> = match answer.and_then(|bytes| wire::decode(&bytes))
                {
                    Ok(value) => value,
                    Err(error) => {
                        host::log(format!("clipboard.read: {error}"));
                        None
                    }
                };
                if let Some(text) = &mut value {
                    wire::truncate_string(text);
                }
                let _ = channel.send(value);
            })
            .discard()
        }
        Action::Write {
            target: kind,
            mut contents,
        } => {
            wire::truncate_string(&mut contents);
            let response =
                host::request("clipboard.write", &wire::encode(&(target(kind), contents)));
            iced::Task::future(async move {
                if let Err(error) = response.await {
                    host::log(format!("clipboard.write: {error}"));
                }
            })
            .discard()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{App, Driver, wire};

    struct ClipboardApp(Option<String>);
    impl App for ClipboardApp {
        type Message = Option<String>;
        fn boot() -> (Self, iced::Task<Self::Message>) {
            (
                Self(None),
                iced::Task::batch([
                    iced::clipboard::write("copy".into()),
                    iced::clipboard::read(),
                ]),
            )
        }
        fn view(&self) -> wire::Node {
            wire::Node::empty()
        }
        fn update(&mut self, value: Self::Message) -> iced::Task<Self::Message> {
            self.0 = value;
            iced::Task::none()
        }
        fn subscription(&self) -> iced::Subscription<Self::Message> {
            iced::Subscription::none()
        }
    }

    #[test]
    fn clipboard_tasks_cross_the_host_boundary_and_resume_the_read() {
        let mut driver = Driver::<ClipboardApp>::new();
        let frame = driver.tick(vec![]);
        assert!(
            frame.requests.iter().any(|r| r.kind == "clipboard.write"),
            "{:#?}",
            frame.requests
        );
        let read = frame
            .requests
            .iter()
            .find(|r| r.kind == "clipboard.read")
            .expect("clipboard read request");
        driver.tick(vec![wire::Event::Response {
            id: read.id,
            result: Ok(wire::encode(&Some("pasted".to_string()))),
            done: true,
        }]);
        assert_eq!(driver.app.0.as_deref(), Some("pasted"));
    }
    #[test]
    fn canceled_clipboard_reads_cancel_the_host_request() {
        use iced_runtime::futures::futures::{channel::oneshot, task::noop_waker};
        let (channel, receiver) = oneshot::channel();
        let task = super::run::<()>(iced_runtime::clipboard::Action::Read {
            target: iced_runtime::core::clipboard::Kind::Primary,
            channel,
        });
        let sent = crate::host::drain_outbox();
        assert_eq!(sent.len(), 1);
        assert_eq!(
            wire::decode::<wire::ClipboardTarget>(&sent[0].payload).unwrap(),
            wire::ClipboardTarget::Primary
        );
        let mut stream = iced_runtime::task::into_stream(task).unwrap();
        let waker = noop_waker();
        let mut context = std::task::Context::from_waker(&waker);
        assert!(stream.as_mut().poll_next(&mut context).is_pending());
        drop(receiver);
        assert!(
            stream.as_mut().poll_next(&mut context).is_ready(),
            "aborted read must stop waiting without a host response"
        );
        assert_eq!(crate::host::drain_cancels(), vec![sent[0].id]);
    }
}
