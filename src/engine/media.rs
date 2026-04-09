// src/engine/media.rs
use super::wallpaper::WebWallpaperEngine;

pub struct VideoFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub struct MediaEngine {
    latest_frame: Option<VideoFrame>,
    time: f32,
    pub web_wallpapers: WebWallpaperEngine,
}

impl MediaEngine {
    pub fn new() -> Self {
        Self {
            latest_frame: None,
            time: 0.0,
            web_wallpapers: WebWallpaperEngine::new(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.web_wallpapers.update();

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

        self.latest_frame = Some(VideoFrame { pixels, width: w, height: h });
    }

    pub fn take_frame(&mut self) -> Option<VideoFrame> {
        self.latest_frame.take()
    }
}
