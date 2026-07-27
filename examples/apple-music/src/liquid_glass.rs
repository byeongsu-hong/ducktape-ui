use iced::widget::shader::{self, Pipeline, Primitive, Program};
use iced::{Rectangle, Size, mouse};
use std::borrow::Cow;
use std::collections::HashMap;

pub fn liquid_glass(layer: i64, blur: f64, refraction: f64, tint: f64, radius: f64) -> LiquidGlass {
    LiquidGlass {
        layer,
        blur: blur.clamp(0.0, 32.0) as f32,
        refraction: refraction.clamp(0.0, 12.0) as f32,
        tint: tint.clamp(0.0, 1.0) as f32,
        radius: radius.clamp(0.0, 64.0) as f32,
    }
}

pub struct LiquidGlass {
    layer: i64,
    blur: f32,
    refraction: f32,
    tint: f32,
    radius: f32,
}

#[derive(Debug)]
pub struct GlassPrimitive {
    layer: i64,
    bounds: Rectangle,
    blur: f32,
    refraction: f32,
    tint: f32,
    radius: f32,
}

struct GlassSurface {
    size: Size<u32>,
    view: iced::wgpu::TextureView,
    uniform: iced::wgpu::Buffer,
    prepared: Prepared,
}

#[derive(Clone, Copy)]
struct Prepared {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

pub struct GlassPipeline {
    device: iced::wgpu::Device,
    format: iced::wgpu::TextureFormat,
    sampler: iced::wgpu::Sampler,
    layout: iced::wgpu::BindGroupLayout,
    glass: iced::wgpu::RenderPipeline,
    composite: iced::wgpu::RenderPipeline,
    surfaces: HashMap<i64, GlassSurface>,
}

impl GlassPipeline {
    fn bind_group(
        &self,
        view: &iced::wgpu::TextureView,
        uniform: &iced::wgpu::Buffer,
    ) -> iced::wgpu::BindGroup {
        self.device
            .create_bind_group(&iced::wgpu::BindGroupDescriptor {
                label: Some("apple_music liquid glass bind group"),
                layout: &self.layout,
                entries: &[
                    iced::wgpu::BindGroupEntry {
                        binding: 0,
                        resource: iced::wgpu::BindingResource::TextureView(view),
                    },
                    iced::wgpu::BindGroupEntry {
                        binding: 1,
                        resource: iced::wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    iced::wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform.as_entire_binding(),
                    },
                ],
            })
    }
}

impl GlassSurface {
    fn new(
        device: &iced::wgpu::Device,
        format: iced::wgpu::TextureFormat,
        size: Size<u32>,
    ) -> Self {
        let texture = device.create_texture(&iced::wgpu::TextureDescriptor {
            label: Some("apple_music liquid glass target"),
            size: iced::wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: iced::wgpu::TextureDimension::D2,
            format,
            usage: iced::wgpu::TextureUsages::RENDER_ATTACHMENT
                | iced::wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let uniform = device.create_buffer(&iced::wgpu::BufferDescriptor {
            label: Some("apple_music liquid glass uniform"),
            size: 48,
            usage: iced::wgpu::BufferUsages::UNIFORM | iced::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            size,
            view: texture.create_view(&iced::wgpu::TextureViewDescriptor::default()),
            uniform,
            prepared: Prepared {
                x: 0.0,
                y: 0.0,
                width: size.width as f32,
                height: size.height as f32,
            },
        }
    }
}

impl Pipeline for GlassPipeline {
    fn new(
        device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self {
        let layout = device.create_bind_group_layout(&iced::wgpu::BindGroupLayoutDescriptor {
            label: Some("apple_music liquid glass layout"),
            entries: &[
                iced::wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: iced::wgpu::ShaderStages::FRAGMENT,
                    ty: iced::wgpu::BindingType::Texture {
                        sample_type: iced::wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: iced::wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                iced::wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: iced::wgpu::ShaderStages::FRAGMENT,
                    ty: iced::wgpu::BindingType::Sampler(iced::wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                iced::wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: iced::wgpu::ShaderStages::FRAGMENT,
                    ty: iced::wgpu::BindingType::Buffer {
                        ty: iced::wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout =
            device.create_pipeline_layout(&iced::wgpu::PipelineLayoutDescriptor {
                label: Some("apple_music liquid glass pipeline layout"),
                bind_group_layouts: &[&layout],
                push_constant_ranges: &[],
            });
        let shader = device.create_shader_module(iced::wgpu::ShaderModuleDescriptor {
            label: Some("apple_music liquid glass shader"),
            source: iced::wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "liquid_glass.wgsl"
            ))),
        });
        let glass = render_pipeline(
            device,
            &pipeline_layout,
            &shader,
            format,
            "glass_fragment",
            None,
        );
        let composite = render_pipeline(
            device,
            &pipeline_layout,
            &shader,
            format,
            "composite_fragment",
            Some(iced::wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );
        let sampler = device.create_sampler(&iced::wgpu::SamplerDescriptor {
            label: Some("apple_music liquid glass sampler"),
            address_mode_u: iced::wgpu::AddressMode::ClampToEdge,
            address_mode_v: iced::wgpu::AddressMode::ClampToEdge,
            mag_filter: iced::wgpu::FilterMode::Linear,
            min_filter: iced::wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            device: device.clone(),
            format,
            sampler,
            layout,
            glass,
            composite,
            surfaces: HashMap::new(),
        }
    }
}

impl Primitive for GlassPrimitive {
    type Pipeline = GlassPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        _bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        let scale = viewport.scale_factor();
        let size = Size::new(
            (self.bounds.width * scale).ceil().max(1.0) as u32,
            (self.bounds.height * scale).ceil().max(1.0) as u32,
        );

        if pipeline
            .surfaces
            .get(&self.layer)
            .is_none_or(|surface| surface.size != size)
        {
            pipeline
                .surfaces
                .insert(self.layer, GlassSurface::new(device, pipeline.format, size));
        }

        let surface = pipeline
            .surfaces
            .get_mut(&self.layer)
            .expect("glass surface was just prepared");
        surface.prepared = Prepared {
            x: self.bounds.x * scale,
            y: self.bounds.y * scale,
            width: size.width as f32,
            height: size.height as f32,
        };
        let screen = viewport.physical_size();
        let values = [
            surface.prepared.x,
            surface.prepared.y,
            screen.width as f32,
            screen.height as f32,
            surface.prepared.width,
            surface.prepared.height,
            self.radius * scale,
            self.blur * scale,
            self.refraction * scale,
            self.tint,
            0.0,
            0.0,
        ];
        queue.write_buffer(&surface.uniform, 0, &uniform_bytes(values));
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut iced::wgpu::CommandEncoder,
        backdrop: &iced::wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        let Some(surface) = pipeline.surfaces.get(&self.layer) else {
            return;
        };
        let backdrop_group = pipeline.bind_group(backdrop, &surface.uniform);

        {
            let mut pass = encoder.begin_render_pass(&iced::wgpu::RenderPassDescriptor {
                label: Some("apple_music liquid glass pass"),
                color_attachments: &[Some(iced::wgpu::RenderPassColorAttachment {
                    view: &surface.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: iced::wgpu::Operations {
                        load: iced::wgpu::LoadOp::Clear(iced::wgpu::Color::TRANSPARENT),
                        store: iced::wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&pipeline.glass);
            pass.set_bind_group(0, &backdrop_group, &[]);
            pass.draw(0..3, 0..1);
        }

        let glass_group = pipeline.bind_group(&surface.view, &surface.uniform);
        let mut pass = encoder.begin_render_pass(&iced::wgpu::RenderPassDescriptor {
            label: Some("apple_music liquid glass composite"),
            color_attachments: &[Some(iced::wgpu::RenderPassColorAttachment {
                view: backdrop,
                depth_slice: None,
                resolve_target: None,
                ops: iced::wgpu::Operations {
                    load: iced::wgpu::LoadOp::Load,
                    store: iced::wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_viewport(
            surface.prepared.x,
            surface.prepared.y,
            surface.prepared.width,
            surface.prepared.height,
            0.0,
            1.0,
        );
        pass.set_scissor_rect(
            clip_bounds.x,
            clip_bounds.y,
            clip_bounds.width,
            clip_bounds.height,
        );
        pass.set_pipeline(&pipeline.composite);
        pass.set_bind_group(0, &glass_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl Program<()> for LiquidGlass {
    type State = ();
    type Primitive = GlassPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        GlassPrimitive {
            layer: self.layer,
            bounds,
            blur: self.blur,
            refraction: self.refraction,
            tint: self.tint,
            radius: self.radius,
        }
    }
}

fn render_pipeline(
    device: &iced::wgpu::Device,
    layout: &iced::wgpu::PipelineLayout,
    shader: &iced::wgpu::ShaderModule,
    format: iced::wgpu::TextureFormat,
    entry_point: &'static str,
    blend: Option<iced::wgpu::BlendState>,
) -> iced::wgpu::RenderPipeline {
    device.create_render_pipeline(&iced::wgpu::RenderPipelineDescriptor {
        label: Some(entry_point),
        layout: Some(layout),
        vertex: iced::wgpu::VertexState {
            module: shader,
            entry_point: Some("vertex"),
            buffers: &[],
            compilation_options: iced::wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(iced::wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry_point),
            targets: &[Some(iced::wgpu::ColorTargetState {
                format,
                blend,
                write_mask: iced::wgpu::ColorWrites::ALL,
            })],
            compilation_options: iced::wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: iced::wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: iced::wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn uniform_bytes(values: [f32; 12]) -> [u8; 48] {
    let mut bytes = [0; 48];
    for (chunk, value) in bytes.chunks_exact_mut(4).zip(values) {
        chunk.copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_effect_settings_before_they_reach_the_gpu() {
        let glass = liquid_glass(7, -1.0, 99.0, 2.0, 100.0);
        assert_eq!(glass.layer, 7);
        assert_eq!(
            (glass.blur, glass.refraction, glass.tint, glass.radius),
            (0.0, 12.0, 1.0, 64.0)
        );
    }
}
