// src/engine/workshop.rs
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use serde::{Serialize, Deserialize};
use anyhow::Result;

use super::ui::WallpaperItem;
use super::media::MediaEngine;

// ====================== WORKSHOP TYPES ======================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopMetadata {
    pub title: String,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    pub preview_url: String,
    pub file_size: u64,
    pub resolution: String,
    pub wallpaper_type: WorkshopType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkshopType {
    Image,
    Video,
    Web,
    Scene,
}

#[derive(Debug, Clone)]
pub struct WorkshopItem {
    pub id: u64,
    pub title: String,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    pub preview_path: PathBuf,
    pub content_path: PathBuf,
    pub wallpaper_type: WorkshopType,
    pub downloads: u32,
    pub rating: f32,
    pub subscribed: bool,
}

// ====================== STEAM PATH DETECTION ======================

#[derive(Debug)]
pub struct SteamPaths {
    pub steam_root: PathBuf,
    pub workshop_content: PathBuf,
    pub workshop_metadata: PathBuf,
}

impl SteamPaths {
    pub fn detect() -> Option<Self> {
        let home = std::env::var("HOME").ok()?;
        
        // Common Steam installation paths
        let candidates = vec![
            PathBuf::from(&home).join(".steam/steam"),
            PathBuf::from(&home).join(".local/share/Steam"),
            PathBuf::from(&home).join(".var/app/com.valvesoftware.Steam/.steam/steam"), // Flatpak
            PathBuf::from("/usr/share/steam"),
        ];
        
        for candidate in candidates {
            if candidate.exists() {
                let workshop_content = candidate
                    .join("steamapps")
                    .join("workshop")
                    .join("content")
                    .join("431960"); // Wallpaper Engine App ID
                
                let workshop_metadata = candidate
                    .join("steamapps")
                    .join("workshop")
                    .join("appworkshop_431960.acf");
                
                if workshop_content.exists() {
                    return Some(Self {
                        steam_root: candidate,
                        workshop_content,
                        workshop_metadata,
                    });
                }
            }
        }
        
        None
    }
}

// ====================== WORKSHOP MANAGER ======================

pub struct WorkshopManager {
    paths: Option<SteamPaths>,
    subscribed_items: Vec<WorkshopItem>,
    local_cache: PathBuf,
}

impl WorkshopManager {
    pub fn new() -> Self {
        let local_cache = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("wallpaper_engine_workshop");
        
        fs::create_dir_all(&local_cache).ok();
        
        let mut manager = Self {
            paths: SteamPaths::detect(),
            subscribed_items: Vec::new(),
            local_cache,
        };
        
        manager.scan_subscribed_items();
        manager
        
    }
    
    // ====================== SCAN WORKSHOP ITEMS ======================
    
    pub fn scan_subscribed_items(&mut self) {
        self.subscribed_items.clear();
        
        let Some(paths) = &self.paths else { return };
        
        if !paths.workshop_content.exists() {
            return;
        }
        
        for entry in fs::read_dir(&paths.workshop_content).unwrap_or_else(|_| fs::read_dir("/tmp").unwrap()) {
            let Ok(entry) = entry else { continue };
            let folder_path = entry.path();
            
            if !folder_path.is_dir() { continue; }
            
            let id = folder_path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|s| s.parse::<u64>().ok());
            
            let Some(id) = id else { continue };
            
            // Parse project.json for metadata
            let project_json = folder_path.join("project.json");
            let metadata = self.parse_project_json(&project_json);
            
            // Find wallpaper file
            let content_path = self.find_wallpaper_file(&folder_path);
            let preview_path = folder_path.join("preview.jpg");
            
            self.subscribed_items.push(WorkshopItem {
                id,
                title: metadata.title.unwrap_or_else(|| format!("Workshop Item {}", id)),
                author: metadata.author.unwrap_or_else(|| "Unknown Author".to_string()),
                description: metadata.description.unwrap_or_default(),
                tags: metadata.tags.unwrap_or_default(),
                preview_path: if preview_path.exists() { preview_path } else { folder_path.join("preview.png") },
                content_path,
                wallpaper_type: self.detect_wallpaper_type(&content_path),
                downloads: metadata.downloads.unwrap_or(0),
                rating: metadata.rating.unwrap_or(0.0),
                subscribed: true,
            });
        }
    }
    
    fn parse_project_json(&self, path: &Path) -> WorkshopMetadataPartial {
        let default = WorkshopMetadataPartial::default();
        
        let content = fs::read_to_string(path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        
        Some(WorkshopMetadataPartial {
            title: json.get("title").and_then(|v| v.as_str()).map(String::from),
            author: json.get("author").and_then(|v| v.as_str()).map(String::from),
            description: json.get("description").and_then(|v| v.as_str()).map(String::from),
            tags: json.get("tags").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|t| t.as_str().map(String::from)).collect()),
            downloads: json.get("downloads").and_then(|v| v.as_u64()).map(|d| d as u32),
            rating: json.get("rating").and_then(|v| v.as_f64()).map(|r| r as f32),
        }).unwrap_or(default)
    }
    
    fn find_wallpaper_file(&self, folder: &Path) -> PathBuf {
        // Look for common wallpaper formats
        for ext in &["mp4", "webm", "png", "jpg", "jpeg", "gif", "html"] {
            let file = folder.join(format!("wallpaper.{}", ext));
            if file.exists() {
                return file;
            }
        }
        
        // Try to find any media file
        if let Ok(entries) = fs::read_dir(folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ["mp4", "webm", "png", "jpg", "jpeg", "gif", "html"].contains(&ext_str.as_str()) {
                        return path;
                    }
                }
            }
        }
        
        PathBuf::new()
    }
    
    fn detect_wallpaper_type(&self, path: &Path) -> WorkshopType {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        match ext.as_str() {
            "mp4" | "webm" => WorkshopType::Video,
            "html" => WorkshopType::Web,
            "png" | "jpg" | "jpeg" | "gif" => WorkshopType::Image,
            _ => WorkshopType::Scene,
        }
    }
    
    // ====================== CONVERT TO UI WALLPAPER ITEM ======================
    
    pub fn get_workshop_items(&self) -> Vec<WallpaperItem> {
        self.subscribed_items.iter().map(|item| {
            WallpaperItem {
                id: item.id as usize,
                title: item.title.clone(),
                author: item.author.clone(),
                size: fs::metadata(&item.content_path)
                    .map(|m| m.len() as f32 / 1024.0 / 1024.0)
                    .unwrap_or(0.0),
                resolution: "1920x1080".to_string(), // Could parse from video/image
                file_type: match item.wallpaper_type {
                    WorkshopType::Image => "IMAGE".to_string(),
                    WorkshopType::Video => "VIDEO".to_string(),
                    WorkshopType::Web => "WEB".to_string(),
                    WorkshopType::Scene => "SCENE".to_string(),
                },
                tags: item.tags.clone(),
                category: self.map_to_filter_type(&item.wallpaper_type),
                description: item.description.clone(),
                downloads: item.downloads,
                rating: item.rating,
                path: item.content_path.clone(),
                thumbnail_id: Some(format!("workshop_{}", item.id)),
            }
        }).collect()
    }
    
    fn map_to_filter_type(&self, wt: &WorkshopType) -> super::ui::FilterType {
        match wt {
            WorkshopType::Image => super::ui::FilterType::Scene,
            WorkshopType::Video => super::ui::FilterType::Video,
            WorkshopType::Web => super::ui::FilterType::Web,
            WorkshopType::Scene => super::ui::FilterType::Scene,
        }
    }
    
    // ====================== DOWNLOAD & INSTALL ======================
    
    pub fn install_workshop_item(&mut self, item_id: u64) -> Result<PathBuf> {
        // Copy from workshop folder to local cache
        let item = self.subscribed_items.iter()
            .find(|i| i.id == item_id)
            .ok_or_else(|| anyhow::anyhow!("Item not found"))?;
        
        let dest = self.local_cache.join(format!("{}_wallpaper", item_id));
        fs::create_dir_all(&dest)?;
        
        let dest_path = dest.join(item.content_path.file_name().unwrap_or_default());
        fs::copy(&item.content_path, &dest_path)?;
        
        Ok(dest_path)
    }
    
    pub fn refresh(&mut self) {
        self.scan_subscribed_items();
    }
    
    // ====================== OPEN IN STEAM ======================
    
    pub fn open_in_steam(&self, item_id: u64) {
        let _ = std::process::Command::new("steam")
            .arg(format!("steam://openurl/https://steamcommunity.com/sharedfiles/filedetails/?id={}", item_id))
            .spawn();
    }
    
    pub fn open_workshop_in_browser(&self) {
        let _ = webbrowser::open("https://steamcommunity.com/app/431960/workshop/");
    }
}

#[derive(Default)]
struct WorkshopMetadataPartial {
    title: Option<String>,
    author: Option<String>,
    description: Option<String>,
    tags: Option<Vec<String>>,
    downloads: Option<u32>,
    rating: Option<f32>,
}

// ====================== ASYNC WORKSHOP BROWSER ======================

pub struct WorkshopBrowser {
    search_results: Vec<WallpaperItem>,
    loading: bool,
    current_query: String,
}

impl WorkshopBrowser {
    pub fn new() -> Self {
        Self {
            search_results: Vec::new(),
            loading: false,
            current_query: String::new(),
        }
    }
    
    pub fn search(&mut self, query: String) {
        self.loading = true;
        self.current_query = query.clone();
        
        let manager = WorkshopManager::new();
        let items = manager.get_workshop_items();
        
        // Filter by search query
        self.search_results = items.into_iter()
            .filter(|item| {
                query.is_empty() || 
                item.title.to_lowercase().contains(&query.to_lowercase()) ||
                item.author.to_lowercase().contains(&query.to_lowercase()) ||
                item.tags.iter().any(|t| t.to_lowercase().contains(&query.to_lowercase()))
            })
            .collect();
        
        self.loading = false;
    }
    
    pub fn get_results(&self) -> &[WallpaperItem] {
        &self.search_results
    }
    
    pub fn is_loading(&self) -> bool {
        self.loading
    }
}// Add these imports at the top of ui.rs
use crate::engine::workshop::{WorkshopManager, WorkshopBrowser};

// Add to SystemMonitorApp struct:
pub struct SystemMonitorApp {
    // ... existing fields ...
    workshop_manager: WorkshopManager,
    workshop_browser: WorkshopBrowser,
    workshop_search_query: String,
}

// Update the new() function:
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
            workshop_manager: WorkshopManager::new(),  // ← Add
            workshop_browser: WorkshopBrowser::new(),  // ← Add
            workshop_search_query: String::new(),      // ← Add
        }
    }
}

// Replace render_workshop_tab with:
fn render_workshop_tab(&mut self, ui: &mut egui::Ui) {
    egui::Frame::none()
        .fill(GlassTheme::bg_card())
        .rounding(Rounding::same(16))
        .stroke(Stroke::new(1.0, GlassTheme::border_light()))
        .inner_margin(Margin::symmetric(24, 20))
        .show(ui, |ui| {
            // Header with refresh and open buttons
            ui.horizontal(|ui| {
                ui.label(RichText::new("🛠️ Steam Workshop").color(GlassTheme::text_primary()).size(20.0).strong());
                
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new("🌐 Open in Browser").size(12.0)).clicked() {
                        self.workshop_manager.open_workshop_in_browser();
                    }
                    
                    ui.add_space(8.0);
                    
                    if ui.button(RichText::new("🔄 Refresh").size(12.0)).clicked() {
                        self.workshop_manager.refresh();
                        self.workshop_browser.search(self.workshop_search_query.clone());
                    }
                });
            });
            
            ui.add_space(20.0);
            
            // Search bar
            ui.horizontal(|ui| {
                let search_response = ui.add(
                    egui::TextEdit::singleline(&mut self.workshop_search_query)
                        .hint_text("🔍 Search subscribed workshop items...")
                        .desired_width(300.0)
                );
                
                if ui.button("Search").clicked() || search_response.lost_focus() {
                    self.workshop_browser.search(self.workshop_search_query.clone());
                }
            });
            
            ui.add_space(16.0);
            
            // Stats
            let items = self.workshop_browser.get_results();
            if items.is_empty() && !self.workshop_browser.is_loading() {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.label(RichText::new("📭").size(64.0).color(GlassTheme::text_muted()));
                    ui.add_space(16.0);
                    ui.label(RichText::new("No subscribed workshop items found")
                        .color(GlassTheme::text_secondary()).size(16.0));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Subscribe to wallpapers in Steam Workshop to see them here")
                        .color(GlassTheme::text_muted()).size(13.0));
                    ui.add_space(24.0);
                    
                    if ui.button(RichText::new("Open Steam Workshop").size(14.0)).clicked() {
                        self.workshop_manager.open_workshop_in_browser();
                    }
                });
                return;
            }
            
            // Show count
            ui.label(RichText::new(format!("📦 {} workshop items", items.len()))
                .color(GlassTheme::text_secondary()).size(13.0));
            
            ui.add_space(16.0);
            
            // Grid of workshop items
            let available_width = ui.available_width();
            let thumb_size = self.thumbnail_size;
            let columns = ((available_width - 40.0) / (thumb_size + 20.0)).floor() as usize;
            let columns = columns.max(2);
            
            ScrollArea::vertical()
                .max_height(550.0)
                .show(ui, |ui| {
                    egui::Grid::new("workshop_grid")
                        .spacing([16.0, 16.0])
                        .show(ui, |ui| {
                            for (idx, item) in items.iter().enumerate() {
                                self.render_workshop_card(ui, item, idx, thumb_size);
                                
                                if (idx + 1) % columns == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
        });
}

// Add helper function to render workshop cards:
fn render_workshop_card(&mut self, ui: &mut egui::Ui, item: &WallpaperItem, idx: usize, thumb_size: f32) {
    egui::Frame::none()
        .fill(GlassTheme::bg_input())
        .rounding(Rounding::same(12))
        .stroke(Stroke::new(1.0, GlassTheme::border_light()))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(thumb_size, thumb_size + 100.0));
            
            ui.vertical(|ui| {
                // Preview area
                let preview_size = Vec2::new(thumb_size - 20.0, thumb_size - 20.0);
                let preview_rect = ui.allocate_exact_size(preview_size, Sense::click()).0;
                
                // Show type icon or thumbnail
                let painter = ui.painter();
                painter.rect_filled(preview_rect, Rounding::same(8), GlassTheme::bg_card());
                
                let (icon, label) = match item.file_type.as_str() {
                    "VIDEO" => ("🎬", "Video"),
                    "WEB" => ("🌐", "Web"),
                    "IMAGE" => ("🖼️", "Image"),
                    _ => ("🎨", "Scene"),
                };
                
                painter.text(
                    preview_rect.center(),
                    Align2::CENTER_CENTER,
                    icon,
                    FontId::proportional(48.0),
                    GlassTheme::text_muted(),
                );
                
                // Workshop badge
                let badge_rect = egui::Rect::from_min_size(
                    preview_rect.min + Vec2::new(8.0, 8.0),
                    Vec2::new(60.0, 20.0),
                );
                
                painter.rect_filled(badge_rect, Rounding::same(4), Color32::from_rgba_unmultiplied(0, 0, 0, 150));
                painter.text(
                    badge_rect.center(),
                    Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(10.0),
                    GlassTheme::text_secondary(),
                );
                
                if preview_rect.clicked() {
                    self.show_preview = true;
                    self.preview_item = Some(item.id);
                }
                
                ui.add_space(8.0);
                
                // Title and author
                ui.label(RichText::new(&item.title).size(13.0).strong().color(GlassTheme::text_primary()));
                ui.label(RichText::new(&item.author).size(11.0).color(GlassTheme::text_muted()));
                
                // Rating and downloads
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("★ {:.1}", item.rating)).size(11.0).color(GlassTheme::accent_warning()));
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("⬇️ {}", item.downloads)).size(10.0).color(GlassTheme::text_muted()));
                });
                
                ui.add_space(8.0);
                
                // Action buttons
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
