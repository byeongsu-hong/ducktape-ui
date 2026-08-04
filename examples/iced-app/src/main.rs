ui_lang::include_app!("src/ui/tasks.ice");

#[cfg(test)]
mod alignment;
#[cfg(test)]
mod background_gradient;
#[cfg(test)]
mod border_radius;
#[cfg(test)]
mod color;
#[cfg(test)]
mod content_fit;
#[cfg(test)]
mod event_status;
#[cfg(test)]
mod font_values;
#[cfg(test)]
mod lazy_context;
#[cfg(test)]
mod length;
#[cfg(test)]
mod mouse_interaction;
#[cfg(test)]
mod redraw_request;
#[cfg(test)]
mod resizable_panes;
#[cfg(test)]
mod rotation;
#[cfg(test)]
mod scroll_delta;
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

mod backend;

fn main() -> iced::Result {
    Tasks::run()
}

#[cfg(test)]
mod tests;
