// src/engine/ui.rs
use crossbeam_channel::{unbounded, Sender, Receiver};

#[derive(Clone)]
pub enum UiCommand {
    Play,
    Pause,
    Quit,
    LoadWallpaper(String),
}

pub struct EngineUI {
    sender: Sender<UiCommand>,
}

impl EngineUI {
    pub fn new(sender: Sender<UiCommand>) -> Self {
        Self { sender }
    }
}

impl eframe::App for EngineUI {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Not-so-much Wallpaper Engine");

            if ui.button("▶ Play").clicked() {
                let _ = self.sender.send(UiCommand::Play);
            }
            if ui.button("⏸ Pause").clicked() {
                let _ = self.sender.send(UiCommand::Pause);
            }
            if ui.button("❌ Quit").clicked() {
                let _ = self.sender.send(UiCommand::Quit);
            }
        });
    }
}

pub fn start_ui_thread(sender: Sender<UiCommand>) {
    let options = eframe::NativeOptions::default();

    if let Err(e) = eframe::run_native(
        "Not-so-much Wallpaper Engine",
        options,
        Box::new(|_| Box::new(EngineUI::new(sender))),
    ) {
        eprintln!("UI error: {}", e);
    }
}
