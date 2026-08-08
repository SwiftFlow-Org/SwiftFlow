mod text;
use text::TextRenderer;

use bytemuck::{Pod, Zeroable};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use swiftflow_core::ffi::sf_init;
use swiftflow_core::node::SFContentMode;
use swiftflow_core::{
    register_backend, sflog, DrawCommand, DrawItem, DrawList, MergedMember, SFBackend, SFBorder,
    SFClip, SFColor, SFShadow,
};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct RectInstance {
    rect: [f32; 4],
    fill: [f32; 4],
    border_color: [f32; 4],
    params: [f32; 4],

    shadow: [f32; 2],
    _pad: [f32; 2],

    clip_rect: [f32; 4],
    clip_radii: [f32; 4],
}

impl RectInstance {
    fn new(
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        fill: SFColor,
        border: SFBorder,
        corner_radius: f32,
        specular: bool,
        shadow: SFShadow,
        clip: SFClip,
    ) -> Self {
        Self {
            rect: [x, y, w, h],
            fill: [fill.r, fill.g, fill.b, fill.a],
            border_color: [
                border.color.r,
                border.color.g,
                border.color.b,
                border.color.a,
            ],
            params: [
                corner_radius,
                border.width,
                if specular { 1.0 } else { 0.0 },
                0.0,
            ],
            shadow: [shadow.radius, shadow.opacity],
            _pad: [0.0, 0.0],
            clip_rect: clip_rect_of(clip),
            clip_radii: clip.radii,
        }
    }
}

fn clip_rect_of(clip: SFClip) -> [f32; 4] {
    [clip.rect.x, clip.rect.y, clip.rect.width, clip.rect.height]
}

const MAX_MERGE_MEMBERS: usize = 4;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct MergedRectInstance {
    bounds: [f32; 4],
    meta: [f32; 4],
    member_rect: [[f32; 4]; MAX_MERGE_MEMBERS],
    member_fill: [[f32; 4]; MAX_MERGE_MEMBERS],
    member_border_color: [[f32; 4]; MAX_MERGE_MEMBERS],
    member_params: [[f32; 4]; MAX_MERGE_MEMBERS],
    clip_rect: [f32; 4],
    clip_radii: [f32; 4],
}

impl MergedRectInstance {
    fn new(
        bounds: [f32; 4],
        blend_k: f32,
        color_blend_k: f32,
        members: &[MergedMember; 4],
        count: u8,
        clip: SFClip,
    ) -> Self {
        let mut member_rect = [[0.0; 4]; MAX_MERGE_MEMBERS];
        let mut member_fill = [[0.0; 4]; MAX_MERGE_MEMBERS];
        let mut member_border_color = [[0.0; 4]; MAX_MERGE_MEMBERS];
        let mut member_params = [[0.0; 4]; MAX_MERGE_MEMBERS];

        for (i, m) in members.iter().enumerate() {
            member_rect[i] = [m.frame.x, m.frame.y, m.frame.width, m.frame.height];
            member_fill[i] = [m.fill.r, m.fill.g, m.fill.b, m.fill.a];
            member_border_color[i] = [
                m.border.color.r,
                m.border.color.g,
                m.border.color.b,
                m.border.color.a,
            ];
            member_params[i] = [
                m.corner_radius,
                m.border.width,
                if m.specular { 1.0 } else { 0.0 },
                0.0,
            ];
        }

        Self {
            bounds,
            meta: [count as f32, blend_k, color_blend_k, 0.0],
            member_rect,
            member_fill,
            member_border_color,
            member_params,
            clip_rect: clip_rect_of(clip),
            clip_radii: clip.radii,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct MaterialGpuParams {
    frame: [f32; 4],
    tint: [f32; 4],
    border_color: [f32; 4],
    params: [f32; 4],

    params2: [f32; 4],
    clip_rect: [f32; 4],
    clip_radii: [f32; 4],
}

impl MaterialGpuParams {
    fn new(
        frame: [f32; 4],
        corner_radius: f32,
        border: SFBorder,
        tint: SFColor,
        blur_radius: f32,
        specular: bool,
        progressive: f32,
        progressive_start: f32,
        glass_refraction: f32,
        glass_interactive: f32,
        clip: SFClip,
    ) -> Self {
        Self {
            clip_rect: clip_rect_of(clip),
            clip_radii: clip.radii,
            frame,
            tint: [tint.r, tint.g, tint.b, tint.a],
            border_color: [
                border.color.r,
                border.color.g,
                border.color.b,
                border.color.a,
            ],
            params: [
                corner_radius,
                border.width,
                blur_radius,
                if specular { 1.0 } else { 0.0 },
            ],
            params2: [progressive, progressive_start, glass_refraction, glass_interactive],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ImageGpuParams {
    frame: [f32; 4],
    uv: [f32; 4],
    tint: [f32; 4],
    params: [f32; 4],
    clip_rect: [f32; 4],
    clip_radii: [f32; 4],
}

impl ImageGpuParams {
    fn new(frame: [f32; 4], corner_radius: f32, tint: SFColor, uv: [f32; 4], clip: SFClip) -> Self {
        Self {
            clip_rect: clip_rect_of(clip),
            clip_radii: clip.radii,
            frame,
            uv,
            tint: [tint.r, tint.g, tint.b, tint.a],
            params: [corner_radius, 0.0, 0.0, 0.0],
        }
    }
}

fn content_mode_uv(
    mode: SFContentMode,
    image_w: u32,
    image_h: u32,
    frame_w: f32,
    frame_h: f32,
) -> [f32; 4] {
    const IDENTITY: [f32; 4] = [1.0, 1.0, 0.0, 0.0];
    if matches!(mode, SFContentMode::Stretch) {
        return IDENTITY;
    }
    if image_w == 0 || image_h == 0 || frame_w <= 0.0 || frame_h <= 0.0 {
        return IDENTITY;
    }

    let ratio = (image_w as f32 / image_h as f32) / (frame_w / frame_h);

    let (scale_x, scale_y) = match (mode, ratio > 1.0) {
        (SFContentMode::Fit, true) => (1.0, ratio),
        (SFContentMode::Fit, false) => (1.0 / ratio, 1.0),
        (SFContentMode::Fill, true) => (1.0 / ratio, 1.0),
        (SFContentMode::Fill, false) => (1.0, ratio),
        (SFContentMode::Stretch, _) => return IDENTITY,
    };

    [
        scale_x,
        scale_y,
        (1.0 - scale_x) * 0.5,
        (1.0 - scale_y) * 0.5,
    ]
}

struct ImageTexture {
    width: u32,
    height: u32,
    bind_group: wgpu::BindGroup,
    _texture: wgpu::Texture,
}

const MAX_IMAGES: usize = 256;
const MAX_MATERIALS: usize = 64;

const BLUR_DOWNSCALE: u32 = 8;

pub fn pyramid_dims(width: u32, height: u32) -> [(u32, u32); 3] {
    let level = |n: u32| ((width / n).max(1), (height / n).max(1));
    [level(2), level(4), level(BLUR_DOWNSCALE)]
}

const BLUR_ITERATIONS: usize = 4;

enum Batch {
    Rects(std::ops::Range<usize>),
    Merged(std::ops::Range<usize>),
    Text(std::ops::Range<usize>),
    Material(usize),

    Image {
        index: usize,
        id: u32,
    },
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Globals {
    resolution: [f32; 2],
    _pad: [f32; 2],
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum SFSurfaceKind {
    MetalLayer = 0,
    RawHandle = 1,
}

#[repr(C)]
pub struct SFSurfaceDescriptor {
    pub kind: SFSurfaceKind,
    pub handle: *mut c_void,
    pub display_handle: *mut c_void,
}

struct WgpuBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,

    rect_pipeline: wgpu::RenderPipeline,
    globals_buffer: wgpu::Buffer,
    globals_bg: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    max_instances: usize,

    merged_pipeline: wgpu::RenderPipeline,
    merged_buffer: wgpu::Buffer,
    merged_bg: wgpu::BindGroup,
    max_merged_instances: usize,

    text_renderer: TextRenderer,

    scene_texture: wgpu::Texture,
    scene_view: wgpu::TextureView,

    pyr_half_texture: wgpu::Texture,
    pyr_half_view: wgpu::TextureView,
    pyr_quarter_texture: wgpu::Texture,
    pyr_quarter_view: wgpu::TextureView,
    blur_a_texture: wgpu::Texture,
    blur_a_view: wgpu::TextureView,
    blur_b_texture: wgpu::Texture,
    blur_b_view: wgpu::TextureView,

    sample_sampler: wgpu::Sampler,
    sample_bgl: wgpu::BindGroupLayout,
    glass_bgl: wgpu::BindGroupLayout,
    glass_bg: wgpu::BindGroup,
    scene_sample_bg: wgpu::BindGroup,
    pyr_half_sample_bg: wgpu::BindGroup,
    pyr_quarter_sample_bg: wgpu::BindGroup,
    blur_a_sample_bg: wgpu::BindGroup,
    blur_b_sample_bg: wgpu::BindGroup,

    material_bgl: wgpu::BindGroupLayout,
    material_buffer: wgpu::Buffer,
    material_bg: wgpu::BindGroup,
    max_materials: usize,

    halve_pipeline: wgpu::RenderPipeline,
    blur_h_pipeline: wgpu::RenderPipeline,
    blur_v_pipeline: wgpu::RenderPipeline,
    material_pipeline: wgpu::RenderPipeline,
    present_pipeline: wgpu::RenderPipeline,

    image_pipeline: wgpu::RenderPipeline,
    image_bgl: wgpu::BindGroupLayout,
    image_buffer: wgpu::Buffer,
    image_textures: std::collections::HashMap<u32, ImageTexture>,
    max_images: usize,

    current_frame: Option<wgpu::SurfaceTexture>,
    rect_instances: Vec<RectInstance>,
    merged_instances: Vec<MergedRectInstance>,
    materials: Vec<MaterialGpuParams>,
    images: Vec<ImageGpuParams>,
    batches: Vec<Batch>,

    text_commands: Vec<DrawItem>,
}

const MAX_RECTS: usize = 4096;
const MAX_MERGED_INSTANCES: usize = 512;

enum SurfaceSource {

    #[cfg_attr(not(any(target_os = "macos", target_os = "ios")), allow(dead_code))]
    Raw(wgpu::SurfaceTargetUnsafe),
    Owned(wgpu::SurfaceTarget<'static>),
}

fn preferred_backends() -> wgpu::Backends {
    wgpu::Backends::PRIMARY
}

impl WgpuBackend {
    async fn new(source: SurfaceSource, width: u32, height: u32) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: preferred_backends(),
            ..Default::default()
        });

        let surface = match source {
            SurfaceSource::Raw(target) => unsafe {
                instance
                    .create_surface_unsafe(target)
                    .expect("could not create a surface from the supplied handle")
            },
            SurfaceSource::Owned(target) => instance
                .create_surface(target)
                .expect("could not create a surface from the supplied window"),
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await

            .expect(
                "no compatible GPU adapter found. SwiftFlow needs Vulkan (Linux/Windows), \
                 Metal (macOS/iOS) or DX12 (Windows); on Linux check that a Vulkan driver \
                 is installed and visible in /usr/share/vulkan/icd.d",
            );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        let caps = surface.get_capabilities(&adapter);
        sflog!(
            "surface caps: formats={:?} alpha={:?} present={:?}",
            caps.formats,
            caps.alpha_modes,
            caps.present_modes
        );
        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Bgra8Unorm)
            .or_else(|| caps.formats.iter().copied().find(|f| !f.is_srgb()))
            .or_else(|| caps.formats.first().copied())
            .expect("the surface reported no supported texture formats");

        let alpha_mode = caps
            .alpha_modes
            .iter()
            .copied()
            .find(|m| *m == wgpu::CompositeAlphaMode::Opaque)
            .unwrap_or(caps.alpha_modes[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let globals = Globals {
            resolution: [width as f32, height as f32],
            _pad: [0.0; 2],
        };
        let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("globals"),
            contents: bytemuck::bytes_of(&globals),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let globals_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let globals_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals_bg"),
            layout: &globals_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rect_instances"),
            size: (MAX_RECTS * std::mem::size_of::<RectInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rect_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rect_pipeline_layout"),
            bind_group_layouts: &[&globals_bgl],
            push_constant_ranges: &[],
        });

        let instance_attrs = wgpu::vertex_attr_array![
            0 => Float32x4,
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x2,
            5 => Float32x2,
            6 => Float32x4,
            7 => Float32x4,
        ];

        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rect_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &instance_attrs,
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        let merged_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("merged_instances"),
            size: (MAX_MERGED_INSTANCES * std::mem::size_of::<MergedRectInstance>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let merged_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("merged_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let merged_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("merged_bg"),
            layout: &merged_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: merged_buffer.as_entire_binding(),
            }],
        });

        let merged_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("merged_pipeline_layout"),
                bind_group_layouts: &[&globals_bgl, &merged_bgl],
                push_constant_ranges: &[],
            });

        let merged_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("merged_pipeline"),
            layout: Some(&merged_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_merged",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_merged",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        let text_renderer = TextRenderer::new(
            &device,
            &queue,
            &globals_bgl,
            wgpu::TextureFormat::Bgra8Unorm,
        );

        let (scene_texture, scene_view) = create_target(&device, width, height, "scene");
        let [(half_w, half_h), (quarter_w, quarter_h), (blur_w, blur_h)] =
            pyramid_dims(width, height);
        let (pyr_half_texture, pyr_half_view) = create_target(&device, half_w, half_h, "pyr_half");
        let (pyr_quarter_texture, pyr_quarter_view) =
            create_target(&device, quarter_w, quarter_h, "pyr_quarter");
        let (blur_a_texture, blur_a_view) = create_target(&device, blur_w, blur_h, "blur_a");
        let (blur_b_texture, blur_b_view) = create_target(&device, blur_w, blur_h, "blur_b");

        let sample_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let sample_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sample_bgl"),
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
        let scene_sample_bg = make_sample_bg(&device, &sample_bgl, &scene_view, &sample_sampler);
        let pyr_half_sample_bg =
            make_sample_bg(&device, &sample_bgl, &pyr_half_view, &sample_sampler);
        let pyr_quarter_sample_bg =
            make_sample_bg(&device, &sample_bgl, &pyr_quarter_view, &sample_sampler);
        let blur_a_sample_bg = make_sample_bg(&device, &sample_bgl, &blur_a_view, &sample_sampler);
        let blur_b_sample_bg = make_sample_bg(&device, &sample_bgl, &blur_b_view, &sample_sampler);

        let glass_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glass_bgl"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let glass_bg = make_glass_bg(
            &device,
            &glass_bgl,
            &blur_a_view,
            &sample_sampler,
            &pyr_half_view,
        );

        let material_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("materials"),
            size: (MAX_MATERIALS * std::mem::size_of::<MaterialGpuParams>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let material_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let material_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material_bg"),
            layout: &material_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: material_buffer.as_entire_binding(),
            }],
        });

        let sample_only_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sample_only_layout"),
            bind_group_layouts: &[&sample_bgl],
            push_constant_ranges: &[],
        });
        let blur_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blur_layout"),
            bind_group_layouts: &[&sample_bgl, &material_bgl],
            push_constant_ranges: &[],
        });
        let material_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("material_pipeline_layout"),
                bind_group_layouts: &[&globals_bgl, &material_bgl, &glass_bgl],
                push_constant_ranges: &[],
            });

        let fullscreen_target = |blend: Option<wgpu::BlendState>| {
            [Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Bgra8Unorm,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })]
        };

        let halve_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("halve_pipeline"),
            layout: Some(&sample_only_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_fullscreen",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_halve",
                targets: &fullscreen_target(None),
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let blur_h_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blur_h_pipeline"),
            layout: Some(&blur_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_fullscreen",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_blur_h",
                targets: &fullscreen_target(None),
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let blur_v_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blur_v_pipeline"),
            layout: Some(&blur_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_fullscreen",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_blur_v",
                targets: &fullscreen_target(None),
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let present_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("present_pipeline"),
            layout: Some(&sample_only_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_fullscreen",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_present",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
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

        let image_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("images"),
            size: (MAX_IMAGES * std::mem::size_of::<ImageGpuParams>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let image_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let image_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("image_pipeline_layout"),
                bind_group_layouts: &[&globals_bgl, &image_bgl],
                push_constant_ranges: &[],
            });
        let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("image_pipeline"),
            layout: Some(&image_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_image",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_image",
                targets: &fullscreen_target(Some(wgpu::BlendState::ALPHA_BLENDING)),
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let material_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("material_pipeline"),
            layout: Some(&material_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_material",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_material",
                targets: &fullscreen_target(Some(wgpu::BlendState::ALPHA_BLENDING)),
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            device,
            queue,
            surface,
            config,
            rect_pipeline,
            globals_buffer,
            globals_bg,
            instance_buffer,
            merged_pipeline,
            merged_buffer,
            merged_bg,
            max_merged_instances: MAX_MERGED_INSTANCES,
            merged_instances: Vec::with_capacity(MAX_MERGED_INSTANCES),
            text_renderer,
            scene_texture,
            scene_view,
            pyr_half_texture,
            pyr_half_view,
            pyr_quarter_texture,
            pyr_quarter_view,
            blur_a_texture,
            blur_a_view,
            blur_b_texture,
            blur_b_view,
            sample_sampler,
            sample_bgl,
            scene_sample_bg,
            glass_bgl,
            glass_bg,
            pyr_half_sample_bg,
            pyr_quarter_sample_bg,
            blur_a_sample_bg,
            blur_b_sample_bg,
            material_bgl,
            material_buffer,
            material_bg,
            max_materials: MAX_MATERIALS,
            halve_pipeline,
            blur_h_pipeline,
            blur_v_pipeline,
            material_pipeline,
            present_pipeline,
            image_pipeline,
            image_bgl,
            image_buffer,
            image_textures: std::collections::HashMap::new(),
            max_images: MAX_IMAGES,
            max_instances: MAX_RECTS,
            current_frame: None,
            rect_instances: Vec::with_capacity(MAX_RECTS),
            materials: Vec::with_capacity(MAX_MATERIALS),
            images: Vec::new(),
            batches: Vec::new(),
            text_commands: Vec::new(),
        }
    }

    fn update_globals(&self, width: u32, height: u32) {
        let globals = Globals {
            resolution: [width as f32, height as f32],
            _pad: [0.0; 2],
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));
    }

    fn resize_offscreen_targets(&mut self, width: u32, height: u32) {
        let (scene_texture, scene_view) = create_target(&self.device, width, height, "scene");
        let [(half_w, half_h), (quarter_w, quarter_h), (blur_w, blur_h)] =
            pyramid_dims(width, height);
        let (pyr_half_texture, pyr_half_view) =
            create_target(&self.device, half_w, half_h, "pyr_half");
        let (pyr_quarter_texture, pyr_quarter_view) =
            create_target(&self.device, quarter_w, quarter_h, "pyr_quarter");
        let (blur_a_texture, blur_a_view) = create_target(&self.device, blur_w, blur_h, "blur_a");
        let (blur_b_texture, blur_b_view) = create_target(&self.device, blur_w, blur_h, "blur_b");

        let bg = |view: &wgpu::TextureView| {
            make_sample_bg(&self.device, &self.sample_bgl, view, &self.sample_sampler)
        };
        self.scene_sample_bg = bg(&scene_view);
        self.pyr_half_sample_bg = bg(&pyr_half_view);
        self.pyr_quarter_sample_bg = bg(&pyr_quarter_view);
        self.blur_a_sample_bg = bg(&blur_a_view);
        self.blur_b_sample_bg = bg(&blur_b_view);

        self.glass_bg = make_glass_bg(
            &self.device,
            &self.glass_bgl,
            &blur_a_view,
            &self.sample_sampler,
            &pyr_half_view,
        );

        self.scene_texture = scene_texture;
        self.scene_view = scene_view;
        self.pyr_half_texture = pyr_half_texture;
        self.pyr_half_view = pyr_half_view;
        self.pyr_quarter_texture = pyr_quarter_texture;
        self.pyr_quarter_view = pyr_quarter_view;
        self.blur_a_texture = blur_a_texture;
        self.blur_a_view = blur_a_view;
        self.blur_b_texture = blur_b_texture;
        self.blur_b_view = blur_b_view;
    }

    fn push_rect_index(&mut self) {
        let idx = self.rect_instances.len() - 1;
        match self.batches.last_mut() {
            Some(Batch::Rects(range)) if range.end == idx => range.end = idx + 1,
            _ => self.batches.push(Batch::Rects(idx..idx + 1)),
        }
    }

    fn push_merged_index(&mut self) {
        let idx = self.merged_instances.len() - 1;
        match self.batches.last_mut() {
            Some(Batch::Merged(range)) if range.end == idx => range.end = idx + 1,
            _ => self.batches.push(Batch::Merged(idx..idx + 1)),
        }
    }
}

fn create_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: &str,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

// TODO: the glare axis is fixed. Motion-tracked highlights need device
// attitude, which no host plumbs through yet.
fn make_glass_bg(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    blur_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    refraction_view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("glass_bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(blur_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(refraction_view),
            },
        ],
    })
}

fn make_sample_bg(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sample_bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

impl WgpuBackend {

    fn reconfigure(&mut self, width: u32, height: u32) {

        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.update_globals(width, height);
        self.resize_offscreen_targets(width, height);
    }

    fn acquire_frame(&mut self, width: u32, height: u32) -> Option<wgpu::SurfaceTexture> {
        match self.surface.get_current_texture() {
            Ok(frame) if !frame.suboptimal => Some(frame),
            Ok(frame) => {

                drop(frame);
                self.reconfigure(width, height);
                self.surface.get_current_texture().ok()
            }
            Err(wgpu::SurfaceError::Outdated) | Err(wgpu::SurfaceError::Lost) => {
                self.reconfigure(width, height);
                self.surface.get_current_texture().ok()
            }
            Err(_) => None,
        }
    }
}

impl SFBackend for WgpuBackend {

    fn begin_frame(&mut self, width: u32, height: u32) {

        let forced = FORCE_RECONFIGURE.swap(false, Ordering::Relaxed);
        if forced || self.config.width != width || self.config.height != height {
            self.reconfigure(width, height);
        }

        self.rect_instances.clear();
        self.merged_instances.clear();
        self.materials.clear();
        self.images.clear();
        self.batches.clear();
        self.text_commands.clear();
        self.current_frame = self.acquire_frame(width, height);
    }

    fn submit(&mut self, list: &DrawList) {
        for item in &list.commands {

            let clip = item.clip;
            match &item.command {
                DrawCommand::Rect {
                    frame,
                    corner_radius,
                    fill,
                    border,
                    specular,
                    shadow,
                } => {
                    if self.rect_instances.len() >= self.max_instances {
                        continue;
                    }
                    self.rect_instances.push(RectInstance::new(
                        frame.x,
                        frame.y,
                        frame.width,
                        frame.height,
                        *fill,
                        *border,
                        *corner_radius,
                        *specular,
                        *shadow,
                        clip,
                    ));
                    self.push_rect_index();
                }
                DrawCommand::Fill { frame, color } => {
                    if self.rect_instances.len() >= self.max_instances {
                        continue;
                    }
                    self.rect_instances.push(RectInstance::new(
                        frame.x,
                        frame.y,
                        frame.width,
                        frame.height,
                        *color,
                        SFBorder::NONE,
                        0.0,
                        false,
                        SFShadow::NONE,
                        clip,
                    ));
                    self.push_rect_index();
                }
                DrawCommand::MergedRect {
                    bounds,
                    blend_k,
                    color_blend_k,
                    members,
                    count,
                } => {
                    if self.merged_instances.len() >= self.max_merged_instances {
                        continue;
                    }
                    self.merged_instances.push(MergedRectInstance::new(
                        [bounds.x, bounds.y, bounds.width, bounds.height],
                        *blend_k,
                        *color_blend_k,
                        members,
                        *count,
                        clip,
                    ));
                    self.push_merged_index();
                }
                DrawCommand::Material {
                    frame,
                    corner_radius,
                    border,
                    tint,
                    blur_radius,
                    specular,
                    progressive,
                    progressive_start,
                    glass_refraction,
                    glass_interactive,
                } => {
                    if self.materials.len() >= self.max_materials {
                        continue;
                    }
                    self.materials.push(MaterialGpuParams::new(
                        [frame.x, frame.y, frame.width, frame.height],
                        *corner_radius,
                        *border,
                        *tint,
                        *blur_radius,
                        *specular,
                        *progressive,
                        *progressive_start,
                        *glass_refraction,
                        *glass_interactive,
                        clip,
                    ));

                    self.batches.push(Batch::Material(self.materials.len() - 1));
                }
                DrawCommand::Text { .. } => {

                    self.text_commands.push(item.clone());
                    let idx = self.text_commands.len() - 1;
                    match self.batches.last_mut() {
                        Some(Batch::Text(range)) if range.end == idx => range.end = idx + 1,
                        _ => self.batches.push(Batch::Text(idx..idx + 1)),
                    }
                }
                DrawCommand::Image {
                    frame,
                    corner_radius,
                    image_id,
                    content_mode,
                    tint,
                } => {
                    if self.images.len() >= self.max_images {
                        continue;
                    }

                    let Some(tex) = self.image_textures.get(image_id) else {
                        continue;
                    };
                    let uv = content_mode_uv(
                        *content_mode,
                        tex.width,
                        tex.height,
                        frame.width,
                        frame.height,
                    );
                    self.images.push(ImageGpuParams::new(
                        [frame.x, frame.y, frame.width, frame.height],
                        *corner_radius,
                        *tint,
                        uv,
                        clip,
                    ));
                    self.batches.push(Batch::Image {
                        index: self.images.len() - 1,
                        id: *image_id,
                    });
                }
            }
        }
    }

    fn end_frame(&mut self) {
        let frame = match self.current_frame.take() {
            Some(f) => f,
            None => return,
        };

        const CLEAR_COLOR: wgpu::Color = wgpu::Color {
            r: 22.0 / 255.0,
            g: 20.0 / 255.0,
            b: 15.0 / 255.0,
            a: 0.0,
        };

        self.text_renderer.begin_frame();
        let mut text_ranges: Vec<std::ops::Range<u32>> = Vec::new();
        for batch in &self.batches {
            if let Batch::Text(cmd_range) = batch {
                let range = self
                    .text_renderer
                    .prepare_run(&self.text_commands[cmd_range.clone()], &self.queue);
                text_ranges.push(range);
            }
        }
        self.text_renderer.finish_frame(&self.queue);

        if !self.rect_instances.is_empty() {
            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&self.rect_instances),
            );
        }
        if !self.merged_instances.is_empty() {
            self.queue.write_buffer(
                &self.merged_buffer,
                0,
                bytemuck::cast_slice(&self.merged_instances),
            );
        }
        if !self.materials.is_empty() {
            self.queue.write_buffer(
                &self.material_buffer,
                0,
                bytemuck::cast_slice(&self.materials),
            );
        }
        if !self.images.is_empty() {
            self.queue
                .write_buffer(&self.image_buffer, 0, bytemuck::cast_slice(&self.images));
        }

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene_clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }

        enum Run<'a> {
            Scene(&'a [Batch]),
            Material(usize),
        }
        let mut runs: Vec<Run> = Vec::new();
        {
            let mut i = 0;
            while i < self.batches.len() {
                if let Batch::Material(idx) = self.batches[i] {
                    runs.push(Run::Material(idx));
                    i += 1;
                } else {
                    let start = i;
                    while i < self.batches.len() && !matches!(self.batches[i], Batch::Material(_)) {
                        i += 1;
                    }
                    runs.push(Run::Scene(&self.batches[start..i]));
                }
            }
        }

        let mut text_range_idx = 0usize;

        for run in runs {
            match run {
                Run::Scene(batches) => {
                    let mut p = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("scene_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.scene_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    for batch in batches {
                        match batch {
                            Batch::Rects(range) => {
                                p.set_pipeline(&self.rect_pipeline);
                                p.set_bind_group(0, &self.globals_bg, &[]);
                                p.set_vertex_buffer(0, self.instance_buffer.slice(..));
                                p.draw(0..6, range.start as u32..range.end as u32);
                            }
                            Batch::Merged(range) => {
                                p.set_pipeline(&self.merged_pipeline);
                                p.set_bind_group(0, &self.globals_bg, &[]);
                                p.set_bind_group(1, &self.merged_bg, &[]);
                                p.draw(0..6, range.start as u32..range.end as u32);
                            }
                            Batch::Text(_) => {
                                let glyph_range = text_ranges[text_range_idx].clone();
                                text_range_idx += 1;
                                self.text_renderer.render_range(
                                    &mut p,
                                    &self.globals_bg,
                                    glyph_range,
                                );
                            }
                            Batch::Image { index, id } => {

                                if let Some(tex) = self.image_textures.get(id) {
                                    p.set_pipeline(&self.image_pipeline);
                                    p.set_bind_group(0, &self.globals_bg, &[]);
                                    p.set_bind_group(1, &tex.bind_group, &[]);
                                    p.draw(0..6, *index as u32..*index as u32 + 1);
                                }
                            }
                            Batch::Material(_) => unreachable!("grouped out above"),
                        }
                    }
                }
                Run::Material(idx) => {

                    for (label, src, dst) in [
                        ("halve_1_pass", &self.scene_sample_bg, &self.pyr_half_view),
                        (
                            "halve_2_pass",
                            &self.pyr_half_sample_bg,
                            &self.pyr_quarter_view,
                        ),
                        (
                            "halve_3_pass",
                            &self.pyr_quarter_sample_bg,
                            &self.blur_a_view,
                        ),
                    ] {
                        let mut p = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some(label),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: dst,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            ..Default::default()
                        });
                        p.set_pipeline(&self.halve_pipeline);
                        p.set_bind_group(0, src, &[]);
                        p.draw(0..3, 0..1);
                    }

                    for _ in 0..BLUR_ITERATIONS {

                        {
                            let mut p = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("blur_h_pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &self.blur_b_view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                ..Default::default()
                            });
                            p.set_pipeline(&self.blur_h_pipeline);
                            p.set_bind_group(0, &self.blur_a_sample_bg, &[]);
                            p.set_bind_group(1, &self.material_bg, &[]);
                            p.draw(0..3, idx as u32..idx as u32 + 1);
                        }

                        {
                            let mut p = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("blur_v_pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &self.blur_a_view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                ..Default::default()
                            });
                            p.set_pipeline(&self.blur_v_pipeline);
                            p.set_bind_group(0, &self.blur_b_sample_bg, &[]);
                            p.set_bind_group(1, &self.material_bg, &[]);
                            p.draw(0..3, idx as u32..idx as u32 + 1);
                        }
                    }

                    let mut p = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("material_composite_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.scene_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    p.set_pipeline(&self.material_pipeline);
                    p.set_bind_group(0, &self.globals_bg, &[]);
                    p.set_bind_group(1, &self.material_bg, &[]);
                    p.set_bind_group(2, &self.glass_bg, &[]);
                    p.draw(0..6, idx as u32..idx as u32 + 1);
                }
            }
        }

        {
            let mut p = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            p.set_pipeline(&self.present_pipeline);
            p.set_bind_group(0, &self.scene_sample_bg, &[]);
            p.draw(0..3, 0..1);
        }

        self.queue.submit([encoder.finish()]);
        frame.present();
    }

    fn upload_image(&mut self, image_id: u32, rgba: &[u8], width: u32, height: u32) {
        if image_id == 0 || width == 0 || height == 0 {
            return;
        }
        if rgba.len() < (width as usize) * (height as usize) * 4 {
            return;
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("image"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_bg"),
            layout: &self.image_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.image_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.sample_sampler),
                },
            ],
        });

        self.image_textures.insert(
            image_id,
            ImageTexture {
                width,
                height,
                bind_group,
                _texture: texture,
            },
        );
    }

    fn drop_image(&mut self, image_id: u32) {
        self.image_textures.remove(&image_id);
    }
}

#[no_mangle]
pub extern "C" fn swiftflow_init(surface: SFSurfaceDescriptor, width: u32, height: u32) {

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        let target = match surface.kind {
            SFSurfaceKind::MetalLayer => {
                wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(surface.handle)
            }

            SFSurfaceKind::RawHandle => unimplemented!(
                "SF_SURFACE_RAW_HANDLE is reserved for Android; desktop uses init_with_target"
            ),
        };
        init_with_source(SurfaceSource::Raw(target), width, height);
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    {
        let _ = (surface, width, height);
        unimplemented!(
            "swiftflow_init is the Apple CAMetalLayer entry point; \
             desktop hosts call sf_desktop_run instead"
        );
    }
}

pub fn init_with_target(target: wgpu::SurfaceTarget<'static>, width: u32, height: u32) {
    init_with_source(SurfaceSource::Owned(target), width, height);
}

fn init_with_source(source: SurfaceSource, width: u32, height: u32) {
    pollster::block_on(async move {
        let backend = WgpuBackend::new(source, width, height).await;
        register_backend(Box::new(backend));
        sf_init(width, height);
    });
}

static FORCE_RECONFIGURE: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "C" fn swiftflow_surface_invalidated() {
    FORCE_RECONFIGURE.store(true, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn swiftflow_resize(_width: u32, _height: u32) {
    swiftflow_surface_invalidated();
}
