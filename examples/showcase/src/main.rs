// A Rust binary is a console program by default, so an installed release build
// would open a terminal behind the window. Debug builds keep the console,
// which is where `cargo run` prints.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod adapters;
#[cfg(test)]
mod frame_probe;

ui_lang::include_app!("src/ui/app.ice");

#[cfg(test)]
mod native_fixtures;

#[cfg(test)]
mod alignment;
#[cfg(test)]
mod background_gradient;
#[cfg(test)]
mod border_radius;
#[cfg(test)]
mod button_status_children;
#[cfg(test)]
mod canvas_text_offset;
#[cfg(test)]
mod color;
#[cfg(test)]
mod content_fit;
#[cfg(test)]
mod event_status;
#[cfg(test)]
mod focus_visible;
#[cfg(test)]
mod font_values;
#[cfg(test)]
mod keyboard_filter;
#[cfg(test)]
mod lazy_cheap_keys;
#[cfg(test)]
mod lazy_cheap_keys_keyed;
#[cfg(test)]
mod lazy_cheap_keys_prop;
#[cfg(test)]
mod lazy_component_state;
#[cfg(test)]
mod lazy_context;
#[cfg(test)]
mod lazy_enum_keys;
#[cfg(test)]
mod lazy_extra_deps;
#[cfg(test)]
mod length;
#[cfg(test)]
mod mouse_interaction;
#[cfg(test)]
mod redraw_request;
#[cfg(test)]
mod resizable_panes;
#[cfg(test)]
mod rich_text_for;
#[cfg(test)]
mod rotation;
#[cfg(test)]
mod scroll_delta;
#[cfg(test)]
mod secret_input;
#[cfg(test)]
mod shadow;
#[cfg(test)]
mod text_values;
#[cfg(test)]
mod theme_mode;
#[cfg(test)]
mod window_id;
#[cfg(test)]
mod window_position;
#[cfg(test)]
mod window_screenshot;
#[cfg(test)]
mod window_values;

#[cfg(test)]
mod backend;

fn main() -> iced::Result {
    Showcase::run()
}

#[cfg(test)]
mod smoke_tests {
    use super::{__ShowcaseMessage, Showcase};

    #[test]
    fn showcase_boots_with_default_component_state() {
        let (mut showcase, _) = Showcase::__boot();

        assert_eq!(showcase.clicks, 0);
        assert!(!showcase.accepted);
        assert!(showcase.notifications);
        assert!(!showcase.dialog_open);

        let _ = showcase.__update(__ShowcaseMessage::Clicked);
        assert_eq!(showcase.clicks, 1);
        let _ = showcase.__update(__ShowcaseMessage::CancelDialog);
        assert_eq!(showcase.dialog_result, "cancelled");
    }
}
