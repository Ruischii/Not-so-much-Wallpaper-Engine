// src/engine/mod.rs
pub mod core;
pub mod render;
pub mod gpu;
pub mod wayland;
pub mod media;
pub mod ui;
pub mod wallpaper;
pub mod videonode;
pub mod workshop;  // ← Add this

// Public re-exports
pub use core::Engine;
pub use workshop::WorkshopManager;
