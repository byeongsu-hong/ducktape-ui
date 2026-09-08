//! Actual wasm QR payloads compared with native QR output at the same coordinates.
use super::layers_tests::{Ui, build, click, container_bounds, redraw, renderer};
use super::*;
use iced::advanced::renderer::Headless;
use iced::widget::qr_code::{Data, ErrorCorrection, Style, Version};
use iced::{Color, Rectangle, Size, mouse};
use iced_test::runtime::{UserInterface, user_interface};

fn guest() -> Arc<Mutex<Guest>> {
    use sha2::{Digest, Sha256};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/qr-fixture/app_store_qr_fixture.wasm");
    let bytes = std::fs::read(&path).expect("bundle QR fixture first");
    let guest = Guest::load(&CatalogEntry {
        preferred_size: None,
        id: "qr-fixture".into(),
        name: "QR fixture".into(),
        description: String::new(),
        capabilities: vec![],
        path: path.to_string_lossy().into_owned(),
        mark: "Q".into(),
        hash: Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    })
    .unwrap();
    Arc::new(Mutex::new(guest))
}
fn key(node: &wire::Node, suffix: &str) -> Option<String> {
    if let wire::Node::Container { key, .. } = node
        && key.ends_with(suffix)
    {
        return Some(key.clone());
    }
    node.children().iter().find_map(|node| key(node, suffix))
}
fn draw(ui: &mut Ui, renderer: &mut iced::Renderer) -> Vec<u8> {
    ui.draw(
        renderer,
        &iced::Theme::Light,
        &iced::advanced::renderer::Style {
            text_color: Color::BLACK,
        },
        mouse::Cursor::Unavailable,
    );
    renderer.screenshot(Size::new(600, 600), 1.0, Color::WHITE)
}
fn crop(pixels: &[u8], rect: Rectangle) -> Vec<u8> {
    let mut out = vec![];
    for y in rect.y.floor() as usize..(rect.y + rect.height).ceil().min(600.0) as usize {
        let x = rect.x.floor() as usize;
        let end = (rect.x + rect.width).ceil().min(600.0) as usize;
        out.extend_from_slice(&pixels[(y * 600 + x) * 4..(y * 600 + end) * 4]);
    }
    out
}
fn reference(name: &str, payload: &str, rect: Rectangle, renderer: &mut iced::Renderer) -> Vec<u8> {
    let data = match name {
        "normal" => Data::with_error_correction(payload, ErrorCorrection::Medium),
        "fixed" => Data::with_version(payload, Version::Normal(4), ErrorCorrection::High),
        "micro" => Data::with_version([0, 255, 164], Version::Micro(4), ErrorCorrection::Low),
        _ => unreachable!(),
    }
    .unwrap();
    let mut qr = ui_lang_runtime::qr_code(Some(data));
    qr = match name {
        "normal" => qr.cell_size(4.0),
        "fixed" => qr.total_size(164.0),
        _ => qr.cell_size(3.0),
    };
    let cell = if name == "fixed" {
        Color::from_rgb8(0, 170, 68)
    } else {
        Color::BLACK
    };
    qr = qr.style(move |_| Style {
        cell,
        background: Color::WHITE,
    });
    let content = iced::widget::pin(qr).x(rect.x).y(rect.y);
    let mut ui: Ui = UserInterface::build(
        content,
        Size::new(600.0, 600.0),
        user_interface::Cache::default(),
        renderer,
    );
    crop(&draw(&mut ui, renderer), rect)
}

#[test]
#[ignore = "requires bundled qr-fixture wasm"]
fn bundled_qr_pixels_match_native_data_and_change_with_guest_payload() {
    let guest = guest();
    let mut renderer = renderer();
    let mut ui = build(
        &guest,
        user_interface::Cache::default(),
        &mut renderer,
        600.0,
    );
    let mut now = Instant::now();
    for _ in 0..4 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    let mut first_matrix = None;
    for payload in ["HELLO WORLD", "DUCKTAPE INVITE"] {
        let pixels = draw(&mut ui, &mut renderer);
        if payload == "HELLO WORLD"
            && let Ok(path) = std::env::var("ICE_QR_CAPTURE")
        {
            std::fs::write(path, &pixels).unwrap();
        }
        for name in ["normal", "fixed", "micro"] {
            let key = key(
                guest.lock().unwrap().frame.root.as_ref().unwrap(),
                &format!("/{name}"),
            )
            .unwrap();
            let rect = container_bounds(&mut ui, &renderer, &key);
            assert!(
                rect.width > 20.0 && rect.height > 20.0,
                "QR {name} must have a native matrix: {rect:?}"
            );
            let actual = crop(&pixels, rect);
            let expected = reference(name, payload, rect, &mut renderer);
            assert!(
                actual == expected,
                "QR {name} must preserve native payload, version, correction, size and colors"
            );
            if name == "normal" {
                if let Some(first) = &first_matrix {
                    assert!(*first != actual, "guest payload changes the encoded matrix");
                } else {
                    first_matrix = Some(actual);
                }
            }
        }
        if payload == "HELLO WORLD" {
            click(&mut ui, &mut renderer, "Change");
            for _ in 0..3 {
                ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
            }
        }
    }
    click(&mut ui, &mut renderer, "Overflow");
    for _ in 0..3 {
        ui = redraw(ui, &guest, &mut renderer, &mut now, 600.0);
    }
    for name in ["normal", "fixed"] {
        let key = key(
            guest.lock().unwrap().frame.root.as_ref().unwrap(),
            &format!("/{name}"),
        )
        .unwrap();
        let rect = container_bounds(&mut ui, &renderer, &key);
        assert_eq!(
            rect.size(),
            Size::ZERO,
            "oversized data must not encode a truncated QR"
        );
    }
}
