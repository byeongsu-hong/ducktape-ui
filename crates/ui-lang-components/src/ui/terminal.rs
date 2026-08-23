//! An alacritty-backed terminal surface: a PTY-hosting engine and the Iced
//! widget that draws its grid.
//!
//! The component owns everything that is true of ANY terminal — spawning a
//! program on a pty, pumping alacritty's grid, painting cells, routing keys,
//! mouse, selection, scrollback and clipboard. It deliberately owns nothing
//! about WHICH program to run: an application decides that and hands over a
//! resolved `(program, args, working_directory)` through [`spawn_session`].

use alacritty_terminal::event::{
    Event as AlacrittyEvent, EventListener, Notify, OnResize, WindowSize,
};
use alacritty_terminal::event_loop::{EventLoop, Msg, Notifier};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Point as GridPoint, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{
    self, ClipboardType, Term, TermMode, cell::Flags, viewport_to_point,
};
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
use iced::keyboard::{self, Key, Location, Modifiers, key::Named};
use iced::mouse::{self, ScrollDelta};
use iced::time::Instant;
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
const MIN_FONT_SIZE: f32 = 8.0;
const MAX_FONT_SIZE: f32 = 32.0;
const LINE_HEIGHT: f32 = 1.4;
const SCROLL_MULTIPLIER: f32 = 3.0;
const TERMINAL_FONT: Font = Font::with_name("JetBrains Mono");
const TERMINAL_WIDE_FONT: Font = Font::with_name("Monoplex KR");
const FRAME_INTERVAL: Duration = Duration::from_millis(8);
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(750);
const CURSOR_BLINK_TIMEOUT: Duration = Duration::from_secs(5);
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

#[derive(Debug, Clone)]
pub struct Notice {
    pub running: bool,
    pub title: String,
    pub attention: bool,
}

enum ClipboardRequest {
    Store(ClipboardType, String),
    Load(
        ClipboardType,
        Arc<dyn Fn(&str) -> String + Sync + Send + 'static>,
    ),
}

#[derive(Debug, Clone)]
pub struct TerminalError {
    pub message: String,
}
/// Start a program on a fresh pty and hand back the session that owns it.
///
/// `program` must already be resolved to a path (the caller owns PATH lookup,
/// because "which binary" is an application question — a desktop app ships its
/// own, a shell demo searches PATH). `title` is the session's initial and
/// default title; the pty may rename it later through OSC.
pub fn spawn_session(
    program: PathBuf,
    args: Vec<String>,
    working_directory: PathBuf,
    title: String,
) -> Result<Session, TerminalError> {
    let id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    let mut terminal =
        Terminal::new(id, program, args, working_directory).map_err(|error| TerminalError {
            message: format!("Could not start {title}: {error}"),
        })?;
    terminal.title = title.clone();
    terminal.default_title = title;

    Ok(Session {
        id,
        terminal: Some(Arc::new(Mutex::new(terminal))),
    })
}

pub fn idle_session() -> Session {
    Session {
        id: 0,
        terminal: None,
    }
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

pub fn terminal_attention(requested: bool) -> iced::Task<()> {
    if !requested {
        return iced::Task::none();
    }

    window::latest().and_then(|window| {
        window::request_user_attention(window, Some(window::UserAttention::Informational))
    })
}

fn handle_event_batch((session, events): (Session, Vec<AlacrittyEvent>)) -> Notice {
    let Some(terminal) = &session.terminal else {
        return Notice {
            running: false,
            title: String::new(),
            attention: false,
        };
    };

    lock(terminal).handle_events(events)
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
            columns: (bounds.width / cell.width.max(1.0)).floor().max(1.0) as u16,
            lines: (bounds.height / cell.height.max(1.0)).floor().max(1.0) as u16,
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
    clipboard_requests: Vec<ClipboardRequest>,
    wakeup_pending: Arc<AtomicBool>,
    size: TerminalSize,
    frame: Arc<TerminalFrame>,
    title: String,
    default_title: String,
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
            clipboard_requests: Vec::new(),
            wakeup_pending,
            size,
            frame: Arc::new(TerminalFrame::empty()),
            title: "Terminal".into(),
            default_title: "Terminal".into(),
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
        let mut attention = false;
        let mut needs_snapshot = false;

        for event in events {
            match event {
                AlacrittyEvent::Wakeup => {
                    self.wakeup_pending.store(false, Ordering::Release);
                    needs_snapshot = true;
                }
                AlacrittyEvent::Title(next) => self.title = next,
                AlacrittyEvent::ResetTitle => self.title.clone_from(&self.default_title),
                AlacrittyEvent::PtyWrite(text) => self.notifier.notify(text.into_bytes()),
                AlacrittyEvent::TextAreaSizeRequest(formatter) => {
                    self.notifier
                        .notify(formatter(self.size.into()).into_bytes());
                }
                AlacrittyEvent::ColorRequest(index, formatter) => {
                    let term = self.term.lock();
                    if let Some(color) = queried_color(index, term.colors()) {
                        self.notifier.notify(formatter(color).into_bytes());
                    }
                }
                AlacrittyEvent::Exit | AlacrittyEvent::ChildExit(_) => running = false,
                AlacrittyEvent::ClipboardStore(kind, text) if self.term.lock().is_focused => {
                    self.clipboard_requests
                        .push(ClipboardRequest::Store(kind, text));
                }
                AlacrittyEvent::ClipboardLoad(kind, formatter) if self.term.lock().is_focused => {
                    self.clipboard_requests
                        .push(ClipboardRequest::Load(kind, formatter));
                }
                AlacrittyEvent::ClipboardStore(..) | AlacrittyEvent::ClipboardLoad(..) => {}
                AlacrittyEvent::CursorBlinkingChange => needs_snapshot = true,
                AlacrittyEvent::Bell => {
                    let term = self.term.lock();
                    attention |= bell_requests_attention(term.is_focused, *term.mode());
                }
                AlacrittyEvent::MouseCursorDirty => {}
            }
        }

        if needs_snapshot {
            self.snapshot();
        }

        Notice {
            running,
            title: self.title.clone(),
            attention,
        }
    }

    fn set_focused(&mut self, focused: bool) {
        let mut term = self.term.lock();
        if term.is_focused == focused {
            return;
        }
        term.is_focused = focused;
        drop(term);
        self.snapshot();
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

    fn write_input(&mut self, bytes: impl Into<std::borrow::Cow<'static, [u8]>>) {
        {
            let mut term = self.term.lock();
            term.scroll_display(Scroll::Bottom);
            term.selection = None;
        }
        self.snapshot();
        self.write(bytes);
    }

    fn service_clipboard(&mut self, clipboard: &mut dyn Clipboard) {
        for reply in service_clipboard_requests(&mut self.clipboard_requests, clipboard) {
            self.write(reply);
        }
    }

    fn mode(&self) -> TermMode {
        *self.term.lock().mode()
    }

    fn paste(&mut self, text: String) {
        self.write_input(paste_bytes(&text, self.mode()));
    }

    fn selected_text(&self) -> Option<String> {
        self.term.lock().selection_to_string()
    }

    fn scroll(&mut self, columns: i32, lines: i32, allow_alternate: bool) {
        if columns == 0 && lines == 0 {
            return;
        }
        let mut term = self.term.lock();
        let mode = *term.mode();
        if allow_alternate && mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL) {
            let mut bytes =
                Vec::with_capacity((lines.unsigned_abs() + columns.unsigned_abs()) as usize * 3);
            let suffix = if lines > 0 { b'A' } else { b'B' };
            for _ in 0..lines.unsigned_abs() {
                bytes.extend_from_slice(&[0x1b, b'O', suffix]);
            }
            let suffix = if columns > 0 { b'D' } else { b'C' };
            for _ in 0..columns.unsigned_abs() {
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

    fn expand_selection(&mut self, cell: GridPoint<usize>, ty: SelectionType, side: Side) {
        let mut term = self.term.lock();
        let point = viewport_to_point(term.grid().display_offset(), cell);
        if let Some(selection) = term.selection.as_mut() {
            selection.ty = ty;
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
        let term = self.term.lock();
        let display_offset = term.grid().display_offset();
        let mode = *term.mode();
        drop(term);
        if cell.line < display_offset {
            return;
        }
        let cell = GridPoint::new(cell.line - display_offset, cell.column);
        if let Some(bytes) = mouse_report_bytes(mode, button, modifiers, cell, pressed) {
            self.write(bytes);
        }
    }

    fn snapshot(&mut self) {
        let mut term = self.term.lock();
        let frame = TerminalFrame::from_term(&term, self.size);
        term.reset_damage();
        self.frame = Arc::new(frame);
    }
}

fn bell_requests_attention(focused: bool, mode: TermMode) -> bool {
    !focused && mode.contains(TermMode::URGENCY_HINTS)
}

fn terminal_mouse_interaction(mode: TermMode, modifiers: Modifiers) -> mouse::Interaction {
    if mode.intersects(TermMode::MOUSE_MODE) && !modifiers.shift() {
        mouse::Interaction::Idle
    } else {
        mouse::Interaction::Text
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

fn queried_color(index: usize, colors: &term::color::Colors) -> Option<Rgb> {
    let color = colors[index].map_or_else(
        || {
            (index != NamedColor::Cursor as usize).then(|| match index {
                index if index == NamedColor::Foreground as usize => {
                    named_color(NamedColor::Foreground)
                }
                index if index == NamedColor::Background as usize => {
                    named_color(NamedColor::Background)
                }
                index => indexed_color(index),
            })
        },
        |rgb| Some(Rgb8(rgb.r, rgb.g, rgb.b)),
    )?;
    let Rgb8(r, g, b) = color;
    Some(Rgb { r, g, b })
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
enum TextFont {
    Terminal,
    Wide,
    PauseSymbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TextStyle {
    color: Rgb8,
    bold: bool,
    italic: bool,
    font: TextFont,
}

#[derive(Debug)]
struct PaintCell {
    row: u16,
    column: u16,
    character: char,
    zerowidth: Vec<char>,
    foreground: Rgb8,
    background: Rgb8,
    underline: Rgb8,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Underline,
    DoubleUnderline,
    Undercurl,
    DottedUnderline,
    DashedUnderline,
    Strikeout,
}

#[derive(Debug)]
struct LineRun {
    row: u16,
    column: u16,
    columns: u16,
    color: Rgb8,
    kind: LineKind,
}

#[derive(Debug, Clone, Copy)]
struct CursorPaint {
    row: u16,
    column: u16,
    columns: u16,
    shape: CursorShape,
    color: Rgb8,
    text_color: Rgb8,
    blinking: bool,
}

#[derive(Debug)]
struct TerminalFrame {
    background: Rgb8,
    text: Vec<TextRun>,
    backgrounds: Vec<ColorRun>,
    lines: Vec<LineRun>,
    cursor: Option<CursorPaint>,
}

impl TerminalFrame {
    fn empty() -> Self {
        Self {
            background: TERMINAL_BACKGROUND,
            text: Vec::new(),
            backgrounds: Vec::new(),
            lines: Vec::new(),
            cursor: None,
        }
    }

    fn from_term(term: &Term<EventProxy>, size: TerminalSize) -> Self {
        let focused = term.is_focused;
        let blinking = term.cursor_style().blinking;
        let content = term.renderable_content();
        let terminal_foreground =
            resolve_color(AnsiColor::Named(NamedColor::Foreground), content.colors);
        let terminal_background =
            resolve_color(AnsiColor::Named(NamedColor::Background), content.colors);
        let display_offset = content.display_offset as i32;
        let cursor_row = content.cursor.point.line.0 + display_offset;
        let cursor_column = content.cursor.point.column.0;
        let cursor_shape = if !focused && content.cursor.shape != CursorShape::Hidden {
            CursorShape::HollowBlock
        } else {
            content.cursor.shape
        };
        let mut cursor = (cursor_shape != CursorShape::Hidden
            && cursor_row >= 0
            && cursor_row < i32::from(size.lines)
            && cursor_column < size.columns as usize)
            .then_some(CursorPaint {
                row: cursor_row as u16,
                column: cursor_column as u16,
                columns: 1,
                shape: cursor_shape,
                color: terminal_foreground,
                text_color: terminal_background,
                blinking,
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
            let selected = content
                .selection
                .is_some_and(|selection| selection.contains(indexed.point));
            if indexed.flags.contains(Flags::INVERSE) || selected {
                std::mem::swap(&mut foreground, &mut background);
            }
            if selected && foreground == background && !indexed.flags.contains(Flags::HIDDEN) {
                foreground = terminal_background;
                background = terminal_foreground;
            }

            let mut underline = indexed
                .underline_color()
                .map_or(foreground, |color| resolve_color(color, content.colors));
            if indexed.flags.intersects(Flags::DIM | Flags::DIM_BOLD) {
                underline = underline.dimmed();
            }

            if let Some(cursor) = cursor.as_mut()
                && cursor.row == row as u16
                && cursor.column == indexed.point.column.0 as u16
            {
                cursor.columns = if indexed.flags.contains(Flags::WIDE_CHAR) {
                    2
                } else {
                    1
                };
                cursor.color = content.colors[NamedColor::Cursor]
                    .map(|rgb| Rgb8(rgb.r, rgb.g, rgb.b))
                    .unwrap_or(foreground);
                cursor.text_color = background;
                if foreground == background {
                    cursor.color = terminal_foreground;
                    cursor.text_color = terminal_background;
                }
                if cursor.shape == CursorShape::Block {
                    foreground = cursor.text_color;
                }
            }

            cells.push(PaintCell {
                row: row as u16,
                column: indexed.point.column.0 as u16,
                character: indexed.c,
                zerowidth: indexed.zerowidth().unwrap_or_default().to_vec(),
                foreground,
                background,
                underline,
                flags: indexed.flags,
            });
        }

        Self {
            background: terminal_background,
            text: build_text_runs(&cells),
            backgrounds: build_color_runs(&cells, terminal_background),
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
            font: text_font(cell.character, cell.flags),
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
                font: text_font(cell.character, cell.flags),
            };
            if cell_style != style || cell.column < next_column {
                break;
            }
            let isolated = cell.flags.contains(Flags::WIDE_CHAR) || !cell.zerowidth.is_empty();
            if !content.is_empty() && isolated {
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
            if isolated {
                break;
            }
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

fn text_font(character: char, flags: Flags) -> TextFont {
    if character == '\u{23f8}' {
        TextFont::PauseSymbol
    } else if flags.contains(Flags::WIDE_CHAR) {
        TextFont::Wide
    } else {
        TextFont::Terminal
    }
}

fn build_color_runs(cells: &[PaintCell], terminal_background: Rgb8) -> Vec<ColorRun> {
    let mut runs: Vec<ColorRun> = Vec::new();
    for cell in cells {
        if cell.background == terminal_background {
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
    for kind in [
        LineKind::Underline,
        LineKind::DoubleUnderline,
        LineKind::Undercurl,
        LineKind::DottedUnderline,
        LineKind::DashedUnderline,
        LineKind::Strikeout,
    ] {
        for cell in cells {
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) || !kind.matches(cell.flags) {
                continue;
            }
            let color = if kind == LineKind::Strikeout {
                cell.foreground
            } else {
                cell.underline
            };
            let columns = if cell.flags.contains(Flags::WIDE_CHAR) {
                2
            } else {
                1
            };
            if let Some(last) = runs.last_mut()
                && last.row == cell.row
                && last.color == color
                && last.kind == kind
                && last.column + last.columns == cell.column
            {
                last.columns += columns;
            } else {
                runs.push(LineRun {
                    row: cell.row,
                    column: cell.column,
                    columns,
                    color,
                    kind,
                });
            }
        }
    }
    runs
}

impl LineKind {
    fn matches(self, flags: Flags) -> bool {
        match self {
            Self::Underline => flags.contains(Flags::UNDERLINE),
            Self::DoubleUnderline => flags.contains(Flags::DOUBLE_UNDERLINE),
            Self::Undercurl => flags.contains(Flags::UNDERCURL),
            Self::DottedUnderline => flags.contains(Flags::DOTTED_UNDERLINE),
            Self::DashedUnderline => flags.contains(Flags::DASHED_UNDERLINE),
            Self::Strikeout => flags.contains(Flags::STRIKEOUT),
        }
    }
}

#[derive(Debug, Default)]
struct SurfaceState {
    session_id: u64,
    focused: bool,
    reported_focus: bool,
    modifiers: Modifiers,
    font_size: f32,
    wide_font_size: f32,
    cell: Size,
    layout: Size,
    mouse_cell: GridPoint<usize>,
    dragging: bool,
    pressed_button: Option<mouse::Button>,
    last_click: Option<advanced_mouse::Click>,
    scroll_pixels: f32,
    scroll_pixels_x: f32,
    ime_preedit: Option<input_method::Preedit>,
    cursor_visible: bool,
    cursor_blink_at: Option<Instant>,
    cursor_blink_timeout: Option<Instant>,
    cursor_blink_timed_out: bool,
    cursor_blinking: bool,
}

/// whether an IME composition is actually in progress.
///
/// A TERMINAL THAT IS FOCUSED ENABLES ITS INPUT METHOD, so `InputMethod::Opened`
/// arrives on every focus and says only that one is AVAILABLE — on X11 it then
/// stays open, composing nothing, until focus leaves. The preedit it opens with
/// is empty, so `ime_preedit.is_some()` is true for an ordinary typing session
/// and gating input on it swallowed every keystroke: the pty saw the mouse and
/// never a key.
///
/// The composition is the preedit TEXT. Only that may claim the keyboard.
fn composing(preedit: Option<&input_method::Preedit>) -> bool {
    preedit.is_some_and(|preedit| !preedit.content.is_empty())
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

fn reset_cursor_blink(state: &mut SurfaceState, now: Instant) {
    state.cursor_visible = true;
    state.cursor_blink_at = Some(now + CURSOR_BLINK_INTERVAL);
    state.cursor_blink_timeout = Some(now + CURSOR_BLINK_TIMEOUT);
    state.cursor_blink_timed_out = false;
}

fn sync_cursor_blinking(state: &mut SurfaceState, enabled: bool, now: Instant) -> bool {
    if state.cursor_blinking == enabled {
        return false;
    }
    state.cursor_blinking = enabled;
    if enabled {
        reset_cursor_blink(state, now);
    } else {
        state.cursor_visible = true;
        state.cursor_blink_at = None;
        state.cursor_blink_timeout = None;
        state.cursor_blink_timed_out = false;
    }
    true
}

fn advance_cursor_blink(state: &mut SurfaceState, now: Instant, enabled: bool) -> Option<Instant> {
    if !enabled {
        state.cursor_visible = true;
        state.cursor_blink_at = None;
        state.cursor_blink_timeout = None;
        state.cursor_blink_timed_out = false;
        return None;
    }
    if state.cursor_blink_timed_out {
        return None;
    }
    if state.cursor_blink_at.is_none() {
        reset_cursor_blink(state, now);
    }

    if state
        .cursor_blink_timeout
        .is_some_and(|timeout| now >= timeout)
    {
        state.cursor_visible = true;
        state.cursor_blink_at = None;
        state.cursor_blink_timed_out = true;
        return None;
    }

    if let Some(mut blink_at) = state.cursor_blink_at
        && now >= blink_at
    {
        while now >= blink_at {
            state.cursor_visible = !state.cursor_visible;
            blink_at += CURSOR_BLINK_INTERVAL;
        }
        state.cursor_blink_at = Some(blink_at);
    }

    match (state.cursor_blink_at, state.cursor_blink_timeout) {
        (Some(blink), Some(timeout)) => Some(blink.min(timeout)),
        (Some(blink), None) => Some(blink),
        _ => None,
    }
}

fn sync_terminal_focus(terminal: &mut Terminal, state: &mut SurfaceState, now: Instant) -> bool {
    if state.focused == state.reported_focus {
        return false;
    }

    terminal.set_focused(state.focused);
    if terminal.mode().contains(TermMode::FOCUS_IN_OUT) {
        terminal.write(if state.focused {
            b"\x1b[I".to_vec()
        } else {
            b"\x1b[O".to_vec()
        });
    }
    state.reported_focus = state.focused;
    reset_cursor_blink(state, now);
    true
}

fn terminal_cell_size(font_size: f32) -> Size {
    let paragraph = <iced::Renderer as text::Renderer>::Paragraph::with_text(text::Text {
        content: "M",
        bounds: Size::INFINITE,
        size: Pixels(font_size),
        line_height: text::LineHeight::Relative(LINE_HEIGHT),
        font: TERMINAL_FONT,
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::None,
    });
    let measured = paragraph.min_bounds();

    Size::new(measured.width.max(1.0), measured.height.max(1.0))
}

fn terminal_wide_font_size(font_size: f32) -> f32 {
    let paragraph = <iced::Renderer as text::Renderer>::Paragraph::with_text(text::Text {
        content: "M",
        bounds: Size::INFINITE,
        size: Pixels(font_size),
        line_height: text::LineHeight::Relative(LINE_HEIGHT),
        font: TERMINAL_WIDE_FONT,
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::None,
    });
    let wide_advance = paragraph.min_bounds().width.max(1.0);

    font_size * terminal_cell_size(font_size).width / wide_advance
}

impl Widget<(), Theme, iced::Renderer> for TerminalSurface {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SurfaceState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(SurfaceState {
            session_id: self.session_id,
            font_size: FONT_SIZE,
            wide_font_size: FONT_SIZE,
            cell: Size::new(
                f32::from(DEFAULT_CELL_WIDTH),
                f32::from(DEFAULT_CELL_HEIGHT),
            ),
            cursor_visible: true,
            ..SurfaceState::default()
        })
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<SurfaceState>();
        if state.session_id != self.session_id {
            *state = SurfaceState {
                session_id: self.session_id,
                font_size: state.font_size,
                wide_font_size: state.wide_font_size,
                cell: state.cell,
                cursor_visible: true,
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
        let state = tree.state.downcast_mut::<SurfaceState>();
        state.cell = terminal_cell_size(state.font_size);
        state.wide_font_size = terminal_wide_font_size(state.font_size);
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
            fill(renderer, bounds, frame.background.iced());

            for run in &frame.backgrounds {
                fill(
                    renderer,
                    cell_rect(bounds, state.cell, run.row, run.column, run.columns),
                    run.color.iced(),
                );
            }

            if let Some(cursor) = frame.cursor
                && state.cursor_visible
                && !composing(state.ime_preedit.as_ref())
            {
                let mut cursor_bounds = cell_rect(
                    bounds,
                    state.cell,
                    cursor.row,
                    cursor.column,
                    cursor.columns,
                );
                let thickness = (state.cell.width * 0.15).round().max(1.0);
                match cursor.shape {
                    CursorShape::Block => fill(renderer, cursor_bounds, cursor.color.iced()),
                    CursorShape::HollowBlock => renderer.fill_quad(
                        renderer::Quad {
                            bounds: cursor_bounds,
                            border: Border {
                                color: cursor.color.iced(),
                                width: thickness,
                                radius: 0.0.into(),
                            },
                            shadow: Shadow::default(),
                            snap: true,
                        },
                        Background::Color(Color::TRANSPARENT),
                    ),
                    CursorShape::Beam => {
                        cursor_bounds.width = thickness;
                        fill(renderer, cursor_bounds, cursor.color.iced());
                    }
                    CursorShape::Underline => {
                        cursor_bounds.y += cursor_bounds.height - thickness;
                        cursor_bounds.height = thickness;
                        fill(renderer, cursor_bounds, cursor.color.iced());
                    }
                    CursorShape::Hidden => {}
                }
            }

            for run in &frame.text {
                if run.style.font == TextFont::PauseSymbol {
                    for bar in pause_symbol_rects(cell_rect(
                        bounds,
                        state.cell,
                        run.row,
                        run.column,
                        run.columns,
                    )) {
                        fill(renderer, bar, run.style.color.iced());
                    }
                    continue;
                }

                let mut font = match run.style.font {
                    TextFont::Terminal => TERMINAL_FONT,
                    TextFont::Wide => TERMINAL_WIDE_FONT,
                    TextFont::PauseSymbol => unreachable!(),
                };
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
                        size: Pixels(if run.style.font == TextFont::Wide {
                            state.wide_font_size
                        } else {
                            state.font_size
                        }),
                        line_height: text::LineHeight::Relative(LINE_HEIGHT),
                        font,
                        align_x: text::Alignment::Left,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::Auto,
                        wrapping: text::Wrapping::None,
                    },
                    terminal_text_origin(bounds, state.cell, run.row, run.column),
                    run.style.color.iced(),
                    clip,
                );
            }

            for line in &frame.lines {
                for line_bounds in line_rects(bounds, state.cell, line) {
                    fill(renderer, line_bounds, line.color.iced());
                }
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
        terminal.service_clipboard(clipboard);
        if terminal.resize(bounds.size(), state.cell) {
            shell.request_redraw();
        }

        if sync_terminal_focus(&mut terminal, state, Instant::now()) {
            shell.request_redraw();
        }
        let blinking = terminal.frame.cursor.is_some_and(|cursor| cursor.blinking)
            && state.focused
            && !composing(state.ime_preedit.as_ref());
        if sync_cursor_blinking(state, blinking, Instant::now()) {
            shell.request_redraw();
        }

        let hovered = cursor.is_over(bounds);
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if hovered => {
                state.focused = true;
                state.pressed_button = Some(mouse::Button::Left);
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
                    let ty = if state.modifiers.control() {
                        SelectionType::Block
                    } else {
                        match click.kind() {
                            advanced_mouse::click::Kind::Single => SelectionType::Simple,
                            advanced_mouse::click::Kind::Double => SelectionType::Semantic,
                            advanced_mouse::click::Kind::Triple => SelectionType::Lines,
                        }
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
            Event::Mouse(mouse::Event::ButtonPressed(
                button @ (mouse::Button::Middle | mouse::Button::Right),
            )) if hovered => {
                state.focused = true;
                state.pressed_button = Some(*button);
                let Some(position) = cursor.position() else {
                    return;
                };
                state.mouse_cell = mouse_cell(position, bounds, state.cell, terminal.size);
                let mode = terminal.mode();
                if mode.intersects(TermMode::MOUSE_MODE) && !state.modifiers.shift() {
                    terminal.mouse_report(
                        mouse_button_code(*button).expect("matched reportable mouse button"),
                        state.modifiers,
                        state.mouse_cell,
                        true,
                    );
                    state.dragging = true;
                } else if *button == mouse::Button::Middle {
                    if let Some(text) = clipboard.read(clipboard::Kind::Primary) {
                        terminal.paste(text);
                        reset_cursor_blink(state, Instant::now());
                        shell.request_redraw();
                    }
                } else {
                    let click = advanced_mouse::Click::new(position, *button, state.last_click);
                    let ty = if state.modifiers.control() {
                        SelectionType::Block
                    } else {
                        match click.kind() {
                            advanced_mouse::click::Kind::Single => SelectionType::Simple,
                            advanced_mouse::click::Kind::Double => SelectionType::Semantic,
                            advanced_mouse::click::Kind::Triple => SelectionType::Lines,
                        }
                    };
                    state.last_click = Some(click);
                    terminal.expand_selection(
                        state.mouse_cell,
                        ty,
                        selection_side(position, bounds, state.cell),
                    );
                    state.dragging = true;
                    shell.request_redraw();
                }
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if hovered || state.dragging => {
                let next_mouse_cell = mouse_cell(*position, bounds, state.cell, terminal.size);
                let cell_changed = next_mouse_cell != state.mouse_cell;
                state.mouse_cell = next_mouse_cell;
                let mode = terminal.mode();
                if !state.modifiers.shift()
                    && let Some(button) = motion_mouse_code(mode, state.pressed_button)
                    && cell_changed
                {
                    terminal.mouse_report(button, state.modifiers, state.mouse_cell, true);
                    shell.capture_event();
                } else if state.dragging
                    && matches!(
                        state.pressed_button,
                        Some(mouse::Button::Left | mouse::Button::Right)
                    )
                {
                    terminal.update_selection(
                        state.mouse_cell,
                        selection_side(*position, bounds, state.cell),
                    );
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(button))
                if state.pressed_button == Some(*button) =>
            {
                if terminal.mode().intersects(TermMode::MOUSE_MODE)
                    && !state.modifiers.shift()
                    && let Some(code) = mouse_button_code(*button)
                {
                    terminal.mouse_report(code, state.modifiers, state.mouse_cell, false);
                } else if matches!(button, mouse::Button::Left | mouse::Button::Right)
                    && let Some(selection) = terminal.selected_text()
                {
                    clipboard.write(clipboard::Kind::Primary, selection);
                }
                state.dragging = false;
                state.pressed_button = None;
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if hovered => {
                let mouse_reporting =
                    terminal.mode().intersects(TermMode::MOUSE_MODE) && !state.modifiers.shift();
                let multiplier = scroll_multiplier(mouse_reporting);
                let (columns, lines) = match delta {
                    ScrollDelta::Lines { x, y } => (
                        (x * multiplier).round() as i32,
                        (y * multiplier).round() as i32,
                    ),
                    ScrollDelta::Pixels { x, y } => {
                        state.scroll_pixels_x += *x * multiplier;
                        state.scroll_pixels += *y * multiplier;
                        let columns = (state.scroll_pixels_x / state.cell.width).trunc() as i32;
                        let lines = (state.scroll_pixels / state.cell.height).trunc() as i32;
                        state.scroll_pixels_x -= columns as f32 * state.cell.width;
                        state.scroll_pixels -= lines as f32 * state.cell.height;
                        (columns, lines)
                    }
                };
                if mouse_reporting {
                    let button = if lines >= 0 { 64 } else { 65 };
                    for _ in 0..lines.unsigned_abs() {
                        terminal.mouse_report(button, state.modifiers, state.mouse_cell, true);
                    }
                    let button = if columns >= 0 { 66 } else { 67 };
                    for _ in 0..columns.unsigned_abs() {
                        terminal.mouse_report(button, state.modifiers, state.mouse_cell, true);
                    }
                } else {
                    terminal.scroll(columns, lines, !state.modifiers.shift());
                    shell.request_redraw();
                }
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modified_key,
                modifiers,
                location,
                text,
                repeat,
                ..
            }) if state.focused => {
                state.modifiers = *modifiers;
                if composing(state.ime_preedit.as_ref()) {
                    // IME commits are delivered separately and must be the only PTY input.
                } else if let Some(font_size) = zoomed_font_size(key, *modifiers, state.font_size) {
                    if font_size != state.font_size {
                        state.font_size = font_size;
                        state.cell = terminal_cell_size(font_size);
                        state.wide_font_size = terminal_wide_font_size(font_size);
                        terminal.resize(bounds.size(), state.cell);
                        if let Some(preedit) = &mut state.ime_preedit {
                            preedit.text_size = Some(Pixels(font_size));
                        }
                        shell.invalidate_layout();
                        shell.request_redraw();
                    }
                } else if is_copy_shortcut(key, *modifiers) {
                    if let Some(selection) = terminal.selected_text() {
                        clipboard.write(clipboard::Kind::Standard, selection);
                    }
                } else if is_paste_shortcut(key, *modifiers) {
                    if let Some(text) = clipboard.read(clipboard::Kind::Standard) {
                        terminal.paste(text);
                        reset_cursor_blink(state, Instant::now());
                        shell.request_redraw();
                    }
                } else if let Some(bytes) = encode_key_event(
                    key,
                    modified_key,
                    text.as_deref(),
                    *modifiers,
                    *location,
                    if *repeat {
                        KeyEventKind::Repeat
                    } else {
                        KeyEventKind::Press
                    },
                    terminal.mode(),
                ) {
                    if is_modifier_key(key) {
                        terminal.write(bytes);
                    } else {
                        terminal.write_input(bytes);
                        reset_cursor_blink(state, Instant::now());
                        shell.request_redraw();
                    }
                }
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::KeyReleased {
                key,
                modified_key,
                modifiers,
                location,
                ..
            }) if state.focused => {
                state.modifiers = *modifiers;
                if !composing(state.ime_preedit.as_ref())
                    && let Some(bytes) = encode_key_event(
                        key,
                        modified_key,
                        None,
                        *modifiers,
                        *location,
                        KeyEventKind::Release,
                        terminal.mode(),
                    )
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
                    text_size: Some(Pixels(state.font_size)),
                });
                shell.request_redraw();
            }
            Event::InputMethod(input_method::Event::Commit(content)) if state.focused => {
                terminal.write_input(content.clone().into_bytes());
                state.ime_preedit = None;
                reset_cursor_blink(state, Instant::now());
                shell.request_redraw();
                shell.capture_event();
            }
            Event::InputMethod(input_method::Event::Closed) => state.ime_preedit = None,
            Event::Window(window::Event::Unfocused) => state.focused = false,
            Event::Window(window::Event::RedrawRequested(now)) => {
                if let Some(at) = advance_cursor_blink(state, *now, state.cursor_blinking) {
                    shell.request_redraw_at(at);
                }
            }
            _ => {}
        }

        if sync_terminal_focus(&mut terminal, state, Instant::now()) {
            shell.request_redraw();
        }
        let blinking = terminal.frame.cursor.is_some_and(|cursor| cursor.blinking)
            && state.focused
            && !composing(state.ime_preedit.as_ref());
        if sync_cursor_blinking(state, blinking, Instant::now()) {
            shell.request_redraw();
        }

        if state.focused {
            let frame = terminal.frame.clone();
            let cursor = frame.cursor.unwrap_or(CursorPaint {
                row: 0,
                column: 0,
                columns: 1,
                shape: CursorShape::Block,
                color: TERMINAL_FOREGROUND,
                text_color: TERMINAL_BACKGROUND,
                blinking: false,
            });
            shell
                .input_method_mut()
                .merge(&input_method::InputMethod::Enabled {
                    cursor: cell_rect(
                        bounds,
                        state.cell,
                        cursor.row,
                        cursor.column,
                        cursor.columns,
                    ),
                    purpose: input_method::Purpose::Terminal,
                    preedit: state.ime_preedit.clone(),
                });
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            let state = tree.state.downcast_ref::<SurfaceState>();
            let mode = lock(&self.terminal).mode();
            terminal_mouse_interaction(mode, state.modifiers)
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

fn pause_symbol_rects(bounds: Rectangle) -> [Rectangle; 2] {
    let bar_width = (bounds.width * 0.16).max(1.0);
    let gap = bar_width;
    let height = bounds.height * 0.58;
    let x = bounds.x + (bounds.width - bar_width * 3.0) / 2.0;
    let y = bounds.y + (bounds.height - height) / 2.0;

    [
        Rectangle::new(Point::new(x, y), Size::new(bar_width, height)),
        Rectangle::new(
            Point::new(x + bar_width + gap, y),
            Size::new(bar_width, height),
        ),
    ]
}

fn terminal_text_origin(bounds: Rectangle, cell: Size, row: u16, column: u16) -> Point {
    Point::new(
        bounds.x + f32::from(column) * cell.width,
        bounds.y + (f32::from(row) + 0.5) * cell.height,
    )
}

fn line_rects(bounds: Rectangle, cell: Size, line: &LineRun) -> Vec<Rectangle> {
    let cell_bounds = cell_rect(bounds, cell, line.row, line.column, line.columns);
    let bottom = cell_bounds.y + cell_bounds.height;
    let solid = |y| {
        Rectangle::new(
            Point::new(cell_bounds.x, y),
            Size::new(cell_bounds.width, 1.0),
        )
    };

    match line.kind {
        LineKind::Underline => vec![solid(bottom - 2.0)],
        LineKind::DoubleUnderline => vec![solid(bottom - 4.0), solid(bottom - 1.0)],
        LineKind::Strikeout => vec![solid(cell_bounds.y + cell_bounds.height * 0.52)],
        LineKind::Undercurl => patterned_line(cell_bounds, 1.0, 1.0, true),
        LineKind::DottedUnderline => patterned_line(cell_bounds, 1.0, 2.0, false),
        LineKind::DashedUnderline => patterned_line(cell_bounds, 4.0, 2.0, false),
    }
}

fn patterned_line(bounds: Rectangle, width: f32, gap: f32, wave: bool) -> Vec<Rectangle> {
    let mut rects = Vec::new();
    let mut x = bounds.x;
    let end = bounds.x + bounds.width;
    let mut raised = false;
    while x < end {
        let segment_width = width.min(end - x);
        let y = bounds.y + bounds.height - if wave && raised { 3.0 } else { 1.0 };
        rects.push(Rectangle::new(
            Point::new(x, y),
            Size::new(segment_width, 1.0),
        ));
        x += width + gap;
        raised = !raised;
    }
    rects
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

fn clipboard_kind(kind: ClipboardType) -> clipboard::Kind {
    match kind {
        ClipboardType::Clipboard => clipboard::Kind::Standard,
        ClipboardType::Selection => clipboard::Kind::Primary,
    }
}

fn is_modifier_key(key: &Key) -> bool {
    matches!(
        key,
        Key::Named(Named::Shift | Named::Control | Named::Alt | Named::Super)
    )
}

fn service_clipboard_requests(
    requests: &mut Vec<ClipboardRequest>,
    clipboard: &mut dyn Clipboard,
) -> Vec<Vec<u8>> {
    let mut replies = Vec::new();
    for request in std::mem::take(requests) {
        match request {
            ClipboardRequest::Store(kind, text) => {
                clipboard.write(clipboard_kind(kind), text);
            }
            ClipboardRequest::Load(kind, formatter) => {
                let text = clipboard.read(clipboard_kind(kind)).unwrap_or_default();
                replies.push(formatter(&text).into_bytes());
            }
        }
    }
    replies
}

fn is_paste_shortcut(key: &Key, modifiers: Modifiers) -> bool {
    key.as_ref() == Key::Character("v")
        && if cfg!(target_os = "macos") {
            modifiers.command()
        } else {
            modifiers.control() && modifiers.shift()
        }
}

fn zoomed_font_size(key: &Key, modifiers: Modifiers, font_size: f32) -> Option<f32> {
    if !modifiers.logo() {
        return None;
    }

    let step = match key.as_ref() {
        Key::Character("+" | "=") => 1.0,
        Key::Character("-") => -1.0,
        _ => return None,
    };

    Some((font_size + step).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE))
}

#[cfg(test)]
fn encode_key(
    key: &Key,
    text: Option<&str>,
    modifiers: Modifiers,
    mode: TermMode,
) -> Option<Vec<u8>> {
    encode_key_event(
        key,
        key,
        text,
        modifiers,
        Location::Standard,
        KeyEventKind::Press,
        mode,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyEventKind {
    Press,
    Repeat,
    Release,
}

fn encode_key_event(
    key: &Key,
    modified_key: &Key,
    text: Option<&str>,
    modifiers: Modifiers,
    location: Location,
    kind: KeyEventKind,
    mode: TermMode,
) -> Option<Vec<u8>> {
    // `alacritty_terminal` owns protocol state but its host key encoder lives in the
    // Alacritty frontend. Keep the same binding-first ordering as the pinned 0.25.1 frontend.
    if kind != KeyEventKind::Release
        && let Some(bytes) = legacy_key_binding(key, modifiers, location, mode)
    {
        return Some(bytes);
    }
    let modifiers = terminal_input_modifiers(key, modifiers);

    let kitty = mode.intersects(
        TermMode::REPORT_ALL_KEYS_AS_ESC
            | TermMode::DISAMBIGUATE_ESC_CODES
            | TermMode::REPORT_EVENT_TYPES,
    );
    if kitty && should_encode_kitty(key, text, modifiers, location, kind, mode) {
        return kitty_key_sequence(key, modified_key, text, modifiers, location, kind, mode);
    }
    if kind == KeyEventKind::Release {
        return None;
    }

    encode_legacy_key(key, text, modifiers, mode)
}

fn terminal_input_modifiers(key: &Key, modifiers: Modifiers) -> Modifiers {
    if cfg!(target_os = "macos")
        && modifiers.alt()
        && matches!(
            key,
            Key::Character(_)
                | Key::Named(
                    Named::Enter | Named::Backspace | Named::Tab | Named::Space | Named::Escape
                )
        )
    {
        modifiers.difference(Modifiers::ALT)
    } else {
        modifiers
    }
}

fn legacy_key_binding(
    key: &Key,
    modifiers: Modifiers,
    location: Location,
    mode: TermMode,
) -> Option<Vec<u8>> {
    let named = match key {
        Key::Named(named) => *named,
        _ => return None,
    };
    let disambiguated = mode.contains(TermMode::DISAMBIGUATE_ESC_CODES);
    let encode_all = mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC);

    if modifiers.is_empty() && mode.contains(TermMode::APP_CURSOR) {
        let final_character = match named {
            Named::Home => 'H',
            Named::End => 'F',
            Named::ArrowUp => 'A',
            Named::ArrowDown => 'B',
            Named::ArrowRight => 'C',
            Named::ArrowLeft => 'D',
            _ => {
                return legacy_non_cursor_binding(
                    named,
                    modifiers,
                    location,
                    disambiguated,
                    encode_all,
                );
            }
        };
        return Some(format!("\x1bO{final_character}").into_bytes());
    }

    legacy_non_cursor_binding(named, modifiers, location, disambiguated, encode_all)
}

fn legacy_non_cursor_binding(
    key: Named,
    modifiers: Modifiers,
    location: Location,
    disambiguated: bool,
    encode_all: bool,
) -> Option<Vec<u8>> {
    if modifiers.is_empty() && !encode_all && !disambiguated {
        let sequence = match key {
            Named::F1 => "\x1bOP",
            Named::F2 => "\x1bOQ",
            Named::F3 => "\x1bOR",
            Named::F4 => "\x1bOS",
            Named::Enter if location == Location::Numpad => "\n",
            _ => "",
        };
        if !sequence.is_empty() {
            return Some(sequence.as_bytes().to_vec());
        }
    }
    if key == Named::Backspace && modifiers.is_empty() && !encode_all {
        return Some(vec![0x7f]);
    }
    if key == Named::Tab && modifiers == Modifiers::SHIFT && !encode_all && !disambiguated {
        return Some(b"\x1b[Z".to_vec());
    }
    if key == Named::Tab
        && modifiers == Modifiers::SHIFT | Modifiers::ALT
        && !encode_all
        && !disambiguated
    {
        return Some(b"\x1b\x1b[Z".to_vec());
    }
    if key == Named::Backspace && modifiers == Modifiers::ALT && !encode_all && !disambiguated {
        return Some(b"\x1b\x7f".to_vec());
    }
    if key == Named::Backspace && modifiers == Modifiers::SHIFT && !encode_all && !disambiguated {
        return Some(vec![0x7f]);
    }

    None
}

fn should_encode_kitty(
    key: &Key,
    text: Option<&str>,
    modifiers: Modifiers,
    location: Location,
    kind: KeyEventKind,
    mode: TermMode,
) -> bool {
    if mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC) {
        return true;
    }
    if kind == KeyEventKind::Release && mode.contains(TermMode::REPORT_EVENT_TYPES) {
        return !matches!(
            key,
            Key::Named(Named::Enter | Named::Tab | Named::Backspace)
        );
    }

    let disambiguated = mode.contains(TermMode::DISAMBIGUATE_ESC_CODES)
        && (matches!(key, Key::Named(Named::Escape))
            || location == Location::Numpad
            || (!modifiers.is_empty()
                && (modifiers != Modifiers::SHIFT
                    || matches!(
                        key,
                        Key::Named(Named::Tab | Named::Enter | Named::Backspace)
                    ))));

    disambiguated
        || match key {
            Key::Named(
                Named::Enter | Named::Backspace | Named::Tab | Named::Space | Named::Escape,
            ) => false,
            Key::Named(_) => true,
            Key::Character(_) => text.is_none_or(str::is_empty),
            Key::Unidentified => text.is_none_or(str::is_empty),
        }
}

fn encode_legacy_key(
    key: &Key,
    text: Option<&str>,
    modifiers: Modifiers,
    mode: TermMode,
) -> Option<Vec<u8>> {
    if matches!(key, Key::Character(_))
        && let Some(text) = text
        && !text.is_empty()
    {
        let mut bytes = Vec::with_capacity(text.len() + usize::from(modifiers.alt()));
        if modifiers.alt() {
            bytes.push(0x1b);
        }
        bytes.extend_from_slice(text.as_bytes());
        return Some(bytes);
    }

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

    if matches!(
        key,
        Key::Named(Named::Enter | Named::Tab | Named::Backspace | Named::Space | Named::Escape)
    ) && let Some(text) = text
        && !text.is_empty()
    {
        let mut bytes = Vec::with_capacity(text.len() + usize::from(modifiers.alt()));
        if modifiers.alt() {
            bytes.push(0x1b);
        }
        bytes.extend_from_slice(text.as_bytes());
        return Some(bytes);
    }

    if let Key::Named(named) = key {
        return named_key(*named, modifiers, mode).map(|sequence| sequence.into_bytes());
    }

    text.filter(|text| !text.is_empty()).map(|text| {
        let mut bytes = Vec::with_capacity(text.len() + usize::from(modifiers.alt()));
        if modifiers.alt() {
            bytes.push(0x1b);
        }
        bytes.extend_from_slice(text.as_bytes());
        bytes
    })
}

fn kitty_key_sequence(
    key: &Key,
    modified_key: &Key,
    text: Option<&str>,
    modifiers: Modifiers,
    location: Location,
    kind: KeyEventKind,
    mode: TermMode,
) -> Option<Vec<u8>> {
    let event_type = mode.contains(TermMode::REPORT_EVENT_TYPES)
        && matches!(kind, KeyEventKind::Repeat | KeyEventKind::Release);
    let associated_text = text.filter(|text| {
        mode.contains(TermMode::REPORT_ASSOCIATED_TEXT)
            && kind != KeyEventKind::Release
            && !text.is_empty()
            && !is_control_character(text)
    });
    let mut modifier_bits = kitty_modifier_bits(modifiers);

    let (mut payload, terminator) = numpad_key_base(modified_key, location)
        .or_else(|| kitty_named_base(modified_key))
        .or_else(|| {
            normal_named_base(
                modified_key,
                modifiers,
                event_type,
                associated_text.is_some(),
            )
        })
        .or_else(|| {
            kitty_control_base(
                modified_key,
                location,
                kind,
                mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC),
                &mut modifier_bits,
            )
        })
        .or_else(|| kitty_text_base(key, modified_key, associated_text, mode))?;

    if event_type || modifier_bits != 0 || associated_text.is_some() {
        payload.push_str(&format!(";{}", modifier_bits + 1));
    }
    if event_type {
        payload.push(':');
        payload.push(match kind {
            KeyEventKind::Press => '1',
            KeyEventKind::Repeat => '2',
            KeyEventKind::Release => '3',
        });
    }
    if let Some(text) = associated_text {
        let mut codepoints = text.chars().map(u32::from);
        if let Some(codepoint) = codepoints.next() {
            payload.push_str(&format!(";{codepoint}"));
        }
        for codepoint in codepoints {
            payload.push_str(&format!(":{codepoint}"));
        }
    }

    Some(format!("\x1b[{payload}{terminator}").into_bytes())
}

fn kitty_modifier_bits(modifiers: Modifiers) -> u8 {
    u8::from(modifiers.shift())
        | u8::from(modifiers.alt()) << 1
        | u8::from(modifiers.control()) << 2
        | u8::from(modifiers.logo()) << 3
}

fn numpad_key_base(key: &Key, location: Location) -> Option<(String, char)> {
    if location != Location::Numpad {
        return None;
    }
    let code = match key.as_ref() {
        Key::Character("0") => 57399,
        Key::Character("1") => 57400,
        Key::Character("2") => 57401,
        Key::Character("3") => 57402,
        Key::Character("4") => 57403,
        Key::Character("5") => 57404,
        Key::Character("6") => 57405,
        Key::Character("7") => 57406,
        Key::Character("8") => 57407,
        Key::Character("9") => 57408,
        Key::Character(".") => 57409,
        Key::Character("/") => 57410,
        Key::Character("*") => 57411,
        Key::Character("-") => 57412,
        Key::Character("+") => 57413,
        Key::Named(Named::Enter) => 57414,
        Key::Character("=") => 57415,
        Key::Named(Named::ArrowLeft) => 57417,
        Key::Named(Named::ArrowRight) => 57418,
        Key::Named(Named::ArrowUp) => 57419,
        Key::Named(Named::ArrowDown) => 57420,
        Key::Named(Named::PageUp) => 57421,
        Key::Named(Named::PageDown) => 57422,
        Key::Named(Named::Home) => 57423,
        Key::Named(Named::End) => 57424,
        Key::Named(Named::Insert) => 57425,
        Key::Named(Named::Delete) => 57426,
        _ => return None,
    };
    Some((code.to_string(), 'u'))
}

fn kitty_named_base(key: &Key) -> Option<(String, char)> {
    let named = match key {
        Key::Named(named) => *named,
        _ => return None,
    };
    let (code, terminator) = match named {
        Named::F3 => (13, '~'),
        Named::F13 => (57376, 'u'),
        Named::F14 => (57377, 'u'),
        Named::F15 => (57378, 'u'),
        Named::F16 => (57379, 'u'),
        Named::F17 => (57380, 'u'),
        Named::F18 => (57381, 'u'),
        Named::F19 => (57382, 'u'),
        Named::F20 => (57383, 'u'),
        Named::F21 => (57384, 'u'),
        Named::F22 => (57385, 'u'),
        Named::F23 => (57386, 'u'),
        Named::F24 => (57387, 'u'),
        Named::F25 => (57388, 'u'),
        Named::F26 => (57389, 'u'),
        Named::F27 => (57390, 'u'),
        Named::F28 => (57391, 'u'),
        Named::F29 => (57392, 'u'),
        Named::F30 => (57393, 'u'),
        Named::F31 => (57394, 'u'),
        Named::F32 => (57395, 'u'),
        Named::F33 => (57396, 'u'),
        Named::F34 => (57397, 'u'),
        Named::F35 => (57398, 'u'),
        Named::ScrollLock => (57359, 'u'),
        Named::PrintScreen => (57361, 'u'),
        Named::Pause => (57362, 'u'),
        Named::ContextMenu => (57363, 'u'),
        Named::MediaPlay => (57428, 'u'),
        Named::MediaPause => (57429, 'u'),
        Named::MediaPlayPause => (57430, 'u'),
        Named::MediaStop => (57432, 'u'),
        Named::MediaFastForward => (57433, 'u'),
        Named::MediaRewind => (57434, 'u'),
        Named::MediaTrackNext => (57435, 'u'),
        Named::MediaTrackPrevious => (57436, 'u'),
        Named::MediaRecord => (57437, 'u'),
        Named::AudioVolumeDown => (57438, 'u'),
        Named::AudioVolumeUp => (57439, 'u'),
        Named::AudioVolumeMute => (57440, 'u'),
        _ => return None,
    };
    Some((code.to_string(), terminator))
}

fn normal_named_base(
    key: &Key,
    modifiers: Modifiers,
    event_type: bool,
    associated_text: bool,
) -> Option<(String, char)> {
    let named = match key {
        Key::Named(named) => *named,
        _ => return None,
    };
    let one = if modifiers.is_empty() && !event_type && !associated_text {
        ""
    } else {
        "1"
    };
    let (payload, terminator) = match named {
        Named::PageUp => ("5", '~'),
        Named::PageDown => ("6", '~'),
        Named::Insert => ("2", '~'),
        Named::Delete => ("3", '~'),
        Named::Home => (one, 'H'),
        Named::End => (one, 'F'),
        Named::ArrowLeft => (one, 'D'),
        Named::ArrowRight => (one, 'C'),
        Named::ArrowUp => (one, 'A'),
        Named::ArrowDown => (one, 'B'),
        Named::F1 => (one, 'P'),
        Named::F2 => (one, 'Q'),
        Named::F3 => (one, 'R'),
        Named::F4 => (one, 'S'),
        Named::F5 => ("15", '~'),
        Named::F6 => ("17", '~'),
        Named::F7 => ("18", '~'),
        Named::F8 => ("19", '~'),
        Named::F9 => ("20", '~'),
        Named::F10 => ("21", '~'),
        Named::F11 => ("23", '~'),
        Named::F12 => ("24", '~'),
        Named::F13 => ("25", '~'),
        Named::F14 => ("26", '~'),
        Named::F15 => ("28", '~'),
        Named::F16 => ("29", '~'),
        Named::F17 => ("31", '~'),
        Named::F18 => ("32", '~'),
        Named::F19 => ("33", '~'),
        Named::F20 => ("34", '~'),
        _ => return None,
    };
    Some((payload.into(), terminator))
}

fn kitty_control_base(
    key: &Key,
    location: Location,
    kind: KeyEventKind,
    encode_all: bool,
    modifiers: &mut u8,
) -> Option<(String, char)> {
    let named = match key {
        Key::Named(named) => *named,
        _ => return None,
    };
    let mut code = match named {
        Named::Tab => 9,
        Named::Enter => 13,
        Named::Escape => 27,
        Named::Space => 32,
        Named::Backspace => 127,
        _ if !encode_all => return None,
        _ => 0,
    };
    code = match (named, location) {
        (Named::Shift, Location::Left) => 57441,
        (Named::Control, Location::Left) => 57442,
        (Named::Alt, Location::Left) => 57443,
        (Named::Super, Location::Left) => 57444,
        (Named::Hyper, Location::Left) => 57445,
        (Named::Meta, Location::Left) => 57446,
        (Named::Shift, _) => 57447,
        (Named::Control, _) => 57448,
        (Named::Alt, _) => 57449,
        (Named::Super, _) => 57450,
        (Named::Hyper, _) => 57451,
        (Named::Meta, _) => 57452,
        (Named::CapsLock, _) => 57358,
        (Named::NumLock, _) => 57360,
        _ => code,
    };
    let pressed = kind != KeyEventKind::Release;
    let flag = match named {
        Named::Shift => Some(1),
        Named::Alt => Some(2),
        Named::Control => Some(4),
        Named::Super => Some(8),
        _ => None,
    };
    if let Some(flag) = flag {
        if pressed {
            *modifiers |= flag;
        } else {
            *modifiers &= !flag;
        }
    }
    (code != 0).then(|| (code.to_string(), 'u'))
}

fn kitty_text_base(
    key: &Key,
    modified_key: &Key,
    associated_text: Option<&str>,
    mode: TermMode,
) -> Option<(String, char)> {
    let modified = match modified_key {
        Key::Character(character) if character.chars().count() == 1 => character.chars().next()?,
        Key::Character(_)
            if mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC) && associated_text.is_some() =>
        {
            return Some(("0".into(), 'u'));
        }
        _ => return None,
    };
    let base = match key {
        Key::Character(character) if character.chars().count() == 1 => character.chars().next()?,
        _ => modified.to_lowercase().next().unwrap_or(modified),
    };
    let base = u32::from(base);
    let alternate = u32::from(modified);
    let payload = if mode.contains(TermMode::REPORT_ALTERNATE_KEYS) && alternate != base {
        format!("{base}:{alternate}")
    } else {
        base.to_string()
    };
    Some((payload, 'u'))
}

fn is_control_character(text: &str) -> bool {
    let Some(codepoint) = text.as_bytes().first().copied() else {
        return false;
    };
    text.len() == 1 && (codepoint < 0x20 || (0x7f..=0x9f).contains(&codepoint))
}

fn paste_bytes(text: &str, mode: TermMode) -> Vec<u8> {
    if mode.contains(TermMode::BRACKETED_PASTE) {
        let filtered = text.replace(['\x1b', '\x03'], "");
        format!("\x1b[200~{filtered}\x1b[201~").into_bytes()
    } else {
        text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
    }
}

fn mouse_report_bytes(
    mode: TermMode,
    button: u8,
    modifiers: Modifiers,
    cell: GridPoint<usize>,
    pressed: bool,
) -> Option<Vec<u8>> {
    let mut modifier_code = 0;
    if modifiers.shift() {
        modifier_code += 4;
    }
    if modifiers.alt() {
        modifier_code += 8;
    }
    if modifiers.control() {
        modifier_code += 16;
    }
    let code = button + modifier_code;

    if mode.contains(TermMode::SGR_MOUSE) {
        let suffix = if pressed { 'M' } else { 'm' };
        Some(
            format!(
                "\x1b[<{code};{};{}{suffix}",
                cell.column.0 + 1,
                cell.line + 1
            )
            .into_bytes(),
        )
    } else {
        let utf8 = mode.contains(TermMode::UTF8_MOUSE);
        let max_point = if utf8 { 2015 } else { 223 };
        if cell.column.0 >= max_point || cell.line >= max_point {
            return None;
        }

        let mut bytes = vec![
            0x1b,
            b'[',
            b'M',
            32 + if pressed { code } else { 3 + modifier_code },
        ];
        let mut encode_position = |position: usize| {
            if utf8 && position >= 95 {
                let position = 33 + position;
                bytes.push((0xc0 + position / 64) as u8);
                bytes.push((0x80 + (position & 63)) as u8);
            } else {
                bytes.push((33 + position) as u8);
            }
        };
        encode_position(cell.column.0);
        encode_position(cell.line);
        Some(bytes)
    }
}

fn mouse_button_code(button: mouse::Button) -> Option<u8> {
    match button {
        mouse::Button::Left => Some(0),
        mouse::Button::Middle => Some(1),
        mouse::Button::Right => Some(2),
        _ => None,
    }
}

fn motion_mouse_code(mode: TermMode, pressed: Option<mouse::Button>) -> Option<u8> {
    if !mode.intersects(TermMode::MOUSE_MOTION | TermMode::MOUSE_DRAG) {
        return None;
    }
    match pressed {
        Some(mouse::Button::Left) => Some(32),
        Some(mouse::Button::Middle) => Some(33),
        Some(mouse::Button::Right) => Some(34),
        None if mode.contains(TermMode::MOUSE_MOTION) => Some(35),
        _ => None,
    }
}

fn scroll_multiplier(mouse_reporting: bool) -> f32 {
    if mouse_reporting {
        1.0
    } else {
        SCROLL_MULTIPLIER
    }
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
        Named::Space if modifiers.control() && modifiers.alt() => "\x1b\0".into(),
        Named::Space if modifiers.control() => "\0".into(),
        Named::Space if modifiers.alt() => "\x1b ".into(),
        Named::Space => " ".into(),
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
        Named::F13 => function_tilde(25, modifier),
        Named::F14 => function_tilde(26, modifier),
        Named::F15 => function_tilde(28, modifier),
        Named::F16 => function_tilde(29, modifier),
        Named::F17 => function_tilde(31, modifier),
        Named::F18 => function_tilde(32, modifier),
        Named::F19 => function_tilde(33, modifier),
        Named::F20 => function_tilde(34, modifier),
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
        ClipboardRequest, EventProxy, Flags, KeyEventKind, LineKind, Modifiers, Named, PaintCell,
        Rgb8, SurfaceState, TerminalFrame, TerminalSize, TextFont, TextStyle, advance_cursor_blink,
        bell_requests_attention, build_line_runs, build_text_runs, clipboard_kind, encode_key,
        encode_key_event, line_rects, motion_mouse_code, mouse_report_bytes, named_key,
        paste_bytes, queried_color, reset_cursor_blink, scroll_multiplier,
        service_clipboard_requests, terminal_cell_size, terminal_input_modifiers,
        terminal_mouse_interaction, terminal_text_origin, terminal_wide_font_size,
        zoomed_font_size,
    };
    use alacritty_terminal::event::{Event as AlacrittyEvent, EventListener};
    use alacritty_terminal::term::{Term, TermMode};
    use alacritty_terminal::vte::ansi::{self, NamedColor};
    use iced::advanced::text::Paragraph as _;
    use iced::advanced::{Clipboard, clipboard};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc;

    fn load_terminal_test_font() {
        use iced::advanced::graphics::text::font_system;
        use std::borrow::Cow;

        font_system()
            .write()
            .expect("font system")
            .load_font(Cow::Borrowed(include_bytes!(
                "../../../../assets/fonts/JetBrainsMono-Regular.ttf"
            )));
        font_system()
            .write()
            .expect("font system")
            .load_font(Cow::Borrowed(include_bytes!(
                "../../../../assets/fonts/MonoplexKR-Regular.ttf"
            )));
    }

    fn test_term(recording: &[u8]) -> Term<EventProxy> {
        let (sender, _) = mpsc::unbounded_channel();
        let proxy = EventProxy {
            sender,
            wakeup_pending: Arc::new(AtomicBool::new(false)),
        };
        let size = TerminalSize {
            columns: 8,
            lines: 2,
            cell_width: 8,
            cell_height: 20,
        };
        let mut term = Term::new(Default::default(), &size, proxy);
        let mut parser: ansi::Processor = ansi::Processor::new();
        parser.advance(&mut term, recording);
        term
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

        assert_eq!(
            encode_key_event(
                &iced::keyboard::Key::Character("2".into()),
                &iced::keyboard::Key::Character("2".into()),
                Some("\0"),
                Modifiers::CTRL,
                iced::keyboard::Location::Standard,
                KeyEventKind::Press,
                TermMode::default(),
            ),
            Some(vec![0])
        );
    }

    #[test]
    fn kitty_protocol_disambiguates_ctrl_space() {
        assert_eq!(
            encode_key(
                &iced::keyboard::Key::Named(Named::Space),
                None,
                Modifiers::CTRL,
                TermMode::DISAMBIGUATE_ESC_CODES,
            ),
            Some(b"\x1b[32;5u".to_vec())
        );
    }

    #[test]
    fn kitty_protocol_reports_repeat_release_alternate_text_and_numpad() {
        let arrow = iced::keyboard::Key::Named(Named::ArrowUp);
        let event_types = TermMode::REPORT_EVENT_TYPES;
        assert_eq!(
            encode_key_event(
                &arrow,
                &arrow,
                None,
                Modifiers::NONE,
                iced::keyboard::Location::Standard,
                KeyEventKind::Repeat,
                event_types,
            ),
            Some(b"\x1b[1;1:2A".to_vec())
        );
        assert_eq!(
            encode_key_event(
                &arrow,
                &arrow,
                None,
                Modifiers::NONE,
                iced::keyboard::Location::Standard,
                KeyEventKind::Release,
                event_types,
            ),
            Some(b"\x1b[1;1:3A".to_vec())
        );
        assert_eq!(
            encode_key_event(
                &iced::keyboard::Key::Named(Named::Enter),
                &iced::keyboard::Key::Named(Named::Enter),
                None,
                Modifiers::NONE,
                iced::keyboard::Location::Standard,
                KeyEventKind::Release,
                event_types,
            ),
            None
        );

        let base = iced::keyboard::Key::Character("a".into());
        let shifted = iced::keyboard::Key::Character("A".into());
        assert_eq!(
            encode_key_event(
                &base,
                &shifted,
                Some("A"),
                Modifiers::SHIFT,
                iced::keyboard::Location::Standard,
                KeyEventKind::Press,
                TermMode::REPORT_ALL_KEYS_AS_ESC
                    | TermMode::REPORT_ALTERNATE_KEYS
                    | TermMode::REPORT_ASSOCIATED_TEXT,
            ),
            Some(b"\x1b[97:65;2;65u".to_vec())
        );

        let one = iced::keyboard::Key::Character("1".into());
        assert_eq!(
            encode_key_event(
                &one,
                &one,
                Some("1"),
                Modifiers::NONE,
                iced::keyboard::Location::Numpad,
                KeyEventKind::Press,
                TermMode::DISAMBIGUATE_ESC_CODES,
            ),
            Some(b"\x1b[57400u".to_vec())
        );
    }

    #[test]
    fn legacy_bindings_precede_kitty_encoding_on_key_press() {
        let arrow = iced::keyboard::Key::Named(Named::ArrowUp);
        let mode = TermMode::APP_CURSOR | TermMode::REPORT_EVENT_TYPES;
        assert_eq!(
            encode_key_event(
                &arrow,
                &arrow,
                None,
                Modifiers::NONE,
                iced::keyboard::Location::Standard,
                KeyEventKind::Press,
                mode,
            ),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            encode_key_event(
                &arrow,
                &arrow,
                None,
                Modifiers::NONE,
                iced::keyboard::Location::Standard,
                KeyEventKind::Release,
                mode,
            ),
            Some(b"\x1b[1;1:3A".to_vec())
        );

        let f1 = iced::keyboard::Key::Named(Named::F1);
        assert_eq!(
            encode_key_event(
                &f1,
                &f1,
                None,
                Modifiers::NONE,
                iced::keyboard::Location::Standard,
                KeyEventKind::Press,
                TermMode::REPORT_EVENT_TYPES,
            ),
            Some(b"\x1bOP".to_vec())
        );
        assert_eq!(
            encode_key_event(
                &f1,
                &f1,
                None,
                Modifiers::NONE,
                iced::keyboard::Location::Standard,
                KeyEventKind::Press,
                TermMode::DISAMBIGUATE_ESC_CODES,
            ),
            Some(b"\x1b[P".to_vec())
        );

        let tab = iced::keyboard::Key::Named(Named::Tab);
        assert_eq!(
            encode_key_event(
                &tab,
                &tab,
                None,
                Modifiers::SHIFT | Modifiers::ALT,
                iced::keyboard::Location::Standard,
                KeyEventKind::Press,
                TermMode::default(),
            ),
            Some(b"\x1b\x1b[Z".to_vec())
        );
        let enter = iced::keyboard::Key::Named(Named::Enter);
        assert_eq!(
            encode_key_event(
                &enter,
                &enter,
                Some("\r"),
                Modifiers::NONE,
                iced::keyboard::Location::Numpad,
                KeyEventKind::Press,
                TermMode::default(),
            ),
            Some(b"\n".to_vec())
        );
    }

    #[test]
    fn legacy_named_controls_use_the_platform_text_payload() {
        let enter = iced::keyboard::Key::Named(Named::Enter);
        assert_eq!(
            encode_key_event(
                &enter,
                &enter,
                Some("\r"),
                Modifiers::ALT,
                iced::keyboard::Location::Standard,
                KeyEventKind::Press,
                TermMode::default(),
            ),
            Some(b"\x1b\r".to_vec())
        );

        let backspace = iced::keyboard::Key::Named(Named::Backspace);
        assert_eq!(
            encode_key_event(
                &backspace,
                &backspace,
                Some("\x08"),
                Modifiers::CTRL,
                iced::keyboard::Location::Standard,
                KeyEventKind::Press,
                TermMode::REPORT_EVENT_TYPES,
            ),
            Some(vec![0x08])
        );

        assert_eq!(
            encode_key_event(
                &iced::keyboard::Key::Unidentified,
                &iced::keyboard::Key::Unidentified,
                Some(""),
                Modifiers::NONE,
                iced::keyboard::Location::Standard,
                KeyEventKind::Press,
                TermMode::default(),
            ),
            None
        );
    }

    #[test]
    fn macos_option_only_modifies_non_text_keys_by_default() {
        let character = iced::keyboard::Key::Character("a".into());
        let character_modifiers = terminal_input_modifiers(&character, Modifiers::ALT);
        assert_eq!(character_modifiers.alt(), !cfg!(target_os = "macos"),);

        let arrow = iced::keyboard::Key::Named(Named::ArrowLeft);
        assert!(terminal_input_modifiers(&arrow, Modifiers::ALT).alt());
    }

    #[test]
    fn paste_matches_terminal_line_and_bracket_safety_rules() {
        assert_eq!(
            paste_bytes("one\r\ntwo\n", TermMode::default()),
            b"one\rtwo\r"
        );
        assert_eq!(
            paste_bytes("safe\x1b[201~still\x03text", TermMode::BRACKETED_PASTE,),
            b"\x1b[200~safe[201~stilltext\x1b[201~"
        );
    }

    #[test]
    fn osc52_preserves_clipboard_kind() {
        assert_eq!(
            clipboard_kind(alacritty_terminal::term::ClipboardType::Clipboard),
            iced::advanced::clipboard::Kind::Standard,
        );
        assert_eq!(
            clipboard_kind(alacritty_terminal::term::ClipboardType::Selection),
            iced::advanced::clipboard::Kind::Primary,
        );
    }

    #[derive(Default)]
    struct TestClipboard {
        standard: Option<String>,
        primary: Option<String>,
    }

    impl Clipboard for TestClipboard {
        fn read(&self, kind: clipboard::Kind) -> Option<String> {
            match kind {
                clipboard::Kind::Standard => self.standard.clone(),
                clipboard::Kind::Primary => self.primary.clone(),
            }
        }

        fn write(&mut self, kind: clipboard::Kind, contents: String) {
            *match kind {
                clipboard::Kind::Standard => &mut self.standard,
                clipboard::Kind::Primary => &mut self.primary,
            } = Some(contents);
        }
    }

    #[test]
    fn osc52_services_every_store_and_load_in_order() {
        let formatter = Arc::new(|text: &str| format!("<{text}>"));
        let mut requests = vec![
            ClipboardRequest::Store(
                alacritty_terminal::term::ClipboardType::Clipboard,
                "first".into(),
            ),
            ClipboardRequest::Load(
                alacritty_terminal::term::ClipboardType::Clipboard,
                formatter.clone(),
            ),
            ClipboardRequest::Store(
                alacritty_terminal::term::ClipboardType::Clipboard,
                "second".into(),
            ),
            ClipboardRequest::Load(
                alacritty_terminal::term::ClipboardType::Clipboard,
                formatter,
            ),
        ];
        let mut clipboard = TestClipboard::default();

        let replies = service_clipboard_requests(&mut requests, &mut clipboard);

        assert!(requests.is_empty());
        assert_eq!(replies, [b"<first>".to_vec(), b"<second>".to_vec()]);
        assert_eq!(
            clipboard.read(clipboard::Kind::Standard),
            Some("second".into())
        );
    }

    #[test]
    fn mouse_reports_preserve_modifiers_utf8_coordinates_and_bounds() {
        let origin = alacritty_terminal::index::Point::new(0, alacritty_terminal::index::Column(0));
        assert_eq!(
            mouse_report_bytes(TermMode::default(), 0, Modifiers::CTRL, origin, false,),
            Some(vec![0x1b, b'[', b'M', 51, 33, 33])
        );

        let utf8_column =
            alacritty_terminal::index::Point::new(0, alacritty_terminal::index::Column(95));
        assert_eq!(
            mouse_report_bytes(TermMode::UTF8_MOUSE, 0, Modifiers::NONE, utf8_column, true,),
            Some(vec![0x1b, b'[', b'M', 32, 0xc2, 0x80, 33])
        );

        let out_of_bounds =
            alacritty_terminal::index::Point::new(0, alacritty_terminal::index::Column(223));
        assert_eq!(
            mouse_report_bytes(TermMode::default(), 0, Modifiers::NONE, out_of_bounds, true,),
            None
        );
    }

    #[test]
    fn mouse_motion_reports_dragged_buttons_and_unpressed_motion() {
        assert_eq!(
            motion_mouse_code(TermMode::MOUSE_DRAG, Some(iced::mouse::Button::Left)),
            Some(32)
        );
        assert_eq!(
            motion_mouse_code(TermMode::MOUSE_DRAG, Some(iced::mouse::Button::Middle),),
            Some(33)
        );
        assert_eq!(motion_mouse_code(TermMode::MOUSE_MOTION, None), Some(35));
        assert_eq!(motion_mouse_code(TermMode::MOUSE_DRAG, None), None);
        assert_eq!(scroll_multiplier(true), 1.0);
        assert_eq!(scroll_multiplier(false), super::SCROLL_MULTIPLIER);
    }

    #[test]
    fn space_key_writes_an_ascii_space() {
        assert_eq!(
            named_key(Named::Space, Modifiers::NONE, TermMode::default()),
            Some(" ".into())
        );
    }

    #[test]
    fn an_open_input_method_only_claims_the_keyboard_while_it_composes() {
        // what `InputMethod::Opened` installs: the method is available, the
        // user has composed nothing. A focused terminal sits here for a whole
        // typing session, so this state must NOT hold the keyboard.
        assert!(!super::composing(None));
        assert!(!super::composing(
            Some(&super::input_method::Preedit::new())
        ));

        let mut hangul = super::input_method::Preedit::new();
        hangul.content = "한".into();
        assert!(super::composing(Some(&hangul)));
    }

    #[test]
    fn command_plus_and_minus_zoom_the_terminal_font() {
        load_terminal_test_font();
        let command = Modifiers::LOGO;

        assert_eq!(
            zoomed_font_size(
                &iced::keyboard::Key::Character("+".into()),
                command | Modifiers::SHIFT,
                super::FONT_SIZE,
            ),
            Some(15.0)
        );
        assert_eq!(
            zoomed_font_size(
                &iced::keyboard::Key::Character("=".into()),
                command,
                super::FONT_SIZE,
            ),
            Some(15.0)
        );
        assert_eq!(
            zoomed_font_size(&iced::keyboard::Key::Character("-".into()), command, 15.0,),
            Some(super::FONT_SIZE)
        );
        assert_eq!(
            zoomed_font_size(
                &iced::keyboard::Key::Character("+".into()),
                Modifiers::SHIFT,
                super::FONT_SIZE,
            ),
            None
        );

        let normal_cell = terminal_cell_size(super::FONT_SIZE);
        let zoomed_cell = terminal_cell_size(15.0);
        let bounds = iced::Size::new(840.0, 392.0);

        assert!(zoomed_cell.width > normal_cell.width);
        assert!(zoomed_cell.height > normal_cell.height);
        assert!(
            TerminalSize::fit(bounds, zoomed_cell).columns
                < TerminalSize::fit(bounds, normal_cell).columns
        );
    }

    #[test]
    fn terminal_cell_matches_the_fractional_glyph_advance() {
        load_terminal_test_font();

        let paragraph = <iced::Renderer as iced::advanced::text::Renderer>::Paragraph::with_text(
            iced::advanced::text::Text {
                content: "M",
                bounds: iced::Size::INFINITE,
                size: iced::Pixels(super::FONT_SIZE),
                line_height: iced::advanced::text::LineHeight::Relative(super::LINE_HEIGHT),
                font: super::TERMINAL_FONT,
                align_x: iced::advanced::text::Alignment::Left,
                align_y: iced::alignment::Vertical::Top,
                shaping: iced::advanced::text::Shaping::Basic,
                wrapping: iced::advanced::text::Wrapping::None,
            },
        );
        let measured = paragraph.min_bounds();

        assert_eq!(terminal_cell_size(super::FONT_SIZE), measured);

        let box_line = <iced::Renderer as iced::advanced::text::Renderer>::Paragraph::with_text(
            iced::advanced::text::Text {
                content: "╭────────╮",
                bounds: iced::Size::INFINITE,
                size: iced::Pixels(super::FONT_SIZE),
                line_height: iced::advanced::text::LineHeight::Relative(super::LINE_HEIGHT),
                font: super::TERMINAL_FONT,
                align_x: iced::advanced::text::Alignment::Left,
                align_y: iced::alignment::Vertical::Top,
                shaping: iced::advanced::text::Shaping::Auto,
                wrapping: iced::advanced::text::Wrapping::None,
            },
        );
        let expected_width = terminal_cell_size(super::FONT_SIZE).width * 10.0;

        assert!((box_line.min_bounds().width - expected_width).abs() < 0.01);
    }

    #[test]
    fn dynamic_background_query_reports_the_terminal_background() {
        let term = test_term(b"");
        let color = queried_color(NamedColor::Background as usize, term.colors()).unwrap();

        assert_eq!((color.r, color.g, color.b), (16, 15, 13));
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
            font: TextFont::Terminal,
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
                underline: style.color,
                flags: Flags::empty(),
            })
            .collect::<Vec<_>>();

        let runs = build_text_runs(&cells);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].content, "claude code");
    }

    #[test]
    fn pause_symbol_uses_an_isolated_native_run() {
        let cells = "A⏸B"
            .chars()
            .enumerate()
            .map(|(column, character)| PaintCell {
                row: 0,
                column: column as u16,
                character,
                zerowidth: Vec::new(),
                foreground: Rgb8(1, 2, 3),
                background: Rgb8(0, 0, 0),
                underline: Rgb8(1, 2, 3),
                flags: Flags::empty(),
            })
            .collect::<Vec<_>>();

        let runs = build_text_runs(&cells);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[1].content, "⏸");
        assert_eq!(runs[1].style.font, TextFont::PauseSymbol);
    }

    #[test]
    fn terminal_text_origin_uses_the_cell_center() {
        let origin = terminal_text_origin(
            iced::Rectangle::new(iced::Point::new(10.0, 20.0), iced::Size::new(400.0, 300.0)),
            iced::Size::new(8.5, 19.5),
            2,
            3,
        );

        assert_eq!(origin, iced::Point::new(35.5, 68.75));
    }

    #[test]
    fn wide_glyphs_start_new_absolute_text_runs() {
        let cells = [
            PaintCell {
                row: 0,
                column: 0,
                character: 'A',
                zerowidth: Vec::new(),
                foreground: Rgb8(1, 2, 3),
                background: Rgb8(0, 0, 0),
                underline: Rgb8(1, 2, 3),
                flags: Flags::empty(),
            },
            PaintCell {
                row: 0,
                column: 1,
                character: '한',
                zerowidth: Vec::new(),
                foreground: Rgb8(1, 2, 3),
                background: Rgb8(0, 0, 0),
                underline: Rgb8(1, 2, 3),
                flags: Flags::WIDE_CHAR,
            },
            PaintCell {
                row: 0,
                column: 2,
                character: ' ',
                zerowidth: Vec::new(),
                foreground: Rgb8(1, 2, 3),
                background: Rgb8(0, 0, 0),
                underline: Rgb8(1, 2, 3),
                flags: Flags::WIDE_CHAR_SPACER,
            },
            PaintCell {
                row: 0,
                column: 3,
                character: 'B',
                zerowidth: Vec::new(),
                foreground: Rgb8(1, 2, 3),
                background: Rgb8(0, 0, 0),
                underline: Rgb8(1, 2, 3),
                flags: Flags::empty(),
            },
        ];

        let runs = build_text_runs(&cells);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].content, "A");
        assert_eq!((runs[1].column, runs[1].columns), (1, 2));
        assert_eq!(runs[1].content, "한");
        assert_eq!((runs[2].column, runs[2].content.as_str()), (3, "B"));
    }

    #[test]
    fn wide_cell_cursor_uses_two_columns_and_unfocused_hollow_shape() {
        let mut term = test_term("한\x1b[H".as_bytes());
        term.is_focused = true;
        let focused = TerminalFrame::from_term(
            &term,
            TerminalSize {
                columns: 8,
                lines: 2,
                cell_width: 8,
                cell_height: 20,
            },
        );
        let cursor = focused.cursor.expect("cursor");
        assert_eq!(cursor.columns, 2);
        assert_eq!(cursor.shape, ansi::CursorShape::Block);

        term.is_focused = false;
        let unfocused = TerminalFrame::from_term(
            &term,
            TerminalSize {
                columns: 8,
                lines: 2,
                cell_width: 8,
                cell_height: 20,
            },
        );
        assert_eq!(
            unfocused.cursor.expect("cursor").shape,
            ansi::CursorShape::HollowBlock
        );
    }

    #[test]
    fn dynamic_terminal_background_fills_the_frame() {
        let term = test_term(b"\x1b]11;rgb:0101/0202/0303\x07");
        let frame = TerminalFrame::from_term(
            &term,
            TerminalSize {
                columns: 8,
                lines: 2,
                cell_width: 8,
                cell_height: 20,
            },
        );

        assert_eq!(frame.background, Rgb8(1, 2, 3));
    }

    #[test]
    fn cursor_color_query_only_replies_after_dynamic_override() {
        let term = test_term(b"");
        assert_eq!(
            queried_color(NamedColor::Cursor as usize, term.colors()),
            None
        );

        let term = test_term(b"\x1b]12;rgb:0909/0808/0707\x07");
        let color = queried_color(NamedColor::Cursor as usize, term.colors()).unwrap();
        assert_eq!((color.r, color.g, color.b), (9, 8, 7));
    }

    #[test]
    fn terminal_line_styles_keep_kind_color_and_wide_width() {
        let cells = [PaintCell {
            row: 0,
            column: 1,
            character: '한',
            zerowidth: Vec::new(),
            foreground: Rgb8(1, 2, 3),
            background: Rgb8(0, 0, 0),
            underline: Rgb8(10, 20, 30),
            flags: Flags::WIDE_CHAR | Flags::UNDERCURL | Flags::STRIKEOUT,
        }];

        let runs = build_line_runs(&cells);
        assert_eq!(runs.len(), 2);
        let undercurl = runs
            .iter()
            .find(|run| run.kind == LineKind::Undercurl)
            .unwrap();
        assert_eq!((undercurl.color, undercurl.columns), (Rgb8(10, 20, 30), 2));
        let strike = runs
            .iter()
            .find(|run| run.kind == LineKind::Strikeout)
            .unwrap();
        assert_eq!(strike.color, Rgb8(1, 2, 3));

        let rects = line_rects(
            iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(100.0, 40.0)),
            iced::Size::new(8.0, 20.0),
            undercurl,
        );
        assert!(rects.len() > 2);
        assert_ne!(rects[0].y, rects[1].y);
    }

    #[test]
    fn cursor_blink_matches_alacritty_interval_and_timeout() {
        let start = iced::time::Instant::now();
        let mut state = SurfaceState::default();
        reset_cursor_blink(&mut state, start);
        assert!(state.cursor_visible);

        let next = advance_cursor_blink(&mut state, start + super::CURSOR_BLINK_INTERVAL, true);
        assert!(!state.cursor_visible);
        assert_eq!(next, Some(start + super::CURSOR_BLINK_INTERVAL * 2));

        assert_eq!(
            advance_cursor_blink(&mut state, start + super::CURSOR_BLINK_TIMEOUT, true,),
            None
        );
        assert!(state.cursor_visible);
        assert!(state.cursor_blink_timed_out);
    }

    #[test]
    fn bell_and_mouse_cursor_follow_terminal_modes() {
        assert!(bell_requests_attention(false, TermMode::URGENCY_HINTS));
        assert!(!bell_requests_attention(true, TermMode::URGENCY_HINTS));
        assert_eq!(
            terminal_mouse_interaction(TermMode::MOUSE_REPORT_CLICK, Modifiers::NONE),
            iced::mouse::Interaction::Idle
        );
        assert_eq!(
            terminal_mouse_interaction(TermMode::MOUSE_REPORT_CLICK, Modifiers::SHIFT),
            iced::mouse::Interaction::Text
        );
    }

    #[test]
    fn monoplex_wide_glyph_occupies_exactly_two_terminal_cells() {
        load_terminal_test_font();
        let wide_size = terminal_wide_font_size(super::FONT_SIZE);
        let paragraph = <iced::Renderer as iced::advanced::text::Renderer>::Paragraph::with_text(
            iced::advanced::text::Text {
                content: "한",
                bounds: iced::Size::INFINITE,
                size: iced::Pixels(wide_size),
                line_height: iced::advanced::text::LineHeight::Relative(super::LINE_HEIGHT),
                font: super::TERMINAL_WIDE_FONT,
                align_x: iced::advanced::text::Alignment::Left,
                align_y: iced::alignment::Vertical::Top,
                shaping: iced::advanced::text::Shaping::Auto,
                wrapping: iced::advanced::text::Wrapping::None,
            },
        );
        let expected = terminal_cell_size(super::FONT_SIZE).width * 2.0;

        assert!(
            (paragraph.min_bounds().width - expected).abs() < 0.01,
            "wide={} expected={expected} wide_size={wide_size}",
            paragraph.min_bounds().width,
        );
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

    #[test]
    fn fractional_cell_advance_determines_the_grid_width() {
        let size = TerminalSize::fit(iced::Size::new(83.9, 196.0), iced::Size::new(8.4, 19.6));

        assert_eq!(size.columns, 9);
        assert_eq!(size.lines, 10);
    }

    #[cfg(unix)]
    fn wait_for_visible(terminal: &mut super::Terminal, expected: &str) {
        // Wedged-detector backstop, not a tuned budget. Wait for an event or
        // one frame interval, then refresh so a lost or coalesced Wakeup cannot
        // leave a valid grid waiting forever. The old 2s deadline lost the race
        // to a loaded CI runner's shell spawn.
        let backstop = Duration::from_secs(60);
        let deadline = Instant::now() + backstop;
        let waiter = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();

        let mut pty_closed = false;
        loop {
            let visible = terminal
                .frame
                .text
                .iter()
                .map(|run| run.content.as_str())
                .collect::<String>();
            if visible.contains(expected) {
                return;
            }
            assert!(
                !pty_closed,
                "`{expected}` never reached the visible frame, and the PTY has closed"
            );

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                panic!(
                    "`{expected}` never reached the visible terminal frame within the \
                     {backstop:?} wedged-detector backstop"
                );
            }
            let batch = waiter.block_on(async {
                let mut receiver = terminal.events.lock().await;
                match tokio::time::timeout(remaining.min(super::FRAME_INTERVAL), receiver.recv())
                    .await
                {
                    Ok(Some(first)) => {
                        let mut batch = vec![first];
                        while let Ok(event) = receiver.try_recv() {
                            batch.push(event);
                        }
                        batch
                    }
                    // The pty is GONE, which for a one-shot child is the
                    // normal end of the program — and its exit carries the
                    // snapshot that makes its last line visible. So check the
                    // frame once more before calling this a failure; the loop
                    // top does exactly that, on an empty batch.
                    Ok(None) => {
                        pty_closed = true;
                        Vec::new()
                    }
                    Err(_) => Vec::new(),
                }
            });
            terminal.handle_events(batch);
            terminal.snapshot();
        }
    }

    #[cfg(unix)]
    #[test]
    fn shell_space_round_trip_reaches_visible_text_runs() {
        let mut terminal = super::Terminal::new(
            u64::MAX - 1,
            "/bin/sh".into(),
            vec![
                "-c".into(),
                // `exec cat` parks the child on a read that never returns. Without
                // it the shell exits the instant it prints, and the event loop can
                // tear the pty down before parsing what it wrote — the test then
                // fails on a teardown race rather than on the round trip it is
                // about. (That race is a real engine bug; it is not this test's.)
                "IFS= read -r value; [ \"$value\" = \"hello world\" ] && \
                 printf SHELL_SPACE_OK; exec cat"
                    .into(),
            ],
            std::env::current_dir().unwrap(),
        )
        .unwrap();
        terminal.write(b"hello".to_vec());
        terminal.write(
            named_key(Named::Space, Modifiers::NONE, TermMode::default())
                .unwrap()
                .into_bytes(),
        );
        terminal.write(b"world\r".to_vec());

        wait_for_visible(&mut terminal, "SHELL_SPACE_OK");
    }
}
