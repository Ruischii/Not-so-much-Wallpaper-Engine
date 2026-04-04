use wayland_client::{
    protocol::{wl_compositor::WlCompositor, wl_surface::WlSurface},
    Connection, QueueHandle,
};

use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
};

pub struct WaylandState {
    pub compositor: WlCompositor,
    pub layer_shell: ZwlrLayerShellV1,
}

pub struct WallpaperSurface {
    pub wl_surface: WlSurface,
    pub layer_surface: ZwlrLayerSurfaceV1,
}
