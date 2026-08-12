pub fn describe_window(window: &dyn iced::window::Window, prefix: String) -> String {
    format!("{prefix}: raw-handle={}", window.window_handle().is_ok())
}

pub fn status_shader(speed: f64) -> StatusShader {
    StatusShader { speed }
}

pub struct StatusShader {
    speed: f64,
}

#[derive(Debug)]
pub struct StatusPrimitive {
    phase: f32,
}

pub struct StatusPipeline;

impl iced::widget::shader::Pipeline for StatusPipeline {
    fn new(
        _device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        _format: iced::wgpu::TextureFormat,
    ) -> Self {
        Self
    }
}

impl iced::widget::shader::Primitive for StatusPrimitive {
    type Pipeline = StatusPipeline;

    fn prepare(
        &self,
        _pipeline: &mut Self::Pipeline,
        _device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        _bounds: &iced::Rectangle,
        _viewport: &iced::widget::shader::Viewport,
    ) {
        let _ = self.phase;
    }
}

impl iced::widget::shader::Program<bool> for StatusShader {
    type State = bool;
    type Primitive = StatusPrimitive;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Option<iced::widget::shader::Action<bool>> {
        let hovered = match event {
            iced::Event::Mouse(iced::mouse::Event::CursorEntered) => true,
            iced::Event::Mouse(iced::mouse::Event::CursorLeft) => false,
            _ => return None,
        };
        *state = hovered;
        Some(iced::widget::shader::Action::publish(hovered))
    }

    fn draw(
        &self,
        state: &Self::State,
        _cursor: iced::mouse::Cursor,
        _bounds: iced::Rectangle,
    ) -> Self::Primitive {
        StatusPrimitive {
            phase: (self.speed as f32) + f32::from(*state),
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        iced::mouse::Interaction::Pointer
    }
}
