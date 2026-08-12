use alacritty_terminal::event::{
    Event as AlacrittyEvent, EventListener, Notify, OnResize, WindowSize,
};
use alacritty_terminal::event_loop::{EventLoop, Msg, Notifier};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Point as GridPoint, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{self, Term, TermMode, cell::Flags, viewport_to_point};
use alacritty_terminal::tty;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, CursorShape, NamedColor, Rgb};
use iced::advanced::text::{Paragraph as _, Renderer as _};
use iced::advanced::widget::{Operation, Tree, operation, tree};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, clipboard, input_method, layout,
    mouse as advanced_mouse, renderer, text,
};
use iced::alignment;
use iced::font::{Style as FontStyle, Weight as FontWeight};
use iced::futures::stream::BoxStream;
use iced::futures::{SinkExt, StreamExt};
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::mouse::{self, ScrollDelta};
use iced::{
    Background, Border, Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Shadow,
    Size, Subscription, Theme, window,
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedReceiver};

const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 1.4;
const FRAME_INTERVAL: Duration = Duration::from_millis(8);
const DEFAULT_COLUMNS: u16 = 80;
const DEFAULT_LINES: u16 = 24;
const DEFAULT_CELL_WIDTH: u16 = 9;
const DEFAULT_CELL_HEIGHT: u16 = 20;

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
    let terminal = Terminal::new(id, program, launch.args, working_directory).map_err(|error| {
        TerminalError {
            message: format!("Could not start {title}: {error}"),
        }
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

    lock(&terminal)
        .subscription()
        .with(session)
        .map(handle_event_batch)
}

pub fn terminal_surface(session: &Session) -> Element<'static, ()> {
    let Some(terminal) = session.terminal.clone() else {
        return iced::widget::container(iced::widget::text("No active terminal"))
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into();
    };

    Element::new(TerminalSurface {
        terminal,
        session_id: session.id,
    })
}

pub fn focus_terminal(session: Session) -> iced::Task<()> {
    let Some(terminal) = session.terminal else {
        return iced::Task::none();
    };
    let widget_id = lock(&terminal).widget_id.clone();

    iced::widget::operation::focus(widget_id)
}

fn handle_event_batch((session, events): (Session, Vec<AlacrittyEvent>)) -> Notice {
    let Some(terminal) = &session.terminal else {
        return Notice {
            running: false,
            title: String::new(),
        };
    };

    lock(terminal).handle_events(events)
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

#[derive(Clone)]
struct TerminalSubscription {
    id: u64,
    events: Arc<tokio::sync::Mutex<UnboundedReceiver<AlacrittyEvent>>>,
}

impl Hash for TerminalSubscription {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

fn terminal_event_stream(data: &TerminalSubscription) -> BoxStream<'static, Vec<AlacrittyEvent>> {
    let events = data.events.clone();
    iced::stream::channel(8, async move |mut output| {
        loop {
            let first = {
                let mut events = events.lock().await;
                events.recv().await
            };
            let Some(first) = first else {
                break;
            };

            let mut batch = vec![first];
            tokio::time::sleep(FRAME_INTERVAL).await;
            {
                let mut events = events.lock().await;
                while let Ok(event) = events.try_recv() {
                    batch.push(event);
                }
            }

            if output.send(batch).await.is_err() {
                break;
            }
        }
    })
    .boxed()
}

#[derive(Clone)]
struct EventProxy {
    sender: mpsc::UnboundedSender<AlacrittyEvent>,
    wakeup_pending: Arc<AtomicBool>,
}

impl EventListener for EventProxy {
    fn send_event(&self, event: AlacrittyEvent) {
        if matches!(event, AlacrittyEvent::Wakeup)
            && self.wakeup_pending.swap(true, Ordering::AcqRel)
        {
            return;
        }

        let _ = self.sender.send(event);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TerminalSize {
    columns: u16,
    lines: u16,
    cell_width: u16,
    cell_height: u16,
}

impl TerminalSize {
    fn fit(bounds: Size, cell: Size) -> Self {
        let cell_width = cell.width.round().max(1.0) as u16;
        let cell_height = cell.height.round().max(1.0) as u16;

        Self {
            columns: (bounds.width / f32::from(cell_width)).floor().max(1.0) as u16,
            lines: (bounds.height / f32::from(cell_height)).floor().max(1.0) as u16,
            cell_width,
            cell_height,
        }
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            columns: DEFAULT_COLUMNS,
            lines: DEFAULT_LINES,
            cell_width: DEFAULT_CELL_WIDTH,
            cell_height: DEFAULT_CELL_HEIGHT,
        }
    }
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        self.lines as usize
    }

    fn screen_lines(&self) -> usize {
        self.lines as usize
    }

    fn columns(&self) -> usize {
        self.columns as usize
    }
}

impl From<TerminalSize> for WindowSize {
    fn from(size: TerminalSize) -> Self {
        Self {
            num_lines: size.lines,
            num_cols: size.columns,
            cell_width: size.cell_width,
            cell_height: size.cell_height,
        }
    }
}

struct Terminal {
    id: u64,
    widget_id: iced::widget::Id,
    term: Arc<FairMutex<Term<EventProxy>>>,
    notifier: Notifier,
    events: Arc<tokio::sync::Mutex<UnboundedReceiver<AlacrittyEvent>>>,
    wakeup_pending: Arc<AtomicBool>,
    size: TerminalSize,
    frame: Arc<TerminalFrame>,
}

impl Terminal {
    fn new(
        id: u64,
        program: PathBuf,
        args: Vec<String>,
        working_directory: PathBuf,
    ) -> std::io::Result<Self> {
        let mut environment = HashMap::new();
        environment.insert("TERM".into(), "xterm-256color".into());
        environment.insert("COLORTERM".into(), "truecolor".into());
        environment.insert("TERM_PROGRAM".into(), "ice-terminal".into());
        let options = tty::Options {
            shell: Some(tty::Shell::new(
                program.to_string_lossy().into_owned(),
                args,
            )),
            working_directory: Some(working_directory),
            env: environment,
            ..tty::Options::default()
        };
        let size = TerminalSize::default();
        let pty = tty::new(&options, size.into(), id)?;
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let wakeup_pending = Arc::new(AtomicBool::new(false));
        let event_proxy = EventProxy {
            sender: event_sender,
            wakeup_pending: wakeup_pending.clone(),
        };
        let term = Arc::new(FairMutex::new(Term::new(
            term::Config::default(),
            &size,
            event_proxy.clone(),
        )));
        let event_loop = EventLoop::new(term.clone(), event_proxy, pty, false, false)?;
        let notifier = Notifier(event_loop.channel());
        let _ = event_loop.spawn();
        let mut terminal = Self {
            id,
            widget_id: iced::widget::Id::unique(),
            term,
            notifier,
            events: Arc::new(tokio::sync::Mutex::new(event_receiver)),
            wakeup_pending,
            size,
            frame: Arc::new(TerminalFrame::empty()),
        };
        terminal.snapshot();

        Ok(terminal)
    }

    fn subscription(&self) -> Subscription<Vec<AlacrittyEvent>> {
        Subscription::run_with(
            TerminalSubscription {
                id: self.id,
                events: self.events.clone(),
            },
            terminal_event_stream,
        )
    }

    fn handle_events(&mut self, events: Vec<AlacrittyEvent>) -> Notice {
        let mut running = true;
        let mut title = String::new();
        let mut needs_snapshot = false;

        for event in events {
            match event {
                AlacrittyEvent::Wakeup => {
                    self.wakeup_pending.store(false, Ordering::Release);
                    needs_snapshot = true;
                }
                AlacrittyEvent::Title(next) => title = next,
                AlacrittyEvent::ResetTitle => title = "Terminal".into(),
                AlacrittyEvent::PtyWrite(text) => self.notifier.notify(text.into_bytes()),
                AlacrittyEvent::TextAreaSizeRequest(formatter) => {
                    self.notifier
                        .notify(formatter(self.size.into()).into_bytes());
                }
                AlacrittyEvent::ColorRequest(index, formatter) => {
                    self.notifier
                        .notify(formatter(palette_rgb(index)).into_bytes());
                }
                AlacrittyEvent::Exit | AlacrittyEvent::ChildExit(_) => running = false,
                AlacrittyEvent::MouseCursorDirty
                | AlacrittyEvent::ClipboardStore(_, _)
                | AlacrittyEvent::ClipboardLoad(_, _)
                | AlacrittyEvent::CursorBlinkingChange
                | AlacrittyEvent::Bell => {}
            }
        }

        if needs_snapshot {
            self.snapshot();
        }

        Notice { running, title }
    }

    fn resize(&mut self, bounds: Size, cell: Size) -> bool {
        let next = TerminalSize::fit(bounds, cell);
        if next == self.size {
            return false;
        }

        self.size = next;
        self.notifier.on_resize(next.into());
        self.term.lock().resize(next);
        self.snapshot();
        true
    }

    fn write(&self, bytes: impl Into<std::borrow::Cow<'static, [u8]>>) {
        self.notifier.notify(bytes);
    }

    fn mode(&self) -> TermMode {
        *self.term.lock().mode()
    }

    fn paste(&self, text: String) {
        if self.mode().contains(TermMode::BRACKETED_PASTE) {
            let text = text.replace('\u{1b}', "");
            self.write(format!("\x1b[200~{text}\x1b[201~").into_bytes());
        } else {
            self.write(text.into_bytes());
        }
    }

    fn selected_text(&self) -> Option<String> {
        self.term.lock().selection_to_string()
    }

    fn scroll(&mut self, lines: i32) {
        if lines == 0 {
            return;
        }
        let mut term = self.term.lock();
        let mode = *term.mode();
        if mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL) {
            let suffix = if lines > 0 { b'A' } else { b'B' };
            let mut bytes = Vec::with_capacity(lines.unsigned_abs() as usize * 3);
            for _ in 0..lines.unsigned_abs() {
                bytes.extend_from_slice(&[0x1b, b'O', suffix]);
            }
            drop(term);
            self.write(bytes);
        } else {
            term.scroll_display(Scroll::Delta(lines));
            drop(term);
            self.snapshot();
        }
    }

    fn start_selection(&mut self, cell: GridPoint<usize>, ty: SelectionType, side: Side) {
        let mut term = self.term.lock();
        let point = viewport_to_point(term.grid().display_offset(), cell);
        term.selection = Some(Selection::new(ty, point, side));
        drop(term);
        self.snapshot();
    }

    fn update_selection(&mut self, cell: GridPoint<usize>, side: Side) {
        let mut term = self.term.lock();
        let point = viewport_to_point(term.grid().display_offset(), cell);
        if let Some(selection) = term.selection.as_mut() {
            selection.update(point, side);
        }
        drop(term);
        self.snapshot();
    }

    fn mouse_report(
        &self,
        button: u8,
        modifiers: Modifiers,
        cell: GridPoint<usize>,
        pressed: bool,
    ) {
        let mode = self.mode();
        let mut code = button;
        if modifiers.shift() {
            code += 4;
        }
        if modifiers.alt() {
            code += 8;
        }
        if modifiers.control() {
            code += 16;
        }

        if mode.contains(TermMode::SGR_MOUSE) {
            let suffix = if pressed { 'M' } else { 'm' };
            self.write(
                format!(
                    "\x1b[<{code};{};{}{suffix}",
                    cell.column.0 + 1,
                    cell.line + 1
                )
                .into_bytes(),
            );
        } else if pressed {
            let column = cell.column.0.min(222) as u8 + 33;
            let line = cell.line.min(222) as u8 + 33;
            self.write(vec![0x1b, b'[', b'M', code + 32, column, line]);
        } else {
            let column = cell.column.0.min(222) as u8 + 33;
            let line = cell.line.min(222) as u8 + 33;
            self.write(vec![0x1b, b'[', b'M', 35, column, line]);
        }
    }

    fn snapshot(&mut self) {
        let mut term = self.term.lock();
        let frame = TerminalFrame::from_term(&term, self.size);
        term.reset_damage();
        self.frame = Arc::new(frame);
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.notifier.0.send(Msg::Shutdown);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rgb8(u8, u8, u8);

impl Rgb8 {
    const fn iced(self) -> Color {
        Color::from_rgb8(self.0, self.1, self.2)
    }

    fn dimmed(self) -> Self {
        Self(
            (f32::from(self.0) * 0.66) as u8,
            (f32::from(self.1) * 0.66) as u8,
            (f32::from(self.2) * 0.66) as u8,
        )
    }
}

const TERMINAL_BACKGROUND: Rgb8 = Rgb8(16, 15, 13);
const TERMINAL_FOREGROUND: Rgb8 = Rgb8(235, 229, 218);
const ANSI_PALETTE: [Rgb8; 16] = [
    Rgb8(35, 33, 29),
    Rgb8(224, 112, 101),
    Rgb8(126, 181, 118),
    Rgb8(218, 173, 91),
    Rgb8(109, 155, 203),
    Rgb8(181, 132, 190),
    Rgb8(102, 174, 169),
    Rgb8(211, 204, 191),
    Rgb8(112, 105, 94),
    Rgb8(239, 137, 124),
    Rgb8(151, 203, 140),
    Rgb8(235, 193, 111),
    Rgb8(139, 181, 225),
    Rgb8(204, 157, 213),
    Rgb8(127, 198, 192),
    Rgb8(246, 241, 232),
];

fn palette_rgb(index: usize) -> Rgb {
    let Rgb8(r, g, b) = indexed_color(index.min(255));
    Rgb { r, g, b }
}

fn indexed_color(index: usize) -> Rgb8 {
    match index {
        0..=15 => ANSI_PALETTE[index],
        16..=231 => {
            let index = index - 16;
            let component = |value: usize| {
                if value == 0 { 0 } else { 55 + value as u8 * 40 }
            };
            Rgb8(
                component(index / 36),
                component(index / 6 % 6),
                component(index % 6),
            )
        }
        232..=255 => {
            let value = 8 + (index as u8 - 232) * 10;
            Rgb8(value, value, value)
        }
        _ => TERMINAL_FOREGROUND,
    }
}

fn named_color(color: NamedColor) -> Rgb8 {
    match color {
        NamedColor::Black => ANSI_PALETTE[0],
        NamedColor::Red => ANSI_PALETTE[1],
        NamedColor::Green => ANSI_PALETTE[2],
        NamedColor::Yellow => ANSI_PALETTE[3],
        NamedColor::Blue => ANSI_PALETTE[4],
        NamedColor::Magenta => ANSI_PALETTE[5],
        NamedColor::Cyan => ANSI_PALETTE[6],
        NamedColor::White => ANSI_PALETTE[7],
        NamedColor::BrightBlack => ANSI_PALETTE[8],
        NamedColor::BrightRed => ANSI_PALETTE[9],
        NamedColor::BrightGreen => ANSI_PALETTE[10],
        NamedColor::BrightYellow => ANSI_PALETTE[11],
        NamedColor::BrightBlue => ANSI_PALETTE[12],
        NamedColor::BrightMagenta => ANSI_PALETTE[13],
        NamedColor::BrightCyan => ANSI_PALETTE[14],
        NamedColor::BrightWhite | NamedColor::BrightForeground => ANSI_PALETTE[15],
        NamedColor::Foreground => TERMINAL_FOREGROUND,
        NamedColor::Background => TERMINAL_BACKGROUND,
        NamedColor::Cursor => Rgb8(232, 177, 93),
        NamedColor::DimBlack => ANSI_PALETTE[0].dimmed(),
        NamedColor::DimRed => ANSI_PALETTE[1].dimmed(),
        NamedColor::DimGreen => ANSI_PALETTE[2].dimmed(),
        NamedColor::DimYellow => ANSI_PALETTE[3].dimmed(),
        NamedColor::DimBlue => ANSI_PALETTE[4].dimmed(),
        NamedColor::DimMagenta => ANSI_PALETTE[5].dimmed(),
        NamedColor::DimCyan => ANSI_PALETTE[6].dimmed(),
        NamedColor::DimWhite => ANSI_PALETTE[7].dimmed(),
        NamedColor::DimForeground => TERMINAL_FOREGROUND.dimmed(),
    }
}

fn resolve_color(color: AnsiColor, colors: &term::color::Colors) -> Rgb8 {
    let dynamic = match color {
        AnsiColor::Named(named) => colors[named],
        AnsiColor::Indexed(index) => colors[index as usize],
        AnsiColor::Spec(rgb) => return Rgb8(rgb.r, rgb.g, rgb.b),
    };
    if let Some(rgb) = dynamic {
        return Rgb8(rgb.r, rgb.g, rgb.b);
    }

    match color {
        AnsiColor::Named(named) => named_color(named),
        AnsiColor::Indexed(index) => indexed_color(index as usize),
        AnsiColor::Spec(_) => unreachable!(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TextStyle {
    color: Rgb8,
    bold: bool,
    italic: bool,
}

#[derive(Debug)]
struct PaintCell {
    row: u16,
    column: u16,
    character: char,
    zerowidth: Vec<char>,
    foreground: Rgb8,
    background: Rgb8,
    flags: Flags,
}

#[derive(Debug)]
struct TextRun {
    row: u16,
    column: u16,
    columns: u16,
    content: String,
    style: TextStyle,
}

#[derive(Debug)]
struct ColorRun {
    row: u16,
    column: u16,
    columns: u16,
    color: Rgb8,
}

#[derive(Debug)]
struct LineRun {
    row: u16,
    column: u16,
    columns: u16,
    color: Rgb8,
    strike: bool,
}

#[derive(Debug, Clone, Copy)]
struct CursorPaint {
    row: u16,
    column: u16,
    shape: CursorShape,
    color: Rgb8,
}

#[derive(Debug)]
struct TerminalFrame {
    text: Vec<TextRun>,
    backgrounds: Vec<ColorRun>,
    lines: Vec<LineRun>,
    cursor: Option<CursorPaint>,
}

impl TerminalFrame {
    fn empty() -> Self {
        Self {
            text: Vec::new(),
            backgrounds: Vec::new(),
            lines: Vec::new(),
            cursor: None,
        }
    }

    fn from_term(term: &Term<EventProxy>, size: TerminalSize) -> Self {
        let content = term.renderable_content();
        let display_offset = content.display_offset as i32;
        let cursor_row = content.cursor.point.line.0 + display_offset;
        let cursor_column = content.cursor.point.column.0;
        let cursor = (content.cursor.shape != CursorShape::Hidden
            && cursor_row >= 0
            && cursor_row < i32::from(size.lines)
            && cursor_column < size.columns as usize)
            .then(|| CursorPaint {
                row: cursor_row as u16,
                column: cursor_column as u16,
                shape: content.cursor.shape,
                color: resolve_color(AnsiColor::Named(NamedColor::Cursor), content.colors),
            });
        let mut cells = Vec::with_capacity(size.lines as usize * size.columns as usize);

        for indexed in content.display_iter {
            let row = indexed.point.line.0 + display_offset;
            if row < 0 || row >= i32::from(size.lines) {
                continue;
            }
            let mut foreground = resolve_color(indexed.fg, content.colors);
            let mut background = resolve_color(indexed.bg, content.colors);
            if indexed.flags.intersects(Flags::DIM | Flags::DIM_BOLD) {
                foreground = foreground.dimmed();
            }
            if indexed.flags.contains(Flags::INVERSE)
                || content
                    .selection
                    .is_some_and(|selection| selection.contains(indexed.point))
            {
                std::mem::swap(&mut foreground, &mut background);
            }
            if cursor.is_some_and(|cursor| {
                cursor.shape == CursorShape::Block
                    && cursor.row == row as u16
                    && cursor.column == indexed.point.column.0 as u16
            }) {
                foreground = background;
            }

            cells.push(PaintCell {
                row: row as u16,
                column: indexed.point.column.0 as u16,
                character: indexed.c,
                zerowidth: indexed.zerowidth().unwrap_or_default().to_vec(),
                foreground,
                background,
                flags: indexed.flags,
            });
        }

        Self {
            text: build_text_runs(&cells),
            backgrounds: build_color_runs(&cells),
            lines: build_line_runs(&cells),
            cursor,
        }
    }
}

fn build_text_runs(cells: &[PaintCell]) -> Vec<TextRun> {
    let mut runs = Vec::new();
    let mut index = 0;
    while index < cells.len() {
        let cell = &cells[index];
        if cell.flags.contains(Flags::WIDE_CHAR_SPACER)
            || cell.flags.contains(Flags::HIDDEN)
            || cell.character == ' '
            || cell.character == '\t'
        {
            index += 1;
            continue;
        }

        let style = TextStyle {
            color: cell.foreground,
            bold: cell.flags.intersects(Flags::BOLD | Flags::DIM_BOLD),
            italic: cell.flags.contains(Flags::ITALIC),
        };
        let row = cell.row;
        let column = cell.column;
        let mut next_column = column;
        let mut content = String::new();
        let mut pending_spaces = 0u16;

        while index < cells.len() {
            let cell = &cells[index];
            if cell.row != row {
                break;
            }
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                index += 1;
                continue;
            }
            let cell_style = TextStyle {
                color: cell.foreground,
                bold: cell.flags.intersects(Flags::BOLD | Flags::DIM_BOLD),
                italic: cell.flags.contains(Flags::ITALIC),
            };
            if cell_style != style || cell.column < next_column {
                break;
            }
            if cell.character == ' ' || cell.character == '\t' || cell.flags.contains(Flags::HIDDEN)
            {
                pending_spaces = pending_spaces.saturating_add(1);
                next_column = cell.column.saturating_add(1);
                index += 1;
                continue;
            }
            if cell.column > next_column {
                pending_spaces = pending_spaces.saturating_add(cell.column - next_column);
            }
            content.extend(std::iter::repeat_n(' ', pending_spaces as usize));
            pending_spaces = 0;
            content.push(cell.character);
            content.extend(cell.zerowidth.iter());
            next_column = cell
                .column
                .saturating_add(if cell.flags.contains(Flags::WIDE_CHAR) {
                    2
                } else {
                    1
                });
            index += 1;
        }

        runs.push(TextRun {
            row,
            column,
            columns: next_column.saturating_sub(column),
            content,
            style,
        });
    }

    runs
}

fn build_color_runs(cells: &[PaintCell]) -> Vec<ColorRun> {
    let mut runs: Vec<ColorRun> = Vec::new();
    for cell in cells {
        if cell.background == TERMINAL_BACKGROUND {
            continue;
        }
        if let Some(last) = runs.last_mut()
            && last.row == cell.row
            && last.color == cell.background
            && last.column + last.columns == cell.column
        {
            last.columns += 1;
        } else {
            runs.push(ColorRun {
                row: cell.row,
                column: cell.column,
                columns: 1,
                color: cell.background,
            });
        }
    }
    runs
}

fn build_line_runs(cells: &[PaintCell]) -> Vec<LineRun> {
    let mut runs: Vec<LineRun> = Vec::new();
    for cell in cells {
        let strike = cell.flags.contains(Flags::STRIKEOUT);
        let underline = cell.flags.intersects(Flags::ALL_UNDERLINES);
        if !strike && !underline {
            continue;
        }
        if let Some(last) = runs.last_mut()
            && last.row == cell.row
            && last.color == cell.foreground
            && last.strike == strike
            && last.column + last.columns == cell.column
        {
            last.columns += 1;
        } else {
            runs.push(LineRun {
                row: cell.row,
                column: cell.column,
                columns: 1,
                color: cell.foreground,
                strike,
            });
        }
    }
    runs
}

#[derive(Debug, Default)]
struct SurfaceState {
    session_id: u64,
    focused: bool,
    reported_focus: bool,
    modifiers: Modifiers,
    cell: Size,
    layout: Size,
    mouse_cell: GridPoint<usize>,
    dragging: bool,
    last_click: Option<advanced_mouse::Click>,
    scroll_pixels: f32,
    ime_preedit: Option<input_method::Preedit>,
}

impl operation::Focusable for SurfaceState {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
        self.dragging = false;
    }
}

struct TerminalSurface {
    terminal: Arc<Mutex<Terminal>>,
    session_id: u64,
}

impl Widget<(), Theme, iced::Renderer> for TerminalSurface {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SurfaceState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(SurfaceState {
            session_id: self.session_id,
            cell: Size::new(
                f32::from(DEFAULT_CELL_WIDTH),
                f32::from(DEFAULT_CELL_HEIGHT),
            ),
            ..SurfaceState::default()
        })
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<SurfaceState>();
        if state.session_id != self.session_id {
            *state = SurfaceState {
                session_id: self.session_id,
                cell: state.cell,
                ..SurfaceState::default()
            };
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let paragraph = <iced::Renderer as text::Renderer>::Paragraph::with_text(text::Text {
            content: "M",
            bounds: Size::INFINITE,
            size: Pixels(FONT_SIZE),
            line_height: text::LineHeight::Relative(LINE_HEIGHT),
            font: Font::MONOSPACE,
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Basic,
            wrapping: text::Wrapping::None,
        });
        let measured = paragraph.min_bounds();
        let state = tree.state.downcast_mut::<SurfaceState>();
        state.cell = Size::new(
            measured.width.ceil().max(1.0),
            measured.height.ceil().max(1.0),
        );
        state.layout = size;

        layout::Node::new(size)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let widget_id = &lock(&self.terminal).widget_id;
        operation.focusable(
            Some(widget_id),
            layout.bounds(),
            tree.state.downcast_mut::<SurfaceState>(),
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<SurfaceState>();
        let frame = lock(&self.terminal).frame.clone();
        let bounds = layout.bounds();
        let clip = bounds.intersection(viewport).unwrap_or(bounds);
        renderer.with_layer(clip, |renderer| {
            fill(renderer, bounds, TERMINAL_BACKGROUND.iced());

            for run in &frame.backgrounds {
                fill(
                    renderer,
                    cell_rect(bounds, state.cell, run.row, run.column, run.columns),
                    run.color.iced(),
                );
            }

            if let Some(cursor) = frame.cursor {
                let mut cursor_bounds = cell_rect(bounds, state.cell, cursor.row, cursor.column, 1);
                match cursor.shape {
                    CursorShape::Block => fill(renderer, cursor_bounds, cursor.color.iced()),
                    CursorShape::HollowBlock => renderer.fill_quad(
                        renderer::Quad {
                            bounds: cursor_bounds,
                            border: Border {
                                color: cursor.color.iced(),
                                width: 1.0,
                                radius: 0.0.into(),
                            },
                            shadow: Shadow::default(),
                            snap: true,
                        },
                        Background::Color(Color::TRANSPARENT),
                    ),
                    CursorShape::Beam => {
                        cursor_bounds.width = 2.0;
                        fill(renderer, cursor_bounds, cursor.color.iced());
                    }
                    CursorShape::Underline => {
                        cursor_bounds.y += cursor_bounds.height - 2.0;
                        cursor_bounds.height = 2.0;
                        fill(renderer, cursor_bounds, cursor.color.iced());
                    }
                    CursorShape::Hidden => {}
                }
            }

            for run in &frame.text {
                let mut font = Font::MONOSPACE;
                if run.style.bold {
                    font.weight = FontWeight::Bold;
                }
                if run.style.italic {
                    font.style = FontStyle::Italic;
                }
                renderer.fill_text(
                    text::Text {
                        content: run.content.clone(),
                        bounds: Size::new(
                            state.cell.width * f32::from(run.columns),
                            state.cell.height,
                        ),
                        size: Pixels(FONT_SIZE),
                        line_height: text::LineHeight::Relative(LINE_HEIGHT),
                        font,
                        align_x: text::Alignment::Left,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::Auto,
                        wrapping: text::Wrapping::None,
                    },
                    Point::new(
                        bounds.x + f32::from(run.column) * state.cell.width,
                        bounds.y + f32::from(run.row) * state.cell.height,
                    ),
                    run.style.color.iced(),
                    clip,
                );
            }

            for line in &frame.lines {
                let mut line_bounds =
                    cell_rect(bounds, state.cell, line.row, line.column, line.columns);
                line_bounds.y += if line.strike {
                    line_bounds.height * 0.52
                } else {
                    line_bounds.height - 2.0
                };
                line_bounds.height = 1.0;
                fill(renderer, line_bounds, line.color.iced());
            }
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, ()>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<SurfaceState>();
        let bounds = layout.bounds();
        let mut terminal = lock(&self.terminal);
        if terminal.resize(bounds.size(), state.cell) {
            shell.request_redraw();
        }

        if state.focused != state.reported_focus {
            if terminal.mode().contains(TermMode::FOCUS_IN_OUT) {
                terminal.write(if state.focused {
                    b"\x1b[I".to_vec()
                } else {
                    b"\x1b[O".to_vec()
                });
            }
            state.reported_focus = state.focused;
        }

        let hovered = cursor.is_over(bounds);
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if hovered => {
                state.focused = true;
                let Some(position) = cursor.position() else {
                    return;
                };
                state.mouse_cell = mouse_cell(position, bounds, state.cell, terminal.size);
                let mode = terminal.mode();
                if mode.intersects(TermMode::MOUSE_MODE) && !state.modifiers.shift() {
                    terminal.mouse_report(0, state.modifiers, state.mouse_cell, true);
                } else {
                    let click =
                        advanced_mouse::Click::new(position, mouse::Button::Left, state.last_click);
                    let ty = match click.kind() {
                        advanced_mouse::click::Kind::Single => SelectionType::Simple,
                        advanced_mouse::click::Kind::Double => SelectionType::Semantic,
                        advanced_mouse::click::Kind::Triple => SelectionType::Lines,
                    };
                    state.last_click = Some(click);
                    terminal.start_selection(
                        state.mouse_cell,
                        ty,
                        selection_side(position, bounds, state.cell),
                    );
                    shell.request_redraw();
                }
                state.dragging = true;
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if hovered => {
                state.mouse_cell = mouse_cell(*position, bounds, state.cell, terminal.size);
                if state.dragging {
                    let mode = terminal.mode();
                    if mode.intersects(TermMode::MOUSE_MOTION | TermMode::MOUSE_DRAG)
                        && !state.modifiers.shift()
                    {
                        terminal.mouse_report(32, state.modifiers, state.mouse_cell, true);
                    } else {
                        terminal.update_selection(
                            state.mouse_cell,
                            selection_side(*position, bounds, state.cell),
                        );
                        shell.request_redraw();
                    }
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
                if terminal.mode().intersects(TermMode::MOUSE_MODE) && !state.modifiers.shift() {
                    terminal.mouse_report(0, state.modifiers, state.mouse_cell, false);
                }
                state.dragging = false;
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if hovered => {
                let lines = match delta {
                    ScrollDelta::Lines { y, .. } => y.round() as i32,
                    ScrollDelta::Pixels { y, .. } => {
                        state.scroll_pixels += *y;
                        let lines = (state.scroll_pixels / state.cell.height).trunc() as i32;
                        state.scroll_pixels -= lines as f32 * state.cell.height;
                        lines
                    }
                };
                if terminal.mode().intersects(TermMode::MOUSE_MODE) && !state.modifiers.shift() {
                    let button = if lines >= 0 { 64 } else { 65 };
                    for _ in 0..lines.unsigned_abs() {
                        terminal.mouse_report(button, state.modifiers, state.mouse_cell, true);
                    }
                } else {
                    terminal.scroll(lines);
                    shell.request_redraw();
                }
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                ..
            }) if state.focused => {
                state.modifiers = *modifiers;
                if is_copy_shortcut(key, *modifiers) {
                    if let Some(selection) = terminal.selected_text() {
                        clipboard.write(clipboard::Kind::Standard, selection);
                    }
                } else if is_paste_shortcut(key, *modifiers) {
                    if let Some(text) = clipboard.read(clipboard::Kind::Standard) {
                        terminal.paste(text);
                    }
                } else if let Some(bytes) =
                    encode_key(key, text.as_deref(), *modifiers, terminal.mode())
                {
                    terminal.write(bytes);
                }
                shell.capture_event();
            }
            Event::InputMethod(input_method::Event::Opened) if state.focused => {
                state.ime_preedit = Some(input_method::Preedit::new());
            }
            Event::InputMethod(input_method::Event::Preedit(content, selection))
                if state.focused =>
            {
                state.ime_preedit = Some(input_method::Preedit {
                    content: content.clone(),
                    selection: selection.clone(),
                    text_size: Some(Pixels(FONT_SIZE)),
                });
                shell.request_redraw();
            }
            Event::InputMethod(input_method::Event::Commit(content)) if state.focused => {
                terminal.write(content.clone().into_bytes());
                state.ime_preedit = None;
                shell.capture_event();
            }
            Event::InputMethod(input_method::Event::Closed) => state.ime_preedit = None,
            Event::Window(window::Event::Unfocused) => state.focused = false,
            _ => {}
        }

        if state.focused {
            let frame = terminal.frame.clone();
            let cursor = frame.cursor.unwrap_or(CursorPaint {
                row: 0,
                column: 0,
                shape: CursorShape::Block,
                color: TERMINAL_FOREGROUND,
            });
            shell
                .input_method_mut()
                .merge(&input_method::InputMethod::Enabled {
                    cursor: cell_rect(bounds, state.cell, cursor.row, cursor.column, 1),
                    purpose: input_method::Purpose::Terminal,
                    preedit: state.ime_preedit.clone(),
                });
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }
}

fn fill(renderer: &mut iced::Renderer, bounds: Rectangle, color: Color) {
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: true,
        },
        Background::Color(color),
    );
}

fn cell_rect(bounds: Rectangle, cell: Size, row: u16, column: u16, columns: u16) -> Rectangle {
    Rectangle {
        x: bounds.x + f32::from(column) * cell.width,
        y: bounds.y + f32::from(row) * cell.height,
        width: f32::from(columns) * cell.width,
        height: cell.height,
    }
}

fn mouse_cell(
    position: Point,
    bounds: Rectangle,
    cell: Size,
    terminal_size: TerminalSize,
) -> GridPoint<usize> {
    let column = ((position.x - bounds.x).max(0.0) / cell.width).floor() as usize;
    let line = ((position.y - bounds.y).max(0.0) / cell.height).floor() as usize;
    GridPoint::new(
        line.min(terminal_size.lines.saturating_sub(1) as usize),
        Column(column.min(terminal_size.columns.saturating_sub(1) as usize)),
    )
}

fn selection_side(position: Point, bounds: Rectangle, cell: Size) -> Side {
    let x = (position.x - bounds.x).max(0.0) % cell.width;
    if x > cell.width / 2.0 {
        Side::Right
    } else {
        Side::Left
    }
}

fn is_copy_shortcut(key: &Key, modifiers: Modifiers) -> bool {
    key.as_ref() == Key::Character("c")
        && if cfg!(target_os = "macos") {
            modifiers.command()
        } else {
            modifiers.control() && modifiers.shift()
        }
}

fn is_paste_shortcut(key: &Key, modifiers: Modifiers) -> bool {
    key.as_ref() == Key::Character("v")
        && if cfg!(target_os = "macos") {
            modifiers.command()
        } else {
            modifiers.control() && modifiers.shift()
        }
}

fn encode_key(
    key: &Key,
    text: Option<&str>,
    modifiers: Modifiers,
    mode: TermMode,
) -> Option<Vec<u8>> {
    if let Key::Character(character) = key
        && modifiers.control()
        && !modifiers.logo()
    {
        let character = character.as_bytes().first()?.to_ascii_lowercase();
        let control = match character {
            b'a'..=b'z' => character - b'a' + 1,
            b'@' | b'['..=b'_' => character & 0x1f,
            b'?' => 0x7f,
            _ => return None,
        };
        let mut bytes = Vec::with_capacity(2);
        if modifiers.alt() {
            bytes.push(0x1b);
        }
        bytes.push(control);
        return Some(bytes);
    }

    if let Key::Named(named) = key {
        return named_key(*named, modifiers, mode).map(|sequence| sequence.into_bytes());
    }

    text.map(|text| {
        let mut bytes = Vec::with_capacity(text.len() + usize::from(modifiers.alt()));
        if modifiers.alt() {
            bytes.push(0x1b);
        }
        bytes.extend_from_slice(text.as_bytes());
        bytes
    })
}

fn named_key(key: Named, modifiers: Modifiers, mode: TermMode) -> Option<String> {
    let modifier = 1
        + usize::from(modifiers.shift())
        + usize::from(modifiers.alt()) * 2
        + usize::from(modifiers.control()) * 4;
    let modified = modifier != 1;
    let cursor = |final_character: char| {
        if modified {
            format!("\x1b[1;{modifier}{final_character}")
        } else if mode.contains(TermMode::APP_CURSOR) {
            format!("\x1bO{final_character}")
        } else {
            format!("\x1b[{final_character}")
        }
    };
    let tilde = |number: u8| {
        if modified {
            format!("\x1b[{number};{modifier}~")
        } else {
            format!("\x1b[{number}~")
        }
    };

    Some(match key {
        Named::Enter => "\r".into(),
        Named::Tab if modifiers.shift() => "\x1b[Z".into(),
        Named::Tab => "\t".into(),
        Named::Backspace if modifiers.alt() => "\x1b\x7f".into(),
        Named::Backspace => "\x7f".into(),
        Named::Escape => "\x1b".into(),
        Named::ArrowUp => cursor('A'),
        Named::ArrowDown => cursor('B'),
        Named::ArrowRight => cursor('C'),
        Named::ArrowLeft => cursor('D'),
        Named::Home => cursor('H'),
        Named::End => cursor('F'),
        Named::Insert => tilde(2),
        Named::Delete => tilde(3),
        Named::PageUp => tilde(5),
        Named::PageDown => tilde(6),
        Named::F1 => function_key('P', modifier),
        Named::F2 => function_key('Q', modifier),
        Named::F3 => function_key('R', modifier),
        Named::F4 => function_key('S', modifier),
        Named::F5 => function_tilde(15, modifier),
        Named::F6 => function_tilde(17, modifier),
        Named::F7 => function_tilde(18, modifier),
        Named::F8 => function_tilde(19, modifier),
        Named::F9 => function_tilde(20, modifier),
        Named::F10 => function_tilde(21, modifier),
        Named::F11 => function_tilde(23, modifier),
        Named::F12 => function_tilde(24, modifier),
        _ => return None,
    })
}

fn function_key(final_character: char, modifier: usize) -> String {
    if modifier == 1 {
        format!("\x1bO{final_character}")
    } else {
        format!("\x1b[1;{modifier}{final_character}")
    }
}

fn function_tilde(number: u8, modifier: usize) -> String {
    if modifier == 1 {
        format!("\x1b[{number}~")
    } else {
        format!("\x1b[{number};{modifier}~")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EventProxy, Flags, Modifiers, Named, PaintCell, Rgb8, TerminalSize, TextStyle,
        build_text_runs, encode_key, launch, named_key, ssh_args,
    };
    use alacritty_terminal::event::{Event as AlacrittyEvent, EventListener};
    use alacritty_terminal::term::TermMode;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc;

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
    fn ctrl_u_uses_the_ascii_control_code() {
        assert_eq!(
            encode_key(
                &iced::keyboard::Key::Character("u".into()),
                None,
                Modifiers::CTRL,
                TermMode::default(),
            ),
            Some(vec![0x15])
        );
    }

    #[test]
    fn cursor_keys_honor_application_mode_and_modifiers() {
        assert_eq!(
            named_key(Named::ArrowUp, Modifiers::NONE, TermMode::APP_CURSOR),
            Some("\x1bOA".into())
        );
        assert_eq!(
            named_key(Named::ArrowUp, Modifiers::CTRL, TermMode::APP_CURSOR),
            Some("\x1b[1;5A".into())
        );
    }

    #[test]
    fn adjacent_cells_are_batched_into_one_text_run() {
        let style = TextStyle {
            color: Rgb8(1, 2, 3),
            bold: false,
            italic: false,
        };
        let cells = "claude code"
            .chars()
            .enumerate()
            .map(|(column, character)| PaintCell {
                row: 0,
                column: column as u16,
                character,
                zerowidth: Vec::new(),
                foreground: style.color,
                background: Rgb8(0, 0, 0),
                flags: Flags::empty(),
            })
            .collect::<Vec<_>>();

        let runs = build_text_runs(&cells);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].content, "claude code");
    }

    #[test]
    fn repeated_terminal_wakeups_occupy_one_queue_slot() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let proxy = EventProxy {
            sender,
            wakeup_pending: Arc::new(AtomicBool::new(false)),
        };

        for _ in 0..64 {
            proxy.send_event(AlacrittyEvent::Wakeup);
        }

        assert!(matches!(receiver.try_recv(), Ok(AlacrittyEvent::Wakeup)));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn subcell_layout_changes_keep_the_same_pty_size() {
        let cell = iced::Size::new(9.0, 20.0);
        let exact = TerminalSize::fit(iced::Size::new(720.0, 480.0), cell);
        let subcell = TerminalSize::fit(iced::Size::new(728.9, 499.9), cell);

        assert_eq!(exact, subcell);
        assert_eq!(exact.columns, 80);
        assert_eq!(exact.lines, 24);
    }

    #[cfg(unix)]
    #[test]
    fn pty_output_reaches_visible_text_runs() {
        let mut terminal = super::Terminal::new(
            u64::MAX,
            "/bin/sh".into(),
            vec!["-lc".into(), "printf ICE_RENDER_PIPELINE".into()],
            std::env::current_dir().unwrap(),
        )
        .unwrap();
        // Wedged-detector backstop, not a tuned budget: every event below is
        // waited on, never polled for, so this only fires when the PTY is
        // stuck outright. The old 2s wall-clock deadline lost the race to a
        // loaded CI runner's shell spawn.
        let backstop = Duration::from_secs(60);
        let deadline = Instant::now() + backstop;
        let waiter = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();

        loop {
            let visible = terminal
                .frame
                .text
                .iter()
                .map(|run| run.content.as_str())
                .collect::<String>();
            if visible.contains("ICE_RENDER_PIPELINE") {
                return;
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            let batch = waiter.block_on(async {
                let mut receiver = terminal.events.lock().await;
                match tokio::time::timeout(remaining, receiver.recv()).await {
                    Ok(Some(first)) => {
                        let mut batch = vec![first];
                        while let Ok(event) = receiver.try_recv() {
                            batch.push(event);
                        }
                        batch
                    }
                    Ok(None) => {
                        panic!("the PTY event channel closed before its output became visible")
                    }
                    Err(_) => panic!(
                        "PTY output never reached the visible terminal frame within the \
                         {backstop:?} wedged-detector backstop"
                    ),
                }
            });
            terminal.handle_events(batch);
        }
    }
}
