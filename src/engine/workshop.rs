// Add these imports at the top of ui.rs
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
