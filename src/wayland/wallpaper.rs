use anyhow::Result;

use wayland_client::{
    protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    Connection, Dispatch, QueueHandle,
};

use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, ZwlrLayerSurfaceV1},
};

use super::state::{WallpaperSurface, WaylandState};

pub fn create_wallpaper_surface(
    state: &WaylandState,
    output: &WlOutput,
    qh: &QueueHandle<WaylandState>,
) -> Result<WallpaperSurface> {
    let surface = state.compositor.create_surface(qh, ());

    let layer_surface = state.layer_shell.get_layer_surface(
        &surface,
        Some(output),
        Layer::Background,
        "not-wallpaper-engine".into(),
        qh,
        (),
    );

    // Fill entire screen
    layer_surface.set_anchor(
        Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
    );

    layer_surface.set_exclusive_zone(-1);
    layer_surface.set_keyboard_interactivity(0);

    surface.commit();

    Ok(WallpaperSurface {
        wl_surface: surface,
        layer_surface,
    })
}
