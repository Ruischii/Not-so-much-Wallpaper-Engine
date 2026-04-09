// src/engine/mod.rs
pub mod core;
pub mod render;
pub mod gpu;
pub mod wayland;
pub mod media;
pub mod ui;
pub mod wallpaper;
pub mod videonode;

// Public re-exports
pub use core::Engine;
pub use media::VideoFrame;
pub use render::{RenderContext, RenderGraph, RenderNode};
pub use ui::UiCommand;
pub use videonode::VideoNode;
pub use wallpaper::{CompositorType, WebWallpaper, WebWallpaperEngine};
