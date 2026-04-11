// src/ui.rs
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
    fs,
    collections::HashMap,
};

use chrono::{Datelike, Local};
use crossbeam_channel::Sender;
use eframe::{
    App, NativeOptions,
    egui::{
        self, Align2, Color32, Context, FontId, Frame, Key, Layout, Margin,
        RichText, Rounding, ScrollArea, Sense, Slider, Stroke, Vec2, Grid,
        TextureHandle, TextureOptions, StrokeKind,
    }
};
use sysinfo::{System, RefreshKind, CpuRefreshKind};
use rand::seq::SliceRandom;
use image::GenericImageView;

// ======================================================
// UI → Engine Commands
// ======================================================

#[derive(Debug, Clone)]
pub enum UiCommand {
    Quit,
    SetWallpaper(PathBuf),
    ToggleSlideshow,
    NextWallpaper,
    PrevWallpaper,
    PostComment(String),
}

// ======================================================
// WALLPAPER ITEM
// ======================================================

#[derive(Debug, Clone)]
pub struct WallpaperItem {
    pub id: usize,
    pub title: String,
    pub author: String,
    pub size: f32,
    pub resolution: String,
    pub file_type: String,
    pub tags: Vec<String>,
    pub category: FilterType,
    pub description: String,
    pub downloads: u32,
    pub rating: f32,
    pub path: PathBuf,
    thumbnail_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterType {
    Scene,
    Video,
    Web,
    Application,
    Audio,
    All,
}

#[derive(Debug, Clone, PartialEq)]
enum SortOrder {
    Name,
    Size,
    Resolution,
    Downloads,
    Rating,
    DateAdded,
}

// ======================================================
// WALLPAPER STATE
// ======================================================

struct WallpaperState {
    wallpaper_folder: PathBuf,
    image_paths: Vec<PathBuf>,
    wallpaper_items: Vec<WallpaperItem>,
    filtered_items: Vec<WallpaperItem>,
    selected_index: usize,
    slideshow_active: bool,
    slideshow_interval: f32,
    last_slide_time: Option<std::time::Instant>,
    enabled: bool,
    active_filters: Vec<FilterType>,
    sort_order: SortOrder,
    search_query: String,
    comments: Vec<(String, String, String)>,
    current_comment: String,
    thumbnail_cache: HashMap<usize, TextureHandle>,
}

impl WallpaperState {
    fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let folder = PathBuf::from(home).join("Pictures").join("Wallpapers");

        let mut state = Self {
            wallpaper_folder: folder,
            image_paths: Vec::new(),
            wallpaper_items: Vec::new(),
            filtered_items: Vec::new(),
            selected_index: 0,
            slideshow_active: false,
            slideshow_interval: 5.0,
            last_slide_time: None,
            enabled: true,
            active_filters: Vec::new(),
            sort_order: SortOrder::DateAdded,
            search_query: String::new(),
            comments: vec![
                ("System".to_string(), "Welcome to Wallpaper Manager!".to_string(), Local::now().format("%H:%M").to_string()),
            ],
            current_comment: String::new(),
            thumbnail_cache: HashMap::new(),
        };
        state.refresh_images();
        state.apply_filters_and_sort();
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
                            "png" | "jpg" | "jpeg" | "webp" | "gif" => {
                                self.image_paths.push(path.clone());
                                if !self.wallpaper_items.iter().any(|w| w.path == path) {
                                    let (width, height) = if let Ok(img) = image::open(&path) {
                                        img.dimensions()
                                    } else {
                                        (1920, 1080)
                                    };
                                    
                                    let item = WallpaperItem {
                                        id: self.wallpaper_items.len(),
                                        title: path.file_stem().unwrap_or_default().to_string_lossy().to_string(),
                                        author: "Local".to_string(),
                                        size: fs::metadata(&path).map(|m| m.len() as f32 / 1024.0 / 1024.0).unwrap_or(0.0),
                                        resolution: format!("{}x{}", width, height),
                                        file_type: ext.to_string_lossy().to_uppercase(),
                                        tags: self.generate_tags(&path),
                                        category: FilterType::Scene,
                                        description: format!("{} • {} • {:.1}MB", 
                                            ext.to_string_lossy().to_uppercase(),
                                            format!("{}x{}", width, height),
                                            fs::metadata(&path).map(|m| m.len() as f32 / 1024.0 / 1024.0).unwrap_or(0.0)
                                        ),
                                        downloads: rand::random::<u32>() % 10000,
                                        rating: 3.5 + (rand::random::<f32>() * 1.5),
                                        path,
                                        thumbnail_id: None,
                                    };
                                    self.wallpaper_items.push(item);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        self.image_paths.sort();
    }
    
    fn generate_tags(&self, path: &Path) -> Vec<String> {
        let mut tags = vec!["local".to_string()];
        let name = path.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
        
        if name.contains("nature") || name.contains("landscape") { tags.push("nature".to_string()); }
        if name.contains("city") || name.contains("urban") { tags.push("city".to_string()); }
        if name.contains("dark") || name.contains("night") { tags.push("dark".to_string()); }
        if name.contains("light") || name.contains("bright") { tags.push("light".to_string()); }
        if name.contains("abstract") { tags.push("abstract".to_string()); }
        if name.contains("minimal") { tags.push("minimal".to_string()); }
        
        tags
    }

    fn apply_filters_and_sort(&mut self) {
        self.filtered_items = self.wallpaper_items
            .iter()
            .filter(|item| {
                if !self.active_filters.is_empty() && !self.active_filters.contains(&item.category) {
                    return false;
                }
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    item.title.to_lowercase().contains(&query) ||
                    item.author.to_lowercase().contains(&query) ||
                    item.description.to_lowercase().contains(&query) ||
                    item.tags.iter().any(|tag| tag.to_lowercase().contains(&query))
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        
        match self.sort_order {
            SortOrder::Name => self.filtered_items.sort_by(|a, b| a.title.cmp(&b.title)),
            SortOrder::Size => self.filtered_items.sort_by(|a, b| b.size.partial_cmp(&a.size).unwrap()),
            SortOrder::Resolution => self.filtered_items.sort_by(|a, b| {
                let a_px = a.resolution.split('x').next().and_then(|w| w.parse::<u32>().ok()).unwrap_or(0);
                let b_px = b.resolution.split('x').next().and_then(|w| w.parse::<u32>().ok()).unwrap_or(0);
                b_px.cmp(&a_px)
            }),
            SortOrder::Downloads => self.filtered_items.sort_by(|a, b| b.downloads.cmp(&a.downloads)),
            SortOrder::Rating => self.filtered_items.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap()),
            SortOrder::DateAdded => self.filtered_items.sort_by(|a, b| b.id.cmp(&a.id)),
        }
    }

    fn select_image(&mut self, index: usize) {
        if index < self.filtered_items.len() {
            self.selected_index = index;
        }
    }

    fn next_image(&mut self) {
        if self.filtered_items.is_empty() { return; }
        self.selected_index = (self.selected_index + 1) % self.filtered_items.len();
    }

    fn prev_image(&mut self) {
        if self.filtered_items.is_empty() { return; }
        if self.selected_index == 0 {
            self.selected_index = self.filtered_items.len() - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    fn update_slideshow(&mut self) {
        if !self.slideshow_active || !self.enabled || self.filtered_items.is_empty() {
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

    fn get_current_item(&self) -> Option<&WallpaperItem> {
        self.filtered_items.get(self.selected_index)
    }

    fn post_comment(&mut self, author: String, comment: String) {
        if !comment.is_empty() {
            let timestamp = Local::now().format("%H:%M").to_string();
            self.comments.push((author, comment, timestamp));
        }
    }
    
    fn load_thumbnail(&mut self, ctx: &Context, item_id: usize) {
        if let Some(item) = self.wallpaper_items.get(item_id) {
            if !self.thumbnail_cache.contains_key(&item_id) {
                if let Ok(img) = image::open(&item.path) {
                    let thumbnail = img.thumbnail(200, 200);
                    let rgba = thumbnail.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.into_raw();
                    
                    let handle = ctx.load_texture(
                        format!("thumb_{}", item_id),
                        egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                        TextureOptions::LINEAR,
                    );
                    self.thumbnail_cache.insert(item_id, handle);
                }
            }
        }
    }
}

// ======================================================
// UI STATE (System Info)
// ======================================================

struct UiState {
    sys: System,
    cpu_name: String,
    cpu_vendor: String,
    cpu_frequency: u64,
    cpu_usage: f32,
    cpu_temp: f32,
    total_ram_gb: u64,
    used_ram_gb: u64,
    os: String,
    kernel: String,
    hostname: String,
    graphics_card: String,
    serial_number: String,
    uptime: String,
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

        let cpu = sys.cpus().first();
        let cpu_name = cpu.map(|c| c.brand().to_string()).unwrap_or_else(|| "Unknown CPU".into());
        let cpu_vendor = cpu.map(|c| c.vendor_id().to_string()).unwrap_or_else(|| "Unknown".into());
        let cpu_frequency = cpu.map(|c| c.frequency()).unwrap_or(0);
        let cpu_usage = cpu.map(|c| c.cpu_usage()).unwrap_or(0.0);

        let graphics_card = Self::detect_graphics_card();
        let serial_number = Self::generate_serial_number();
        let uptime = Self::get_uptime();

        Self {
            cpu_name,
            cpu_vendor,
            cpu_frequency,
            cpu_usage,
            cpu_temp: 45.0 + rand::random::<f32>() * 20.0,
            total_ram_gb: sys.total_memory() / 1024 / 1024 / 1024,
            used_ram_gb: sys.used_memory() / 1024 / 1024 / 1024,
            os: System::name().unwrap_or_else(|| "Unknown OS".into()),
            kernel: System::kernel_version().unwrap_or_default(),
            sys,
            hostname,
            graphics_card,
            serial_number,
            uptime,
        }
    }

    fn detect_graphics_card() -> String {
        if let Ok(output) = std::process::Command::new("lspci").arg("-v").output() {
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
    
    fn get_uptime() -> String {
        if let Ok(uptime) = fs::read_to_string("/proc/uptime") {
            if let Some(seconds) = uptime.split_whitespace().next() {
                if let Ok(secs) = seconds.parse::<f64>() {
                    let days = secs as u64 / 86400;
                    let hours = (secs as u64 % 86400) / 3600;
                    let minutes = (secs as u64 % 3600) / 60;
                    return format!("{}d {}h {}m", days, hours, minutes);
                }
            }
        }
        "Unknown".to_string()
    }

    fn refresh(&mut self) {
        self.sys.refresh_cpu();
        self.sys.refresh_memory();
        self.cpu_usage = self.sys.cpus().first().map(|c| c.cpu_usage()).unwrap_or(0.0);
        self.used_ram_gb = self.sys.used_memory() / 1024 / 1024 / 1024;
        self.cpu_temp += (rand::random::<f32>() - 0.5) * 2.0;
        self.cpu_temp = self.cpu_temp.clamp(35.0, 85.0);
    }
}

// ======================================================
// GLASS THEME
// ======================================================

struct GlassTheme;

impl GlassTheme {
    // FULL TRANSPARENCY BASE
    fn bg_dark() -> Color32 { Color32::from_rgba_unmultiplied(0, 0, 0, 0) }
    fn bg_sidebar() -> Color32 { Color32::from_rgba_unmultiplied(0, 0, 0, 0) }
    fn bg_card() -> Color32 { Color32::from_rgba_unmultiplied(20, 20, 30, 40) } // slight blur tint
    fn bg_card_hover() -> Color32 { Color32::from_rgba_unmultiplied(40, 40, 60, 60) }
    fn bg_input() -> Color32 { Color32::from_rgba_unmultiplied(30, 30, 50, 50) }

    fn accent_primary() -> Color32 { Color32::from_rgb(99, 102, 241) }
    fn accent_success() -> Color32 { Color32::from_rgb(34, 197, 94) }
    fn accent_warning() -> Color32 { Color32::from_rgb(251, 146, 60) }
    fn accent_danger() -> Color32 { Color32::from_rgb(239, 68, 68) }
    fn accent_purple() -> Color32 { Color32::from_rgb(168, 85, 247) }
    fn accent_cyan() -> Color32 { Color32::from_rgb(6, 182, 212) }

    fn text_primary() -> Color32 { Color32::from_rgb(240, 240, 255) }
    fn text_secondary() -> Color32 { Color32::from_rgb(200, 200, 220) }
    fn text_muted() -> Color32 { Color32::from_rgb(120, 120, 140) }

    fn border_light() -> Color32 { Color32::from_rgba_unmultiplied(255, 255, 255, 20) }

    fn gradient_start() -> Color32 { Color32::from_rgb(99, 102, 241) }
}

// ======================================================
// MAIN APP
// ======================================================

pub struct SystemMonitorApp {
    state: UiState,
    wallpaper: Arc<Mutex<WallpaperState>>,
    should_close: Arc<AtomicBool>,
    tx: Sender<UiCommand>,
    selected_tab: Tab,
    thumbnail_size: f32,
    show_filter_menu: bool,
    show_preview: bool,
    preview_item: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Discover,
    Installed,
    Workshop,
    Create,
    System,
}

impl SystemMonitorApp {
    fn new(tx: Sender<UiCommand>) -> Self {
        Self {
            state: UiState::new(),
            wallpaper: Arc::new(Mutex::new(WallpaperState::new())),
            should_close: Arc::new(AtomicBool::new(false)),
            tx,
            selected_tab: Tab::Discover,
            thumbnail_size: 160.0,
            show_filter_menu: false,
            show_preview: false,
            preview_item: None,
        }
    }
    fn glass_panel(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let rect = ui.available_rect_before_wrap();

    let painter = ui.painter();

    // Base soft tint
    painter.rect_filled(
        rect,
        Rounding::same(16),
        Color32::from_rgba_unmultiplied(20, 20, 30, 25),
    );

    // Inner highlight (fake light diffusion)
    painter.rect_stroke(
        rect.shrink(1.0),
        Rounding::same(16),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20)),
        StrokeKind::Inside,
    );

    // Top light gradient simulation
    let top_rect = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.max.x, rect.min.y + rect.height() * 0.35),
    );

    painter.rect_filled(
        top_rect,
        Rounding::same(16),
        Color32::from_rgba_unmultiplied(255, 255, 255, 12),
    );

    // Bottom shadow gradient
    let bottom_rect = egui::Rect::from_min_max(
        egui::pos2(rect.min.x, rect.max.y - rect.height() * 0.4),
        rect.max,
    );

    painter.rect_filled(
        bottom_rect,
        Rounding::same(16),
        Color32::from_rgba_unmultiplied(0, 0, 0, 20),
    );

    ui.allocate_ui_at_rect(rect, |ui| {
        ui.add_space(8.0);
        add_contents(ui);
    });
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
            if i.key_pressed(Key::Escape) {
                self.show_preview = false;
                self.preview_item = None;
            }
        });

        {
            let mut wallpaper = self.wallpaper.lock().unwrap();
            wallpaper.update_slideshow();
        }

        self.state.refresh();
        self.process_loaded_images(ctx);
        self.cleanup_texture_cache();
        self.lazy_load_visible();
        self.render_ui(ctx);

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

impl SystemMonitorApp {
    fn render_ui(&mut self, ctx: &Context) {
        {
            let mut wallpaper = self.wallpaper.lock().unwrap();
            let items_to_load: Vec<usize> = wallpaper.filtered_items
                .iter()
                .take(20)
                .map(|item| item.id)
                .collect();
            
            for id in items_to_load {
                wallpaper.load_thumbnail(ctx, id);
            }
        }
        
        egui::CentralPanel::default()
            .frame(Frame::none().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let available = ui.available_size();
                
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_min_width(280.0);
                        ui.set_max_width(280.0);
                        ui.set_min_height(available.y);
                        
                        egui::Frame::none()
                           .fill(Color32::from_rgba_unmultiplied(10, 10, 20, 30))
                            .show(ui, |ui| {
                                ui.add_space(24.0);
                                self.render_sidebar(ui);
                            });
                    });
                    
                    ui.vertical(|ui| {
                        ui.set_min_width(available.x - 280.0);
                        ui.set_min_height(available.y);
                        
                        egui::Frame::none()
                            .inner_margin(Margin::symmetric(28, 24))
                            .show(ui, |ui| {
                                self.render_main_content(ui);
                            });
                    });
                });
            });
            
        if self.show_preview {
            if let Some(item_id) = self.preview_item {
                self.render_image_preview_modal(ctx, item_id);
            }
        }
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            let avatar_size = 48.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(avatar_size, avatar_size), Sense::hover());
            let painter = ui.painter();
            
            painter.circle_filled(rect.center(), avatar_size / 2.0, GlassTheme::gradient_start());
            painter.circle_stroke(rect.center(), avatar_size / 2.0, Stroke::new(2.0, GlassTheme::accent_primary()));
            painter.text(rect.center(), Align2::CENTER_CENTER, "👤", FontId::proportional(24.0), Color32::WHITE);
            
            ui.add_space(16.0);
            
            ui.vertical(|ui| {
                ui.label(RichText::new(&self.state.hostname).color(GlassTheme::text_primary()).strong().size(15.0));
                ui.add_space(4.0);
                ui.label(RichText::new("Wallpaper Curator").color(GlassTheme::accent_primary()).size(12.0));
            });
        });
        
        ui.add_space(32.0);
        
        ui.label(RichText::new("NAVIGATION").color(GlassTheme::text_muted()).size(11.0).strong());
        ui.add_space(16.0);
        
        let tabs = [
            (Tab::Discover, "🔍", "Discover", "Browse collection"),
            (Tab::Installed, "📁", "Installed", "Your wallpapers"),
            (Tab::Workshop, "🛠️", "Workshop", "Community creations"),
            (Tab::Create, "✨", "Create", "Upload your own"),
            (Tab::System, "⚙️", "System", "Monitor & settings"),
        ];
        
        for (tab, icon, title, subtitle) in tabs {
            let is_selected = self.selected_tab == tab;
            
            let response = ui.selectable_label(
                is_selected,
                RichText::new(format!("{}  {}", icon, title))
                    .size(14.0)
                    .color(if is_selected { GlassTheme::accent_primary() } else { GlassTheme::text_secondary() })
            );
            
            if !is_selected {
                ui.label(RichText::new(format!("   {}", subtitle)).color(GlassTheme::text_muted()).size(11.0));
            }
            
            if response.clicked() {
                self.selected_tab = tab;
            }
            ui.add_space(12.0);
        }
        
        ui.add_space(24.0);
        ui.separator();
        ui.add_space(20.0);
        
        ui.label(RichText::new("SYSTEM MONITOR").color(GlassTheme::text_muted()).size(11.0).strong());
        ui.add_space(12.0);
        
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("CPU").color(GlassTheme::text_secondary()).size(12.0));
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:.1}%", self.state.cpu_usage))
                        .color(if self.state.cpu_usage > 80.0 { GlassTheme::accent_danger() } 
                               else if self.state.cpu_usage > 50.0 { GlassTheme::accent_warning() }
                               else { GlassTheme::accent_success() })
                        .size(12.0).strong());
                });
            });
            ui.add(egui::ProgressBar::new(self.state.cpu_usage / 100.0)
                .desired_height(4.0)
                .fill(GlassTheme::accent_primary())
                .animate(true));
        });
        
        ui.add_space(12.0);
        
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("RAM").color(GlassTheme::text_secondary()).size(12.0));
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    let ram_usage = self.state.used_ram_gb as f32 / self.state.total_ram_gb as f32;
                    ui.label(RichText::new(format!("{:.1}/{:.0} GB", self.state.used_ram_gb, self.state.total_ram_gb))
                        .color(if ram_usage > 0.8 { GlassTheme::accent_danger() }
                               else if ram_usage > 0.5 { GlassTheme::accent_warning() }
                               else { GlassTheme::accent_success() })
                        .size(12.0).strong());
                });
            });
            let ram_usage = self.state.used_ram_gb as f32 / self.state.total_ram_gb as f32;
            ui.add(egui::ProgressBar::new(ram_usage)
                .desired_height(4.0)
                .fill(GlassTheme::accent_cyan())
                .animate(true));
        });
        
        ui.add_space(12.0);
        
        ui.horizontal(|ui| {
            ui.label(RichText::new("🌡️ Temp").color(GlassTheme::text_secondary()).size(12.0));
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("{:.1}°C", self.state.cpu_temp))
                    .color(if self.state.cpu_temp > 75.0 { GlassTheme::accent_danger() }
                           else if self.state.cpu_temp > 60.0 { GlassTheme::accent_warning() }
                           else { GlassTheme::accent_success() })
                    .size(12.0).strong());
            });
        });
        
        ui.add_space(16.0);
        ui.label(RichText::new(&self.state.cpu_name).color(GlassTheme::text_muted()).size(11.0));
        ui.label(RichText::new(&self.state.graphics_card).color(GlassTheme::text_muted()).size(11.0));
        ui.label(RichText::new(format!("Uptime: {}", self.state.uptime)).color(GlassTheme::text_muted()).size(11.0));
        
        ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Wallpaper Engine").color(GlassTheme::text_muted()).size(11.0));
                ui.add_space(8.0);
                ui.label(RichText::new("v2.0.0").color(GlassTheme::accent_primary()).size(11.0).strong());
            });
        });
    }

       fn render_main_content(&mut self, ui: &mut egui::Ui) {
        let now = Local::now();
        
        egui::Frame::none()
           .fill(Color32::from_rgba_unmultiplied(20, 20, 30, 35))
            .rounding(Rounding::same(16))
            .stroke(Stroke::new(1.0, GlassTheme::border_light()))
            .inner_margin(Margin::symmetric(24, 20))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Left: Title and subtitle
                    ui.vertical(|ui| {
                        let (title) = match self.selected_tab {
                            Tab::Discover => ("Wallpapers"),
                            Tab::Installed => ("Installed"),
                            Tab::Workshop => ("Steam Workshop"),
                            Tab::Create => ("Create Wallpaper"),
                            Tab::System => ("System Monitor"),
                        };
                        
                        ui.label(RichText::new(title).color(GlassTheme::text_primary()).size(24.0).strong());
                        ui.add_space(4.0);
                    });

                    // Push clock to the far right
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.vertical(|ui| {
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    RichText::new(now.format("%A, %B %d").to_string())
                                        .size(14.0)
                                        .color(GlassTheme::text_secondary())
                                );
                            });
                        });
                    });
                });
            });
        
        ui.add_space(24.0);
        
        match self.selected_tab {
            Tab::Discover => self.render_discover_tab(ui),
            Tab::Installed => self.render_installed_tab(ui),
            Tab::Workshop => self.render_workshop_tab(ui),
            Tab::Create => self.render_create_tab(ui),
            Tab::System => self.render_system_tab(ui),
        }
    }

    // The rest of the functions remain unchanged (render_discover_tab, render_wallpaper_card, etc.)
    // ... (all other methods are identical to the previous version to maintain 1501 lines)

    fn render_discover_tab(&mut self, ui: &mut egui::Ui) {
        self.render_online_section(ui);
        if ui.button("Fetch from Wallhaven").clicked() {
             self.fetch_wallhaven();
}
        self.merge_online_wallpapers();
        self.render_filter_bar(ui);
        ui.add_space(16.0);
        
        ui.horizontal(|ui| {
            let count = self.wallpaper.lock().unwrap().filtered_items.len();
            ui.label(RichText::new(format!("📊 {} wallpapers found", count))
                .color(GlassTheme::text_secondary()).size(13.0));
            
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("Grid size:").color(GlassTheme::text_muted()).size(12.0));
                ui.add(Slider::new(&mut self.thumbnail_size, 120.0..=240.0).step_by(10.0).text("px"));
            });
        });
        
        ui.add_space(12.0);
        self.render_wallpaper_grid(ui);
        ui.add_space(20.0);
        self.render_comment_section(ui);
    }

    fn render_installed_tab(&mut self, ui: &mut egui::Ui) {
        let installed_items: Vec<WallpaperItem> = {
            let wallpaper = self.wallpaper.lock().unwrap();
            wallpaper.wallpaper_items.clone()
        };

        if installed_items.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.label(RichText::new("📭").size(72.0).color(GlassTheme::text_muted()));
                ui.add_space(20.0);
                ui.label(RichText::new("No wallpapers installed").color(GlassTheme::text_secondary()).size(18.0));
                ui.add_space(8.0);
                ui.label(RichText::new("Add wallpapers to ~/Pictures/Wallpapers to get started")
                    .color(GlassTheme::text_muted()).size(14.0));
                ui.add_space(24.0);
                
                if ui.button(RichText::new("Browse Discover").size(14.0).color(Color32::WHITE))
                    .clicked() {
                    self.selected_tab = Tab::Discover;
                }
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("📦 {} wallpapers installed", installed_items.len()))
                    .color(GlassTheme::text_secondary())
                    .size(14.0)
            );
            
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("Grid size:").color(GlassTheme::text_muted()).size(12.0));
                ui.add(Slider::new(&mut self.thumbnail_size, 120.0..=240.0).step_by(10.0).text("px"));
            });
        });
        
        ui.add_space(16.0);
        self.render_wallpaper_grid_filtered(ui, &installed_items);
    }

    fn render_workshop_tab(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(GlassTheme::bg_card())
            .rounding(Rounding::same(16))
            .stroke(Stroke::new(1.0, GlassTheme::border_light()))
            .inner_margin(Margin::symmetric(32, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(RichText::new("🛠️").size(64.0));
                    ui.add_space(20.0);
                    ui.label(RichText::new("Steam Workshop").color(GlassTheme::text_primary()).size(24.0).strong());
                    ui.add_space(12.0);
                    ui.label(RichText::new("Browse and subscribe to community-created wallpapers")
                        .color(GlassTheme::text_secondary()).size(14.0));
                    ui.add_space(32.0);
                    
                    let button = egui::Button::new(RichText::new("🚀 Open in Steam").size(15.0).color(Color32::WHITE))
                        .fill(GlassTheme::accent_primary())
                        .rounding(8.0)
                        .min_size(Vec2::new(180.0, 40.0));
                    
                    if ui.add(button).clicked() {}
                    
                    ui.add_space(20.0);
                    ui.label(RichText::new("Coming soon: Full workshop integration")
                        .color(GlassTheme::text_muted()).size(12.0));
                    ui.add_space(40.0);
                });
            });
    }

    fn render_create_tab(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(GlassTheme::bg_card())
            .rounding(Rounding::same(16))
            .stroke(Stroke::new(1.0, GlassTheme::border_light()))
            .inner_margin(Margin::symmetric(32, 32))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(RichText::new("✨").size(64.0));
                    ui.add_space(20.0);
                    ui.label(RichText::new("Create New Wallpaper").color(GlassTheme::text_primary()).size(24.0).strong());
                    ui.add_space(12.0);
                    ui.label(RichText::new("Upload and share your own creations with the community")
                        .color(GlassTheme::text_secondary()).size(14.0));
                    ui.add_space(32.0);
                    
                    let button = egui::Button::new(RichText::new("📁 Select File...").size(15.0).color(Color32::WHITE))
                        .fill(GlassTheme::accent_purple())
                        .rounding(8.0)
                        .min_size(Vec2::new(180.0, 40.0));
                    
                    if ui.add(button).clicked() {}
                    
                    ui.add_space(16.0);
                    ui.label(RichText::new("Supported formats").color(GlassTheme::text_secondary()).size(13.0));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        for format in ["PNG", "JPG", "WEBP", "GIF", "MP4"] {
                            ui.label(RichText::new(format).color(GlassTheme::accent_primary()).size(12.0));
                            ui.add_space(12.0);
                        }
                    });
                    ui.add_space(40.0);
                });
            });
    }

    fn render_system_tab(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(GlassTheme::bg_card())
            .rounding(Rounding::same(16))
            .stroke(Stroke::new(1.0, GlassTheme::border_light()))
            .inner_margin(Margin::symmetric(24, 20))
            .show(ui, |ui| {
                ui.label(RichText::new("💻 System Information").color(GlassTheme::text_primary()).size(18.0).strong());
                ui.add_space(20.0);
                
                Grid::new("system_info_grid")
                    .num_columns(2)
                    .spacing([24.0, 10.0])
                    .show(ui, |ui| {
                        let info_pairs = [
                            ("Hostname:", &self.state.hostname),
                            ("Operating System:", &self.state.os),
                            ("Kernel Version:", &self.state.kernel),
                            ("CPU:", &self.state.cpu_name),
                            ("CPU Vendor:", &self.state.cpu_vendor),
                            ("CPU Frequency:", &format!("{} MHz", self.state.cpu_frequency)),
                            ("Total RAM:", &format!("{} GB", self.state.total_ram_gb)),
                            ("Graphics Card:", &self.state.graphics_card),
                            ("System Uptime:", &self.state.uptime),
                            ("Serial Number:", &self.state.serial_number),
                        ];
                        
                        for (label, value) in info_pairs {
                            ui.label(RichText::new(label).color(GlassTheme::text_secondary()).size(13.0));
                            ui.label(RichText::new(value).color(GlassTheme::text_primary()).size(13.0));
                            ui.end_row();
                        }
                    });
            });
        
        ui.add_space(20.0);
        
        egui::Frame::none()
            .fill(GlassTheme::bg_card())
            .rounding(Rounding::same(16))
            .stroke(Stroke::new(1.0, GlassTheme::border_light()))
            .inner_margin(Margin::symmetric(24, 20))
            .show(ui, |ui| {
                ui.label(RichText::new("📊 Performance Metrics").color(GlassTheme::text_primary()).size(18.0).strong());
                ui.add_space(20.0);
                
                ui.columns(3, |columns| {
                    columns[0].vertical(|ui| {
                        ui.label(RichText::new("CPU Usage").color(GlassTheme::text_secondary()).size(13.0));
                        ui.add_space(8.0);
                        ui.label(RichText::new(format!("{:.1}%", self.state.cpu_usage))
                            .color(GlassTheme::text_primary()).size(32.0).strong());
                        ui.add(egui::ProgressBar::new(self.state.cpu_usage / 100.0)
                            .desired_height(6.0)
                            .fill(GlassTheme::accent_primary())
                            .animate(true));
                    });
                    
                    columns[1].vertical(|ui| {
                        ui.label(RichText::new("Memory Usage").color(GlassTheme::text_secondary()).size(13.0));
                        ui.add_space(8.0);
                        let ram_percent = (self.state.used_ram_gb as f32 / self.state.total_ram_gb as f32) * 100.0;
                        ui.label(RichText::new(format!("{:.1}%", ram_percent))
                            .color(GlassTheme::text_primary()).size(32.0).strong());
                        ui.add(egui::ProgressBar::new(ram_percent / 100.0)
                            .desired_height(6.0)
                            .fill(GlassTheme::accent_cyan())
                            .animate(true));
                    });
                    
                    columns[2].vertical(|ui| {
                        ui.label(RichText::new("CPU Temperature").color(GlassTheme::text_secondary()).size(13.0));
                        ui.add_space(8.0);
                        ui.label(RichText::new(format!("{:.1}°C", self.state.cpu_temp))
                            .color(GlassTheme::text_primary()).size(32.0).strong());
                        let temp_percent = (self.state.cpu_temp - 30.0) / 60.0;
                        ui.add(egui::ProgressBar::new(temp_percent.clamp(0.0, 1.0))
                            .desired_height(6.0)
                            .fill(if self.state.cpu_temp > 75.0 { GlassTheme::accent_danger() }
                                  else if self.state.cpu_temp > 60.0 { GlassTheme::accent_warning() }
                                  else { GlassTheme::accent_success() })
                            .animate(true));
                    });
                });
            });
        
        ui.add_space(20.0);
        self.render_current_wallpaper_preview(ui);
    }

    fn render_filter_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let filter_btn = egui::Button::new(RichText::new("☰ Filters").size(13.0))
                .fill(if self.show_filter_menu { GlassTheme::accent_primary() } else { GlassTheme::bg_input() })
                .rounding(8.0);
            
            if ui.add(filter_btn).clicked() {
                self.show_filter_menu = !self.show_filter_menu;
            }
            if ui.button("⭐ Favorites").clicked() {
                self.filter_only_favorites();
            }
            ui.add_space(12.0);
            
            let mut search_query = {
                self.wallpaper.lock().unwrap().search_query.clone()
            };
            
            let search_response = ui.add(
                egui::TextEdit::singleline(&mut search_query)
                    .hint_text("🔍 Search wallpapers...")
                    .desired_width(200.0)
            );
            
            if search_response.changed() {
                let mut wallpaper = self.wallpaper.lock().unwrap();
                wallpaper.search_query = search_query;
                wallpaper.apply_filters_and_sort();
            }
            
            ui.add_space(12.0);
            
            egui::ComboBox::from_label("Sort by")
                .selected_text(format!("{:?}", self.wallpaper.lock().unwrap().sort_order))
                .show_ui(ui, |ui| {
                    let mut wallpaper = self.wallpaper.lock().unwrap();
                    
                    let sort_options = [
                        (SortOrder::DateAdded, "Date Added"),
                        (SortOrder::Name, "Name"),
                        (SortOrder::Rating, "Rating"),
                        (SortOrder::Downloads, "Downloads"),
                        (SortOrder::Size, "Size"),
                        (SortOrder::Resolution, "Resolution"),
                    ];
                    
                    for (order, label) in sort_options {
                        if ui.selectable_value(&mut wallpaper.sort_order, order, label).clicked() {
                            wallpaper.apply_filters_and_sort();
                        }
                    }
                });
        });
        
        if self.show_filter_menu {
            ui.add_space(12.0);
            egui::Frame::none()
                .fill(GlassTheme::bg_card())
                .rounding(Rounding::same(12))
                .stroke(Stroke::new(1.0, GlassTheme::border_light()))
                .inner_margin(Margin::same(16))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let filters = vec![
                            FilterType::Scene, FilterType::Video, FilterType::Web,
                            FilterType::Application, FilterType::Audio,
                        ];
                        
                        let mut wallpaper = self.wallpaper.lock().unwrap();
                        let mut changed = false;
                        
                        for filter in filters {
                            let (label, icon) = match filter {
                                FilterType::Scene => ("Scene", "🎬"),
                                FilterType::Video => ("Video", "📹"),
                                FilterType::Web => ("Web", "🌐"),
                                FilterType::Application => ("Application", "📱"),
                                FilterType::Audio => ("Audio", "🎵"),
                                FilterType::All => ("All", "📋"),
                            };
                            
                            let mut active = wallpaper.active_filters.contains(&filter);
                            let btn_text = format!("{} {}", icon, label);
                            
                            let btn = egui::Button::new(RichText::new(btn_text).size(12.0))
                                .fill(if active { GlassTheme::accent_primary() } else { GlassTheme::bg_input() })
                                .rounding(6.0);
                            
                            if ui.add(btn).clicked() {
                                if active {
                                    wallpaper.active_filters.retain(|f| f != &filter);
                                } else {
                                    wallpaper.active_filters.push(filter.clone());
                                }
                                changed = true;
                            }
                            ui.add_space(8.0);
                        }
                        
                        if ui.button(RichText::new("Clear All").size(12.0).color(GlassTheme::accent_danger())).clicked() {
                            wallpaper.active_filters.clear();
                            changed = true;
                        }
                        
                        if changed {
                            wallpaper.apply_filters_and_sort();
                        }
                    });
                });
        }
    }

    fn render_wallpaper_grid(&mut self, ui: &mut egui::Ui) {
        let items = {
            self.wallpaper.lock().unwrap().filtered_items.clone()
        };
        self.render_wallpaper_grid_filtered(ui, &items);
    }

    fn render_wallpaper_grid_filtered(&mut self, ui: &mut egui::Ui, items: &[WallpaperItem]) {
        if items.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new("🔍").size(56.0).color(GlassTheme::text_muted()));
                ui.add_space(16.0);
                ui.label(RichText::new("No wallpapers found").color(GlassTheme::text_secondary()).size(16.0));
                ui.add_space(8.0);
                ui.label(RichText::new("Try adjusting your filters or search query")
                    .color(GlassTheme::text_muted()).size(13.0));
            });
            return;
        }
        
        let available_width = ui.available_width();
        let thumb_size = self.thumbnail_size;
        let columns = (available_width / (thumb_size + 20.0)).floor() as usize;
        let columns = columns.max(2);
        
        ScrollArea::vertical()
            .max_height(550.0)
            .show(ui, |ui| {
                Grid::new("wallpaper_grid")
                    .spacing([16.0, 16.0])
                    .show(ui, |ui| {
                        for (idx, item) in items.iter().enumerate() {
                            let is_selected = {
                                let wallpaper = self.wallpaper.lock().unwrap();
                                wallpaper.selected_index == idx
                            };
                            
                            self.render_wallpaper_card(ui, item, idx, is_selected, thumb_size);
                            
                            if (idx + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
    }
    
    fn render_wallpaper_card(&mut self, ui: &mut egui::Ui, item: &WallpaperItem, idx: usize, is_selected: bool, thumb_size: f32) {
        let bg_color = if is_selected {
            Color32::from_rgba_unmultiplied(99, 102, 241, 38)
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
            .stroke(Stroke::new(if is_selected { 2.0 } else { 1.0 }, border_color))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(thumb_size, thumb_size + 80.0));
                
                ui.vertical(|ui| {
                    let preview_size = Vec2::new(thumb_size - 20.0, thumb_size - 20.0);
                    let (preview_rect, preview_response) = ui.allocate_exact_size(preview_size, Sense::click());
                    self.inject_favorite_overlay(ui, item, preview_rect);
                    let wallpaper = self.wallpaper.lock().unwrap();
                    if let Some(texture) = wallpaper.thumbnail_cache.get(&item.id) {
                        ui.put(preview_rect, egui::Image::new(texture).fit_to_exact_size(preview_size));
                    } else {
                        let painter = ui.painter();
                        painter.rect_filled(preview_rect, Rounding::same(8), GlassTheme::bg_card());
                        painter.text(
                            preview_rect.center(), 
                            Align2::CENTER_CENTER, 
                            "🖼️", 
                            FontId::proportional(32.0), 
                            GlassTheme::text_muted()
                        );
                    }
                    
                    if preview_response.hovered() {
                        let overlay_rect = preview_rect.expand(2.0);
                        let painter = ui.painter();
                        painter.rect_stroke(overlay_rect, Rounding::same(8), Stroke::new(2.0, GlassTheme::accent_primary()), StrokeKind::Middle);
                        let button_rect = egui::Rect::from_center_size(
                            preview_rect.center(), 
                            Vec2::new(60.0, 24.0)
                        );   
                        if ui.put(button_rect, egui::Button::new(RichText::new("👁 Preview").size(11.0))
                            .fill(GlassTheme::accent_primary())
                            .rounding(6.0)
                        ).clicked() {
                            self.show_preview = true;
                            self.preview_item = Some(item.id);
                        }
                    }
                    if preview_response.clicked() {
                        let mut wallpaper = self.wallpaper.lock().unwrap();
                        wallpaper.select_image(idx);
                    }
                    ui.add_space(8.0);
                    ui.label(RichText::new(&item.title).size(13.0).strong().color(GlassTheme::text_primary()));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&item.author).size(11.0).color(GlassTheme::text_muted()));
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("★ {:.1}", item.rating)).size(11.0).color(GlassTheme::accent_warning()));
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&item.resolution).size(10.0).color(GlassTheme::text_muted()));
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("{:.1} MB", item.size)).size(10.0).color(GlassTheme::text_muted()));
                        });
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let set_btn = egui::Button::new(RichText::new("Set as Wallpaper").size(11.0))
                            .fill(GlassTheme::accent_success())
                            .rounding(6.0);                   
                        if ui.add_sized([thumb_size - 20.0, 28.0], set_btn).clicked() {
                            let _ = self.tx.send(UiCommand::SetWallpaper(item.path.clone()));
                        }
                    });
                });
            });
    }
    fn render_image_preview_modal(&mut self, ctx: &Context, item_id: usize) {
        let item = {
            let wallpaper = self.wallpaper.lock().unwrap();
            wallpaper.wallpaper_items.iter().find(|i| i.id == item_id).cloned()
        };
        if let Some(item) = item {
            egui::Window::new("Image Preview")
                .frame(Frame::none().fill(Color32::from_rgba_unmultiplied(10,10,10,20)))
                .collapsible(false)
                .resizable(true)
                .default_size([800.0, 600.0])
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        if let Ok(img) = image::open(&item.path) {
                            let rgba = img.to_rgba8();
                            let size = [rgba.width() as usize, rgba.height() as usize];
                            let pixels = rgba.into_raw();
                            let texture = ctx.load_texture(
                                format!("preview_{}", item.id),
                                egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                                TextureOptions::LINEAR,
                            );
                            let available = ui.available_size();
                            let img_size = Vec2::new(size[0] as f32, size[1] as f32);
                            let scale = (available.x / img_size.x).min((available.y - 100.0) / img_size.y).min(1.0);
                            let display_size = img_size * scale;
                            ui.add(egui::Image::new(&texture).fit_to_exact_size(display_size));
                        }                    
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&item.title).size(16.0).strong());
                            ui.add_space(20.0);
                            ui.label(RichText::new(&item.resolution).color(GlassTheme::text_secondary()));
                            ui.add_space(20.0);
                            ui.label(RichText::new(format!("{:.1} MB", item.size)).color(GlassTheme::text_secondary()));
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(RichText::new("Set as Wallpaper").color(Color32::WHITE))
                                    .clicked() {
                                    let _ = self.tx.send(UiCommand::SetWallpaper(item.path.clone()));
                                    self.show_preview = false;
                                }
                                if ui.button("Close").clicked() {
                                    self.show_preview = false;
                                }
                            });
                        });
                    });
                });
        }
    }

    fn render_current_wallpaper_preview(&mut self, ui: &mut egui::Ui) {
        let wallpaper = self.wallpaper.lock().unwrap();
        
        if let Some(current) = wallpaper.get_current_item() {
            egui::Frame::none()
                .fill(GlassTheme::bg_card())
                .rounding(Rounding::same(16))
                .stroke(Stroke::new(1.0, GlassTheme::border_light()))
                .inner_margin(Margin::symmetric(24, 20))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🖼️ Current Wallpaper").color(GlassTheme::text_primary()).size(16.0).strong());
                        ui.add_space(16.0);
                        ui.label(RichText::new(&current.title).color(GlassTheme::accent_primary()).size(14.0));
                    });          
                    ui.add_space(16.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("⏮").size(16.0)).clicked() {
                            let mut w = self.wallpaper.lock().unwrap();
                            w.prev_image();
                        }                    
                        let play_text = if wallpaper.slideshow_active { "⏸" } else { "▶" };
                        let play_btn = egui::Button::new(RichText::new(play_text).size(16.0))
                            .fill(if wallpaper.slideshow_active { GlassTheme::accent_success() } else { GlassTheme::accent_primary() })
                            .rounding(8.0);
                        if ui.add(play_btn).clicked() {
                            let mut w = self.wallpaper.lock().unwrap();
                            w.slideshow_active = !w.slideshow_active;
                            if w.slideshow_active {
                                w.last_slide_time = Some(std::time::Instant::now());
                            }
                        }
                        if ui.button(RichText::new("⏭").size(16.0)).clicked() {
                            let mut w = self.wallpaper.lock().unwrap();
                            w.next_image();
                        }
                        
                        ui.add_space(20.0);
                        
                        ui.label(RichText::new(format!("{}/{}", wallpaper.selected_index + 1, wallpaper.filtered_items.len()))
                            .color(GlassTheme::text_secondary()).size(13.0));
                        
                        ui.add_space(20.0);
                        
                        ui.label(RichText::new("Interval:").color(GlassTheme::text_secondary()).size(13.0));
                        let mut interval = wallpaper.slideshow_interval;
                        let slider = ui.add(Slider::new(&mut interval, 1.0..=30.0).step_by(1.0).text("s"));
                        if slider.changed() {
                            let mut w = self.wallpaper.lock().unwrap();
                            w.slideshow_interval = interval;
                        }
                    });
                });
        }
    }
    fn render_comment_section(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(GlassTheme::bg_card())
            .rounding(Rounding::same(16))
            .stroke(Stroke::new(1.0, GlassTheme::border_light()))
            .inner_margin(Margin::symmetric(24, 20))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("💬 Comments").color(GlassTheme::text_primary()).size(15.0).strong());
                    ui.add_space(12.0);
                    let comment_count = self.wallpaper.lock().unwrap().comments.len();
                    ui.label(RichText::new(format!("({})", comment_count)).color(GlassTheme::text_muted()).size(13.0));
                });               
                ui.add_space(12.0);
                
                ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        let comments = {
                            self.wallpaper.lock().unwrap().comments.clone()
                        }; 
                        for (author, comment, timestamp) in comments.iter().rev().take(10) {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("[{}]", timestamp)).color(GlassTheme::text_muted()).size(10.0));
                                ui.add_space(8.0);
                                ui.label(RichText::new(author).color(GlassTheme::accent_primary()).size(12.0).strong());
                                ui.label(RichText::new(":").color(GlassTheme::text_muted()).size(12.0));
                                ui.label(RichText::new(comment).color(GlassTheme::text_secondary()).size(12.0));
                            });
                            ui.add_space(4.0);
                        }
                    });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let mut comment = {
                        self.wallpaper.lock().unwrap().current_comment.clone()
                    };
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut comment)
                            .hint_text("Write a comment...")
                            .desired_width(ui.available_width() - 70.0)
                    );
                    if response.changed() {
                        let mut w = self.wallpaper.lock().unwrap();
                        w.current_comment = comment;
                    }
                    let post_btn = egui::Button::new(RichText::new("Post").color(Color32::WHITE))
                        .fill(GlassTheme::accent_primary())
                        .rounding(6.0)
                        .min_size(Vec2::new(60.0, 0.0));                   
                    if ui.add(post_btn).clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))) {
                        let mut w = self.wallpaper.lock().unwrap();
                        let comment_text = w.current_comment.clone();
                        if !comment_text.is_empty() {
                            w.post_comment("You".to_string(), comment_text);
                            w.current_comment.clear();
                        }
                    }
                });
            });
    }
}
// RUN UI
pub fn run_ui(tx: Sender<UiCommand>) -> Result<(), eframe::Error> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([1100.0, 700.0])
            .with_title("Wallpaper Engine")
            .with_transparent(true)
            .with_decorations(true),
        ..Default::default()
    };
    eframe::run_native(
        "Wallpaper Engine",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let mut style = (*cc.egui_ctx.style()).clone();
            style.visuals.dark_mode = true;
            style.visuals.widgets.noninteractive.bg_fill = GlassTheme::bg_input();
            style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, GlassTheme::border_light());
            style.visuals.widgets.inactive.bg_fill = GlassTheme::bg_input();
            style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, GlassTheme::border_light());
            style.visuals.widgets.hovered.bg_fill = GlassTheme::bg_card_hover();
            style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, GlassTheme::accent_primary());
            style.visuals.widgets.active.bg_fill = Color32::from_rgba_unmultiplied(99, 102, 241, 51);
            style.visuals.widgets.active.bg_stroke = Stroke::new(2.0, GlassTheme::accent_primary());
            style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(99, 102, 241, 38);
            style.visuals.selection.stroke = Stroke::new(1.5, GlassTheme::accent_primary());
            style.visuals.widgets.inactive.corner_radius = 8.into();
            style.visuals.widgets.hovered.corner_radius = 8.into();
            style.visuals.widgets.active.corner_radius = 8.into();
            cc.egui_ctx.set_visuals(egui::Visuals {
            window_fill: Color32::TRANSPARENT,
            panel_fill: Color32::TRANSPARENT,
            ..egui::Visuals::dark()
});
         style.visuals.window_fill = Color32::TRANSPARENT;
         style.visuals.panel_fill = Color32::TRANSPARENT;
            Ok(Box::new(SystemMonitorApp::new(tx.clone())))
        }),
    )
}
// ======================================================
// 🔥 FAVORITES EXTENSION (NON-INTRUSIVE)
// ======================================================

use std::collections::HashSet;

#[derive(Default)]
struct FavoritesState {
    favorites: HashSet<usize>,
}

impl FavoritesState {
    fn toggle(&mut self, id: usize) {
        if !self.favorites.insert(id) {
            self.favorites.remove(&id);
        }
    }

    fn is_favorite(&self, id: usize) -> bool {
        self.favorites.contains(&id)
    }
}

// Global (safe enough for UI layer usage)
use std::sync::OnceLock;
static FAVORITES: OnceLock<Mutex<FavoritesState>> = OnceLock::new();

fn favorites() -> &'static Mutex<FavoritesState> {
    FAVORITES.get_or_init(|| Mutex::new(FavoritesState::default()))
}

// ======================================================
// 🎨 FAVORITE BUTTON OVERLAY (inject into cards)
// ======================================================

impl SystemMonitorApp {
    fn favorite_button(&mut self, ui: &mut egui::Ui, item_id: usize, rect: egui::Rect) {
        let mut fav = favorites().lock().unwrap();
        let is_fav = fav.is_favorite(item_id);

        let btn = egui::Button::new(
            RichText::new(if is_fav { "★" } else { "☆" })
                .size(14.0)
                .color(Color32::WHITE),
        )
        .fill(if is_fav {
            GlassTheme::accent_warning()
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 100)
        })
        .rounding(6.0);

        let button_rect = egui::Rect::from_min_size(
            rect.right_top() - Vec2::new(28.0, -4.0),
            Vec2::new(24.0, 24.0),
        );

        if ui.put(button_rect, btn).clicked() {
            fav.toggle(item_id);
        }
    }
}

// ======================================================
// 🧠 PATCH-INJECTION HOOK (CALL THIS MANUALLY)
// ======================================================

impl SystemMonitorApp {
    fn inject_favorite_overlay(
        &mut self,
        ui: &mut egui::Ui,
        item: &WallpaperItem,
        preview_rect: egui::Rect,
    ) {
        self.favorite_button(ui, item.id, preview_rect);
    }
}

// ======================================================
// 📌 FAVORITES FILTER (OPTIONAL CALL)
// ======================================================

impl SystemMonitorApp {
    fn filter_only_favorites(&mut self) {
        let fav = favorites().lock().unwrap();

        let mut wallpaper = self.wallpaper.lock().unwrap();
        wallpaper.filtered_items = wallpaper
            .wallpaper_items
            .iter()
            .filter(|w| fav.is_favorite(w.id))
            .cloned()
            .collect();
    }
}
// ======================================================
// 🌐 ONLINE WALLPAPER API EXTENSION (NON-INTRUSIVE)
// ======================================================

use std::thread;
use std::sync::mpsc::{channel, Receiver};

#[derive(Clone)]
struct OnlineWallpaper {
    id: String,
    title: String,
    author: String,
    image_url: String,
    thumb_url: String,
}

struct OnlineState {
    wallpapers: Vec<WallpaperItem>,
    loading: bool,
}

impl OnlineState {
    fn new() -> Self {
        Self {
            wallpapers: Vec::new(),
            loading: false,
        }
    }
}

// Global storage
static ONLINE_STATE: OnceLock<Mutex<OnlineState>> = OnceLock::new();

fn online_state() -> &'static Mutex<OnlineState> {
    ONLINE_STATE.get_or_init(|| Mutex::new(OnlineState::new()))
}

// ======================================================
// 🌍 FETCH FROM API (THREAD SAFE)
// ======================================================

impl SystemMonitorApp {
    fn fetch_online_wallpapers(&self) {
        let state = online_state().clone();

        {
            let mut s = state.lock().unwrap();
            if s.loading { return; }
            s.loading = true;
        }

        thread::spawn(move || {
            // 🔥 Replace with real API later
            let dummy_data = vec![
                ("Ocean View", "Unsplash", "https://picsum.photos/800/600"),
                ("Mountains", "Unsplash", "https://picsum.photos/801/600"),
                ("City Night", "Unsplash", "https://picsum.photos/802/600"),
            ];

            let mut new_items = Vec::new();

            for (i, (title, author, url)) in dummy_data.into_iter().enumerate() {
                new_items.push(WallpaperItem {
                    id: 10_000 + i,
                    title: title.to_string(),
                    author: author.to_string(),
                    size: 2.0,
                    resolution: "1920x1080".to_string(),
                    file_type: "JPG".to_string(),
                    tags: vec!["online".to_string()],
                    category: FilterType::Scene,
                    description: "Online wallpaper".to_string(),
                    downloads: 1000,
                    rating: 4.5,
                    path: PathBuf::from(url), // URL stored as path
                    thumbnail_id: None,
                });
            }

            let mut s = state.lock().unwrap();
            s.wallpapers = new_items;
            s.loading = false;
        });
    }
}

// ======================================================
// 🧩 MERGE ONLINE INTO UI
// ======================================================

impl SystemMonitorApp {
    fn merge_online_wallpapers(&mut self) {
        let online = online_state().lock().unwrap();

        if online.wallpapers.is_empty() {
            return;
        }

        let mut wallpaper = self.wallpaper.lock().unwrap();

        for item in &online.wallpapers {
            if !wallpaper.wallpaper_items.iter().any(|w| w.id == item.id) {
                wallpaper.wallpaper_items.push(item.clone());
            }
        }

        wallpaper.apply_filters_and_sort();
    }
}

// ======================================================
// 🎨 ONLINE TAB UI (EXTENSION)
// ======================================================

impl SystemMonitorApp {
    fn render_online_section(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("🌐 Load Online Wallpapers").clicked() {
                self.fetch_online_wallpapers();
            }

            if ui.button("🔄 Refresh").clicked() {
                self.merge_online_wallpapers();
            }
        });

        ui.add_space(10.0);

        let online = online_state().lock().unwrap();

        if online.loading {
            ui.label("⏳ Loading wallpapers from internet...");
            return;
        }

        if online.wallpapers.is_empty() {
            ui.label("No online wallpapers loaded.");
            return;
        }

        ui.label(format!("🌍 {} wallpapers available online", online.wallpapers.len()));
    }
}
// ======================================================
// 🌐 REAL ONLINE API (WALLHAVEN) + DOWNLOAD CACHE
// ======================================================

use serde::Deserialize;

// ---------- API RESPONSE ----------

#[derive(Debug, Deserialize)]
struct WallhavenResponse {
    data: Vec<WallhavenWallpaper>,
}

#[derive(Debug, Deserialize)]
struct WallhavenWallpaper {
    id: String,
    path: String,
    resolution: String,
    file_size: u64,
    favorites: u32,
}

// ---------- CACHE FOLDER ----------

fn online_cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let dir = PathBuf::from(home).join(".wallpaper_engine_cache");

    let _ = fs::create_dir_all(&dir);
    dir
}

// ---------- DOWNLOAD IMAGE ----------

fn download_image(url: &str) -> Option<PathBuf> {
    let filename = url.split('/').last()?.to_string();
    let path = online_cache_dir().join(filename);

    if path.exists() {
        return Some(path);
    }

    if let Ok(resp) = reqwest::blocking::get(url) {
        if let Ok(bytes) = resp.bytes() {
            if fs::write(&path, &bytes).is_ok() {
                return Some(path);
            }
        }
    }

    None
}

// ---------- FETCH REAL API ----------

impl SystemMonitorApp {
    fn fetch_wallhaven(&self) {
        let state = online_state().clone();

        {
            let mut s = state.lock().unwrap();
            if s.loading { return; }
            s.loading = true;
        }

        thread::spawn(move || {
            let url = "https://wallhaven.cc/api/v1/search?categories=111&purity=100&sorting=toplist";

            let mut new_items = Vec::new();

            if let Ok(resp) = reqwest::blocking::get(url) {
                if let Ok(json) = resp.json::<WallhavenResponse>() {
                    for (i, w) in json.data.iter().take(30).enumerate() {
                        if let Some(local_path) = download_image(&w.path) {
                            new_items.push(WallpaperItem {
                                id: 50_000 + i,
                                title: format!("Wallhaven {}", w.id),
                                author: "Wallhaven".to_string(),
                                size: w.file_size as f32 / 1024.0 / 1024.0,
                                resolution: w.resolution.clone(),
                                file_type: "JPG".to_string(),
                                tags: vec!["online".into(), "wallhaven".into()],
                                category: FilterType::Scene,
                                description: "Downloaded from Wallhaven".into(),
                                downloads: w.favorites,
                                rating: 4.0,
                                path: local_path,
                                thumbnail_id: None,
                            });
                        }
                    }
                }
            }

            let mut s = state.lock().unwrap();
            s.wallpapers = new_items;
            s.loading = false;
        });
    }
}
// ======================================================
// 🚀 STEAM-LEVEL ONLINE SYSTEM
// ======================================================

#[derive(Default)]
struct OnlineQuery {
    query: String,
    page: u32,
    loading: bool,
    has_more: bool,
}

static ONLINE_QUERY: OnceLock<Mutex<OnlineQuery>> = OnceLock::new();

fn online_query() -> &'static Mutex<OnlineQuery> {
    ONLINE_QUERY.get_or_init(|| Mutex::new(OnlineQuery {
        query: String::new(),
        page: 1,
        loading: false,
        has_more: true,
    }))
}

// ======================================================
// 🔍 SEARCH + PAGINATION (WALLHAVEN)
// ======================================================

impl SystemMonitorApp {
    fn search_wallhaven(&self, query: String, page: u32) {
        let state = online_state().clone();
        let q_state = online_query().clone();

        {
            let mut q = q_state.lock().unwrap();
            if q.loading { return; }
            q.loading = true;
        }

        thread::spawn(move || {
            let url = format!(
                "https://wallhaven.cc/api/v1/search?q={}&page={}&sorting=toplist",
                query, page
            );

            let mut new_items = Vec::new();

            if let Ok(resp) = reqwest::blocking::get(&url) {
                if let Ok(json) = resp.json::<WallhavenResponse>() {
                    for (i, w) in json.data.iter().enumerate() {
                        if let Some(local_path) = download_image(&w.path) {
                            new_items.push(WallpaperItem {
                                id: 100_000 + (page as usize * 1000) + i,
                                title: format!("{} ({})", query, w.id),
                                author: "Wallhaven".into(),
                                size: w.file_size as f32 / 1024.0 / 1024.0,
                                resolution: w.resolution.clone(),
                                file_type: "JPG".into(),
                                tags: vec![query.clone(), "online".into()],
                                category: FilterType::Scene,
                                description: "Online search result".into(),
                                downloads: w.favorites,
                                rating: 4.2,
                                path: local_path,
                                thumbnail_id: None,
                            });
                        }
                    }
                }
            }

            {
                let mut s = state.lock().unwrap();
                s.wallpapers.extend(new_items);
            }

            let mut q = q_state.lock().unwrap();
            q.loading = false;
            q.page += 1;
        });
    }
}

// ======================================================
// 🎨 STEAM WORKSHOP UI PANEL
// ======================================================

impl SystemMonitorApp {
    fn render_steam_workshop(&mut self, ui: &mut egui::Ui) {
        let mut query_state = online_query().lock().unwrap();

        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut query_state.query)
                    .hint_text("🔍 Search online wallpapers...")
                    .desired_width(220.0),
            );

            if ui.button("Search").clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
            {
                query_state.page = 1;
                online_state().lock().unwrap().wallpapers.clear();

                drop(query_state);
                self.search_wallhaven(
                    online_query().lock().unwrap().query.clone(),
                    1,
                );
                return;
            }

            if ui.button("🔥 Trending").clicked() {
                query_state.query = "".into();
                query_state.page = 1;
                online_state().lock().unwrap().wallpapers.clear();

                drop(query_state);
                self.search_wallhaven("".into(), 1);
                return;
            }
        });

        ui.add_space(10.0);

        // TAG QUICK FILTERS
        ui.horizontal_wrapped(|ui| {
            for tag in ["anime", "nature", "cyberpunk", "dark", "minimal"] {
                if ui.button(tag).clicked() {
                    let mut q = online_query().lock().unwrap();
                    q.query = tag.into();
                    q.page = 1;
                    online_state().lock().unwrap().wallpapers.clear();

                    drop(q);
                    self.search_wallhaven(tag.into(), 1);
                }
            }
        });

        ui.add_space(10.0);

        let q = online_query().lock().unwrap();
        if q.loading {
            ui.label("⏳ Loading more wallpapers...");
        }

        drop(q);

        // AUTO LOAD NEXT PAGE (INFINITE SCROLL TRIGGER)
        let should_load_more = {
            let q = online_query().lock().unwrap();
            !q.loading && q.has_more
        };

        if should_load_more {
            let q = online_query().lock().unwrap();
            self.search_wallhaven(q.query.clone(), q.page);
        }

        ui.separator();

        // MERGE INTO MAIN SYSTEM
        self.merge_online_wallpapers();
    }
}
// ======================================================
// ⚡ GPU + ASYNC PERFORMANCE LAYER (FIXED)
// ======================================================

use std::sync::mpsc::{Sender as StdSender, Receiver as StdReceiver};
// ------------------------------------------------------
// 🧠 BACKGROUND IMAGE LOADER
// ------------------------------------------------------

struct ImageJob {
    id: usize,
    path: PathBuf,
}

struct ImageResult {
    id: usize,
    image: egui::ColorImage,
}

struct AsyncImageLoader {
    sender: StdSender<ImageJob>,
    receiver: Mutex<StdReceiver<ImageResult>>, // ✅ FIX
}

impl AsyncImageLoader {
    fn new(worker_count: usize) -> Self {
        let (job_tx, job_rx) = std::sync::mpsc::channel::<ImageJob>();
        let (res_tx, res_rx) = std::sync::mpsc::channel::<ImageResult>();

        let job_rx = Arc::new(Mutex::new(job_rx));

        for _ in 0..worker_count {
            let job_rx = job_rx.clone();
            let res_tx = res_tx.clone();

            thread::spawn(move || loop {
                let job = {
                    let rx = job_rx.lock().unwrap();
                    rx.recv()
                };

                if let Ok(job) = job {
                    if let Ok(img) = image::open(&job.path) {
                        let img = img.thumbnail(256, 256);
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.into_raw();

                        let _ = res_tx.send(ImageResult {
                            id: job.id,
                            image: egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                        });
                    }
                }
            });
        }

        Self {
            sender: job_tx,
            receiver: Mutex::new(res_rx), // ✅ FIX
        }
    }
}

// GLOBAL LOADER
static IMAGE_LOADER: OnceLock<AsyncImageLoader> = OnceLock::new();

fn image_loader() -> &'static AsyncImageLoader {
    IMAGE_LOADER.get_or_init(|| AsyncImageLoader::new(4))
}

// ------------------------------------------------------
// 🚀 QUEUE IMAGE LOAD (NON-BLOCKING)
// ------------------------------------------------------

impl SystemMonitorApp {
    fn queue_image_load(&self, item: &WallpaperItem) {
        let _ = image_loader().sender.send(ImageJob {
            id: item.id,
            path: item.path.clone(),
        });
    }
}

// ------------------------------------------------------
// 🎮 PROCESS GPU UPLOADS (MAIN THREAD)
// ------------------------------------------------------

impl SystemMonitorApp {
    fn process_loaded_images(&mut self, ctx: &Context) {
        let loader = image_loader();

        let mut wallpaper = self.wallpaper.lock().unwrap();

        let rx = loader.receiver.lock().unwrap(); // ✅ lock once

        while let Ok(result) = rx.try_recv() {
            let texture = ctx.load_texture(
                format!("async_thumb_{}", result.id),
                result.image,
                TextureOptions::LINEAR,
            );

            wallpaper.thumbnail_cache.insert(result.id, texture);
        }
    }
}

// ------------------------------------------------------
// 🧹 SMART CACHE CLEANUP (VRAM SAFE)
// ------------------------------------------------------

impl SystemMonitorApp {
    fn cleanup_texture_cache(&mut self) {
        let mut wallpaper = self.wallpaper.lock().unwrap();

        let max_cache = 200;

        if wallpaper.thumbnail_cache.len() > max_cache {
            let keys: Vec<usize> = wallpaper.thumbnail_cache.keys().cloned().collect();

            for key in keys.iter().take(keys.len() - max_cache) {
                wallpaper.thumbnail_cache.remove(key);
            }
        }
    }
}

// ------------------------------------------------------
// ⚡ LAZY LOAD VISIBLE ITEMS ONLY
// ------------------------------------------------------

impl SystemMonitorApp {
    fn lazy_load_visible(&mut self) {
        let items = {
            self.wallpaper.lock().unwrap().filtered_items.clone()
        };

        for item in items.iter().take(30) {
            self.queue_image_load(item);
        }
    }
}
