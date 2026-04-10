// src/loader.rs
use std::path::Path;
use egui_extras::RetainedImage;
use std::sync::Arc;

/// Simple synchronous image loader with error handling
pub fn load_retained_image(path: &Path) -> Result<RetainedImage, String> {
    RetainedImage::from_image_path(path)
        .map_err(|e| format!("Failed to load image '{}': {}", path.display(), e))
}

/// Load image from raw bytes (useful for embedded assets or network)
pub fn load_from_bytes(name: &str, bytes: &[u8]) -> Result<RetainedImage, String> {
    RetainedImage::from_image_bytes(name, bytes)
        .map_err(|e| format!("Failed to load bytes as image '{}': {}", name, e))
}

/// Async-friendly wrapper (you can call this from a thread if needed)
pub fn load_retained_image_async(
    path: PathBuf,
) -> impl FnOnce() -> Result<RetainedImage, String> + Send {
    move || load_retained_image(&path)
}

/// Helper to install image loaders in eframe (call once in run_native)
pub fn install_image_loaders(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    println!("[loader] Image loaders installed (PNG, JPEG, WEBP, etc.)");
}