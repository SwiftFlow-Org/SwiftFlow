use crate::draw::DrawList;
use crate::draw_pass::{compose_scale, draw};
use crate::layout::layout;
use crate::node::{SFNode, SFNodeKind, SFSizing};
use crate::types::{SFPoint, SFRect, SFScale};
use crate::with_font_system;
use crate::{font, sflog};
use std::sync::Mutex;

pub trait SFBackend: Send + Sync {
    fn begin_frame(&mut self, width: u32, height: u32);
    fn submit(&mut self, list: &DrawList);
    fn end_frame(&mut self);

    fn upload_image(&mut self, image_id: u32, rgba: &[u8], width: u32, height: u32);

    fn drop_image(&mut self, image_id: u32);
}

static mut BACKEND: Option<Box<dyn SFBackend>> = None;
static mut DRAW_LIST: Option<DrawList> = None;
pub static SCALE: Mutex<f32> = Mutex::new(1.0);
static FRAME: Mutex<u32> = Mutex::new(0);

pub fn register_backend(backend: Box<dyn SFBackend>) {
    unsafe {
        BACKEND = Some(backend);
        DRAW_LIST = Some(DrawList::new());
    }
}

pub fn release_backend() {
    unsafe {
        BACKEND = None;
        DRAW_LIST = None;
    }
}

#[no_mangle]
pub extern "C" fn sf_render_tree(root: *mut SFNode, width: f32, height: f32, scale: f32) {
    let current_frame = {
        let mut f = FRAME.lock().unwrap();
        *f += 1;
        *f
    };
    {
        let mut current_scale = SCALE.lock().unwrap();
        if (*current_scale - scale).abs() > 0.01 {
            with_font_system(|fs| {
                fs.atlas.glyphs.clear();

                fs.atlas.accum.clear();
                fs.atlas.overflowed.clear();
                fs.cursor_x = 2;
                fs.cursor_y = 2;
                fs.row_height = 0;
                fs.dirty = true;
            });
        }
        *current_scale = scale;
    }

    let backend = unsafe { BACKEND.as_mut() };
    let draw_list = unsafe { DRAW_LIST.as_mut() };

    let (Some(backend), Some(draw_list)) = (backend, draw_list) else {
        return;
    };

    let node = unsafe { &mut *root };

    node.set_sizing(SFSizing::Fill);

    let available = SFRect::new(0.0, 0.0, width, height);
    layout(node, available);

    draw_list.clear();
    draw(node, draw_list);

    backend.begin_frame(width as u32, height as u32);
    backend.submit(draw_list);
    backend.end_frame();

    with_font_system(|fs| fs.refine(current_frame));
}

#[no_mangle]
pub extern "C" fn sf_init(width: u32, height: u32) {
    let _ = (width, height);
    crate::glyph::init_glyph_loader();
    font::init_font_system();

}

#[no_mangle]
pub extern "C" fn swiftflow_get_atlas_debug(out_ptr: *mut *const u8, out_len: *mut usize) {

    with_font_system(|fs| {
        let data = &fs.atlas.data;
        unsafe {
            *out_ptr = data.as_ptr();
            *out_len = data.len();
        }
    });
}

#[no_mangle]
pub extern "C" fn sf_hit_test_path(
    root: *const SFNode,
    x: f32,
    y: f32,
    out: *mut u32,
    cap: usize,
) -> usize {
    if root.is_null() || out.is_null() || cap == 0 {
        return 0;
    }
    let node = unsafe { &*root };
    let mut path: Vec<u32> = Vec::new();
    hit_path_node(
        node,
        x,
        y,
        SFPoint { x: 0.0, y: 0.0 },
        SFScale::DEFAULT,
        &mut path,
    );

    let count = path.len().min(cap);
    let dst = unsafe { std::slice::from_raw_parts_mut(out, count) };
    dst.copy_from_slice(&path[..count]);
    count
}

fn hit_path_node(
    node: &SFNode,
    x: f32,
    y: f32,
    translate: SFPoint,
    scale: SFScale,
    out: &mut Vec<u32>,
) -> bool {
    let (translate, scale) = compose_scale(node, translate, scale);
    let node_origin = SFPoint {
        x: translate.x + node.frame.x * scale.x,
        y: translate.y + node.frame.y * scale.y,
    };
    let node_w = node.frame.width * scale.x;
    let node_h = node.frame.height * scale.y;

    let inside = x >= node_origin.x
        && x <= node_origin.x + node_w
        && y >= node_origin.y
        && y <= node_origin.y + node_h;

    if !inside {
        return false;
    }

    if !node.children.is_null() && node.children_len > 0 {
        let children = unsafe { std::slice::from_raw_parts(node.children, node.children_len) };
        for child in children.iter().rev() {
            if hit_path_node(child, x, y, node_origin, scale, out) {
                break;
            }
        }
    }

    if node.node_id != 0 {
        out.push(node.node_id);
    }
    true
}

#[no_mangle]
pub extern "C" fn sf_hit_test(root: *const SFNode, x: f32, y: f32) -> u32 {
    let node = unsafe { &*root };
    let result = hit_test_node(node, x, y, SFPoint { x: 0.0, y: 0.0 }, SFScale::DEFAULT)
        .unwrap_or(0);
    result
}

fn hit_test_node(node: &SFNode, x: f32, y: f32, translate: SFPoint, scale: SFScale) -> Option<u32> {
    let (translate, scale) = compose_scale(node, translate, scale);
    let node_origin = SFPoint {
        x: translate.x + node.frame.x * scale.x,
        y: translate.y + node.frame.y * scale.y,
    };
    let node_w = node.frame.width * scale.x;
    let node_h = node.frame.height * scale.y;

    let inside = x >= node_origin.x
        && x <= node_origin.x + node_w
        && y >= node_origin.y
        && y <= node_origin.y + node_h;

    if !inside {
        return None;
    }

    if !node.children.is_null() && node.children_len > 0 {
        let children = unsafe { std::slice::from_raw_parts(node.children, node.children_len) };
        for child in children.iter().rev() {
            if let Some(id) = hit_test_node(child, x, y, node_origin, scale) {
                return Some(id);
            }
        }
    }

    if node.node_id != 0 {
        Some(node.node_id)
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn sf_hit_test_scroll(root: *const SFNode, x: f32, y: f32) -> u32 {
    let node = unsafe { &*root };
    hit_test_scroll_node(node, x, y, SFPoint { x: 0.0, y: 0.0 }, SFScale::DEFAULT).unwrap_or(0)
}

fn hit_test_scroll_node(
    node: &SFNode,
    x: f32,
    y: f32,
    translate: SFPoint,
    scale: SFScale,
) -> Option<u32> {
    let (translate, scale) = compose_scale(node, translate, scale);
    let node_origin = SFPoint {
        x: translate.x + node.frame.x * scale.x,
        y: translate.y + node.frame.y * scale.y,
    };
    let node_w = node.frame.width * scale.x;
    let node_h = node.frame.height * scale.y;

    let inside = x >= node_origin.x
        && x <= node_origin.x + node_w
        && y >= node_origin.y
        && y <= node_origin.y + node_h;

    if !inside {
        return None;
    }

    if !node.children.is_null() && node.children_len > 0 {
        let children = unsafe { std::slice::from_raw_parts(node.children, node.children_len) };
        for child in children.iter().rev() {
            if let Some(id) = hit_test_scroll_node(child, x, y, node_origin, scale) {
                return Some(id);
            }
        }
    }

    if node.kind == SFNodeKind::Scroll && node.scroll_id != 0 {
        Some(node.scroll_id)
    } else {
        None
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SFScrollHit {
    pub scroll_id: u32,

    pub axis: u32,
}

#[no_mangle]
pub extern "C" fn sf_hit_test_scroll_path(
    root: *const SFNode,
    x: f32,
    y: f32,
    out: *mut SFScrollHit,
    cap: usize,
) -> usize {
    if root.is_null() || out.is_null() || cap == 0 {
        return 0;
    }
    let node = unsafe { &*root };
    let mut path: Vec<SFScrollHit> = Vec::new();
    scroll_path_node(
        node,
        x,
        y,
        SFPoint { x: 0.0, y: 0.0 },
        SFScale::DEFAULT,
        &mut path,
    );

    let count = path.len().min(cap);
    for (index, hit) in path.into_iter().take(count).enumerate() {
        unsafe { out.add(index).write(hit) };
    }
    count
}

fn scroll_path_node(
    node: &SFNode,
    x: f32,
    y: f32,
    translate: SFPoint,
    scale: SFScale,
    out: &mut Vec<SFScrollHit>,
) -> bool {
    let (translate, scale) = compose_scale(node, translate, scale);
    let node_origin = SFPoint {
        x: translate.x + node.frame.x * scale.x,
        y: translate.y + node.frame.y * scale.y,
    };
    let node_w = node.frame.width * scale.x;
    let node_h = node.frame.height * scale.y;

    let inside = x >= node_origin.x
        && x <= node_origin.x + node_w
        && y >= node_origin.y
        && y <= node_origin.y + node_h;

    if !inside {
        return false;
    }

    if !node.children.is_null() && node.children_len > 0 {
        let children = unsafe { std::slice::from_raw_parts(node.children, node.children_len) };
        for child in children.iter().rev() {
            if scroll_path_node(child, x, y, node_origin, scale, out) {
                break;
            }
        }
    }

    if node.kind == SFNodeKind::Scroll && node.scroll_id != 0 {
        out.push(SFScrollHit {
            scroll_id: node.scroll_id,
            axis: node.axis as u32,
        });
    }
    true
}

#[repr(C)]
pub struct SFScrollMetrics {
    pub viewport_height: f32,
    pub content_height: f32,
    pub viewport_width: f32,
    pub content_width: f32,
}

#[no_mangle]
pub extern "C" fn sf_get_scroll_metrics(root: *const SFNode, scroll_id: u32) -> SFScrollMetrics {
    let node = unsafe { &*root };
    find_scroll_metrics(node, scroll_id).unwrap_or(SFScrollMetrics {
        viewport_height: 0.0,
        content_height: 0.0,
        viewport_width: 0.0,
        content_width: 0.0,
    })
}

#[no_mangle]
pub extern "C" fn sf_get_node_frame(root: *const SFNode, node_id: u32) -> SFRect {
    if root.is_null() || node_id == 0 {
        return SFRect::ZERO;
    }
    let node = unsafe { &*root };
    find_node_frame(node, node_id, SFPoint { x: 0.0, y: 0.0 }, SFScale::DEFAULT)
        .unwrap_or(SFRect::ZERO)
}

fn find_node_frame(
    node: &SFNode,
    node_id: u32,
    translate: SFPoint,
    scale: SFScale,
) -> Option<SFRect> {
    let (translate, scale) = compose_scale(node, translate, scale);
    let node_origin = SFPoint {
        x: translate.x + node.frame.x * scale.x,
        y: translate.y + node.frame.y * scale.y,
    };

    if node.node_id == node_id {
        return Some(SFRect::new(
            node_origin.x,
            node_origin.y,
            node.frame.width * scale.x,
            node.frame.height * scale.y,
        ));
    }

    if node.children.is_null() || node.children_len == 0 {
        return None;
    }
    let children = unsafe { std::slice::from_raw_parts(node.children, node.children_len) };
    children
        .iter()
        .find_map(|child| find_node_frame(child, node_id, node_origin, scale))
}

#[repr(C)]
pub struct SFImageSize {
    pub width: f32,
    pub height: f32,
}

impl SFImageSize {
    const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };
}

#[no_mangle]
pub extern "C" fn sf_register_image(id: u32, bytes: *const u8, len: usize) -> SFImageSize {
    if id == 0 || bytes.is_null() || len == 0 {
        return SFImageSize::ZERO;
    }
    let data = unsafe { std::slice::from_raw_parts(bytes, len) };

    let Some(img) = crate::image_decode::decode(data) else {
        sflog!("sf_register_image: could not decode {} bytes for id {}", len, id);
        return SFImageSize::ZERO;
    };

    let Some(backend) = (unsafe { BACKEND.as_mut() }) else {
        return SFImageSize::ZERO;
    };
    backend.upload_image(id, &img.rgba, img.width, img.height);

    SFImageSize {
        width: img.width as f32,
        height: img.height as f32,
    }
}

#[no_mangle]
pub extern "C" fn sf_unregister_image(id: u32) {
    if let Some(backend) = unsafe { BACKEND.as_mut() } {
        backend.drop_image(id);
    }
}

#[no_mangle]
pub extern "C" fn sf_log(message: *const std::ffi::c_char) {
    if message.is_null() {
        return;
    }
    let text = unsafe { std::ffi::CStr::from_ptr(message) };
    crate::log::nslog(&text.to_string_lossy());
}

fn find_scroll_metrics(node: &SFNode, scroll_id: u32) -> Option<SFScrollMetrics> {
    if node.kind == SFNodeKind::Scroll && node.scroll_id == scroll_id {
        return Some(SFScrollMetrics {
            viewport_height: node.frame.height,
            content_height: node.content_height,
            viewport_width: node.frame.width,
            content_width: node.content_width,
        });
    }

    if !node.children.is_null() && node.children_len > 0 {
        let children = unsafe { std::slice::from_raw_parts(node.children, node.children_len) };
        for child in children {
            if let Some(m) = find_scroll_metrics(child, scroll_id) {
                return Some(m);
            }
        }
    }

    None
}
