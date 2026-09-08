//! Window effects commit on the app update thread against its current running list.
use super::*;

const MAX_WINDOW_REQUESTS: usize = 32;

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Queued,
    Prepared,
    Submitted,
}

struct Pending {
    id: u64,
    command: wire::WindowCommand,
    phase: Phase,
}

#[derive(Default)]
pub(crate) struct WindowEffects {
    pending: Vec<Pending>,
}

impl WindowEffects {
    pub(super) fn push(&mut self, id: u64, payload: &[u8]) -> Result<(), String> {
        if self.pending.len() >= MAX_WINDOW_REQUESTS {
            return Err("RequestError: too many pending window commands".into());
        }
        if self.pending.iter().any(|request| request.id == id) {
            return Err("RequestError: duplicate pending window request ID".into());
        }
        let command = wire::WindowCommand::decode(payload)?;
        self.pending.push(Pending {
            id,
            command,
            phase: Phase::Queued,
        });
        Ok(())
    }

    pub(super) fn cancel(&mut self, id: u64) {
        self.pending.retain(|request| request.id != id);
    }

    pub(crate) fn queued(&self) -> bool {
        self.pending
            .iter()
            .any(|request| request.phase == Phase::Queued)
    }
}

/// Host-only authorization envelope: neither the window ID nor the instance token
/// can be supplied by a guest. Clones cannot submit a request twice.
#[derive(Clone, Debug)]
pub struct WindowEffect {
    surface: Surface,
    instance: Arc<AtomicBool>,
    id: u64,
}

/// Produces app messages only; no native effect is submitted by this stage.
pub fn prepare_window_effects(
    running: Vec<Running>,
    window: iced::window::Id,
) -> iced::Task<WindowEffect> {
    let Some(app) = running.iter().find(|app| app.window == window) else {
        return iced::Task::none();
    };
    let mut guest = app.surface.0.lock().expect("guest lock");
    if guest.fault.is_some() || !guest.alive.load(Ordering::Relaxed) {
        return iced::Task::none();
    }
    let instance = guest.alive.clone();
    iced::Task::batch(
        guest
            .window_effects
            .pending
            .iter_mut()
            .filter_map(|request| {
                if request.phase != Phase::Queued {
                    return None;
                }
                request.phase = Phase::Prepared;
                Some(iced::Task::done(WindowEffect {
                    surface: app.surface.clone(),
                    instance: instance.clone(),
                    id: request.id,
                }))
            }),
    )
}

fn current<'a>(running: &'a [Running], effect: &WindowEffect) -> Option<&'a Running> {
    running.iter().find(|app| app.surface == effect.surface)
}

fn same_instance(guest: &Guest, effect: &WindowEffect) -> bool {
    Arc::ptr_eq(&guest.alive, &effect.instance)
        && effect.instance.load(Ordering::Relaxed)
        && guest.fault.is_none()
}

fn reject(guest: &mut Guest, effect: &WindowEffect, message: &str) {
    let error = format!("RequestError: {message}");
    guest.log_session.append(&error);
    // Never deliver an old request ID to a replacement instance.
    if same_instance(guest, effect) {
        guest.window_effects.cancel(effect.id);
        guest.reply(Instant::now(), effect.id, Err(error));
    }
}

/// Called by an Ice handler on the UI thread. Revalidate after the prepared
/// message was queued, before returning any native window action.
pub fn commit_window_effect(
    running: Vec<Running>,
    effect: WindowEffect,
) -> iced::Task<WindowEffect> {
    let mut guest = effect.surface.0.lock().expect("guest lock");
    let Some(app) = current(&running, &effect) else {
        reject(&mut guest, &effect, "guest window is no longer running");
        return iced::Task::none();
    };
    if !same_instance(&guest, &effect) {
        reject(
            &mut guest,
            &effect,
            "window command belongs to a replaced or stopped instance",
        );
        return iced::Task::none();
    }
    let Some(request) = guest
        .window_effects
        .pending
        .iter_mut()
        .find(|request| request.id == effect.id && request.phase == Phase::Prepared)
    else {
        // Cancellation has no response, and a duplicate must not cancel the
        // original submitted request's eventual acknowledgement.
        guest
            .log_session
            .append("RequestError: window command was cancelled or already submitted");
        return iced::Task::none();
    };
    request.phase = Phase::Submitted;
    let task = match request.command {
        wire::WindowCommand::Focus => iced::window::gain_focus(app.window),
        wire::WindowCommand::Resize { width, height } => {
            iced::window::resize(app.window, iced::Size::new(width, height))
        }
        wire::WindowCommand::Close => iced::window::close(app.window),
    };
    drop(guest);
    task.chain(iced::Task::done(effect))
}

/// Acknowledges native runtime submission, not that the OS applied the change.
/// A closed window has no guest left to receive an acknowledgement.
pub fn complete_window_effect(running: &[Running], effect: WindowEffect) -> bool {
    let mut guest = effect.surface.0.lock().expect("guest lock");
    if current(running, &effect).is_none() || !same_instance(&guest, &effect) {
        reject(
            &mut guest,
            &effect,
            "window completion belongs to a closed or replaced instance",
        );
        return false;
    }
    if guest
        .window_effects
        .pending
        .iter()
        .any(|request| request.id == effect.id && request.phase == Phase::Submitted)
    {
        guest.window_effects.cancel(effect.id);
        guest.reply(Instant::now(), effect.id, Ok(wire::encode(&())));
        return true;
    }
    false
}

#[derive(Clone, Debug)]
pub struct GuestNotice {
    pub window: iced::window::Id,
    pub kind: String,
}

pub fn guest_notice(window: iced::window::Id, kind: String) -> GuestNotice {
    GuestNotice { window, kind }
}

#[cfg(test)]
#[path = "window_effects_tests.rs"]
mod tests;
