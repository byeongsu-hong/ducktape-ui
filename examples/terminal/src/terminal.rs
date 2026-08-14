//! The demo's launcher policy: WHICH program a button means, and where it runs.
//!
//! Everything below the pty — the engine, the grid, the widget, input, mouse,
//! selection, clipboard — is `ui_lang_components::ui::terminal`. This file is
//! only the part a real application would write for itself: resolving a shell
//! or an ssh target into an argv, and a working directory into a path.

use std::path::PathBuf;

use ui_lang_components::ui::terminal;

pub use terminal::{
    Notice, Session, TerminalError, focus_terminal, idle_session, terminal_attention,
    terminal_events, terminal_surface,
};

#[derive(Clone)]
pub struct Environment {
    pub shell: String,
    pub directory: String,
    pub ssh_available: bool,
    pub claude_available: bool,
    pub codex_available: bool,
}

#[derive(Clone)]
pub struct Started {
    pub session: Session,
    pub title: String,
}

struct Launch {
    program: String,
    args: Vec<String>,
    title: String,
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
    let title = launch.title;
    let session = terminal::spawn_session(program, launch.args, working_directory, title.clone())?;

    Ok(Started { session, title })
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

#[cfg(test)]
mod tests {
    use super::{launch, ssh_args};

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
}
