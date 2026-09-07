//! A terminal selected by the host, with lifetime independent from its widget.
use iced::{Length, widget};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ui_lang_components::ui::terminal as native;
use ui_lang_wire::SurfaceValue as Value;

const POLL_INTERVAL: Duration = Duration::from_millis(16);

pub(crate) struct Terminal {
    session: native::Session,
    running: bool,
    title: String,
    next_poll: Option<Instant>,
}

impl Terminal {
    /// The caller checked the manifest capability. No guest value chooses a
    /// program, argument or working directory.
    pub(crate) fn configured(program: Option<PathBuf>) -> Result<Self, String> {
        let Some(program) = program else {
            return Ok(Self {
                session: native::idle_session(),
                running: false,
                title: "Terminal is not configured by this host".into(),
                next_poll: None,
            });
        };
        if !program.is_absolute() {
            return Err("ICE_TERMINAL_PROGRAM must be an absolute executable path".into());
        }
        let title = "Terminal".to_owned();
        let session = native::spawn_session(
            program,
            Vec::new(),
            std::env::current_dir().map_err(|error| error.to_string())?,
            title.clone(),
        )
        .map_err(|error| error.message)?;
        Ok(Self {
            session,
            running: true,
            title,
            next_poll: Some(Instant::now()),
        })
    }

    pub(crate) fn next_poll(&self) -> Option<Instant> {
        self.next_poll
    }

    pub(crate) fn notice(&self, attention: bool) -> Value {
        Value::Record {
            name: "TerminalNotice".into(),
            fields: vec![
                ("running".into(), Value::Bool(self.running)),
                ("title".into(), Value::Str(self.title.clone())),
                ("attention".into(), Value::Bool(attention)),
            ],
        }
    }

    /// Native output updates the host's frame even with no mounted terminal
    /// node. Only metadata changes need a guest event and a wasm tick.
    pub(crate) fn poll(&mut self, now: Instant) -> Option<Value> {
        if self.next_poll.is_none_or(|due| due > now) {
            return None;
        }
        self.next_poll = Some(now + POLL_INTERVAL);
        let notice = native::poll_terminal_events(&self.session)?;
        let title: String = notice.title.chars().take(512).collect();
        let changed = self.running != notice.running || self.title != title || notice.attention;
        self.running = notice.running;
        self.title = title;
        if !self.running {
            self.next_poll = None;
        }
        changed.then(|| self.notice(notice.attention))
    }
}

pub(crate) fn provider(terminal: Arc<Mutex<Terminal>>) -> ui_lang_runtime::view_tree::Surface {
    Box::new(move |_key, args| {
        if !args.is_empty() {
            return widget::text("invalid terminal arguments").into();
        }
        let session = terminal.lock().expect("host terminal").session.clone();
        widget::container(native::terminal_surface(&session).map(|()| Value::Unit))
            .width(Length::Fill)
            .height(240)
            .into()
    })
}
