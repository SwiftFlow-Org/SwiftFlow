use eframe::egui::{self, Color32};

pub const SIDEBAR_BG: Color32 = Color32::from_rgb(42, 42, 44);
pub const CANVAS_BG: Color32 = Color32::from_rgb(30, 30, 32);
pub const INSPECTOR_BG: Color32 = Color32::from_rgb(42, 42, 44);
pub const SEPARATOR: Color32 = Color32::from_rgb(58, 58, 60);

pub const SELECTION: Color32 = Color32::from_rgb(10, 94, 219);

pub const TEXT: Color32 = Color32::from_rgb(232, 232, 234);
pub const TEXT_DIM: Color32 = Color32::from_rgb(152, 152, 157);

pub const WELL_BG: Color32 = Color32::from_rgb(46, 46, 48);
pub const WELL_BORDER: Color32 = Color32::from_rgb(74, 74, 77);

pub const WELL_ACTIVE: Color32 = Color32::from_rgb(10, 94, 219);

pub const CHECKER_A: Color32 = Color32::from_rgb(68, 68, 70);
pub const CHECKER_B: Color32 = Color32::from_rgb(58, 58, 60);
pub const CHECKER_SIZE: f32 = 8.0;

pub const WARNING: Color32 = Color32::from_rgb(229, 165, 60);
pub const ERROR: Color32 = Color32::from_rgb(224, 92, 84);

pub fn apply(ctx: &egui::Context) {

    ctx.all_styles_mut(|style| {
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = CANVAS_BG;
    style.visuals.window_fill = SIDEBAR_BG;
    style.visuals.extreme_bg_color = Color32::from_rgb(24, 24, 26);
    style.visuals.override_text_color = Some(TEXT);

    style.visuals.widgets.noninteractive.bg_fill = SIDEBAR_BG;

    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, SEPARATOR);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(58, 58, 60);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(52, 52, 54);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 70, 73);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(64, 64, 67);
    style.visuals.widgets.active.bg_fill = SELECTION;
    style.visuals.widgets.active.weak_bg_fill = SELECTION;
    style.visuals.selection.bg_fill = SELECTION;
    style.visuals.selection.stroke = egui::Stroke::new(1.0, TEXT);

    style.visuals.window_corner_radius = 6.into();
    style.spacing.item_spacing = egui::vec2(6.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    });
}

pub fn checkerboard(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_filled(rect, 0.0, CHECKER_B);
    let cols = (rect.width() / CHECKER_SIZE).ceil() as usize;
    let rows = (rect.height() / CHECKER_SIZE).ceil() as usize;
    for row in 0..rows {
        for col in 0..cols {
            if (row + col) % 2 != 0 {
                continue;
            }
            let min = rect.min + egui::vec2(col as f32 * CHECKER_SIZE, row as f32 * CHECKER_SIZE);
            let square = egui::Rect::from_min_size(min, egui::Vec2::splat(CHECKER_SIZE))
                .intersect(rect);
            painter.rect_filled(square, 0.0, CHECKER_A);
        }
    }
}
