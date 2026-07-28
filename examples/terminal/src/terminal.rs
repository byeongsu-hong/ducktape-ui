use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer};
use iced::{Element, Length, Rectangle, Size, Subscription, Theme, mouse};
use iced_term::actions::Action;
use iced_term::settings::{BackendSettings, FontSettings, Settings, ThemeSettings};
use iced_term::{ColorPalette, Command, Event, Terminal, TerminalView};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct Session {
    id: u64,
    terminal: Option<Arc<Mutex<Terminal>>>,
}

impl Hash for Session {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Clone)]
pub struct Environment {
    pub shell: String,
    pub directory: String,
    pub ssh_available: bool,
    pub claude_available: bool,
    pub codex_available: bool,
}

#[derive(Debug, Clone)]
pub struct Notice {
    pub running: bool,
    pub title: String,
}

#[derive(Clone)]
pub struct Started {
    pub session: Session,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct TerminalError {
    pub message: String,
}

struct Launch {
    program: String,
    args: Vec<String>,
    title: String,
}

pub fn idle_session() -> Session {
    Session {
        id: 0,
        terminal: None,
    }
}

pub fn detect_environment() -> Environment {
    let shell = system_shell();
    Environment {
        shell: shell.clone(),
        directory: std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        ssh_available: command_available("ssh"),
        claude_available: command_available("claude"),
        codex_available: command_available("codex"),
    }
}

pub async fn start_session(
    kind: String,
    target: String,
    directory: String,
) -> Result<Started, TerminalError> {
    let working_directory = resolve_directory(&directory)?;
    let launch = launch(&kind, &target)?;
    let program = which::which(&launch.program).map_err(|_| TerminalError {
        message: format!(
            "`{}` is not installed or is not available on PATH.",
            launch.program
        ),
    })?;
    let id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    let title = launch.title;
    let settings = terminal_settings(program, launch.args, working_directory);
    let terminal = Terminal::new(id, settings).map_err(|error| TerminalError {
        message: format!("Could not start {title}: {error}"),
    })?;

    Ok(Started {
        session: Session {
            id,
            terminal: Some(Arc::new(Mutex::new(terminal))),
        },
        title,
    })
}

pub fn terminal_events(session: Session) -> Subscription<Notice> {
    let Some(terminal) = session.terminal.clone() else {
        return Subscription::none();
    };
    let subscription = lock(&terminal).subscription();

    subscription.with(session).map(handle_subscription_event)
}

pub fn terminal_surface(session: &Session) -> Element<'static, ()> {
    let Some(terminal) = session.terminal.clone() else {
        return iced::widget::text("No active terminal").into();
    };

    Element::new(SharedTerminal {
        terminal,
        id: session.id,
    })
}

fn process_event(terminal: &Arc<Mutex<Terminal>>, id: u64, event: Event) -> Action {
    let Event::BackendCall(event_id, command) = event;
    if event_id != id {
        return Action::Ignore;
    }

    lock(terminal).handle(Command::ProxyToBackend(command))
}

fn handle_subscription_event((session, event): (Session, Event)) -> Notice {
    let Some(terminal) = &session.terminal else {
        return Notice {
            running: false,
            title: String::new(),
        };
    };
    let action = process_event(terminal, session.id, event);
    match action {
        Action::Shutdown => Notice {
            running: false,
            title: "Session ended".into(),
        },
        Action::ChangeTitle(title) => Notice {
            running: true,
            title,
        },
        Action::Ignore => Notice {
            running: true,
            title: String::new(),
        },
    }
}

fn terminal_settings(program: PathBuf, args: Vec<String>, working_directory: PathBuf) -> Settings {
    let mut environment = HashMap::new();
    environment.insert("TERM".into(), "xterm-256color".into());
    environment.insert("COLORTERM".into(), "truecolor".into());
    environment.insert("TERM_PROGRAM".into(), "ice-terminal".into());

    let palette = ColorPalette {
        foreground: "#e7eaf0".into(),
        background: "#090b0e".into(),
        black: "#1b2028".into(),
        red: "#ff7b86".into(),
        green: "#6fdc8c".into(),
        yellow: "#f4c95d".into(),
        blue: "#7c9cff".into(),
        magenta: "#c792ea".into(),
        cyan: "#66d9d0".into(),
        white: "#d9dee8".into(),
        bright_black: "#697386".into(),
        bright_red: "#ff9aa3".into(),
        bright_green: "#8be9a8".into(),
        bright_yellow: "#ffe08a".into(),
        bright_blue: "#a6baff".into(),
        bright_magenta: "#ddb3f5".into(),
        bright_cyan: "#8ce8e1".into(),
        bright_white: "#ffffff".into(),
        ..ColorPalette::default()
    };

    Settings {
        font: FontSettings {
            size: 14.0,
            scale_factor: 1.25,
            font_type: iced::Font::MONOSPACE,
        },
        theme: ThemeSettings::new(Box::new(palette)),
        backend: BackendSettings {
            program: program.to_string_lossy().into_owned(),
            args,
            env: environment,
            working_directory: Some(working_directory),
        },
    }
}

fn launch(kind: &str, target: &str) -> Result<Launch, TerminalError> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "shell" => Ok(Launch {
            program: system_shell(),
            args: Vec::new(),
            title: "Local shell".into(),
        }),
        "ssh" => {
            let args = ssh_args(target)?;
            Ok(Launch {
                program: "ssh".into(),
                args,
                title: format!("SSH · {}", target.trim()),
            })
        }
        "claude" => Ok(Launch {
            program: "claude".into(),
            args: Vec::new(),
            title: "Claude Code".into(),
        }),
        "codex" => Ok(Launch {
            program: "codex".into(),
            args: Vec::new(),
            title: "Codex".into(),
        }),
        _ => Err(TerminalError {
            message: format!("Unknown terminal session kind `{}`.", kind.trim()),
        }),
    }
}

fn ssh_args(target: &str) -> Result<Vec<String>, TerminalError> {
    let mut args = shell_words::split(target.trim()).map_err(|error| TerminalError {
        message: format!("Invalid SSH destination: {error}"),
    })?;
    if args.first().is_some_and(|arg| arg == "ssh") {
        args.remove(0);
    }
    if args.is_empty() {
        return Err(TerminalError {
            message: "Enter an SSH destination such as `user@host`.".into(),
        });
    }

    Ok(args)
}

fn resolve_directory(directory: &str) -> Result<PathBuf, TerminalError> {
    let input = directory.trim();
    let path = if input.is_empty() {
        std::env::current_dir().map_err(|error| TerminalError {
            message: format!("Could not read the current directory: {error}"),
        })?
    } else {
        expand_home(input)
    };
    let path = path.canonicalize().map_err(|error| TerminalError {
        message: format!(
            "Working directory `{}` is not accessible: {error}",
            path.display()
        ),
    })?;
    if !path.is_dir() {
        return Err(TerminalError {
            message: format!("Working directory `{}` is not a directory.", path.display()),
        });
    }

    Ok(path)
}

fn expand_home(input: &str) -> PathBuf {
    if (input == "~" || input.starts_with("~/") || input.starts_with("~\\"))
        && let Some(home) = home_directory()
    {
        return if input.len() == 1 {
            home
        } else {
            home.join(&input[2..])
        };
    }

    PathBuf::from(input)
}

fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn system_shell() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    }
}

fn command_available(program: &str) -> bool {
    which::which(program).is_ok()
}

fn lock(terminal: &Arc<Mutex<Terminal>>) -> MutexGuard<'_, Terminal> {
    terminal
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct SharedTerminal {
    terminal: Arc<Mutex<Terminal>>,
    id: u64,
}

struct SharedTerminalState {
    session_id: u64,
}

impl SharedTerminalState {
    fn switch_to(&mut self, session_id: u64) -> bool {
        if self.session_id == session_id {
            false
        } else {
            self.session_id = session_id;
            true
        }
    }
}

impl Widget<(), Theme, iced::Renderer> for SharedTerminal {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SharedTerminalState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(SharedTerminalState {
            session_id: self.id,
        })
    }

    fn children(&self) -> Vec<Tree> {
        let terminal = lock(&self.terminal);
        let content = TerminalView::show(&terminal);
        vec![Tree::new(content.as_widget())]
    }

    fn diff(&self, tree: &mut Tree) {
        let terminal = lock(&self.terminal);
        let content = TerminalView::show(&terminal);
        let session_changed = tree
            .state
            .downcast_mut::<SharedTerminalState>()
            .switch_to(self.id);
        if session_changed {
            tree.children = vec![Tree::new(content.as_widget())];
        } else {
            tree.diff_children(&[content.as_widget()]);
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let terminal = lock(&self.terminal);
        let mut content = TerminalView::show(&terminal);
        content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let terminal = lock(&self.terminal);
        let mut content = TerminalView::show(&terminal);
        operation.traverse(&mut |operation| {
            content
                .as_widget_mut()
                .operate(&mut tree.children[0], layout, renderer, operation);
        });
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let terminal = lock(&self.terminal);
        let content = TerminalView::show(&terminal);
        content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, ()>,
        viewport: &Rectangle,
    ) {
        let mut messages = Vec::new();
        let mut terminal_shell = Shell::new(&mut messages);
        {
            let terminal = lock(&self.terminal);
            let mut content = TerminalView::show(&terminal);
            content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                &mut terminal_shell,
                viewport,
            );
        }
        let terminal = self.terminal.clone();
        let id = self.id;
        shell.merge(terminal_shell, move |event| {
            process_event(&terminal, id, event);
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let terminal = lock(&self.terminal);
        let content = TerminalView::show(&terminal);
        content
            .as_widget()
            .mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }
}

#[cfg(test)]
mod tests {
    use super::{SharedTerminalState, launch, ssh_args};

    #[test]
    fn ssh_accepts_a_destination_or_full_command() {
        assert_eq!(ssh_args("user@example.com").unwrap(), ["user@example.com"]);
        assert_eq!(
            ssh_args("ssh -p 2222 'user@example.com'").unwrap(),
            ["-p", "2222", "user@example.com"]
        );
    }

    #[test]
    fn ssh_rejects_empty_or_unclosed_input() {
        assert!(ssh_args("ssh").is_err());
        assert!(ssh_args("ssh 'user@example.com").is_err());
    }

    #[test]
    fn agent_sessions_launch_the_cli_directly() {
        let claude = launch("claude", "ignored").unwrap();
        assert_eq!(claude.program, "claude");
        assert!(claude.args.is_empty());

        let codex = launch("codex", "ignored").unwrap();
        assert_eq!(codex.program, "codex");
        assert!(codex.args.is_empty());
    }

    #[test]
    fn terminal_view_state_resets_for_a_new_session() {
        let mut state = SharedTerminalState { session_id: 1 };

        assert!(!state.switch_to(1));
        assert!(state.switch_to(2));
        assert!(!state.switch_to(2));
    }
}
