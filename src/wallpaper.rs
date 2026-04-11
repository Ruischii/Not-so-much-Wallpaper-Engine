// src/engine/wallpaper.rs
use std::{
    env, fs,
    path::PathBuf,
    process::Command,
};

use anyhow::Result;

#[derive(Debug, PartialEq)]
pub enum CompositorType {
    Hyprland,
    Niri,
    Sway,
    River,
    Other,
}

fn simple_hash(input: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn get_cache_dir() -> PathBuf {
    if let Ok(cache_home) = env::var("XDG_CACHE_HOME") {
        return PathBuf::from(cache_home).join("web-wallpapers");
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home).join(".cache").join("web-wallpapers");
    }
    PathBuf::from("/tmp").join("web-wallpapers")
}

pub struct WebWallpaper {
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub is_playing: bool,
    pub cache_path: PathBuf,
}

impl WebWallpaper {
    pub fn new(url: String, width: u32, height: u32) -> Self {
        let cache_dir = get_cache_dir();
        fs::create_dir_all(&cache_dir).ok();

        let hash = simple_hash(&url);
        let cache_path = cache_dir.join(format!("{hash}.mp4"));

        Self {
            url,
            width,
            height,
            framerate: 30,
            is_playing: true,
            cache_path,
        }
    }

    pub fn download(&self) -> Result<PathBuf> {
        if self.cache_path.exists() {
            return Ok(self.cache_path.clone());
        }

        let output = Command::new("curl")
            .arg("-L")
            .arg("-o")
            .arg(&self.cache_path)
            .arg(&self.url)
            .output()?;

        if output.status.success() {
            Ok(self.cache_path.clone())
        } else {
            anyhow::bail!("download failed")
        }
    }
}

pub struct WebWallpaperEngine {
    wallpapers: std::collections::HashMap<String, WebWallpaper>,
    active_wallpaper: Option<String>,
    compositor_type: CompositorType,
}

impl WebWallpaperEngine {
    pub fn new() -> Self {
        Self {
            wallpapers: std::collections::HashMap::new(),
            active_wallpaper: None,
            compositor_type: CompositorType::Other,
        }
    }

    pub fn add_wallpaper(&mut self, url: String, width: u32, height: u32) {
        self.wallpapers.insert(url.clone(), WebWallpaper::new(url, width, height));
    }

    pub fn set_active(&mut self, url: &str) -> Result<()> {
        if !self.wallpapers.contains_key(url) {
            anyhow::bail!("Wallpaper not found");
        }
        self.active_wallpaper = Some(url.to_string());
        Ok(())
    }

    pub fn update(&mut self) {}
}
