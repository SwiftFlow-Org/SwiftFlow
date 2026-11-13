// A canvas node, rendered headless: the painter registered under its id is
// called at its place in the draw order, with its rectangle, and what it
// draws lands in the frame inside that rectangle and nowhere else.

use std::sync::{Arc, Mutex};
use swiftflow_core::node::{SFNode, SFNodeKind, SFSizing};
use swiftflow_wgpu::canvas::{self, CanvasFrame, CanvasPainter};

extern "C" {
    fn sf_render_tree(root: *mut SFNode, width: f32, height: f32, scale: f32);
    fn sf_read_frame(out: *mut u8, cap: usize) -> usize;
}

struct Red {
    seen: Arc<Mutex<Option<([f32; 4], [u32; 4])>>>,
}

impl CanvasPainter for Red {
    fn paint(&mut self, frame: &mut CanvasFrame<'_>) {
        *self.seen.lock().unwrap() = Some((frame.rect, frame.visible));
        let _pass = frame.begin_pass(Some(wgpu::Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }));
    }
}

const CANVAS: u32 = 7;

#[test]
fn a_canvas_paints_its_own_rectangle_and_nothing_else() {
    let (w, h) = (64u32, 48u32);
    swiftflow_wgpu::swiftflow_init_offscreen(w, h);

    let seen = Arc::new(Mutex::new(None));
    canvas::register(CANVAS, Box::new(Red { seen: seen.clone() }));
    assert!(canvas::swiftflow_canvas_take_frame_request(), "registering should ask for a frame");
    assert!(!canvas::swiftflow_canvas_take_frame_request(), "the request should clear once taken");

    let mut child = SFNode::empty();
    child.kind = SFNodeKind::Canvas;
    child.image_id = CANVAS;
    child.sizing_x = SFSizing::Fixed;
    child.sizing_y = SFSizing::Fixed;
    child.fixed_width = 32.0;
    child.fixed_height = 24.0;

    let mut children = vec![child];
    let mut root = SFNode::empty();
    root.kind = SFNodeKind::Stack;
    root.axis = swiftflow_core::node::SFAxis::Depth;
    root.children = children.as_mut_ptr();
    root.children_len = children.len();

    unsafe { sf_render_tree(&mut root, w as f32, h as f32, 1.0) };

    let (rect, visible) = seen.lock().unwrap().expect("the painter was never called");
    assert_eq!(rect, [16.0, 12.0, 32.0, 24.0], "the painter got the wrong rectangle");
    assert_eq!(visible, [16, 12, 32, 24]);

    let mut buf = vec![0u8; (w * h * 4) as usize];
    let n = unsafe { sf_read_frame(buf.as_mut_ptr(), buf.len()) };
    assert_eq!(n, buf.len());
    let px = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        [buf[i], buf[i + 1], buf[i + 2]]
    };

    // BGRA.
    assert_eq!(px(32, 24), [0, 0, 255], "the middle of the canvas should be red");
    assert_eq!(px(16, 12), [0, 0, 255], "the canvas's top-left pixel should be red");
    assert_eq!(px(47, 35), [0, 0, 255], "the canvas's bottom-right pixel should be red");
    for (x, y) in [(15, 24), (48, 24), (32, 11), (32, 36), (0, 0)] {
        assert_eq!(px(x, y), [15, 20, 22], "({x}, {y}) is outside the canvas and was painted");
    }

    canvas::unregister(CANVAS);
}
