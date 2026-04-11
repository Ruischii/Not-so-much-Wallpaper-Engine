use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct WorkshopItem {
    pub id: u64,
    pub title: String,
    pub path: PathBuf,
}

// ======================================================
// PUBLIC ENTRY
// ======================================================

pub fn load_workshop_items() -> Vec<WorkshopItem> {
    let mut items = Vec::new();

    if let Some(workshop_path) = get_wallpaper_engine_workshop_path() {
        if let Ok(entries) = fs::read_dir(workshop_path) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    if let Some(id) = path.file_name()
                        .and_then(|s| s.to_str())
                        .and_then(|s| s.parse::<u64>().ok())
                    {
                        let title = read_title(&path)
                            .unwrap_or_else(|| format!("Workshop {}", id));

                        items.push(WorkshopItem {
                            id,
                            title,
                            path,
                        });
                    }
                }
            }
        }
    }

    items
}

// ======================================================
// PATH DETECTION
// ======================================================

fn get_wallpaper_engine_workshop_path() -> Option<PathBuf> {
    let steam = get_steam_path()?;

    let path = steam
        .join("steamapps")
        .join("workshop")
        .join("content")
        .join("431960"); // Wallpaper Engine app ID

    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn get_steam_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;

    let candidates = vec![
        ".steam/steam",
        ".local/share/Steam",
        ".var/app/com.valvesoftware.Steam/.steam/steam", // Flatpak
    ];

    for c in candidates {
        let path = Path::new(&home).join(c);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

// ======================================================
// METADATA PARSING
// ======================================================

fn read_title(folder: &Path) -> Option<String> {
    let project_json = folder.join("project.json");

    if let Ok(data) = fs::read_to_string(project_json) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(title) = json.get("title").and_then(|v| v.as_str()) {
                return Some(title.to_string());
            }
        }
    }

    None
}