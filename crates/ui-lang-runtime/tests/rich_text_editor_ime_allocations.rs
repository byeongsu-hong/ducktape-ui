use iced::advanced::clipboard;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, Widget, input_method, layout, mouse, widget};
use iced::widget::text_editor::Content;
use iced::{Event, Font, Length, Pixels, Point, Rectangle, Size, Theme};
use stats_alloc::Region;
use ui_lang_runtime::{ContentVersion, RichTextEditor, rich_text_editor::Action};

mod common;
use common::GLOBAL;

#[test]
#[ignore = "rich-text editor allocation contract run explicitly in CI"]
fn performance_contract_preedit_moves_the_parsed_display_line_map() {
    const LINES: usize = 4_096;
    // Budgets, not exact pins: the preedit glyph is shaped against the host's
    // fonts, so the count moves a little per machine (1,212 allocations /
    // 564,355 bytes on the dev box, 1,214 / 578,051 on CI's ubuntu). Cloning the
    // parsed display line map instead of moving it costs 1,415 / 624,767 —
    // above both budgets, which is the regression this contract exists to catch.
    const ALLOCATION_BUDGET: usize = 1_300;
    const ALLOCATED_BYTES_BUDGET: usize = 600_000;

    let source = (0..LINES)
        .map(|index| format!("line {index}\n"))
        .collect::<String>();
    let content = Content::with_text(&source);
    let mut editor = RichTextEditor::new(&content, ContentVersion::new(1, 0))
        .width(Length::Fixed(800.0))
        .height(Length::Fixed(600.0))
        .on_action(|action| action);
    let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
    let renderer = iced_test::futures::futures::executor::block_on(
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")),
    )
    .expect("headless renderer");
    let limits = layout::Limits::new(Size::ZERO, Size::new(800.0, 600.0));
    let viewport = Rectangle::with_size(Size::new(800.0, 600.0));
    let mut node = editor.layout(&mut tree, &renderer, &limits);
    let mut clipboard = clipboard::Null;
    let mut messages: Vec<Action> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    editor.update(
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(10.0, 10.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
        &viewport,
    );
    drop(shell);
    messages.clear();
    let preedit = Event::InputMethod(input_method::Event::Preedit("응".into(), Some(3..3)));

    let region = Region::new(GLOBAL);
    let mut shell = Shell::new(&mut messages);
    editor.update(
        &mut tree,
        &preedit,
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &renderer,
        &mut clipboard,
        &mut shell,
        &viewport,
    );
    assert!(shell.is_layout_invalid());
    shell.revalidate_layout(|| {
        node = editor.layout(&mut tree, &renderer, &limits);
    });
    drop(shell);
    let stats = region.change();

    eprintln!(
        "{LINES}-line preedit: allocations={} bytes={}",
        stats.allocations, stats.bytes_allocated
    );
    assert!(messages.is_empty());
    assert!(
        stats.allocations <= ALLOCATION_BUDGET,
        "preedit allocated {} times (budget {ALLOCATION_BUDGET})",
        stats.allocations
    );
    assert!(
        stats.bytes_allocated <= ALLOCATED_BYTES_BUDGET,
        "preedit allocated {} bytes (budget {ALLOCATED_BYTES_BUDGET})",
        stats.bytes_allocated
    );
}
