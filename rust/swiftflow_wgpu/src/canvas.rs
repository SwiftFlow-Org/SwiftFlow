//! Canvases: rectangles in the layout that an app draws into with its own
//! wgpu code.
//!
//! A view puts a `Canvas(id:)` in its body. The renderer reaches that node
//! in draw order, ends whatever pass it had open, and calls the painter
//! registered under the same id. The painter gets the renderer's device,
//! queue and command encoder, and records into the scene texture, the same
//! texture every glass pane and material samples. So a panel floating over
//! a canvas blurs and refracts the drawing under it, and anything drawn
//! after the canvas lands on top of it.
//!
//! Painters live in Rust because the point is to share the renderer's
//! `wgpu::Device`. That only works when the painter is compiled into the
//! same library as the renderer, so an app that has one builds its own
//! cdylib depending on `swiftflow_desktop` (or `swiftflow_wgpu` on iOS) as
//! an rlib, and links that in place of the stock one.
//!
//! ```ignore
//! struct Paper;
//! impl CanvasPainter for Paper {
//!     fn paint(&mut self, frame: &mut CanvasFrame) {
//!         let pass = frame.begin_pass(Some(wgpu::Color::WHITE));
//!         // draw…
//!     }
//! }
//! swiftflow_wgpu::canvas::register(1, Box::new(Paper));
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Draws one canvas, once per frame it is on screen.
pub trait CanvasPainter: Send {
    /// Records this frame's drawing. Called on the render thread, between
    /// the scene draws that come before the canvas node and the ones after.
    fn paint(&mut self, frame: &mut CanvasFrame<'_>);
}

/// Everything a painter needs for one frame.
pub struct CanvasFrame<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,

    /// The scene texture. Draw into it with `LoadOp::Load` (or use
    /// [`CanvasFrame::begin_pass`], which sets that up), or the interface
    /// already drawn under the canvas is lost.
    pub target: &'a wgpu::TextureView,
    pub target_format: wgpu::TextureFormat,
    pub target_size: (u32, u32),

    /// The canvas node's rectangle, in physical pixels from the top left of
    /// the window.
    pub rect: [f32; 4],

    /// The part of `rect` that is visible, after the clips of every scroll
    /// view and clipped stack around the canvas, in whole pixels:
    /// `[x, y, width, height]`. Zero-sized when the canvas is clipped away.
    pub visible: [u32; 4],

    /// Physical pixels per point.
    pub scale: f32,
}

impl CanvasFrame<'_> {
    /// Opens a render pass on the scene texture with the viewport set to the
    /// canvas (so clip space covers exactly the canvas) and the scissor set
    /// to its visible part.
    ///
    /// `clear` fills the visible part first; `None` keeps what is there.
    /// A clear here covers the whole target, not only the canvas, because
    /// wgpu's clears ignore the scissor, so this draws a quad instead.
    pub fn begin_pass(&mut self, clear: Option<wgpu::Color>) -> wgpu::RenderPass<'_> {
        let mut pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("canvas_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        let [x, y, w, h] = self.rect;
        pass.set_viewport(x, y, w.max(1.0), h.max(1.0), 0.0, 1.0);
        let [sx, sy, sw, sh] = self.visible;
        pass.set_scissor_rect(sx, sy, sw, sh);
        if let Some(colour) = clear {
            fill(&mut pass, self.device, self.target_format, colour);
        }
        pass
    }

    /// Whether any of the canvas is on screen this frame.
    pub fn is_visible(&self) -> bool {
        self.visible[2] > 0 && self.visible[3] > 0
    }
}

/// Clears the scissored part of a pass to `colour` by drawing over it. The
/// colour rides in as the blend constant (source factor `Constant`,
/// destination `Zero`), so the pipeline needs no buffer or device feature.
fn fill(
    pass: &mut wgpu::RenderPass<'_>,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    colour: wgpu::Color,
) {
    static FILL: OnceLock<Mutex<HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>>> =
        OnceLock::new();
    let mut cache = FILL.get_or_init(Default::default).lock().unwrap();
    let pipeline = cache.entry(format).or_insert_with(|| {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas_fill"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fs() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }
"#
                .into(),
            ),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("canvas_fill"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("canvas_fill"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Constant,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Constant,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
    });
    pass.set_pipeline(pipeline);
    pass.set_blend_constant(colour);
    pass.draw(0..3, 0..1);
}

fn painters() -> &'static Mutex<HashMap<u32, Box<dyn CanvasPainter>>> {
    static PAINTERS: OnceLock<Mutex<HashMap<u32, Box<dyn CanvasPainter>>>> = OnceLock::new();
    PAINTERS.get_or_init(Default::default)
}

/// Puts `painter` in charge of every canvas node whose id is `id`,
/// replacing any painter already there. Id 0 is reserved for "no canvas".
pub fn register(id: u32, painter: Box<dyn CanvasPainter>) {
    assert!(id != 0, "canvas id 0 means no canvas");
    painters().lock().unwrap().insert(id, painter);
    request_frame();
}

/// Removes the painter for `id`. Its canvas nodes then draw nothing.
pub fn unregister(id: u32) {
    painters().lock().unwrap().remove(&id);
}

/// Runs `paint` on the painter registered under `id`, if there is one.
pub(crate) fn paint(id: u32, frame: &mut CanvasFrame<'_>) {
    let mut painters = painters().lock().unwrap();
    if let Some(painter) = painters.get_mut(&id) {
        painter.paint(frame);
    }
}

static FRAME_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Asks for another frame: a painter whose content changed outside of
/// any view update (a stroke arriving, playback advancing) calls this so the
/// host draws again even though no view did anything.
pub fn request_frame() {
    FRAME_REQUESTED.store(true, Ordering::Release);
}

/// Whether a painter asked for a frame since the host last checked, and
/// clears the request. The hosts call it once per frame tick.
#[no_mangle]
pub extern "C" fn swiftflow_canvas_take_frame_request() -> bool {
    FRAME_REQUESTED.swap(false, Ordering::AcqRel)
}

/// The whole-pixel part of `rect` that lies inside both `clip` (when it is
/// active, i.e. has a non-negative size) and the target.
pub(crate) fn visible_rect(rect: [f32; 4], clip: [f32; 4], target: (u32, u32)) -> [u32; 4] {
    let mut x0 = rect[0];
    let mut y0 = rect[1];
    let mut x1 = rect[0] + rect[2];
    let mut y1 = rect[1] + rect[3];
    if clip[2] >= 0.0 && clip[3] >= 0.0 {
        x0 = x0.max(clip[0]);
        y0 = y0.max(clip[1]);
        x1 = x1.min(clip[0] + clip[2]);
        y1 = y1.min(clip[1] + clip[3]);
    }
    let x0 = x0.max(0.0).floor() as u32;
    let y0 = y0.max(0.0).floor() as u32;
    let x1 = (x1.ceil().max(0.0) as u32).min(target.0);
    let y1 = (y1.ceil().max(0.0) as u32).min(target.1);
    if x1 <= x0 || y1 <= y0 {
        return [x0.min(target.0), y0.min(target.1), 0, 0];
    }
    [x0, y0, x1 - x0, y1 - y0]
}

#[cfg(test)]
mod tests {
    use super::visible_rect;

    #[test]
    fn an_unclipped_canvas_is_all_visible() {
        assert_eq!(visible_rect([10.0, 20.0, 100.0, 50.0], [0.0, 0.0, -1.0, -1.0], (800, 600)), [10, 20, 100, 50]);
    }

    #[test]
    fn a_clip_trims_the_canvas() {
        assert_eq!(visible_rect([0.0, 0.0, 100.0, 100.0], [50.0, 25.0, 100.0, 50.0], (800, 600)), [50, 25, 50, 50]);
    }

    #[test]
    fn the_target_edge_trims_the_canvas() {
        assert_eq!(visible_rect([-20.0, 590.0, 100.0, 100.0], [0.0, 0.0, -1.0, -1.0], (800, 600)), [0, 590, 80, 10]);
    }

    #[test]
    fn a_canvas_clipped_away_has_no_size() {
        let v = visible_rect([0.0, 0.0, 100.0, 100.0], [200.0, 200.0, 10.0, 10.0], (800, 600));
        assert_eq!((v[2], v[3]), (0, 0));
    }
}
