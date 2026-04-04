use eframe::{egui, epi};

pub struct WallpaperGui;

impl epi::App for WallpaperGui {
    fn name(&self) -> &str { "Wallpaper Engine UI" }

    fn update(&mut self, ctx: &egui::CtxRef, _: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Not so much Wallpaper Engine");

            if ui.button("Refresh Wallpapers").clicked() {
                // call engine refresh logic here
            }

            if ui.button("Select Wallpaper").clicked() {
                // implement a file dialog to pick wallpapers
            }
        });
    }
}
