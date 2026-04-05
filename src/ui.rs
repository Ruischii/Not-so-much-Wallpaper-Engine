use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

//
// ============================================================
// WALLPAPER ENTRY
// ============================================================
//

#[derive(Clone, Debug)]
pub struct Wallpaper {
    pub path: PathBuf,
}

//
// ============================================================
// WALLPAPER BROWSER (UI LOGIC)
// ============================================================
//

pub struct WallpaperBrowser {
    pub wallpapers: Vec<Wallpaper>,
    selected: Option<usize>,
}

impl WallpaperBrowser {
    pub fn new() -> Result<Self> {
        let mut browser = Self {
            wallpapers: Vec::new(),
            selected: None,
        };

        browser.scan_default_dirs()?;
        Ok(browser)
    }

    fn scan_default_dirs(&mut self) -> Result<()> {
        let home = dirs::home_dir().unwrap();

        let wallpaper_dir = home.join("Pictures").join("wallpapers");

        if !wallpaper_dir.exists() {
            println!("[ui] wallpaper folder not found: {:?}", wallpaper_dir);
            return Ok(());
        }

        for entry in fs::read_dir(wallpaper_dir)? {
            let entry = entry?;
            let path = entry.path();

            if Self::is_supported(&path) {
                self.wallpapers.push(Wallpaper { path });
            }
        }

        println!("[ui] loaded {} wallpapers", self.wallpapers.len());

        if !self.wallpapers.is_empty() {
            self.selected = Some(0);
        }

        Ok(())
    }

    fn is_supported(path: &Path) -> bool {
        match path.extension().and_then(|e| e.to_str()) {
            Some("jpg") | Some("jpeg") | Some("png") | Some("webp") | Some("mp4") => true,
            _ => false,
        }
    }

    pub fn selected(&self) -> Option<&Wallpaper> {
        self.selected.map(|i| &self.wallpapers[i])
    }
}
