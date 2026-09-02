use crate::core::{Color, Rectangle, Size};
use crate::graphics::compositor::{self, Information};
use crate::graphics::damage;
use crate::graphics::error::{self, Error};
use crate::graphics::{self, Shell, Viewport};
use crate::{Layer, Renderer, Settings};

use std::collections::VecDeque;
use std::num::NonZeroU32;

pub struct Compositor {
    context: softbuffer::Context<Box<dyn compositor::Display>>,
    settings: Settings,
}

pub struct Surface {
    window: softbuffer::Surface<
        Box<dyn compositor::Display>,
        Box<dyn compositor::Window>,
    >,
    clip_mask: tiny_skia::Mask,
    layer_stack: VecDeque<Vec<Layer>>,
    background_color: Color,
    max_age: u8,
}

impl crate::graphics::Compositor for Compositor {
    type Renderer = Renderer;
    type Surface = Surface;

    async fn with_backend(
        settings: graphics::Settings,
        display: impl compositor::Display,
        _compatible_window: impl compositor::Window,
        _shell: Shell,
        backend: Option<&str>,
    ) -> Result<Self, Error> {
        match backend {
            None | Some("tiny-skia") | Some("tiny_skia") => {
                Ok(new(settings.into(), display))
            }
            Some(backend) => Err(Error::GraphicsAdapterNotFound {
                backend: "tiny-skia",
                reason: error::Reason::DidNotMatch {
                    preferred_backend: backend.to_owned(),
                },
            }),
        }
    }

    fn create_renderer(&self) -> Self::Renderer {
        Renderer::new(
            self.settings.default_font,
            self.settings.default_text_size,
        )
    }

    fn create_surface<W: compositor::Window + Clone>(
        &mut self,
        window: W,
        width: u32,
        height: u32,
    ) -> Self::Surface {
        let window = softbuffer::Surface::new(
            &self.context,
            Box::new(window.clone()) as _,
        )
        .expect("Create softbuffer surface for window");

        let mut surface = Surface {
            window,
            clip_mask: tiny_skia::Mask::new(1, 1).expect("Create clip mask"),
            layer_stack: VecDeque::new(),
            background_color: Color::BLACK,
            max_age: 0,
        };

        if width > 0 && height > 0 {
            self.configure_surface(&mut surface, width, height);
        }

        surface
    }

    fn configure_surface(
        &mut self,
        surface: &mut Self::Surface,
        width: u32,
        height: u32,
    ) {
        surface
            .window
            .resize(
                NonZeroU32::new(width).expect("Non-zero width"),
                NonZeroU32::new(height).expect("Non-zero height"),
            )
            .expect("Resize surface");

        surface.clip_mask =
            tiny_skia::Mask::new(width, height).expect("Create clip mask");
        surface.layer_stack.clear();
    }

    fn information(&self) -> Information {
        Information {
            adapter: String::from("CPU"),
            backend: String::from("tiny-skia"),
        }
    }

    fn present(
        &mut self,
        renderer: &mut Self::Renderer,
        surface: &mut Self::Surface,
        viewport: &Viewport,
        background_color: Color,
        on_pre_present: impl FnOnce(),
    ) -> Result<(), compositor::SurfaceError> {
        present(
            renderer,
            surface,
            viewport,
            background_color,
            on_pre_present,
        )
    }

    fn screenshot(
        &mut self,
        renderer: &mut Self::Renderer,
        viewport: &Viewport,
        background_color: Color,
    ) -> Vec<u8> {
        screenshot(renderer, viewport, background_color)
    }
}

pub fn new(
    settings: Settings,
    display: impl compositor::Display,
) -> Compositor {
    #[allow(unsafe_code)]
    let context = softbuffer::Context::new(Box::new(display) as _)
        .expect("Create softbuffer context");

    Compositor { context, settings }
}

pub fn present(
    renderer: &mut Renderer,
    surface: &mut Surface,
    viewport: &Viewport,
    background_color: Color,
    on_pre_present: impl FnOnce(),
) -> Result<(), compositor::SurfaceError> {
    let physical_size = viewport.physical_size();

    let mut buffer = surface
        .window
        .buffer_mut()
        .map_err(|_| compositor::SurfaceError::Lost)?;

    let age = buffer.age();
    surface.max_age = surface.max_age.max(age);
    surface.layer_stack.truncate(surface.max_age as usize);

    let bounds = Rectangle::with_size(viewport.logical_size());
    let mut damage_since = |layers: Option<&Vec<Layer>>| {
        layers
            .and_then(|layers| {
                (surface.background_color == background_color).then(|| {
                    damage::diff(
                        layers,
                        renderer.layers(),
                        |layer| vec![layer.bounds],
                        Layer::damage,
                    )
                })
            })
            .unwrap_or_else(|| vec![bounds])
    };

    // The buffer holds the frame from `age` presents ago; the display shows
    // the last one presented. They are the same buffer on X11 (age 1) and
    // never on Wayland, which hands out the back buffer (age 2).
    let damage = damage_since(match age {
        0 => None,
        age => surface.layer_stack.get(age as usize - 1),
    });
    let on_screen = if age == 1 {
        damage.clone()
    } else {
        damage_since(surface.layer_stack.front())
    };

    // The display already shows this frame. Presenting would copy the whole
    // window to the display server for no pixel — and a multi-window program
    // redraws every window after every update.
    if on_screen.is_empty() {
        return Ok(());
    }

    surface.layer_stack.push_front(renderer.layers().to_vec());
    surface.background_color = background_color;

    let damage = damage::group(damage, bounds);

    let mut pixels = tiny_skia::PixmapMut::from_bytes(
        bytemuck::cast_slice_mut(&mut buffer),
        physical_size.width,
        physical_size.height,
    )
    .expect("Create pixel map");

    renderer.draw(
        &mut pixels,
        &mut surface.clip_mask,
        viewport,
        &damage,
        background_color,
    );

    // Only the pixels that differ from the display cross to it: a backend
    // that can (X11 with shared memory, Wayland) puts each rectangle alone.
    let on_screen = physical_rects(
        &damage::group(on_screen, bounds),
        viewport.scale_factor() as f32,
        physical_size,
    );

    on_pre_present();
    if on_screen.is_empty() {
        buffer.present()
    } else {
        buffer.present_with_damage(&on_screen)
    }
    .map_err(|_| compositor::SurfaceError::Lost)
}

/// Snaps logical regions outward to whole physical pixels, dropping any that
/// end up empty and cutting them to the buffer so the backend never sees a
/// rectangle past its edge.
fn physical_rects(
    regions: &[Rectangle],
    scale: f32,
    size: Size<u32>,
) -> Vec<softbuffer::Rect> {
    regions
        .iter()
        .filter_map(|region| {
            let x0 = (region.x * scale).floor().max(0.0) as u32;
            let y0 = (region.y * scale).floor().max(0.0) as u32;
            let x1 = (((region.x + region.width) * scale).ceil() as u32)
                .min(size.width);
            let y1 = (((region.y + region.height) * scale).ceil() as u32)
                .min(size.height);

            Some(softbuffer::Rect {
                x: x0,
                y: y0,
                width: NonZeroU32::new(x1.saturating_sub(x0))?,
                height: NonZeroU32::new(y1.saturating_sub(y0))?,
            })
        })
        .collect()
}

pub fn screenshot(
    renderer: &mut Renderer,
    viewport: &Viewport,
    background_color: Color,
) -> Vec<u8> {
    let size = viewport.physical_size();

    let mut offscreen_buffer: Vec<u32> =
        vec![0; size.width as usize * size.height as usize];

    let mut clip_mask = tiny_skia::Mask::new(size.width, size.height)
        .expect("Create clip mask");

    renderer.draw(
        &mut tiny_skia::PixmapMut::from_bytes(
            bytemuck::cast_slice_mut(&mut offscreen_buffer),
            size.width,
            size.height,
        )
        .expect("Create offscreen pixel map"),
        &mut clip_mask,
        viewport,
        &[Rectangle::with_size(Size::new(
            size.width as f32,
            size.height as f32,
        ))],
        background_color,
    );

    offscreen_buffer.iter().fold(
        Vec::with_capacity(offscreen_buffer.len() * 4),
        |mut acc, pixel| {
            const A_MASK: u32 = 0xFF_00_00_00;
            const R_MASK: u32 = 0x00_FF_00_00;
            const G_MASK: u32 = 0x00_00_FF_00;
            const B_MASK: u32 = 0x00_00_00_FF;

            let a = ((A_MASK & pixel) >> 24) as u8;
            let r = ((R_MASK & pixel) >> 16) as u8;
            let g = ((G_MASK & pixel) >> 8) as u8;
            let b = (B_MASK & pixel) as u8;

            acc.extend([r, g, b, a]);
            acc
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_rects_snap_outward_clip_and_drop_empty() {
        let rects = physical_rects(
            &[
                Rectangle::new((10.2, 3.0).into(), (5.5, 4.0).into()),
                Rectangle::new((95.0, 0.0).into(), (20.0, 10.0).into()),
                Rectangle::new((150.0, 0.0).into(), (1.0, 1.0).into()),
            ],
            1.5,
            Size::new(160, 30),
        );

        let rects: Vec<_> = rects
            .iter()
            .map(|rect| (rect.x, rect.y, rect.width.get(), rect.height.get()))
            .collect();
        assert_eq!(
            rects,
            vec![(15, 4, 9, 7), (142, 0, 18, 15)],
            "15.3..23.55 spans 15..24, 142.5..172.5 clips at 160, 225 is past the edge"
        );
    }
}
