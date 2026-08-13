use iced::advanced::clipboard;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, Widget, input_method, layout, mouse, text, widget};
use iced::keyboard;
use iced::keyboard::key::{Code, Named, Physical};
use iced::keyboard::{Key, Location, Modifiers};
use iced::widget::text_editor::{self, Content, Cursor, Edit, Position};
use iced::{Color, Event, Font, Length, Pixels, Point, Rectangle, Size, Theme};
use std::fs::OpenOptions;
use std::hint::black_box;
use std::io::Write as _;
use std::ops::Range;
use std::path::PathBuf;
use ui_lang_runtime::rich_text_editor::Format;
use ui_lang_runtime::{ContentVersion, EditorChange, RichTextEditor, rich_text_editor::Action};

#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

const LINE_COUNT: usize = 100_000;

#[test]
#[ignore = "rich-motion allocation contract run explicitly by the performance-contract filter"]
fn performance_contract_rich_motion_allocations() {
    let content = Content::with_text("one two three four five six\nseven eight nine");
    let renderer = headless_renderer();
    let limits = layout::Limits::new(Size::ZERO, Size::new(70.0, 120.0));
    let viewport = Rectangle::with_size(Size::new(70.0, 120.0));
    let version = ContentVersion::new(92, 0);
    let mut editor = RichTextEditor::<_, Action>::new(&content, version)
        .width(Length::Fixed(70.0))
        .height(Length::Fixed(120.0))
        .wrapping(text::Wrapping::Word)
        .on_action(|action| action)
        .highlight_with::<WholeLine>((), 0, |_| Format::default());
    let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
    let node = editor.layout(&mut tree, &renderer, &limits);
    let mut clipboard = clipboard::Null;
    let mut messages = Vec::with_capacity(6);

    let mut shell = Shell::new(&mut messages);
    editor.update(
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(5.0, 5.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
        &viewport,
    );
    messages.clear();

    let commands = [
        (Named::ArrowUp, Code::ArrowUp),
        (Named::ArrowDown, Code::ArrowDown),
        (Named::Home, Code::Home),
        (Named::End, Code::End),
        (Named::PageUp, Code::PageUp),
        (Named::PageDown, Code::PageDown),
    ];
    let profiler = dhat::Profiler::builder().testing().build();
    let before = dhat::HeapStats::get();
    for (named, code) in commands {
        let key = Key::Named(named);
        let event = Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: Physical::Code(code),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: false,
        });
        let mut shell = Shell::new(&mut messages);
        editor.update(
            &mut tree,
            &event,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
    }
    let after = dhat::HeapStats::get();
    drop(profiler);

    assert_eq!(messages.len(), commands.len());
    assert!(
        messages
            .iter()
            .all(|message| matches!(message, Action::MoveTo(_)))
    );
    let allocated = (
        after.total_blocks - before.total_blocks,
        after.total_bytes - before.total_bytes,
    );
    eprintln!(
        "six rich-motion commands: {} allocations, {} bytes",
        allocated.0, allocated.1
    );
    assert_eq!(
        allocated,
        (0, 0),
        "six rich-motion commands must not allocate"
    );
}

#[derive(Default)]
struct WholeLine {
    current_line: usize,
}

impl text::Highlighter for WholeLine {
    type Settings = ();
    type Highlight = ();
    type Iterator<'a> = std::iter::Once<(Range<usize>, ())>;

    fn new(_settings: &Self::Settings) -> Self {
        Self::default()
    }

    fn update(&mut self, _new_settings: &Self::Settings) {}

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.current_line += 1;
        std::iter::once((0..line.len(), ()))
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

#[derive(Clone, Copy)]
struct AllocationRecord {
    scenario: &'static str,
    iterations: usize,
    allocation_count: u64,
    allocated_bytes: u64,
    allocation_count_budget: u64,
    allocated_bytes_budget: u64,
}

#[test]
#[ignore = "large-document allocation contract run explicitly in CI"]
fn allocation_contract_100k_total_allocations() {
    let source = large_source();
    let mut content = Content::with_text(&source);
    let renderer = headless_renderer();
    let limits = layout::Limits::new(Size::ZERO, Size::new(800.0, 600.0));
    let version = ContentVersion::new(91, 0);
    let mut editor = RichTextEditor::<_, ()>::new(&content, version)
        .width(Length::Fixed(800.0))
        .height(Length::Fixed(600.0))
        .wrapping(text::Wrapping::None)
        .highlight_with::<WholeLine>((), 0, |_| Format::default());
    let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
    editor.layout(&mut tree, &renderer, &limits);
    drop(editor);

    // Starting after the 100k-line fixture is fully shaped isolates allocation
    // totals for the named operation. HeapStats snapshots do not allocate and
    // total_blocks/total_bytes are monotonic while this profiler is active.
    let profiler = dhat::Profiler::builder().testing().build();
    let mut records = Vec::with_capacity(6);

    measure(&mut records, "caret_1000", 1_000, 2_000, 1_000_000, || {
        for event in 0..1_000 {
            let line = event * 97 % LINE_COUNT;
            content.move_to(Cursor {
                position: Position {
                    line,
                    column: event % 8,
                },
                selection: None,
            });
            let mut editor = RichTextEditor::<_, ()>::new(&content, version)
                .width(Length::Fixed(800.0))
                .height(Length::Fixed(600.0))
                .wrapping(text::Wrapping::None)
                .highlight_with::<WholeLine>((), 0, |_| Format::default());
            black_box(editor.layout(&mut tree, &renderer, &limits));
        }
    });

    // A colour-only format key flip re-shapes the revealed viewport window;
    // the walk classifies the rest allocation-free and defers their rebuild
    // to the pass that scrolls them into view. The caret loop above left the
    // viewport deep, so this once shaped ~97k offscreen paragraphs — 2.7M
    // blocks and 918MB — for lines no frame would draw.
    measure(
        &mut records,
        "format_key_only",
        1,
        50_000,
        20_000_000,
        || {
            let mut editor = RichTextEditor::<_, ()>::new(&content, version)
                .width(Length::Fixed(800.0))
                .height(Length::Fixed(600.0))
                .wrapping(text::Wrapping::None)
                .highlight_with::<WholeLine>((), 1, |_| Format {
                    color: Some(Color::BLACK),
                    ..Format::default()
                });
            black_box(editor.layout(&mut tree, &renderer, &limits));
        },
    );

    content.move_to(Cursor {
        position: Position {
            line: 50_000,
            column: 4,
        },
        selection: None,
    });
    let revised_version = ContentVersion::new(91, 1);
    // The count is dominated by materializing the changed source through
    // `Content::text()`, which pays one small allocation per document line;
    // the highlight pass itself is bounded by the revealed viewport and the
    // compare walk reuses scratch buffers. Splicing the source through the
    // change hint instead of re-materializing it is the next cut available.
    measure(
        &mut records,
        "one_char_insertion",
        1,
        150_000,
        20_000_000,
        || {
            content.perform(text_editor::Action::Edit(Edit::Insert('x')));
            let mut editor = RichTextEditor::<_, ()>::new(&content, revised_version)
                .change_hint(EditorChange::new(version, revised_version, 50_000, 1, 1))
                .width(Length::Fixed(800.0))
                .height(Length::Fixed(600.0))
                .wrapping(text::Wrapping::None)
                .highlight_with::<WholeLine>((), 1, |_| Format {
                    color: Some(Color::BLACK),
                    ..Format::default()
                });
            black_box(editor.layout(&mut tree, &renderer, &limits));
        },
    );

    let resized_limits = layout::Limits::new(Size::ZERO, Size::new(640.0, 600.0));
    // With wrapping off a width change cannot reflow a glyph, so the pass
    // does not even open: the measured cost is zero allocations. The budget
    // exists to catch a reflow sneaking back into pure-resize layout.
    measure(&mut records, "viewport_resize", 1, 2_000, 1_000_000, || {
        let mut editor = RichTextEditor::<_, ()>::new(&content, revised_version)
            .width(Length::Fixed(640.0))
            .height(Length::Fixed(600.0))
            .wrapping(text::Wrapping::None)
            .highlight_with::<WholeLine>((), 1, |_| Format {
                color: Some(Color::BLACK),
                ..Format::default()
            });
        black_box(editor.layout(&mut tree, &renderer, &resized_limits));
    });

    let resized_viewport = Rectangle::with_size(Size::new(640.0, 600.0));
    let anchor = Point::new(10.0, 10.0);
    let mut clipboard = clipboard::Null;
    let mut messages = Vec::with_capacity(1);
    let mut editor = editable_editor(&content, revised_version, 640.0);
    let mut node = editor.layout(&mut tree, &renderer, &resized_limits);
    {
        let mut shell = Shell::new(&mut messages);
        editor.update(
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(anchor),
            &renderer,
            &mut clipboard,
            &mut shell,
            &resized_viewport,
        );
    }
    let cursor = take_move(&mut messages);
    drop(editor);
    content.move_to(cursor);

    measure(
        &mut records,
        "selection_drag_1000",
        1_000,
        2_000,
        1_000_000,
        || {
            for event in 0..1_000 {
                let point = Point::new(
                    20.0 + (event % 40) as f32 * 9.0,
                    10.0 + (event % 20) as f32 * 25.0,
                );
                let mut editor = editable_editor(&content, revised_version, 640.0);
                node = editor.layout(&mut tree, &renderer, &resized_limits);
                {
                    let mut shell = Shell::new(&mut messages);
                    editor.update(
                        &mut tree,
                        &Event::Mouse(mouse::Event::CursorMoved { position: point }),
                        Layout::new(&node),
                        mouse::Cursor::Available(point),
                        &renderer,
                        &mut clipboard,
                        &mut shell,
                        &resized_viewport,
                    );
                }
                let cursor = take_move(&mut messages);
                drop(editor);
                content.move_to(cursor);
            }
        },
    );

    let mut editor = editable_editor(&content, revised_version, 640.0);
    node = editor.layout(&mut tree, &renderer, &resized_limits);
    {
        let mut shell = Shell::new(&mut messages);
        editor.update(
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(anchor),
            &renderer,
            &mut clipboard,
            &mut shell,
            &resized_viewport,
        );
    }
    drop(editor);
    assert!(messages.is_empty());

    measure(
        &mut records,
        "hangul_ime_sequence",
        3,
        50_000,
        400_000_000,
        || {
            let mut editor = editable_editor(&content, revised_version, 640.0);
            node = editor.layout(&mut tree, &renderer, &resized_limits);
            for stage in ["ㅇ", "으", "응"] {
                let mut shell = Shell::new(&mut messages);
                editor.update(
                    &mut tree,
                    &Event::InputMethod(input_method::Event::Preedit(stage.into(), Some(3..3))),
                    Layout::new(&node),
                    mouse::Cursor::Unavailable,
                    &renderer,
                    &mut clipboard,
                    &mut shell,
                    &resized_viewport,
                );
                shell.revalidate_layout(|| {
                    node = editor.layout(&mut tree, &renderer, &resized_limits);
                });
                assert!(messages.is_empty());
            }
        },
    );

    drop(profiler);
    assert_allocation_budgets(&records);
}

fn measure(
    records: &mut Vec<AllocationRecord>,
    scenario: &'static str,
    iterations: usize,
    allocation_count_budget: u64,
    allocated_bytes_budget: u64,
    operation: impl FnOnce(),
) {
    let before = dhat::HeapStats::get();
    operation();
    let after = dhat::HeapStats::get();
    let allocation_count = after.total_blocks - before.total_blocks;
    let allocated_bytes = after.total_bytes - before.total_bytes;
    let injected = std::env::var_os("ICE_EDITOR_PERF_INJECT_HEAP_FAILURE");
    let (allocation_count_budget, allocated_bytes_budget) = allocation_budgets(
        scenario,
        allocation_count_budget,
        allocated_bytes_budget,
        injected.as_deref(),
    );
    let record = AllocationRecord {
        scenario,
        iterations,
        allocation_count,
        allocated_bytes,
        allocation_count_budget,
        allocated_bytes_budget,
    };
    write_record(&record);
    records.push(record);
}

fn allocation_budgets(
    scenario: &str,
    allocation_count_budget: u64,
    allocated_bytes_budget: u64,
    injected_failure: Option<&std::ffi::OsStr>,
) -> (u64, u64) {
    if injected_failure == Some(std::ffi::OsStr::new(scenario)) {
        (1, 1)
    } else {
        (allocation_count_budget, allocated_bytes_budget)
    }
}

fn editable_editor(
    content: &Content,
    version: ContentVersion,
    width: f32,
) -> RichTextEditor<'_, WholeLine, Action> {
    RichTextEditor::new(content, version)
        .width(Length::Fixed(width))
        .height(Length::Fixed(600.0))
        .wrapping(text::Wrapping::None)
        .on_action(|action| action)
        .highlight_with::<WholeLine>((), 1, |_| Format {
            color: Some(Color::BLACK),
            ..Format::default()
        })
}

fn take_move(messages: &mut Vec<Action>) -> Cursor {
    let [Action::MoveTo(cursor)] = messages.as_slice() else {
        panic!("pointer event must publish one cursor action: {messages:?}");
    };
    let cursor = *cursor;
    messages.clear();
    cursor
}

fn headless_renderer() -> iced::Renderer {
    iced_test::futures::futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::DEFAULT,
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("headless renderer")
}

fn large_source() -> String {
    (0..LINE_COUNT)
        .map(|index| format!("line {index}\n"))
        .collect()
}

fn write_record(record: &AllocationRecord) {
    let Some(path) = std::env::var_os("ICE_EDITOR_PERF_JSONL") else {
        eprintln!(
            "{}: {} allocations, {} bytes",
            record.scenario, record.allocation_count, record.allocated_bytes
        );
        return;
    };
    write_record_to(&PathBuf::from(path), record);
}

fn write_record_to(path: &std::path::Path, record: &AllocationRecord) {
    let mut output = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap_or_else(|error| panic!("failed to open {}: {error}", path.display()));
    let value = serde_json::json!({
        "schema": "ice.rich-text-editor.performance.v1",
        "kind": "heap",
        "scenario": record.scenario,
        "document_lines": LINE_COUNT + 1,
        "iterations": record.iterations,
        "collector": "dhat-0.3.3",
        "scope": "operation-only",
        "allocation_count": record.allocation_count,
        "allocated_bytes": record.allocated_bytes,
        "allocation_count_budget": record.allocation_count_budget,
        "allocated_bytes_budget": record.allocated_bytes_budget,
    });
    writeln!(output, "{value}")
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
    output
        .flush()
        .unwrap_or_else(|error| panic!("failed to flush {}: {error}", path.display()));
    output
        .sync_all()
        .unwrap_or_else(|error| panic!("failed to sync {}: {error}", path.display()));
}

fn allocation_budget_failures(records: &[AllocationRecord]) -> Vec<String> {
    let mut failures = Vec::new();
    for record in records {
        if record.allocation_count > record.allocation_count_budget {
            failures.push(format!(
                "{} allocated {} blocks; budget is {}",
                record.scenario, record.allocation_count, record.allocation_count_budget
            ));
        }
        if record.allocated_bytes > record.allocated_bytes_budget {
            failures.push(format!(
                "{} allocated {} bytes; budget is {}",
                record.scenario, record.allocated_bytes, record.allocated_bytes_budget
            ));
        }
    }
    failures
}

fn assert_allocation_budgets(records: &[AllocationRecord]) {
    if let Err(message) = allocation_budget_gate(records) {
        panic!("{message}");
    }
}

fn allocation_budget_gate(records: &[AllocationRecord]) -> Result<(), String> {
    let failures = allocation_budget_failures(records);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "RichTextEditor allocation budgets failed:\n{}",
            failures.join("\n")
        ))
    }
}

#[test]
fn performance_evidence_failure_injection_preserves_heap_record() {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

    let path = std::env::temp_dir().join(format!(
        "ice-editor-heap-evidence-{}-{}.jsonl",
        std::process::id(),
        NEXT_PATH.fetch_add(1, Ordering::Relaxed)
    ));
    let (allocation_count_budget, allocated_bytes_budget) = allocation_budgets(
        "caret_1000",
        2_000,
        1_000_000,
        Some(std::ffi::OsStr::new("caret_1000")),
    );
    let record = AllocationRecord {
        scenario: "caret_1000",
        iterations: 1_000,
        allocation_count: 2,
        allocated_bytes: 3,
        allocation_count_budget,
        allocated_bytes_budget,
    };
    write_record_to(&path, &record);
    let failures = allocation_budget_failures(&[record]);

    let raw = std::fs::read_to_string(&path).expect("failure evidence must be readable");
    std::fs::remove_file(&path).expect("temporary failure evidence must be removable");
    let lines = raw.lines().collect::<Vec<_>>();
    let [line] = lines.as_slice() else {
        panic!("failure evidence must contain exactly one record: {raw:?}");
    };
    let value: serde_json::Value = serde_json::from_str(line).expect("valid failure evidence");
    assert_eq!(value["scenario"], "caret_1000");
    assert_eq!(value["allocation_count"], 2);
    assert_eq!(value["allocated_bytes"], 3);
    assert_eq!(failures.len(), 2);
    assert!(failures[0].contains("allocated 2 blocks; budget is 1"));
    assert!(failures[1].contains("allocated 3 bytes; budget is 1"));
    assert!(allocation_budget_gate(&[record]).is_err());
}
