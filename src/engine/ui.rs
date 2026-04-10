// src/ui.rs
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
    fs,
};

use chrono::Datelike;
use crossbeam_channel::Sender;
use eframe::{
    App, NativeOptions, egui::{
        self, Align2, Color32, Context, FontId, Frame, Image, Key, Layout, Margin, RichText, Rounding, ScrollArea, Sense, Slider, Stroke, Vec2
    }
};
use sysinfo::{System, RefreshKind, CpuRefreshKind};
use rand::seq::SliceRandom;

// ======================================================
// UI → Engine Commands
// ======================================================

#[derive(Debug, Clone)]
pub enum UiCommand {
    Quit,
}

// ======================================================
// WALLPAPER STATE
// ======================================================

#[derive(Default)]
struct WallpaperState {
    wallpaper_folder: PathBuf,
    image_paths: Vec<PathBuf>,
    selected_index: usize,
    slideshow_active: bool,
    slideshow_interval: f32,
    last_slide_time: Option<std::time::Instant>,
    enabled: bool,
}

impl WallpaperState {
    fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let folder = PathBuf::from(home).join("Pictures").join("Wallpapers");

        let mut state = Self {
            wallpaper_folder: folder,
            selected_index: 0,
            slideshow_active: false,
            slideshow_interval: 5.0,
            last_slide_time: None,
            enabled: true,
            ..Default::default()
        };
        state.refresh_images();
        state
    }

    fn refresh_images(&mut self) {
        self.image_paths.clear();
        if self.wallpaper_folder.exists() {
            if let Ok(entries) = fs::read_dir(&self.wallpaper_folder) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        match ext.to_string_lossy().to_lowercase().as_str() {
                            "png" | "jpg" | "jpeg" | "webp" => self.image_paths.push(path),
                            _ => {}
                        }
                    }
                }
            }
        }
        self.image_paths.sort();
    }

    fn select_image(&mut self, index: usize) {
        if index < self.image_paths.len() {
            self.selected_index = index;
        }
    }

    fn next_image(&mut self) {
        if self.image_paths.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.image_paths.len();
    }

    fn prev_image(&mut self) {
        if self.image_paths.is_empty() {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.image_paths.len() - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    fn update_slideshow(&mut self) {
        if !self.slideshow_active || !self.enabled || self.image_paths.is_empty() {
            return;
        }

        if let Some(last_time) = self.last_slide_time {
            if last_time.elapsed().as_secs_f32() >= self.slideshow_interval {
                self.next_image();
                self.last_slide_time = Some(std::time::Instant::now());
            }
        } else {
            self.last_slide_time = Some(std::time::Instant::now());
        }
    }

    fn get_current_image(&self) -> Option<PathBuf> {
        self.image_paths.get(self.selected_index).cloned()
    }
}

// ======================================================
// UI STATE
// ======================================================

struct UiState {
    sys: System,
    cpu_name: String,
    cpu_vendor: String,
    cpu_frequency: u64,
    total_ram_gb: u64,
    os: String,
    kernel: String,
    hostname: String,
    graphics_card: String,
    serial_number: String,
}

impl UiState {
    fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
        );
        sys.refresh_all();

        let hostname = std::process::Command::new("hostname")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "localhost".to_string());

        // Get CPU details
        let cpu = sys.cpus().first();
        let cpu_name = cpu.map(|c| c.brand().to_string()).unwrap_or_else(|| "Unknown CPU".into());
        let cpu_vendor = cpu.map(|c| c.vendor_id().to_string()).unwrap_or_else(|| "Unknown".into());
        let cpu_frequency = cpu.map(|c| c.frequency()).unwrap_or(0);

        // Get graphics card info (attempt to detect)
        let graphics_card = Self::detect_graphics_card();

        // Generate a pseudo serial number
        let serial_number = Self::generate_serial_number();

        Self {
            cpu_name,
            cpu_vendor,
            cpu_frequency,
            total_ram_gb: sys.total_memory() / 1024 / 1024,
            os: System::name().unwrap_or_else(|| "Unknown OS".into()),
            kernel: System::kernel_version().unwrap_or_default(),
            sys,
            hostname,
            graphics_card,
            serial_number,
        }
    }

    fn detect_graphics_card() -> String {
        if let Ok(output) = std::process::Command::new("lspci")
            .arg("-v")
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains("VGA") || line.contains("3D") || line.contains("Display") {
                    if let Some(start) = line.find(':') {
                        return line[start + 1..].trim().to_string();
                    }
                }
            }
        }
        "Unknown Graphics".to_string()
    }

    fn generate_serial_number() -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        if let Ok(output) = std::process::Command::new("hostid").output() {
            String::from_utf8_lossy(&output.stdout).hash(&mut hasher);
        }
        
        let hash = hasher.finish();
        format!("{:X}", hash).chars().take(12).collect()
    }

    fn refresh(&mut self) {
        self.sys.refresh_cpu();
        self.sys.refresh_memory();
    }

    fn get_model_name(&self) -> String {
        if let Ok(content) = fs::read_to_string("/sys/devices/virtual/dmi/id/product_name") {
            return content.trim().to_string();
        }
        
        if self.cpu_name.contains("Intel") {
            "Computer".to_string()
        } else if self.cpu_name.contains("AMD") {
            "Workstation".to_string()
        } else {
            "System".to_string()
        }
    }

    fn get_display_info(&self) -> String {
        "Display".to_string()
    }
}

// ======================================================
// GLASS THEME - Using functions instead of consts
// ======================================================

struct GlassTheme;

impl GlassTheme {
    // Background colors
    fn bg_dark() -> Color32 { Color32::from_rgba_unmultiplied(10, 12, 20, 250) }
    fn bg_sidebar() -> Color32 { Color32::from_rgba_unmultiplied(18, 20, 30, 240) }
    fn bg_card() -> Color32 { Color32::from_rgba_unmultiplied(25, 28, 40, 220) }
    fn bg_card_hover() -> Color32 { Color32::from_rgba_unmultiplied(35, 38, 55, 230) }
    fn bg_input() -> Color32 { Color32::from_rgba_unmultiplied(40, 44, 60, 200) }
    
    // Accent colors
    fn accent_primary() -> Color32 { Color32::from_rgb(100, 150, 255) }
    fn accent_success() -> Color32 { Color32::from_rgb(80, 200, 120) }
    fn accent_warning() -> Color32 { Color32::from_rgb(255, 180, 80) }
    fn accent_danger() -> Color32 { Color32::from_rgb(255, 80, 100) }
    fn accent_purple() -> Color32 { Color32::from_rgb(180, 130, 255) }
    
    // Text colors
    fn text_primary() -> Color32 { Color32::from_rgb(240, 242, 255) }
    fn text_secondary() -> Color32 { Color32::from_rgb(180, 185, 210) }
    fn text_muted() -> Color32 { Color32::from_rgb(120, 125, 150) }
    
    // Border colors
    fn border_light() -> Color32 { Color32::from_rgba_unmultiplied(255, 255, 255, 15) }
    fn border_glow() -> Color32 { Color32::from_rgba_unmultiplied(100, 150, 255, 40) }
}

// ======================================================
// MAIN APP
// ======================================================

pub struct SystemMonitorApp {
    state: UiState,
    wallpaper: Arc<Mutex<WallpaperState>>,
    should_close: Arc<AtomicBool>,
    tx: Sender<UiCommand>,
    selected_category: String,
    thumbnail_size: f32,
}

impl SystemMonitorApp {
    fn new(tx: Sender<UiCommand>) -> Self {
        Self {
            state: UiState::new(),
            wallpaper: Arc::new(Mutex::new(WallpaperState::new())),
            should_close: Arc::new(AtomicBool::new(false)),
            tx,
            selected_category: "Wallpaper".to_string(),
            thumbnail_size: 130.0,
        }
    }
}

impl App for SystemMonitorApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if self.should_close.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        ctx.input(|i| {
            if i.key_pressed(Key::Q) && i.modifiers.ctrl {
                let _ = self.tx.send(UiCommand::Quit);
                self.should_close.store(true, Ordering::SeqCst);
            }
        });

        // Update slideshow
        {
            let mut wallpaper = self.wallpaper.lock().unwrap();
            wallpaper.update_slideshow();
        }

        self.state.refresh();
        self.render_ui(ctx);

        ctx.request_repaint_after(Duration::from_millis(50));
    }
}

impl SystemMonitorApp {
    fn render_ui(&mut self, ctx: &Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().fill(GlassTheme::bg_dark()))
            .show(ctx, |ui| {
                let available = ui.available_size();
                
                ui.horizontal(|ui| {
                    // Sidebar
                    ui.vertical(|ui| {
                        ui.set_min_width(240.0);
                        ui.set_max_width(240.0);
                        ui.set_min_height(available.y);
                        
                        // Glass sidebar background
                        egui::Frame::none()
                            .fill(GlassTheme::bg_sidebar())
                            .show(ui, |ui| {
                                ui.add_space(20.0);
                                self.render_sidebar(ui);
                            });
                    });
                    
                    // Main content
                    ui.vertical(|ui| {
                        ui.set_min_width(available.x - 240.0);
                        ui.set_min_height(available.y);
                        
                        egui::Frame::none()
                            .inner_margin(Margin::symmetric(24, 20))
                            .show(ui, |ui| {
                                self.render_main_content(ui);
                            });
                    });
                });
            });
    }

    fn render_sidebar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            // Avatar circle
            let avatar_size = 40.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(avatar_size, avatar_size), Sense::hover());
            let painter = ui.painter();
            painter.circle_filled(
                rect.center(),
                avatar_size / 2.0,
                GlassTheme::accent_primary(),
            );
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "👤",
                FontId::proportional(20.0),
                Color32::WHITE,
            );
            
            ui.add_space(12.0);
            
            ui.vertical(|ui| {
                ui.label(RichText::new(&self.state.hostname).color(GlassTheme::text_primary()).strong().size(14.0));
                ui.label(RichText::new("Local Account").color(GlassTheme::text_muted()).size(11.0));
            });
        });
        
        ui.add_space(28.0);
        
        // Navigation section
        ui.label(
            RichText::new("PREFERENCES")
                .color(GlassTheme::text_muted())
                .size(10.0)
                .strong()
        );
        ui.add_space(12.0);
        
        let is_selected = self.selected_category == "Wallpaper";
        
        let response = ui.selectable_label(
            is_selected,
            RichText::new("🖼️  Wallpaper")
                .size(14.0)
                .color(if is_selected { GlassTheme::accent_primary() } else { GlassTheme::text_secondary() })
        );
        
        if response.clicked() {
            // Handle category change
        }
        
        ui.add_space(8.0);
        
        // System info in sidebar
        ui.separator();
        ui.add_space(8.0);
        
        ui.label(RichText::new("SYSTEM").color(GlassTheme::text_muted()).size(10.0).strong());
        ui.add_space(8.0);
        
        ui.label(RichText::new(&self.state.cpu_name).color(GlassTheme::text_secondary()).size(11.0));
        ui.label(RichText::new(format!("{} GB RAM", self.state.total_ram_gb)).color(GlassTheme::text_secondary()).size(11.0));
        ui.label(RichText::new(&self.state.os).color(GlassTheme::text_secondary()).size(11.0));
        
        // Bottom info
        ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(16.0);
            ui.label(
                RichText::new("v0.1.0")
                    .color(GlassTheme::text_muted())
                    .size(11.0)
            );
        });
    }

    fn render_main_content(&mut self, ui: &mut egui::Ui) {
        let now = std::time::SystemTime::now();
        let datetime: chrono::DateTime<chrono::Local> = now.into();
        
        // Header with glass effect
        egui::Frame::none()
            .fill(GlassTheme::bg_card())
            .rounding(Rounding::same(16))
            .stroke(Stroke::new(1.0, GlassTheme::border_light()))
            .inner_margin(Margin::symmetric(20, 16))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Wallpaper Manager").color(GlassTheme::text_primary()).size(22.0).strong());
                        
                        ui.label(
                            RichText::new(format!("{} {:02}", 
                                datetime.format("%A"), 
                                datetime.day()))
                                .size(14.0)
                                .color(GlassTheme::text_secondary())
                        );
                    });
                    
                    ui.with_layout(Layout::right_to_left(egui::Align::TOP), |ui| {
                        ui.label(
                            RichText::new(datetime.format("%l:%M %p").to_string().trim_start())
                                .size(14.0)
                                .color(GlassTheme::accent_primary())
                                .strong()
                        );
                    });
                });
            });
        
        ui.add_space(20.0);
        
        self.render_wallpaper_settings(ui);
    }

    fn render_wallpaper_settings(&mut self, ui: &mut egui::Ui) {
        let (enabled, image_paths, selected_index, slideshow_active, slideshow_interval) = {
            let wallpaper = self.wallpaper.lock().unwrap();
            (
                wallpaper.enabled,
                wallpaper.image_paths.clone(),
                wallpaper.selected_index,
                wallpaper.slideshow_active,
                wallpaper.slideshow_interval,
            )
        };
        
        // Settings card
        egui::Frame::none()
            .fill(GlassTheme::bg_card())
            .rounding(Rounding::same(16))
            .stroke(Stroke::new(1.0, GlassTheme::border_light()))
            .inner_margin(Margin::symmetric(20, 18))
            .show(ui, |ui| {
                ui.label(RichText::new("🖼️ Wallpaper Settings").color(GlassTheme::text_primary()).size(18.0).strong());
                ui.add_space(16.0);
                
                // Enable Wallpaper toggle with glass style
                ui.horizontal(|ui| {
                    let mut enabled_mut = enabled;
                    ui.checkbox(&mut enabled_mut, "");
                    ui.label(
                        RichText::new("Enable Wallpaper")
                            .color(if enabled_mut { GlassTheme::accent_primary() } else { GlassTheme::text_secondary() })
                            .size(14.0)
                    );
                    if enabled_mut != enabled {
                        let mut wallpaper = self.wallpaper.lock().unwrap();
                        wallpaper.enabled = enabled_mut;
                    }
                });
                
                ui.add_space(20.0);
                
                if enabled && !image_paths.is_empty() {
                    // Current wallpaper preview
                    if let Some(current_image) = image_paths.get(selected_index) {
                        ui.label(RichText::new("Current Wallpaper").color(GlassTheme::text_secondary()).size(13.0).strong());
                        ui.add_space(8.0);
                        
                        let preview_size = Vec2::new(ui.available_width() * 0.7, 220.0);
                        
                        egui::Frame::none()
                            .fill(GlassTheme::bg_input())
                            .rounding(Rounding::same(14))
                            .stroke(Stroke::new(1.5, GlassTheme::border_glow()))
                            .inner_margin(Margin::same(12))
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    let file_uri = format!("file://{}", current_image.display());
                                    ui.add(
                                        Image::new(&file_uri)
                                            .max_size(preview_size - Vec2::new(24.0, 24.0))
                                            .maintain_aspect_ratio(true)
                                    );
                                });
                            });
                        
                        ui.add_space(16.0);
                    }
                    
                    // Slideshow controls
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Slideshow:").color(GlassTheme::text_secondary()).size(13.0));
                        ui.add_space(8.0);
                        
                        if ui.button(RichText::new("◀").size(14.0).color(GlassTheme::text_primary())).clicked() {
                            let mut wallpaper = self.wallpaper.lock().unwrap();
                            wallpaper.prev_image();
                        }
                        
                        let play_pause_text = if slideshow_active { "⏸" } else { "▶" };
                        if ui.button(RichText::new(play_pause_text).size(14.0).color(GlassTheme::accent_primary())).clicked() {
                            let mut wallpaper = self.wallpaper.lock().unwrap();
                            wallpaper.slideshow_active = !wallpaper.slideshow_active;
                            if wallpaper.slideshow_active {
                                wallpaper.last_slide_time = Some(std::time::Instant::now());
                            }
                        }
                        
                        if ui.button(RichText::new("▶").size(14.0).color(GlassTheme::text_primary())).clicked() {
                            let mut wallpaper = self.wallpaper.lock().unwrap();
                            wallpaper.next_image();
                        }
                        
                        ui.add_space(16.0);
                        
                        ui.label(RichText::new("Interval:").color(GlassTheme::text_secondary()).size(13.0));
                        
                        let mut interval = slideshow_interval;
                        let slider = ui.add(
                            Slider::new(&mut interval, 1.0..=30.0)
                                .step_by(1.0)
                                .text("s")
                        );
                        if slider.changed() {
                            let mut wallpaper = self.wallpaper.lock().unwrap();
                            wallpaper.slideshow_interval = interval;
                        }
                    });
                    
                    ui.add_space(8.0);
                    
                    // Image counter
                    ui.label(
                        RichText::new(format!("📸 {} of {} images", selected_index + 1, image_paths.len()))
                            .color(GlassTheme::text_muted())
                            .size(12.0)
                    );
                } else if enabled {
                    ui.vertical_centered(|ui| {
                        ui.add_space(30.0);
                        ui.label(RichText::new("📂").size(48.0).color(GlassTheme::text_muted()));
                        ui.add_space(8.0);
                        ui.label(RichText::new("No wallpapers found").color(GlassTheme::text_secondary()).size(14.0));
                        
                        let folder = {
                            let wallpaper = self.wallpaper.lock().unwrap();
                            wallpaper.wallpaper_folder.clone()
                        };
                        
                        ui.label(
                            RichText::new(format!("Add images to: {}", folder.display()))
                                .color(GlassTheme::text_muted())
                                .size(11.0)
                        );
                    });
                }
            });
        
        if enabled && !image_paths.is_empty() {
            ui.add_space(20.0);
            
            // Gallery card
            egui::Frame::none()
                .fill(GlassTheme::bg_card())
                .rounding(Rounding::same(16))
                .stroke(Stroke::new(1.0, GlassTheme::border_light()))
                .inner_margin(Margin::symmetric(20, 18))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🖼️ Gallery").color(GlassTheme::text_primary()).size(16.0).strong());
                        
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("Thumbnail size:").color(GlassTheme::text_secondary()).size(12.0));
                            ui.add_space(8.0);
                            ui.add(
                                Slider::new(&mut self.thumbnail_size, 80.0..=200.0)
                                    .step_by(10.0)
                                    .text("px")
                            );
                        });
                    });
                    
                    ui.add_space(16.0);
                    
                    // Grid of wallpapers
                    let available_width = ui.available_width();
                    let thumb_size = self.thumbnail_size;
                    let columns = (available_width / (thumb_size + 16.0)).floor() as usize;
                    let columns = columns.max(2);
                    
                    ScrollArea::vertical()
                        .max_height(350.0)
                        .show(ui, |ui| {
                            egui::Grid::new("wallpaper_grid")
                                .spacing([12.0, 12.0])
                                .show(ui, |ui| {
                                    for (idx, path) in image_paths.iter().enumerate() {
                                        let is_selected = idx == selected_index;
                                        
                                        let bg_color = if is_selected {
                                            Color32::from_rgba_unmultiplied(100, 150, 255, 38)
                                        } else {
                                            GlassTheme::bg_input()
                                        };
                                        
                                        let border_color = if is_selected {
                                            GlassTheme::accent_primary()
                                        } else {
                                            GlassTheme::border_light()
                                        };
                                        
                                        egui::Frame::none()
                                            .fill(bg_color)
                                            .rounding(Rounding::same(12))
                                            .stroke(Stroke::new(
                                                if is_selected { 2.0 } else { 1.0 },
                                                border_color
                                            ))
                                            .inner_margin(Margin::same(8))
                                            .show(ui, |ui| {
                                                ui.set_min_size(Vec2::new(thumb_size, thumb_size + 35.0));
                                                
                                                ui.vertical(|ui| {
                                                    // Thumbnail image
                                                    let file_uri = format!("file://{}", path.display());
                                                    let img_response = ui.add(
                                                        Image::new(&file_uri)
                                                            .max_size(Vec2::new(thumb_size - 16.0, thumb_size - 16.0))
                                                            .maintain_aspect_ratio(true)
                                                            .sense(Sense::click())
                                                    );
                                                    
                                                    if img_response.clicked() {
                                                        let mut wallpaper = self.wallpaper.lock().unwrap();
                                                        wallpaper.select_image(idx);
                                                    }
                                                    
                                                    ui.add_space(4.0);
                                                    
                                                    // Filename
                                                    let name = path.file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy();
                                                    let display_name = if name.len() > 15 {
                                                        format!("{}...", &name[..12])
                                                    } else {
                                                        name.to_string()
                                                    };
                                                    
                                                    ui.label(
                                                        RichText::new(display_name)
                                                            .size(10.0)
                                                            .color(if is_selected {
                                                                GlassTheme::accent_primary()
                                                            } else {
                                                                GlassTheme::text_secondary()
                                                            })
                                                    );
                                                    
                                                    // Selection indicator
                                                    if is_selected {
                                                        ui.label(
                                                            RichText::new("✓ Selected")
                                                                .size(9.0)
                                                                .color(GlassTheme::accent_primary())
                                                        );
                                                    } else {
                                                        ui.label(
                                                            RichText::new("")
                                                                .size(9.0)
                                                        );
                                                    }
                                                });
                                            });
                                        
                                        if (idx + 1) % columns == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                    
                    ui.add_space(12.0);
                    
                    // Add wallpaper button with glass style
                    if ui.button(
                        RichText::new("+ Add Wallpapers...")
                            .color(GlassTheme::accent_primary())
                            .size(13.0)
                    ).clicked() {
                        // Would open file picker
                    }
                });
        }
    }
}

// ======================================================
// RUN UI
// ======================================================

pub fn run_ui(tx: Sender<UiCommand>) -> Result<(), eframe::Error> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1050.0, 780.0])
            .with_min_inner_size([900.0, 650.0])
            .with_title("Wallpaper Manager")
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "Wallpaper Manager",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            let mut style = (*cc.egui_ctx.style()).clone();
            
            // Dark glass theme
            style.visuals.dark_mode = true;
            style.visuals.widgets.noninteractive.bg_fill = GlassTheme::bg_input();
            style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, GlassTheme::border_light());
            style.visuals.widgets.inactive.bg_fill = GlassTheme::bg_input();
            style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, GlassTheme::border_light());
            style.visuals.widgets.hovered.bg_fill = GlassTheme::bg_card_hover();
            style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, GlassTheme::accent_primary());
            style.visuals.widgets.active.bg_fill = Color32::from_rgba_unmultiplied(100, 150, 255, 51);
            style.visuals.widgets.active.bg_stroke = Stroke::new(2.0, GlassTheme::accent_primary());
            style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(100, 150, 255, 38);
            style.visuals.selection.stroke = Stroke::new(1.5, GlassTheme::accent_primary());
            
            // Button rounding
            style.visuals.widgets.inactive.corner_radius = 8.into();
            style.visuals.widgets.hovered.corner_radius = 8.into();
            style.visuals.widgets.active.corner_radius = 8.into();
            
            cc.egui_ctx.set_style(style);
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            Ok(Box::new(SystemMonitorApp::new(tx.clone())))
        }),
    )
}