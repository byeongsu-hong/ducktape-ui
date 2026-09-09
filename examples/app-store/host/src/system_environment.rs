//! Current OS theme and bounded listeners owned by one guest instance.
use ui_lang_wire::{Event, system::Theme};

const MAX_WAITERS: usize = 32;

#[derive(Default)]
pub(crate) struct Environment {
    pub(crate) theme: Option<Theme>,
    waiting: Vec<(u64, bool)>,
}

impl Environment {
    pub(crate) fn request(
        &mut self,
        id: u64,
        stream: bool,
        payload: &[u8],
    ) -> Result<Option<Event>, String> {
        if !payload.is_empty() {
            return Err("system theme request takes no payload".into());
        }
        if stream || self.theme.is_none() {
            if self.waiting.len() >= MAX_WAITERS {
                return Err("too many system theme listeners".into());
            }
            self.waiting.push((id, stream));
        }
        Ok(self.theme.map(|theme| reply(id, stream, theme)))
    }

    pub(crate) fn update(&mut self, theme: Theme) -> Vec<Event> {
        if self.theme == Some(theme) {
            return Vec::new();
        }
        self.theme = Some(theme);
        let events = self
            .waiting
            .iter()
            .map(|(id, stream)| reply(*id, *stream, theme))
            .collect();
        self.waiting.retain(|(_, stream)| *stream);
        events
    }

    pub(crate) fn cancel(&mut self, id: u64) {
        self.waiting.retain(|(pending, _)| *pending != id);
    }
}

fn reply(id: u64, stream: bool, theme: Theme) -> Event {
    Event::Response {
        id,
        result: Ok(ui_lang_wire::encode(&theme)),
        done: !stream,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_bounds_waiters_and_cancels_before_initial_answer() {
        let mut environment = Environment::default();
        assert!(environment.request(0, false, b"unexpected").is_err());
        for id in 0..MAX_WAITERS as u64 {
            assert!(environment.request(id, false, &[]).unwrap().is_none());
        }
        assert!(environment.request(100, true, &[]).is_err());
        environment.cancel(0);
        assert!(environment.request(100, true, &[]).unwrap().is_none());
        let answers = environment.update(Theme::None);
        assert_eq!(answers.len(), MAX_WAITERS);
        assert!(
            !answers
                .iter()
                .any(|event| matches!(event, Event::Response { id: 0, .. }))
        );
        assert!(answers.iter().all(|event| matches!(event, Event::Response { result: Ok(bytes), .. } if Theme::decode(bytes) == Ok(Theme::None))));
        assert!(environment.update(Theme::None).is_empty());
        assert_eq!(
            environment.update(Theme::Dark).len(),
            1,
            "only stream remains after one-shot answers"
        );
        environment.cancel(100);
        assert!(environment.update(Theme::Light).is_empty());
    }
}
