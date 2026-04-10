// src/engine/media.rs
use std::path::PathBuf;
use super::wallpaper::WebWallpaperEngine;

pub struct VideoFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub enum MediaMode {
    Video,
    Image,
}

pub struct MediaEngine {
    latest_frame: Option<VideoFrame>,
    time: f32,
    pub web_wallpapers: WebWallpaperEngine,
    mode: MediaMode,
}

impl MediaEngine {
    pub fn new() -> Self {
        Self {
            latest_frame: None,
            time: 0.0,
            web_wallpapers: WebWallpaperEngine::new(),
            mode: MediaMode::Video,
        }
    }

    // ====================== FIX: LOAD METHOD ======================
    pub fn load(&mut self, path: PathBuf) {
        println!("[media] load: {:?}", path);

        // For now: treat everything as image wallpaper
        self.set_image_mode();
    }
    // =============================================================

    pub fn set_image_mode(&mut self) {
        println!("[media] switched to IMAGE mode");
        self.mode = MediaMode::Image;
        self.latest_frame = None;
    }

    pub fn set_video_mode(&mut self) {
        println!("[media] switched to VIDEO mode");
        self.mode = MediaMode::Video;
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.web_wallpapers.update();

        // 🚫 Do NOT generate frames in image mode
        if let MediaMode::Image = self.mode {
            return;
        }

        // ================= VIDEO FRAME GENERATION =================
        let w = 640;
        let h = 360;
        let mut pixels = vec![0u8; (w * h * 4) as usize];

        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                pixels[i]     = (x as f32 + self.time * 60.0) as u8;
                pixels[i + 1] = (y as f32 + self.time * 40.0) as u8;
                pixels[i + 2] = 180;
                pixels[i + 3] = 255;
            }
        }

        self.latest_frame = Some(VideoFrame {
            pixels,
            width: w,
            height: h,
        });
    }

    pub fn take_frame(&mut self) -> Option<VideoFrame> {
        self.latest_frame.take()
    }
}