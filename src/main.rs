use eframe::egui;
use gilrs::{Button, EventType, Gilrs};
use serde::Deserialize;
use std::{error::Error, fs, process::Command};

#[derive(Debug, Deserialize, Clone)]
struct AppEntry {
    #[allow(dead_code)]
    name: String, // Name for interacting with json, not used
    run: String,
    icon: String,
}

type AppConfig = Vec<AppEntry>;

const GRID_ROWS: usize = 2;
const GRID_COLS: usize = 3;

struct HtpcApp {
    apps: Vec<AppEntry>,
    selected: usize,
    bg_texture: Option<egui::TextureHandle>,
    animation_start: Option<std::time::Instant>,
    animation_idx: Option<usize>,
    gilrs: Gilrs,
    last_any_pressed: bool,
}

impl HtpcApp {
    fn load_from_json(path: &str) -> Result<Vec<AppEntry>, Box<dyn Error>> {
        let file = fs::read_to_string(path)?;
        let apps: AppConfig = serde_json::from_str(&file)?;
        Ok(apps)
    }

    fn new() -> Result<Self, Box<dyn Error>> {
        let apps = Self::load_from_json(
            shellexpand::tilde("~/.config/htpc_app_manager/apps.json")
                .to_string()
                .as_str(),
        )?;

        let gilrs = Gilrs::new().unwrap();

        // Open gamepad
        for (_id, gamepad) in gilrs.gamepads() {
            println!("Found gamepad: {}", gamepad.name());
        }

        Ok(Self {
            apps,
            selected: 0,
            bg_texture: None,
            animation_start: None,
            animation_idx: None,
            gilrs: Gilrs::new().unwrap(),
            last_any_pressed: false,
        })
    }

    fn launch(&self, idx: usize) -> Result<(), Box<dyn Error>> {
        if let Some(entry) = self.apps.get(idx) {
            let script_path = shellexpand::tilde(&entry.run).to_string();
            let _ = Command::new("bash").arg(script_path).spawn()?;
        }

        Ok(())
    }
    fn gamepad_actions(&mut self) -> (bool, bool, bool, bool, bool) {
        let mut left = false;
        let mut right = false;
        let mut up = false;
        let mut down = false;
        let mut activate = false;

        self.gilrs.inc();

        // Event queue
        while let Some(ev) = self.gilrs.next_event() {
            match ev.event {
                EventType::ButtonPressed(Button::DPadLeft, _) => left = true,
                EventType::ButtonPressed(Button::DPadRight, _) => right = true,
                EventType::ButtonPressed(Button::DPadUp, _) => up = true,
                EventType::ButtonPressed(Button::DPadDown, _) => down = true,
                EventType::ButtonPressed(Button::South, _) => activate = true,
                _ => {}
            }
        }

        // Polling state
        for (_id, gamepad) in self.gilrs.gamepads() {
            if gamepad.is_pressed(Button::DPadLeft) {
                left = true;
            }
            if gamepad.is_pressed(Button::DPadRight) {
                right = true;
            }
            if gamepad.is_pressed(Button::DPadUp) {
                up = true;
            }
            if gamepad.is_pressed(Button::DPadDown) {
                down = true;
            }
            if gamepad.is_pressed(Button::South) {
                activate = true;
            }
        }

        // Debounce by frame
        let any_pressed = left || right || up || down || activate;
        let trigger = any_pressed && !self.last_any_pressed;

        self.last_any_pressed = any_pressed;

        if trigger {
            (left, right, up, down, activate)
        } else {
            (false, false, false, false, false)
        }
    }
}

impl eframe::App for HtpcApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Keeps input active for dpad
        ctx.request_repaint();

        // Update every 30s for clock
        ctx.request_repaint_after(std::time::Duration::from_secs(30));

        let focused = frame.info().window_info.focused;

        // Checks if the home screen is focused before taking input
        if focused {
            // Sets up gamepad/keyboard actions
            let (gp_left, gp_right, gp_up, gp_down, gp_activate) = self.gamepad_actions();
            let key_right = ctx.input(|i| i.key_pressed(egui::Key::ArrowRight));
            let key_left = ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft));
            let key_down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
            let key_up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
            let key_enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));

            // 'c' closes app
            if ctx.input(|i| i.key_pressed(egui::Key::C)) {
                frame.close();
                return;
            }

            // Arrow keys move
            if key_right || gp_right {
                if self.selected + 1 < self.apps.len() && (self.selected + 1) % GRID_COLS != 0 {
                    self.selected += 1;
                }
            }
            if key_left || gp_left {
                if self.selected % GRID_COLS != 0 {
                    self.selected -= 1;
                }
            }
            if key_down || gp_down {
                let next = self.selected + GRID_COLS;
                if next < self.apps.len() {
                    self.selected = next;
                }
            }
            if key_up || gp_up {
                if self.selected >= GRID_COLS {
                    self.selected -= GRID_COLS;
                }
            }

            // Launch App
            if key_enter || gp_activate {
                self.animation_start = Some(std::time::Instant::now());
                self.animation_idx = Some(self.selected);
                self.launch(self.selected).expect("Failed to launch app");
            }
        }

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

            // Load background
            if self.bg_texture.is_none() {
                if let Some(tex) = load_texture(
                    ui,
                    "background",
                    shellexpand::tilde("~/.config/htpc_app_manager/background.jpg")
                        .to_string()
                        .as_str(),
                ) {
                    self.bg_texture = Some(tex);
                }
            }

            let screen_rect = ctx.screen_rect();
            let screen_w = screen_rect.width();
            let screen_h = screen_rect.height();

            // Draw background
            if let Some(bg) = &self.bg_texture {
                let img_w = bg.size()[0] as f32;
                let img_h = bg.size()[1] as f32;

                let screen_aspect = screen_w / screen_h;
                let img_aspect = img_w / img_h;

                let (uv_min, uv_max) = if img_aspect > screen_aspect {
                    let scale = screen_h / img_h;
                    let scaled_w = img_w * scale;
                    let excess = (scaled_w - screen_w) / scaled_w;
                    let crop = excess / 2.0;
                    (egui::pos2(crop, 0.0), egui::pos2(1.0 - crop, 1.0))
                } else {
                    let scale = screen_w / img_w;
                    let scaled_h = img_h * scale;
                    let excess = (scaled_h - screen_h) / scaled_h;
                    let crop = excess / 2.0;
                    (egui::pos2(0.0, crop), egui::pos2(1.0, 1.0 - crop))
                };

                let painter = ctx.layer_painter(egui::LayerId::background());

                painter.image(
                    bg.id(),
                    screen_rect,
                    egui::Rect {
                        min: uv_min,
                        max: uv_max,
                    },
                    egui::Color32::WHITE,
                );
            }

            // Draw tint
            let painter = ctx.layer_painter(egui::LayerId::background());
            painter.rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
            );

            // Add top buffer
            ui.add_space(offset_y);

            for row in 0..GRID_ROWS {
                ui.horizontal(|ui| {
                    ui.add_space(offset_x);

                    for col in 0..GRID_COLS {
                        let idx = row * GRID_COLS + col;
                        let (rect, _) = ui.allocate_exact_size(tile_size, egui::Sense::hover());

                        // Draw background
                        if let Some(app) = self.apps.get(idx) {
                            let bg_color = if idx == self.selected {
                                ui.visuals().selection.bg_fill
                            } else {
                                ui.visuals().faint_bg_color
                            };
                            ui.painter().rect_filled(rect, 12.0, bg_color);

                            // Flash animation on press
                            if Some(idx) == self.animation_idx {
                                if let Some(start) = self.animation_start {
                                    let elapsed = start.elapsed().as_secs_f32();
                                    let duration = 0.25;

                                    if elapsed < duration {
                                        // Flash overlay
                                        let alpha = (1.0 - (elapsed / duration)).clamp(0.0, 1.0);
                                        let flash = egui::Color32::from_rgba_unmultiplied(
                                            255,
                                            255,
                                            255,
                                            (200.0 * alpha) as u8,
                                        );
                                        ui.painter().rect_filled(rect, 12.0, flash);

                                        ctx.request_repaint(); // Animate
                                    } else {
                                        self.animation_idx = None;
                                        self.animation_start = None;
                                    }
                                }
                            }

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

        // Clock
        let now = chrono::Local::now();
        let time_string = now.format("%I:%M %p").to_string();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            "clock_layer".into(),
        ));

        let screen_rect = ctx.screen_rect();
        let pos = egui::pos2(screen_rect.max.x - 20.0, screen_rect.min.y + 20.0);

        painter.text(
            pos,
            egui::Align2::RIGHT_TOP,
            time_string,
            egui::FontId::proportional(100.0),
            egui::Color32::WHITE,
        );
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
