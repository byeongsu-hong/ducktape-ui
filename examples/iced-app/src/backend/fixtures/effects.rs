use super::*;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{LazyLock, Mutex};
use std::task::{Context, Poll, Waker};

#[derive(Default)]
struct ControlledRequests {
    ready: HashMap<i64, String>,
    wakers: HashMap<i64, Waker>,
    cancelled: HashSet<i64>,
}

static CONTROLLED_REQUESTS: LazyLock<Mutex<ControlledRequests>> =
    LazyLock::new(|| Mutex::new(ControlledRequests::default()));

pub struct ControlledRequest {
    id: i64,
    completed: bool,
}

impl Future for ControlledRequest {
    type Output = Result<String, AppError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let result = {
            let mut requests = CONTROLLED_REQUESTS.lock().unwrap();
            if let Some(value) = requests.ready.remove(&self.id) {
                Some(value)
            } else {
                requests.wakers.insert(self.id, context.waker().clone());
                None
            }
        };
        if let Some(value) = result {
            self.completed = true;
            Poll::Ready(Ok(value))
        } else {
            Poll::Pending
        }
    }
}

impl Drop for ControlledRequest {
    fn drop(&mut self) {
        if !self.completed {
            let mut requests = CONTROLLED_REQUESTS.lock().unwrap();
            requests.ready.remove(&self.id);
            requests.wakers.remove(&self.id);
            requests.cancelled.insert(self.id);
        }
    }
}

pub fn controlled_request(id: i64) -> ControlledRequest {
    ControlledRequest {
        id,
        completed: false,
    }
}

pub fn complete_controlled_request(id: i64, value: &str) {
    let waker = {
        let mut requests = CONTROLLED_REQUESTS.lock().unwrap();
        requests.ready.insert(id, value.to_owned());
        requests.wakers.remove(&id)
    };
    if let Some(waker) = waker {
        waker.wake();
    }
}

pub fn controlled_request_was_cancelled(id: i64) -> bool {
    CONTROLLED_REQUESTS.lock().unwrap().cancelled.contains(&id)
}

#[cfg(test)]
pub fn copy_text(text: String) -> iced::Task<Result<(), AppError>> {
    iced::Task::batch([iced::clipboard::write::<()>(text), iced::Task::done(())]).map(Ok)
}

#[cfg(test)]
pub fn count_stream(limit: i64) -> impl iced::futures::Stream<Item = i64> + Send + 'static {
    iced::futures::stream::iter(0..limit.max(0))
}

#[cfg(test)]
pub fn range_stream(
    start: i64,
    limit: i64,
) -> impl iced::futures::Stream<Item = i64> + Send + 'static {
    iced::futures::stream::iter(start..start.saturating_add(limit.max(0)))
}

#[cfg(test)]
pub fn fallible_stream() -> impl iced::futures::Stream<Item = Result<i64, AppError>> + Send + 'static
{
    iced::futures::stream::iter([
        Ok(1),
        Err(AppError {
            message: "stream failed".into(),
        }),
    ])
}

#[cfg(test)]
pub struct CounterRecipe {
    id: i64,
}

#[cfg(test)]
impl iced::advanced::subscription::Recipe for CounterRecipe {
    type Output = i64;

    fn hash(&self, state: &mut iced::advanced::subscription::Hasher) {
        std::hash::Hash::hash(&self.id, state);
    }

    fn stream(
        self: Box<Self>,
        _input: iced::advanced::subscription::EventStream,
    ) -> iced::futures::stream::BoxStream<'static, Self::Output> {
        Box::pin(iced::futures::stream::iter([self.id]))
    }
}

#[cfg(test)]
pub fn counter_recipe(id: i64) -> CounterRecipe {
    CounterRecipe { id }
}

#[cfg(test)]
pub fn raw_event(event: iced::advanced::subscription::Event) -> Option<String> {
    Some(match event {
        iced::advanced::subscription::Event::Interaction { status, .. } => {
            format!("{status:?}")
        }
        iced::advanced::subscription::Event::SystemThemeChanged(mode) => {
            format!("{mode:?}")
        }
    })
}

#[cfg(test)]
pub fn by_kind(kind: String) -> impl iced::widget::selector::Selector<Output = String> {
    move |candidate: iced::widget::selector::Candidate<'_>| {
        let candidate_kind = match candidate {
            iced::widget::selector::Candidate::Container { .. } => "container",
            iced::widget::selector::Candidate::Focusable { .. } => "focusable",
            iced::widget::selector::Candidate::Scrollable { .. } => "scrollable",
            iced::widget::selector::Candidate::TextInput { .. } => "text-input",
            iced::widget::selector::Candidate::Text { .. } => "text",
            iced::widget::selector::Candidate::Custom { .. } => "custom",
        };
        (candidate_kind == kind).then(|| kind.clone())
    }
}

#[cfg(test)]
pub fn count_sip(limit: i64) -> impl iced::task::Sipper<i64, i64> + Send + 'static {
    iced::task::sipper(move |mut sender| async move {
        let limit = limit.max(0);
        for value in 1..=limit {
            sender.send(value).await;
        }
        limit
    })
}

#[cfg(test)]
pub fn fallible_sip(limit: i64) -> impl iced::task::Straw<i64, i64, AppError> + Send + 'static {
    iced::task::sipper(move |mut sender| async move {
        sender.send(1).await;
        if limit < 0 {
            Err(AppError {
                message: "sip failed".into(),
            })
        } else {
            Ok(limit)
        }
    })
}

#[cfg(test)]
pub fn double_task(value: i64) -> iced::Task<i64> {
    iced::Task::done(value * 2)
}

#[cfg(test)]
pub fn optional_task(value: i64) -> iced::Task<Option<i64>> {
    iced::Task::done((value > 0).then_some(value))
}

#[cfg(test)]
pub fn fallible_task(value: i64) -> iced::Task<Result<i64, AppError>> {
    iced::Task::done(if value >= 0 {
        Ok(value)
    } else {
        Err(AppError {
            message: "task failed".into(),
        })
    })
}

#[cfg(test)]
pub async fn refresh_time() -> i64 {
    1
}

#[cfg(test)]
pub fn even_refresh(value: i64) -> Option<i64> {
    (value % 2 == 0).then_some(value)
}

#[cfg(test)]
pub fn visible_pointer(x: f64, y: f64) -> Option<String> {
    (x >= 0.0 && y >= 0.0).then(|| format!("{x},{y}"))
}

#[cfg(test)]
pub fn allow_frame() -> Option<bool> {
    Some(true)
}

#[cfg(test)]
pub fn network_task(value: i64) -> iced::Task<Result<i64, NetworkError>> {
    iced::Task::done(if value >= 0 {
        Ok(value)
    } else {
        Err(NetworkError {
            message: "network failed".into(),
        })
    })
}

#[cfg(test)]
pub fn normalize_error(error: NetworkError) -> AppError {
    AppError {
        message: error.message,
    }
}

#[cfg(test)]
pub fn event_name(event: iced::Event) -> String {
    match event {
        iced::Event::Keyboard(_) => "keyboard",
        iced::Event::Mouse(_) => "mouse",
        iced::Event::Window(_) => "window",
        iced::Event::Touch(_) => "touch",
        iced::Event::InputMethod(_) => "input-method",
    }
    .into()
}

#[cfg(test)]
pub fn event_label(event: iced::Event) -> Option<String> {
    Some(event_name(event))
}

#[cfg(test)]
pub fn app_events() -> iced::Subscription<bool> {
    iced::event::listen_with(|event, _status, _window| focus_event(event))
}

#[cfg(test)]
fn focus_event(event: iced::Event) -> Option<bool> {
    matches!(event, iced::Event::Window(iced::window::Event::Focused)).then_some(true)
}

#[cfg(test)]
mod tests {
    use super::focus_event;

    #[test]
    fn subscription_ignores_high_frequency_pointer_events() {
        assert_eq!(
            focus_event(iced::Event::Window(iced::window::Event::Focused)),
            Some(true)
        );
        assert_eq!(
            focus_event(iced::Event::Mouse(iced::mouse::Event::CursorLeft)),
            None
        );
    }
}
