use eframe::egui;
use serde::Deserialize;
use std::{collections::BTreeMap, error::Error, fs, process::Command};

#[derive(Debug, Deserialize, Clone)]
struct AppEntry {
    run: String,
    icon: String,
}

type AppConfig = BTreeMap<String, AppEntry>;

const GRID_ROWS: usize = 2;
const GRID_COLS: usize = 3;

struct HtpcApp {
    apps: Vec<(String, AppEntry)>,
    selected: usize,
}

impl HtpcApp {
    fn load_from_json(path: &str) -> Result<Vec<(String, AppEntry)>, Box<dyn Error>> {
        let file = fs::read_to_string(path)?;
        let parsed: AppConfig = serde_json::from_str(&file)?;
        Ok(parsed.into_iter().collect())
    }

    fn new() -> Result<Self, Box<dyn Error>> {
        let apps = Self::load_from_json(
            shellexpand::tilde("~/.config/htpc_app_manager/apps.json")
                .to_string()
                .as_str(),
        )?;
        Ok(Self { apps, selected: 0 })
    }

    fn launch(&self, idx: usize) -> Result<(), Box<dyn Error>> {
        if let Some((_name, entry)) = self.apps.get(idx) {
            let script_path = shellexpand::tilde(&entry.run).to_string();
            let _ = Command::new("bash").arg(script_path).spawn()?;
        }

        Ok(())
    }
}

impl eframe::App for HtpcApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // 'c' closes app
        if ctx.input(|i| i.key_pressed(egui::Key::C)) {
            frame.close();
            return;
        }

        // Arrow keys move
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            if self.selected + 1 < self.apps.len() && (self.selected + 1) % GRID_COLS != 0 {
                self.selected += 1;
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            if self.selected % GRID_COLS != 0 {
                self.selected -= 1;
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            let next = self.selected + GRID_COLS;
            if next < self.apps.len() {
                self.selected = next;
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            if self.selected >= GRID_COLS {
                self.selected -= GRID_COLS;
            }
        }

        // Launch app
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.launch(self.selected).expect("Failed to launch app");
        }

        // Clock
        let now = chrono::Local::now();
        let time_string = now.format("%I:%M %p").to_string();

        egui::TopBottomPanel::top("clock_panel")
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(time_string)
                            .size(40.0)
                            .color(ui.visuals().text_color()),
                    );
                    ui.add_space(20.0);
                });
            });

        // Display apps
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();

            let tile_width = available.x / GRID_COLS as f32 * 0.75;
            let tile_height = available.y / GRID_ROWS as f32 * 0.75;

            let tile_size = egui::vec2(tile_width, tile_height);

            // Space between tiles
            let tile_gap_x = 40.0;
            let tile_gap_y = 40.0;

            let total_width = tile_width * GRID_COLS as f32;
            let total_height = tile_height * GRID_ROWS as f32;
            let offset_x = (available.x - total_width) / 2.0;
            let offset_y = (available.y - total_height) / 2.0;

            ui.add_space(offset_y);

            for row in 0..GRID_ROWS {
                ui.horizontal(|ui| {
                    ui.add_space(offset_x);

                    for col in 0..GRID_COLS {
                        let idx = row * GRID_COLS + col;
                        let (rect, _) = ui.allocate_exact_size(tile_size, egui::Sense::hover());

                        // Draw background
                        if let Some((_name, app)) = self.apps.get(idx) {
                            let bg_color = if idx == self.selected {
                                ui.visuals().selection.bg_fill
                            } else {
                                ui.visuals().faint_bg_color
                            };
                            ui.painter().rect_filled(rect, 12.0, bg_color);

                            // Draw icon
                            if let Some(texture) =
                                load_texture(ui, &format!("icon_{}", idx), &app.icon)
                            {
                                let padding = rect.width() * 0.10;

                                let icon_rect = egui::Rect::from_min_max(
                                    rect.min + egui::vec2(padding, padding),
                                    rect.max - egui::vec2(padding, padding),
                                );

                                ui.painter().image(
                                    texture.id(),
                                    icon_rect,
                                    egui::Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    egui::Color32::WHITE,
                                );
                            }
                        }
                        // Horizontal spacing between tiles
                        if col < GRID_COLS - 1 {
                            ui.add_space(tile_gap_x);
                        }
                    }
                });
                // Vertical spacing between tiles
                if row < GRID_ROWS - 1 {
                    ui.add_space(tile_gap_y);
                }
            }
        });
    }
}

// Load icon texture from file
fn load_texture(ui: &egui::Ui, name: &str, path: &str) -> Option<egui::TextureHandle> {
    let path = shellexpand::tilde(path).to_string();
    let data = fs::read(path).ok()?;
    let image = image::load_from_memory(&data).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    Some(ui.ctx().load_texture(name, color_image, Default::default()))
}

fn main() {
    let options = eframe::NativeOptions {
        fullscreen: true,
        always_on_top: false,
        ..Default::default()
    };

    let _ = eframe::run_native(
        "HTPC App Manager",
        options,
        Box::new(|_cc| Box::new(HtpcApp::new().expect("Failed to create apps"))),
    );
}
