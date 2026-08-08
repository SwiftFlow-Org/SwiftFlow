mod preview;
mod theme;

use eframe::egui;
use preview::{format_size, PreviewCache};
use std::path::PathBuf;
use swiftflow_assets::{Catalog, Scale};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 700.0])
            .with_min_inner_size([760.0, 440.0])
            .with_title("SwiftFlow Assets"),
        ..Default::default()
    };

    let initial = std::env::args().nth(1).map(PathBuf::from);

    eframe::run_native(
        "SwiftFlow Assets",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(App::new(initial)))
        }),
    )
}

enum StatusKind {
    Info,
    Warn,
    Error,
}

struct Status {
    text: String,
    kind: StatusKind,
}

impl Status {
    fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: StatusKind::Info,
        }
    }
    fn warn(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: StatusKind::Warn,
        }
    }
    fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: StatusKind::Error,
        }
    }
}

struct App {
    catalog: Option<Catalog>,
    selected: Option<String>,
    search: String,

    rename_buffer: Option<String>,
    status: Option<Status>,
    previews: PreviewCache,

    wells: Vec<(Scale, egui::Rect)>,
}

impl App {
    fn new(initial: Option<PathBuf>) -> Self {
        let mut app = Self {
            catalog: None,
            selected: None,
            search: String::new(),
            rename_buffer: None,
            status: None,
            previews: PreviewCache::default(),
            wells: Vec::new(),
        };
        if let Some(path) = initial {
            app.open_catalog(path);
        }
        app
    }

    fn open_catalog(&mut self, path: PathBuf) {
        match Catalog::open(&path) {
            Ok(catalog) => {
                let count = catalog.sets.len();
                self.selected = catalog.sets.first().map(|s| s.name.clone());
                self.catalog = Some(catalog);
                self.previews.clear();
                self.rename_buffer = None;
                self.status = Some(Status::info(format!(
                    "{} — {count} image set(s)",
                    path.display()
                )));
            }
            Err(e) => self.status = Some(Status::error(e.to_string())),
        }
    }

    fn apply<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Catalog) -> swiftflow_assets::Result<()>,
    {
        let Some(catalog) = self.catalog.as_mut() else {
            return;
        };
        match f(catalog) {
            Ok(()) => {
                self.previews.clear();
                self.status = None;
            }
            Err(e) => self.status = Some(Status::error(e.to_string())),
        }
    }

    fn selected_set(&self) -> Option<&swiftflow_assets::ImageSet> {
        let catalog = self.catalog.as_ref()?;
        catalog.get(self.selected.as_deref()?)
    }

    fn unused_name(&self) -> String {
        let Some(catalog) = self.catalog.as_ref() else {
            return "Image".to_string();
        };
        let taken = |name: &str| catalog.sets.iter().any(|s| s.name == name);
        if !taken("Image") {
            return "Image".to_string();
        }
        (1..)
            .map(|n| format!("Image {n}"))
            .find(|n| !taken(n))
            .unwrap()
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if dropped.is_empty() {
            return;
        }

        let pointer = ctx.input(|i| i.pointer.latest_pos());

        for path in dropped {

            if path.is_dir() {
                self.open_catalog(path);
                continue;
            }

            let target = pointer.and_then(|pos| {
                self.wells
                    .iter()
                    .find(|(_, rect)| rect.contains(pos))
                    .map(|(scale, _)| *scale)
            });
            let (Some(scale), Some(name)) = (target, self.selected.clone()) else {
                self.status = Some(Status::warn(
                    "Drop an image onto one of the 1x / 2x / 3x wells",
                ));
                continue;
            };
            self.apply(|c| c.set_slot(&name, scale, &path));
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_drops(&ui.ctx().clone());

        self.toolbar(ui);
        self.sidebar(ui);
        if self.selected_set().is_some() {
            self.inspector(ui);
        }
        self.canvas(ui);
    }
}

impl App {
    fn toolbar(&mut self, parent: &mut egui::Ui) {
        egui::Panel::top("toolbar")
            .exact_size(38.0)
            .frame(
                egui::Frame::NONE
                    .fill(theme::SIDEBAR_BG)
                    .inner_margin(egui::Margin::symmetric(10, 6)),
            )
            .show(parent, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.button("Open Catalogue…").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            self.open_catalog(dir);
                        }
                    }
                    if self.catalog.is_some() && ui.button("Reload").clicked() {
                        self.previews.clear();
                        self.apply(|c| c.reload());
                    }

                    ui.add_space(8.0);
                    if let Some(status) = &self.status {
                        let color = match status.kind {
                            StatusKind::Info => theme::TEXT_DIM,
                            StatusKind::Warn => theme::WARNING,
                            StatusKind::Error => theme::ERROR,
                        };
                        ui.colored_label(color, &status.text);
                    }
                });
            });
    }
}

impl App {
    fn sidebar(&mut self, parent: &mut egui::Ui) {
        egui::Panel::left("sidebar")
            .exact_size(240.0)
            .resizable(false)
            .frame(egui::Frame::NONE.fill(theme::SIDEBAR_BG))
            .show(parent, |ui| {

                egui::Panel::bottom("sidebar_actions")
                    .exact_size(34.0)
                    .frame(
                        egui::Frame::NONE
                            .fill(theme::SIDEBAR_BG)
                            .inner_margin(egui::Margin::symmetric(8, 4)),
                    )
                    .show(ui, |ui| {
                        ui.horizontal_centered(|ui| {
                            let enabled = self.catalog.is_some();
                            if ui
                                .add_enabled(enabled, egui::Button::new("＋"))
                                .on_hover_text("New image set")
                                .clicked()
                            {
                                let name = self.unused_name();
                                self.apply(|c| c.create_set(&name));
                                self.selected = Some(name);
                                self.rename_buffer = None;
                            }
                            let has_selection = self.selected_set().is_some();
                            if ui
                                .add_enabled(has_selection, egui::Button::new("－"))
                                .on_hover_text("Delete image set")
                                .clicked()
                            {
                                if let Some(name) = self.selected.clone() {
                                    self.apply(|c| c.delete_set(&name));
                                    self.selected = self
                                        .catalog
                                        .as_ref()
                                        .and_then(|c| c.sets.first())
                                        .map(|s| s.name.clone());
                                    self.rename_buffer = None;
                                }
                            }
                        });
                    });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let width = (ui.available_width() - 8.0).max(40.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Search")
                            .desired_width(width),
                    );
                });
                ui.add_space(4.0);

                if self.catalog.is_none() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.colored_label(theme::TEXT_DIM, "No catalogue open");
                    });
                    return;
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.asset_list(ui);
                });
            });
    }

    fn asset_list(&mut self, ui: &mut egui::Ui) {
        let query = self.search.to_lowercase();
        let rows: Vec<(String, Option<PathBuf>)> = self
            .catalog
            .as_ref()
            .map(|c| {
                c.sets
                    .iter()
                    .filter(|s| query.is_empty() || s.name.to_lowercase().contains(&query))
                    .map(|s| {

                        let thumb = [Scale::Three, Scale::Two, Scale::One]
                            .iter()
                            .find_map(|&sc| s.file_for(sc));
                        (s.name.clone(), thumb)
                    })
                    .collect()
            })
            .unwrap_or_default();

        if rows.is_empty() {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                let message = if query.is_empty() {
                    "No image sets yet"
                } else {
                    "No matches"
                };
                ui.colored_label(theme::TEXT_DIM, message);
            });
            return;
        }

        for (name, thumb) in rows {
            let is_selected = self.selected.as_deref() == Some(name.as_str());
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::click());

            if is_selected {
                ui.painter().rect_filled(rect, 5.0, theme::SELECTION);
            } else if response.hovered() {
                ui.painter()
                    .rect_filled(rect, 5.0, egui::Color32::from_rgb(54, 54, 57));
            }

            let thumb_rect =
                egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 5.0), egui::Vec2::splat(20.0));
            if let Some(preview) = thumb.as_ref().and_then(|p| self.previews.get(ui.ctx(), p)) {
                let fitted = fit(thumb_rect, preview.width as f32, preview.height as f32);
                let id = preview.texture.id();
                ui.painter().image(
                    id,
                    fitted,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                ui.painter().rect_filled(thumb_rect, 3.0, theme::WELL_BG);
            }

            ui.painter().text(
                rect.min + egui::vec2(36.0, 15.0),
                egui::Align2::LEFT_CENTER,
                &name,
                egui::FontId::proportional(13.0),
                if is_selected {
                    egui::Color32::WHITE
                } else {
                    theme::TEXT
                },
            );

            if response.clicked() {
                self.selected = Some(name.clone());
                self.rename_buffer = None;
            }
        }
    }
}

impl App {
    fn canvas(&mut self, parent: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme::CANVAS_BG)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(parent, |ui| {
                self.wells.clear();

                let Some(name) = self.selected.clone() else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(120.0);
                        ui.colored_label(
                            theme::TEXT_DIM,
                            if self.catalog.is_some() {
                                "Select an image set, or press ＋ to add one"
                            } else {
                                "Open a catalogue folder to begin"
                            },
                        );
                    });
                    return;
                };
                let Some(set) = self.selected_set() else {
                    return;
                };
                let slots = set.slots();
                let dir = set.dir.clone();

                ui.label(
                    egui::RichText::new(&name)
                        .size(15.0)
                        .color(theme::TEXT)
                        .strong(),
                );
                ui.add_space(2.0);
                ui.colored_label(theme::TEXT_DIM, "Universal");
                ui.add_space(20.0);

                let dragging = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
                let hover = ui.ctx().input(|i| i.pointer.hover_pos());

                ui.horizontal(|ui| {
                    for slot in &slots {
                        let path = slot.filename.as_ref().map(|f| dir.join(f));
                        self.well(ui, &name, slot.scale, path, dragging, hover);
                        ui.add_space(18.0);
                    }
                });
            });
    }

    fn well(
        &mut self,
        ui: &mut egui::Ui,
        set_name: &str,
        scale: Scale,
        path: Option<PathBuf>,
        dragging: bool,
        hover: Option<egui::Pos2>,
    ) {
        const SIZE: f32 = 150.0;
        let (outer, _) =
            ui.allocate_exact_size(egui::vec2(SIZE, SIZE + 22.0), egui::Sense::hover());
        let rect = egui::Rect::from_min_size(outer.min, egui::Vec2::splat(SIZE));
        self.wells.push((scale, rect));

        let targeted = dragging && hover.map(|p| rect.contains(p)).unwrap_or(false);
        ui.painter().rect_filled(rect, 6.0, theme::WELL_BG);

        let mut dimensions = None;
        let loaded = path
            .as_ref()
            .and_then(|p| self.previews.get(ui.ctx(), p))
            .map(|p| (p.texture.id(), p.width, p.height, p.file_size));

        match loaded {
            Some((texture, width, height, file_size)) => {
                let fitted = fit(rect.shrink(6.0), width as f32, height as f32);
                theme::checkerboard(ui.painter(), fitted);
                ui.painter().image(
                    texture,
                    fitted,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                dimensions = Some((width, height, file_size));
            }
            None if path.is_some() => {

                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "unreadable",
                    egui::FontId::proportional(12.0),
                    theme::ERROR,
                );
            }
            None => {}
        }

        let border = if targeted {
            egui::Stroke::new(2.0, theme::WELL_ACTIVE)
        } else {
            egui::Stroke::new(1.0, theme::WELL_BORDER)
        };
        if path.is_none() && !targeted {
            dashed_rect(ui.painter(), rect, border);
        } else {
            ui.painter()
                .rect_stroke(rect, 6.0, border, egui::StrokeKind::Inside);
        }

        if path.is_some() {
            let button = egui::Rect::from_min_size(
                egui::pos2(rect.max.x - 22.0, rect.min.y + 6.0),
                egui::Vec2::splat(16.0),
            );
            let response = ui.interact(
                button,
                ui.id().with((set_name, scale.as_str(), "clear")),
                egui::Sense::click(),
            );
            let tint = if response.hovered() {
                theme::ERROR
            } else {
                theme::TEXT_DIM
            };
            ui.painter()
                .rect_filled(button, 8.0, egui::Color32::from_black_alpha(140));
            ui.painter().text(
                button.center(),
                egui::Align2::CENTER_CENTER,
                "✕",
                egui::FontId::proportional(11.0),
                tint,
            );
            if response.clicked() {
                let name = set_name.to_string();
                self.apply(|c| c.clear_slot(&name, scale));
            }
        }

        ui.painter().text(
            egui::pos2(rect.center().x, rect.max.y + 12.0),
            egui::Align2::CENTER_CENTER,
            scale.as_str(),
            egui::FontId::proportional(12.0),
            theme::TEXT_DIM,
        );

        if let Some((w, h, _)) = dimensions {
            ui.painter().text(
                egui::pos2(rect.center().x, rect.min.y - 10.0),
                egui::Align2::CENTER_CENTER,
                format!("{w} × {h}"),
                egui::FontId::proportional(10.0),
                theme::TEXT_DIM,
            );
        }
    }
}

impl App {
    fn inspector(&mut self, parent: &mut egui::Ui) {
        egui::Panel::right("inspector")
            .exact_size(230.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(theme::INSPECTOR_BG)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(parent, |ui| {
                let Some(set) = self.selected_set() else {
                    return;
                };
                let current_name = set.name.clone();
                let slots = set.slots();
                let dir = set.dir.clone();

                ui.label(
                    egui::RichText::new("Image Set")
                        .size(13.0)
                        .color(theme::TEXT)
                        .strong(),
                );
                ui.add_space(10.0);

                ui.colored_label(theme::TEXT_DIM, "Name");
                let mut buffer = self
                    .rename_buffer
                    .clone()
                    .unwrap_or_else(|| current_name.clone());
                let response =
                    ui.add(egui::TextEdit::singleline(&mut buffer).desired_width(f32::INFINITY));
                if response.changed() {
                    self.rename_buffer = Some(buffer.clone());
                }

                if response.lost_focus() && self.rename_buffer.is_some() {
                    let new_name = buffer.trim().to_string();
                    self.rename_buffer = None;
                    if !new_name.is_empty() && new_name != current_name {
                        let old = current_name.clone();
                        let renamed = new_name.clone();
                        self.apply(move |c| c.rename_set(&old, &renamed));
                        if self.status.is_none() {
                            self.selected = Some(new_name);
                        }
                    }
                }

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(8.0);
                ui.colored_label(theme::TEXT_DIM, "Images");
                ui.add_space(6.0);

                for slot in &slots {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(slot.scale.as_str())
                                .size(12.0)
                                .color(theme::TEXT),
                        );
                        ui.add_space(6.0);
                        match slot.filename.as_ref() {
                            Some(filename) => {
                                let path = dir.join(filename);
                                let detail = self
                                    .previews
                                    .get(ui.ctx(), &path)
                                    .map(|p| {
                                        format!(
                                            "{}×{} · {}",
                                            p.width,
                                            p.height,
                                            format_size(p.file_size)
                                        )
                                    })
                                    .unwrap_or_else(|| "unreadable".to_string());
                                ui.colored_label(theme::TEXT_DIM, detail);
                            }
                            None => {
                                ui.colored_label(theme::TEXT_DIM, "—");
                            }
                        }
                    });
                }

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(8.0);
                ui.colored_label(theme::TEXT_DIM, "Referenced as");
                ui.add_space(4.0);

                ui.code(format!("Image(\"{current_name}\")"));
            });
    }
}

fn fit(bounds: egui::Rect, w: f32, h: f32) -> egui::Rect {
    if w <= 0.0 || h <= 0.0 {
        return bounds;
    }
    let scale = (bounds.width() / w).min(bounds.height() / h).min(1.0);
    egui::Rect::from_center_size(bounds.center(), egui::vec2(w * scale, h * scale))
}

fn dashed_rect(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    const DASH: f32 = 5.0;
    const GAP: f32 = 4.0;

    let edge = |from: egui::Pos2, to: egui::Pos2| {
        let delta = to - from;
        let length = delta.length();
        if length <= 0.0 {
            return;
        }
        let dir = delta / length;
        let mut travelled = 0.0;
        while travelled < length {
            let end = (travelled + DASH).min(length);
            painter.line_segment([from + dir * travelled, from + dir * end], stroke);
            travelled = end + GAP;
        }
    };

    edge(rect.left_top(), rect.right_top());
    edge(rect.right_top(), rect.right_bottom());
    edge(rect.right_bottom(), rect.left_bottom());
    edge(rect.left_bottom(), rect.left_top());
}
