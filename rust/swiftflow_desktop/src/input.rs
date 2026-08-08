use winit::event::MouseScrollDelta;

pub const LINE_SCROLL_POINTS: f32 = 50.0;

#[cfg(target_os = "macos")]
pub const MACOS_TITLEBAR_POINTS: f32 = 28.0;

pub fn safe_area_points() -> (f32, f32, f32, f32) {
    #[cfg(target_os = "macos")]
    {
        (MACOS_TITLEBAR_POINTS, 0.0, 0.0, 0.0)
    }
    #[cfg(not(target_os = "macos"))]
    {
        (0.0, 0.0, 0.0, 0.0)
    }
}

pub fn scroll_delta_pixels(delta: MouseScrollDelta, scale: f32) -> (f32, f32) {
    let (x, y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (
            -x * LINE_SCROLL_POINTS * scale,
            -y * LINE_SCROLL_POINTS * scale,
        ),
        MouseScrollDelta::PixelDelta(pos) => (-pos.x as f32, -pos.y as f32),
    };
    (x, y)
}

pub const LIFECYCLE_FOREGROUND: u32 = 0;
pub const LIFECYCLE_BACKGROUND: u32 = 1;
pub const LIFECYCLE_TERMINATE: u32 = 2;

#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::PhysicalPosition;

    #[test]
    fn a_wheel_notch_scrolls_a_readable_distance() {

        let (_, y) = scroll_delta_pixels(MouseScrollDelta::LineDelta(0.0, 1.0), 1.0);
        assert_eq!(y, -LINE_SCROLL_POINTS);

        let (_, y2x) = scroll_delta_pixels(MouseScrollDelta::LineDelta(0.0, 1.0), 2.0);
        assert_eq!(y2x, -LINE_SCROLL_POINTS * 2.0);
    }

    #[test]
    fn trackpad_pixels_pass_through_unscaled() {

        let (x, y) = scroll_delta_pixels(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(3.0, 12.0)),
            2.0,
        );
        assert_eq!((x, y), (-3.0, -12.0));
    }

    #[test]
    fn scrolling_down_increases_the_content_offset() {

        let (_, wheel) = scroll_delta_pixels(MouseScrollDelta::LineDelta(0.0, -1.0), 1.0);
        assert!(wheel > 0.0, "wheeling down should increase offsetY");

        let (_, trackpad) = scroll_delta_pixels(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -10.0)),
            1.0,
        );
        assert!(trackpad > 0.0, "both devices must agree on direction");
    }

    #[test]
    fn safe_area_matches_the_window_style() {
        let (top, bottom, leading, trailing) = safe_area_points();
        assert_eq!((bottom, leading, trailing), (0.0, 0.0, 0.0));

        #[cfg(target_os = "macos")]
        assert_eq!(
            top, MACOS_TITLEBAR_POINTS,
            "content runs under a transparent titlebar, so the bar must inset"
        );
        #[cfg(not(target_os = "macos"))]
        assert_eq!(
            top, 0.0,
            "server-side decorations already exclude their own chrome"
        );
    }
}
