use wayland_client::protocol::wl_buffer::WlBuffer;

pub struct SurfaceManager {
    pub buffers: Vec<WlBuffer>,
}

impl SurfaceManager {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
        }
    }

    pub fn attach(&self) {
        // later:
        // wl_surface.attach(...)
    }

    pub fn commit(&self) {
        // wl_surface.commit()
    }
}
