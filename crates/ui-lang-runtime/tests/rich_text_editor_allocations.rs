use iced::advanced::clipboard;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, Widget, input_method, layout, mouse, text, widget};
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

    measure(
        &mut records,
        "format_key_only",
        1,
        4_000_000,
        1_700_000_000,
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
    measure(
        &mut records,
        "one_char_insertion",
        1,
        600_000,
        400_000_000,
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
    measure(
        &mut records,
        "viewport_resize",
        1,
        3_500_000,
        1_300_000_000,
        || {
            let mut editor = RichTextEditor::<_, ()>::new(&content, revised_version)
                .width(Length::Fixed(640.0))
                .height(Length::Fixed(600.0))
                .wrapping(text::Wrapping::None)
                .highlight_with::<WholeLine>((), 1, |_| Format {
                    color: Some(Color::BLACK),
                    ..Format::default()
                });
            black_box(editor.layout(&mut tree, &renderer, &resized_limits));
        },
    );

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
        1_200_000,
        1_100_000_000,
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
    write_records(&records);
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
    assert!(
        allocation_count <= allocation_count_budget,
        "{scenario} allocated {allocation_count} blocks; budget is {allocation_count_budget}"
    );
    assert!(
        allocated_bytes <= allocated_bytes_budget,
        "{scenario} allocated {allocated_bytes} bytes; budget is {allocated_bytes_budget}"
    );
    records.push(AllocationRecord {
        scenario,
        iterations,
        allocation_count,
        allocated_bytes,
        allocation_count_budget,
        allocated_bytes_budget,
    });
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

fn write_records(records: &[AllocationRecord]) {
    let Some(path) = std::env::var_os("ICE_EDITOR_PERF_JSONL") else {
        for record in records {
            eprintln!(
                "{}: {} allocations, {} bytes",
                record.scenario, record.allocation_count, record.allocated_bytes
            );
        }
        return;
    };
    let path = PathBuf::from(path);
    let mut output = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap_or_else(|error| panic!("failed to open {}: {error}", path.display()));
    for record in records {
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
    }
}
