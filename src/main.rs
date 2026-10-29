use eframe::egui;
use image::{ImageReader, Rgba, RgbaImage};
use std::path::PathBuf;
use std::process::Command;
use std::io::Cursor;
use base64::Engine;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum AppStyle {
    Original,
    SuruBlueGrey,
    SuruInverted,
}

#[derive(Serialize, Deserialize)]
struct AppSettings {
    last_dir: Option<PathBuf>,
    style: AppStyle,
    brightness: f32,
    contrast: f32,
    zoom: f32,
}

struct SurunizerApp {
    file_path: Option<PathBuf>,
    original_image: Option<RgbaImage>,
    processed_image: Option<RgbaImage>,
    
    // State flags
    is_modified: bool,
    zoom: f32,
    style: AppStyle,
    
    // Adjustments
    brightness: f32, // -100 to 100
    contrast: f32,   // -100 to 100
    
    // Crop
    crop_enabled: bool,
    crop_x: u32,
    crop_y: u32,
    crop_w: u32,
    crop_h: u32,
    
    // Transparency Color Key
    color_key: Option<[u8; 3]>,
    color_tolerance: f32, // 0 to 100
    picking_color: bool,
    
    // Pending color pick event
    pending_pick: Option<(u32, u32)>,
    
    // Last opened directory
    last_dir: PathBuf,
}

impl SurunizerApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            file_path: None,
            original_image: None,
            processed_image: None,
            is_modified: false,
            zoom: 1.0,
            style: AppStyle::Original,
            brightness: 0.0,
            contrast: 0.0,
            crop_enabled: false,
            crop_x: 0,
            crop_y: 0,
            crop_w: 100,
            crop_h: 100,
            color_key: None,
            color_tolerance: 10.0,
            picking_color: false,
            pending_pick: None,
            last_dir: PathBuf::from("/home/zm/.icons/Suru++"),
        };

        app.load_config();

        if let Some(arg) = std::env::args().nth(1) {
            let p = PathBuf::from(arg);
            if p.exists() {
                app.open_image(p);
            }
        }

        app
    }

    fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/zm".to_string());
        PathBuf::from(home).join(".config/surunizer/config.json")
    }

    fn load_config(&mut self) {
        let path = Self::config_path();
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(settings) = serde_json::from_str::<AppSettings>(&data) {
                if let Some(dir) = settings.last_dir {
                    if dir.exists() {
                        self.last_dir = dir;
                    }
                }
                self.style = settings.style;
                self.brightness = settings.brightness;
                self.contrast = settings.contrast;
                self.zoom = settings.zoom;
            }
        }
    }

    fn save_config(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let settings = AppSettings {
            last_dir: Some(self.last_dir.clone()),
            style: self.style,
            brightness: self.brightness,
            contrast: self.contrast,
            zoom: self.zoom,
        };
        if let Ok(data) = serde_json::to_string_pretty(&settings) {
            let _ = std::fs::write(path, data);
        }
    }

    fn open_image(&mut self, path: PathBuf) {
        if let Some(parent) = path.parent() {
            self.last_dir = parent.to_path_buf();
            self.save_config();
        }

        let load_path = if path.extension().and_then(|s| s.to_str()) == Some("svg") {
            let temp_png = std::env::temp_dir().join("surunizer_temp_render.png");
            let status = Command::new("rsvg-convert")
                .arg("-o")
                .arg(&temp_png)
                .arg(&path)
                .status();

            if status.is_ok() && status.unwrap().success() {
                temp_png
            } else {
                let status_magick = Command::new("convert")
                    .arg(&path)
                    .arg(&temp_png)
                    .status();
                if status_magick.is_ok() && status_magick.unwrap().success() {
                    temp_png
                } else {
                    path.clone()
                }
            }
        } else {
            path.clone()
        };

        if let Ok(reader) = ImageReader::open(&load_path) {
            if let Ok(img) = reader.decode() {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                self.original_image = Some(rgba);
                self.file_path = Some(path);
                self.is_modified = false;
                
                // Reset crop defaults to full image
                self.crop_x = 0;
                self.crop_y = 0;
                self.crop_w = w;
                self.crop_h = h;
                
                self.update_processed();
            }
        }
    }

    fn close_image(&mut self) {
        self.original_image = None;
        self.processed_image = None;
        self.file_path = None;
        self.is_modified = false;
        self.color_key = None;
        self.crop_enabled = false;
        self.brightness = 0.0;
        self.contrast = 0.0;
    }

    fn update_processed(&mut self) {
        let Some(orig) = &self.original_image else { return };
        let (width, height) = orig.dimensions();

        // 1. Crop
        let mut img = if self.crop_enabled {
            let x = self.crop_x.min(width);
            let y = self.crop_y.min(height);
            let w = self.crop_w.min(width.saturating_sub(x)).max(1);
            let h = self.crop_h.min(height.saturating_sub(y)).max(1);
            
            image::imageops::crop_imm(orig, x, y, w, h).to_image()
        } else {
            orig.clone()
        };

        // Handle pending pick if any
        if let Some((px, py)) = self.pending_pick.take() {
            let (w, h) = img.dimensions();
            if px < w && py < h {
                let pixel = img.get_pixel(px, py);
                self.color_key = Some([pixel[0], pixel[1], pixel[2]]);
                self.picking_color = false;
            }
        }

        // Suru BlueGrey Palette:
        // #455a64 (69, 90, 100)
        // #78909c (120, 144, 156)
        // #fcefe8 (252, 239, 232)
        // #ffffff (255, 255, 255)
        let palette_normal = [
            [69, 90, 100],
            [120, 144, 156],
            [252, 239, 232],
            [255, 255, 255],
        ];

        let palette_inverted = [
            [255, 255, 255],
            [252, 239, 232],
            [120, 144, 156],
            [69, 90, 100],
        ];

        let contrast_factor = (259.0 * (self.contrast + 255.0)) / (255.0 * (259.0 - self.contrast));

        for pixel in img.pixels_mut() {
            let Rgba([r, g, b, a]) = *pixel;
            if a == 0 {
                continue;
            }

            // Transparency Color Key check
            if let Some([ck_r, ck_g, ck_b]) = self.color_key {
                let dist = (((r as f32 - ck_r as f32).powi(2) +
                             (g as f32 - ck_g as f32).powi(2) +
                             (b as f32 - ck_b as f32).powi(2)) / 3.0).sqrt();
                if dist <= self.color_tolerance * 2.55 {
                    *pixel = Rgba([0, 0, 0, 0]);
                    continue;
                }
            }

            // Brightness & Contrast
            let apply_bc = |val: u8| -> u8 {
                let mut v = val as f32 + self.brightness * 2.55;
                v = contrast_factor * (v - 128.0) + 128.0;
                v.clamp(0.0, 255.0) as u8
            };

            let nr = apply_bc(r);
            let ng = apply_bc(g);
            let nb = apply_bc(b);

            // Style Mapping
            let final_rgb = match self.style {
                AppStyle::Original => [nr, ng, nb],
                AppStyle::SuruBlueGrey | AppStyle::SuruInverted => {
                    let lum = 0.2126 * nr as f32 + 0.7152 * ng as f32 + 0.0722 * nb as f32;
                    let t = (lum / 255.0).clamp(0.0, 1.0);
                    let palette = if self.style == AppStyle::SuruBlueGrey {
                        &palette_normal
                    } else {
                        &palette_inverted
                    };

                    let idx_float = t * 3.0;
                    let idx = idx_float.floor() as usize;
                    let frac = idx_float - idx as f32;

                    if idx >= 3 {
                        palette[3]
                    } else {
                        let c1 = palette[idx];
                        let c2 = palette[idx + 1];
                        [
                            (c1[0] as f32 * (1.0 - frac) + c2[0] as f32 * frac) as u8,
                            (c1[1] as f32 * (1.0 - frac) + c2[1] as f32 * frac) as u8,
                            (c1[2] as f32 * (1.0 - frac) + c2[2] as f32 * frac) as u8,
                        ]
                    }
                }
            };

            *pixel = Rgba([final_rgb[0], final_rgb[1], final_rgb[2], a]);
        }

        self.processed_image = Some(img);
        self.is_modified = true;
        self.save_config();
    }

    fn save_to_path(&mut self, path: PathBuf) {
        if let Some(img) = &self.processed_image {
            let is_svg = path.extension().and_then(|s| s.to_str()) == Some("svg");
            
            if is_svg {
                let (w, h) = img.dimensions();
                let dyn_img = image::DynamicImage::ImageRgba8(img.clone());
                let mut png_bytes = Vec::new();
                if dyn_img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png).is_ok() {
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
                    let svg_content = format!(
                        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <image xlink:href="data:image/png;base64,{b64}" width="{w}" height="{h}"/>
</svg>"#
                    );
                    if std::fs::write(&path, svg_content).is_ok() {
                        self.file_path = Some(path);
                        self.is_modified = false;
                    } else {
                        eprintln!("Failed to write SVG file to {:?}", path);
                    }
                }
            } else {
                match img.save(&path) {
                    Ok(_) => {
                        self.file_path = Some(path);
                        self.is_modified = false;
                    }
                    Err(e) => {
                        eprintln!("Failed to save image to {:?}: {}", path, e);
                    }
                }
            }
        }
    }
}

impl eframe::App for SurunizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drag-and-drop support
        ctx.input(|i| {
            if let Some(file) = i.raw.dropped_files.first() {
                if let Some(path) = &file.path {
                    self.open_image(path.clone());
                }
            }
        });

        // Top Menu Bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Surunizer");
                ui.separator();

                if ui.button("📁 Open...").clicked() {
                    let folder = self.last_dir.clone();
                    if let Some(path) = rfd::FileDialog::new()
                        .set_directory(&folder)
                        .add_filter("Images", &["png", "jpg", "jpeg", "svg", "webp", "bmp", "ico"])
                        .pick_file()
                    {
                        self.open_image(path);
                    }
                }

                let has_image = self.original_image.is_some();
                if ui.add_enabled(has_image, egui::Button::new("❌ Close")).clicked() {
                    self.close_image();
                }

                let save_enabled = self.is_modified && self.file_path.is_some();
                if ui.add_enabled(save_enabled, egui::Button::new("💾 Save")).clicked() {
                    if let Some(path) = self.file_path.clone() {
                        self.save_to_path(path);
                    }
                }

                if ui.button("💾 Save As...").clicked() {
                    let folder = self.last_dir.clone();
                    if let Some(path) = rfd::FileDialog::new()
                        .set_directory(&folder)
                        .add_filter("SVG Image", &["svg"])
                        .add_filter("PNG Image", &["png"])
                        .add_filter("JPEG Image", &["jpg", "jpeg"])
                        .add_filter("WebP Image", &["webp"])
                        .save_file()
                    {
                        self.save_to_path(path);
                    }
                }

                if ui.button("🚪 Quit").clicked() {
                    self.save_config();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                ui.separator();
                ui.label(format!(
                    "File: {}",
                    self.file_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("None")
                ));

                if self.is_modified {
                    ui.colored_label(egui::Color32::YELLOW, "[Modified]");
                }
            });
        });

        // Left Sidebar: Controls & Adjustments
        egui::SidePanel::left("controls_panel").default_width(280.0).show(ctx, |ui| {
            ui.heading("Style & Effects");
            ui.separator();

            let mut style_changed = false;
            ui.horizontal(|ui| {
                ui.label("Style Mode:");
                if ui.selectable_value(&mut self.style, AppStyle::Original, "Original").changed() {
                    style_changed = true;
                }
                if ui.selectable_value(&mut self.style, AppStyle::SuruBlueGrey, "Style 1 (Suru)").changed() {
                    style_changed = true;
                }
                if ui.selectable_value(&mut self.style, AppStyle::SuruInverted, "Style 3 (Inverted)").changed() {
                    style_changed = true;
                }
            });

            if style_changed {
                self.update_processed();
            }

            ui.add_space(10.0);
            ui.heading("Brightness & Contrast");
            ui.separator();

            let mut bc_changed = false;
            ui.horizontal(|ui| {
                ui.label("Brightness:");
                if ui.add(egui::Slider::new(&mut self.brightness, -100.0..=100.0)).changed() {
                    bc_changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Contrast:");
                if ui.add(egui::Slider::new(&mut self.contrast, -100.0..=100.0)).changed() {
                    bc_changed = true;
                }
            });

            if ui.button("Reset Adjustments").clicked() {
                self.brightness = 0.0;
                self.contrast = 0.0;
                bc_changed = true;
            }

            if bc_changed {
                self.update_processed();
            }

            ui.add_space(10.0);
            ui.heading("Transparency Color Key");
            ui.separator();

            let mut key_changed = false;
            if let Some(ck) = self.color_key {
                ui.horizontal(|ui| {
                    ui.label(format!("Key Color: RGB({}, {}, {})", ck[0], ck[1], ck[2]));
                    if ui.button("Clear Key").clicked() {
                        self.color_key = None;
                        key_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Tolerance:");
                    if ui.add(egui::Slider::new(&mut self.color_tolerance, 0.0..=50.0)).changed() {
                        key_changed = true;
                    }
                });
            } else {
                ui.label("No transparency color keyed.");
            }

            if ui.checkbox(&mut self.picking_color, "🎯 Pick Color for Transparent").changed() {
                // Toggle picking mode
            }

            if key_changed {
                self.update_processed();
            }

            ui.add_space(10.0);
            ui.heading("Crop Image");
            ui.separator();

            let mut crop_changed = false;
            if ui.checkbox(&mut self.crop_enabled, "Enable Crop").changed() {
                crop_changed = true;
            }

            if self.crop_enabled {
                if let Some(orig) = &self.original_image {
                    let (w, h) = orig.dimensions();
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        if ui.add(egui::DragValue::new(&mut self.crop_x).range(0..=w)).changed() { crop_changed = true; }
                        ui.label("Y:");
                        if ui.add(egui::DragValue::new(&mut self.crop_y).range(0..=h)).changed() { crop_changed = true; }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        if ui.add(egui::DragValue::new(&mut self.crop_w).range(1..=w)).changed() { crop_changed = true; }
                        ui.label("Height:");
                        if ui.add(egui::DragValue::new(&mut self.crop_h).range(1..=h)).changed() { crop_changed = true; }
                    });
                }
            }

            if crop_changed {
                self.update_processed();
            }

            ui.add_space(10.0);
            ui.heading("Zoom & View");
            ui.separator();
            let mut zoom_changed = false;
            ui.horizontal(|ui| {
                ui.label("Zoom:");
                if ui.add(egui::Slider::new(&mut self.zoom, 0.1..=5.0).text("x")).changed() {
                    zoom_changed = true;
                }
                if ui.button("Reset").clicked() {
                    self.zoom = 1.0;
                    zoom_changed = true;
                }
            });
            if zoom_changed {
                self.save_config();
            }
        });

        // Central Panel: Image Viewer with auto-centering & transparency checkerboard
        let mut click_info = None;
        if let Some(img) = &self.processed_image {
            let (w, h) = img.dimensions();
            let image_data = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                img.as_raw(),
            );
            
            let texture = ctx.load_texture(
                "surunizer_image",
                image_data,
                egui::TextureOptions::LINEAR,
            );

            let sz = vec2_scale(texture.size_vec2(), self.zoom);

            egui::CentralPanel::default().show(ctx, |ui| {
                let available = ui.available_size();
                let x_margin = (available.x - sz.x).max(0.0) / 2.0;
                let y_margin = (available.y - sz.y).max(0.0) / 2.0;

                let scroll_area = egui::ScrollArea::both().auto_shrink([false, false]);
                scroll_area.show(ui, |ui| {
                    ui.add_space(y_margin);
                    ui.horizontal(|ui| {
                        ui.add_space(x_margin);
                        let (response, painter) = ui.allocate_painter(sz, egui::Sense::click_and_drag());
                        
                        draw_checkerboard(&painter, response.rect, 10.0);

                        painter.image(
                            texture.id(),
                            response.rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );

                        if self.picking_color && response.clicked() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                let rel = pos - response.rect.min;
                                let ix = ((rel.x / response.rect.width()) * w as f32) as u32;
                                let iy = ((rel.y / response.rect.height()) * h as f32) as u32;
                                click_info = Some((ix, iy));
                            }
                        }
                    });
                    ui.add_space(y_margin);
                });
            });
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.heading("No image loaded. Click 'Open...' to select an image from Suru++ icons.\n(e.g., application-blank.svg)");
                });
            });
        }

        if let Some((px, py)) = click_info {
            self.pending_pick = Some((px, py));
            self.update_processed();
        }
    }
}

fn vec2_scale(v: egui::Vec2, scale: f32) -> egui::Vec2 {
    egui::vec2(v.x * scale, v.y * scale)
}

fn draw_checkerboard(painter: &egui::Painter, rect: egui::Rect, size: f32) {
    let cols = (rect.width() / size).ceil() as i32;
    let rows = (rect.height() / size).ceil() as i32;

    for y in 0..rows {
        for x in 0..cols {
            let color = if (x + y) % 2 == 0 {
                egui::Color32::from_gray(200)
            } else {
                egui::Color32::from_gray(240)
            };
            let x0 = rect.min.x + x as f32 * size;
            let y0 = rect.min.y + y as f32 * size;
            let x1 = (x0 + size).min(rect.max.x);
            let y1 = (y0 + size).min(rect.max.y);
            painter.rect_filled(egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1)), 0.0, color);
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_title("Surunizer - Suru++ Image Customizer"),
        ..Default::default()
    };
    eframe::run_native(
        "Surunizer",
        options,
        Box::new(|cc| Ok(Box::new(SurunizerApp::new(cc)))),
    )
}
