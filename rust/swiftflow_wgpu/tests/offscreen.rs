// Headless verification of the offscreen render + readback path (stage 3).
// Renders an empty tree into an owned texture, reads it back, and checks the
// frame is the full size and a single uniform colour (the clear colour) —
// proving init → render → texture→buffer copy → map → unpad all work.

extern "C" {
    fn sf_create_empty_root() -> *mut std::ffi::c_void;
    fn sf_render_tree(root: *mut std::ffi::c_void, width: f32, height: f32, scale: f32);
    fn sf_read_frame(out: *mut u8, cap: usize) -> usize;
}

#[test]
fn offscreen_renders_and_reads_back_a_full_frame() {
    let (w, h) = (64u32, 48u32);
    swiftflow_wgpu::swiftflow_init_offscreen(w, h);

    let root = unsafe { sf_create_empty_root() };
    unsafe { sf_render_tree(root, w as f32, h as f32, 1.0) };

    let mut buf = vec![0u8; (w * h * 4) as usize];
    let n = unsafe { sf_read_frame(buf.as_mut_ptr(), buf.len()) };
    assert_eq!(n, buf.len(), "expected a full width*height*4 frame");

    // Every pixel should equal the clear colour: proves the readback isn't a
    // stale/zero buffer and rows were unpadded correctly.
    let first = &buf[0..4];
    let uniform = buf.chunks_exact(4).all(|px| px == first);
    let mid = ((h / 2 * w + w / 2) * 4) as usize;
    eprintln!(
        "offscreen clear pixel BGRA = {:?}; uniform = {}",
        &buf[mid..mid + 4],
        uniform
    );
    assert!(uniform, "frame should be a single uniform colour for an empty tree");
    // The clear colour is (r=22,g=20,b=15); in BGRA that's [15,20,22,_].
    assert_eq!(&buf[mid..mid + 3], &[15u8, 20, 22], "unexpected clear colour");
}
