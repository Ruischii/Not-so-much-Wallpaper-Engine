mod state;
mod wallpaper;

use anyhow::Result;
use wayland_client::{Connection, EventQueue};

use smithay_client_toolkit::registry::{RegistryHandler, RegistryState};

use state::WaylandState;

pub fn run_wayland_wallpaper() -> Result<()> {
    let conn = Connection::connect_to_env()?;

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    // registry setup
    let display = conn.display();

    println!("Connected to Wayland compositor");

    loop {
        event_queue.blocking_dispatch(&mut ())?;
    }
}
