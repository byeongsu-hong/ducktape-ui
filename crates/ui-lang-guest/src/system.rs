//! Copied host OS theme through the existing cancellable request channel.
use crate::{host, wire};
use iced::futures::StreamExt;

fn read(answer: host::Answer) -> Option<String> {
    match answer.and_then(|bytes| wire::system::Theme::decode(&bytes)) {
        Ok(theme) => Some(theme.name().to_owned()),
        Err(error) => {
            host::log(format!("host system theme: {error}"));
            None
        }
    }
}

pub fn theme() -> iced::Task<String> {
    iced::Task::future(async { read(host::request("host.system-theme", &[]).await) })
        .then(|mode| mode.map_or_else(iced::Task::none, iced::Task::done))
}

pub fn theme_changes() -> iced::Subscription<String> {
    iced::Subscription::run(|| {
        host::subscribe("host.system-theme-changes", &[])
            .filter_map(|answer| async move { read(answer) })
    })
}
