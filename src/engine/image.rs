// src/image.rs
use std::path::{Path, PathBuf};
use egui_extras::RetainedImage;
use rand::seq::SliceRandom;
use std::fs;

#[derive(Default)]
pub struct WallpaperManager {
    pub current_image: Option<RetainedImage>,
    pub wallpaper_folder: PathBuf,
    pub image_paths: Vec<PathBuf>,
}

impl WallpaperManager {
    pub fn new(folder: Option<PathBuf>) -> Self {
        let wallpaper_folder = folder.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
            PathBuf::from(home).join("Pictures").join("Wallpapers")
        });

        let mut manager = Self {
            wallpaper_folder,
            ..Default::default()
        };

        manager.refresh_images();
        manager.pick_random();
        manager
    }

    pub fn refresh_images(&mut self) {
        self.image_paths.clear();

        if !self.wallpaper_folder.exists() {
            return;
        }

        if let Ok(entries) = fs::read_dir(&self.wallpaper_folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    match ext.to_string_lossy().to_lowercase().as_str() {
                        "png" | "jpg" | "jpeg" | "webp" | "gif" => {
                            self.image_paths.push(path);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn pick_random(&mut self) {
        if self.image_paths.is_empty() {
            self.current_image = None;
            return;
        }

        if let Some(path) = self.image_paths.choose(&mut rand::thread_rng()) {
            self.load_image_from_path(path);
        }
    }

    pub fn load_image_from_path(&mut self, path: &Path) {
        match RetainedImage::from_image_path(path) {
            Ok(img) => {
                self.current_image = Some(img);
                println!("[image] Loaded: {}", path.display());
            }
            Err(e) => {
                eprintln!("[image] Failed to load {}: {}", path.display(), e);
                self.current_image = None;
            }
        }
    }

    pub fn get_folder_display(&self) -> String {
        self.wallpaper_folder.display().to_string()
    }

    pub fn image_count(&self) -> usize {
        self.image_paths.len()
    }
}