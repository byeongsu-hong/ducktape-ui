ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    ViewerFixture,
    "Viewer fixture",
    "Native copied-image zoom and pan",
    []
);

pub fn photo() -> iced::advanced::image::Handle {
    let colors = [
        [220, 30, 40, 255],
        [20, 180, 60, 255],
        [30, 70, 220, 255],
        [240, 190, 20, 255],
    ];
    let pixels: Vec<u8> = (0..8)
        .flat_map(|y| (0..8).flat_map(move |x| colors[(x + 2 * y) % 4]))
        .collect();
    iced::advanced::image::Handle::from_rgba(8, 8, pixels)
}
