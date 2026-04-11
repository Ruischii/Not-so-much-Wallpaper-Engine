// src/main.rs
mod engine;           // This declares the engine module (folder)
mod image;
mod loader;

use crossbeam_channel::unbounded;
use engine::ui::{run_ui, UiCommand};   // ← Updated path

fn main() {
    // Create channel for communication between UI and engine
    let (tx, rx) = unbounded();

    // Start the engine in a background thread FIRST
    let engine_handle = std::thread::spawn(move || {
        println!("[engine] Started");

        for cmd in rx {
            match cmd {
                UiCommand::Quit => {
                    println!("[engine] Quit received, shutting down");
                    break;
                }
            }
        }
        println!("[engine] Stopped");
    });

    // Run UI on the MAIN thread (required for egui + Wayland)
    println!("[ui] Starting on main thread");
    if let Err(e) = run_ui(tx) {
        eprintln!("[ui] Error: {}", e);
    }
    println!("[ui] Closed");

    // Wait for engine thread to finish
    if let Err(e) = engine_handle.join() {
        eprintln!("[engine] Join error: {:?}", e);
    }
}