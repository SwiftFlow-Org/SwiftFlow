use bytemuck::{Pod, Zeroable};
use swiftflow_core::ffi::SCALE;
use swiftflow_core::{with_font_system, DrawCommand, DrawItem, SFClip};

fn clip_rect_of(clip: SFClip) -> [f32; 4] {
    [clip.rect.x, clip.rect.y, clip.rect.width, clip.rect.height]
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GlyphInstance {
    rect: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],

    clip_rect: [f32; 4],
    clip_radii: [f32; 4],

    blur: [f32; 4],
}

const MAX_GLYPHS: usize = 16384;

pub struct TextRenderer {
    pipeline: wgpu::RenderPipeline,
    atlas_texture: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    atlas_sampler: wgpu::Sampler,
    atlas_bg: wgpu::BindGroup,
    atlas_bgl: wgpu::BindGroupLayout,
    instance_buffer: wgpu::Buffer,
    pub instances: Vec<GlyphInstance>,
    atlas_dirty: bool,
}

impl TextRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        globals_bgl: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Self {
        let atlas_size = 2048u32;

        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("font_atlas"),
            size: wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let atlas_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atlas_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let atlas_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas_bg"),
            layout: &atlas_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glyph_instances"),
            size: (MAX_GLYPHS * std::mem::size_of::<GlyphInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("text_shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text_pipeline_layout"),
            bind_group_layouts: &[globals_bgl, &atlas_bgl],
            push_constant_ranges: &[],
        });

        let glyph_attrs = wgpu::vertex_attr_array![
            0 => Float32x4,
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
        ];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GlyphInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &glyph_attrs,
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &vec![0u8; (atlas_size * atlas_size * 4) as usize],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(atlas_size * 4),
                rows_per_image: Some(atlas_size),
            },
            wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
        );

        Self {
            pipeline,
            atlas_texture,
            atlas_view,
            atlas_sampler,
            atlas_bg,
            atlas_bgl,
            instance_buffer,
            instances: Vec::with_capacity(MAX_GLYPHS),
            atlas_dirty: true,
        }
    }

    pub fn read_atlas_debug(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        let atlas_size = 2048u32;
        let bytes_per_row = atlas_size * 4;

        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atlas_readback"),
            size: (bytes_per_row * atlas_size) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(atlas_size),
                },
            },
            wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
        );

        queue.submit([encoder.finish()]);

        let slice = readback_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();

        let data = slice.get_mapped_range().to_vec();
        data
    }

    pub fn begin_frame(&mut self) {
        self.instances.clear();
    }

    pub fn prepare_run(
        &mut self,
        commands: &[DrawItem],
        queue: &wgpu::Queue,
    ) -> std::ops::Range<u32> {
        let start = self.instances.len();
        for item in commands {
            let clip = item.clip;
            let DrawCommand::Text {
                frame,
                content,
                font_size,
                render_scale,
                weight,
                family,
                blur,
                color,
            } = &item.command
            else {
                continue;
            };
            let scale = *SCALE.lock().unwrap();

            let raster_size = font_size * scale;
            let render_scale = *render_scale;
            let weight = *weight;
            let family = *family;
            let blur = *blur;

            let first_char = content.chars().next();
            let icon_run = first_char.is_some_and(swiftflow_core::is_icon);
            let baseline_y = if let (true, Some(c)) = (icon_run, first_char) {
                frame.y
                    + with_font_system(|fs| fs.ascender_for(c, raster_size, weight, family))
                        * render_scale
            } else {
                let font_ascender = with_font_system(|fs| fs.ascender(raster_size, family));
                frame.y + font_ascender * render_scale
            };
            let mut cursor_x = frame.x;

            for c in content.chars() {
                if c == ' ' {
                    cursor_x += with_font_system(|fs| fs.space_width(raster_size, weight, family))
                        * render_scale;
                    continue;
                }

                let glyph = with_font_system(|fs| fs.glyph(c, raster_size, weight, family));
                let Some(glyph) = glyph else { continue };

                let render_w = glyph.width * render_scale;
                let render_h = glyph.height * render_scale;
                let offset_x = glyph.offset_x * render_scale;
                let offset_y = glyph.offset_y * render_scale;
                let advance = glyph.advance * render_scale;

                let x = cursor_x + offset_x;
                let y = baseline_y - glyph.top * render_scale;

                let uv = [glyph.uv_x, glyph.uv_y, glyph.uv_width, glyph.uv_height];

                self.instances.push(GlyphInstance {
                    rect: [x, y, render_w, render_h],
                    uv,
                    color: [color.r, color.g, color.b, color.a],
                    clip_rect: clip_rect_of(clip),
                    clip_radii: clip.radii,
                    blur: [blur, render_w, render_h, 0.0],
                });

                cursor_x += advance;
            }
        }

        let end = self.instances.len();
        if end > start {
            queue.write_buffer(
                &self.instance_buffer,
                (start * std::mem::size_of::<GlyphInstance>()) as u64,
                bytemuck::cast_slice(&self.instances[start..end]),
            );
        }
        start as u32..end as u32
    }

    pub fn finish_frame(&mut self, queue: &wgpu::Queue) {
        self.upload_atlas(queue);
    }

    pub fn render_range<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        globals_bg: &'a wgpu::BindGroup,
        range: std::ops::Range<u32>,
    ) {
        if range.is_empty() {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, globals_bg, &[]);
        pass.set_bind_group(1, &self.atlas_bg, &[]);
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        pass.draw(0..6, range);
    }

    fn upload_atlas(&self, queue: &wgpu::Queue) {
        let atlas_size = 2048u32;
        let row_bytes = (atlas_size * 4) as usize;

        with_font_system(|fs| {
            let Some((y0, y1)) = fs.atlas_dirty_rows() else {
                return;
            };
            let start = y0 as usize * row_bytes;
            let end = (y1 as usize * row_bytes).min(fs.atlas.data.len());
            if end <= start {
                return;
            }

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: 0, y: y0, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                &fs.atlas.data[start..end],
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(atlas_size * 4),
                    rows_per_image: Some(y1 - y0),
                },
                wgpu::Extent3d {
                    width: atlas_size,
                    height: y1 - y0,
                    depth_or_array_layers: 1,
                },
            );
        });
    }
}
