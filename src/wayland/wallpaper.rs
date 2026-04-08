use anyhow::Result;
use memmap2::MmapMut;
use std::{fs::File, os::fd::AsFd, time::Instant};

use tempfile::tempfile;

use wayland_client::{
    globals::registry_queue_init,
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_output::WlOutput,
        wl_registry,
        wl_shm::{Format, WlShm},
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, QueueHandle,
};

use wayland_client::globals::GlobalListContents;

use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, Event as LayerEvent, ZwlrLayerSurfaceV1},
};
/// Application State
pub(crate) struct App {
    compositor: Option<WlCompositor>,
    shm: Option<WlShm>,
    layer_shell: Option<ZwlrLayerShellV1>,
    surface: Option<WlSurface>,
    layer_surface: Option<ZwlrLayerSurfaceV1>,
    buffer: Option<WlBuffer>,
    mmap: Option<MmapMut>,

    width: u32,
    height: u32,
    start: Instant,
}

impl App {
    fn new() -> Self {
        Self {
            compositor: None,
            shm: None,
            layer_shell: None,
            surface: None,
            layer_surface: None,
            buffer: None,
            mmap: None,
            width: 1920,
            height: 1080,
            start: Instant::now(),
        }
    }
}
/// Registry Handling
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for App {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor =
                        Some(registry.bind(name, version, qh, ()));
                }
                "wl_shm" => {
                    state.shm =
                        Some(registry.bind(name, version, qh, ()));
                }
                "zwlr_layer_shell_v1" => {
                    state.layer_shell =
                        Some(registry.bind(name, version, qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<WlCompositor, ()> for App {
    fn event(
        _: &mut Self,
        _: &WlCompositor,
        _: wayland_client::protocol::wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<WlShm, ()> for App {
    fn event(
        _: &mut Self,
        _: &WlShm,
        _: wayland_client::protocol::wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<ZwlrLayerShellV1, ()> for App {
    fn event(
        _: &mut Self,
        _: &ZwlrLayerShellV1,
        _: wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {}
}
/// Layer Surface Events
impl Dispatch<ZwlrLayerSurfaceV1, ()> for App {
    fn event(
        state: &mut Self,
        layer: &ZwlrLayerSurfaceV1,
        event: LayerEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let LayerEvent::Configure { serial, width, height } = event {
            layer.ack_configure(serial);

            if width > 0 {
                state.width = width;
            }
            if height > 0 {
                state.height = height;
            }
        }
    }
}

impl Dispatch<WlSurface, ()> for App {
    fn event(
        _: &mut Self,
        _: &WlSurface,
        _: wayland_client::protocol::wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

use wayland_client::protocol::wl_shm_pool::WlShmPool;
impl Dispatch<WlShmPool, ()> for App {
    fn event(
        _: &mut Self,
        _: &WlShmPool,
        _: wayland_client::protocol::wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlBuffer, ()> for App {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        _: wayland_client::protocol::wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
/// SHM Buffer Creation
pub(crate) fn create_buffer(
    shm: &WlShm,
    width: u32,
    height: u32,
    qh: &QueueHandle<App>,
) -> Result<(WlBuffer, MmapMut)> {
    let stride = width * 4;
    let size = (stride * height) as usize;

    let file: File = tempfile()?;
    file.set_len(size as u64)?;

    let mmap = unsafe { MmapMut::map_mut(&file)? };

    let pool = shm.create_pool(file.as_fd(), size as i32, qh, ());
    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride as i32,
        Format::Argb8888,
        qh,
        (),
    );

    Ok((buffer, mmap))
}
/// Engine Initialization
pub fn init() -> Result<(WlShm, QueueHandle<App>, WlSurface)> {
    let conn = Connection::connect_to_env()?;
    let (_globals, mut event_queue) = registry_queue_init::<App>(&conn)?;

    let qh = event_queue.handle();
    let mut app = App::new();

    event_queue.blocking_dispatch(&mut app)?;

    let compositor = app
        .compositor
        .as_ref()
        .expect("wl_compositor not available");

    let layer_shell = app
        .layer_shell
        .as_ref()
        .expect("layer shell not available");

    let surface = compositor.create_surface(&qh, ());

    let layer_surface = layer_shell.get_layer_surface(
        &surface,
        None::<&WlOutput>,
        Layer::Background,
        "wallpaper".into(),
        &qh,
        (),
    );

    layer_surface.set_anchor(
        Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
    );
    layer_surface.set_exclusive_zone(-1);

    surface.commit();

    event_queue.blocking_dispatch(&mut app)?;

    Ok((
        app.shm.clone().expect("wl_shm not available"),
        qh,
        surface,
    ))
}
/// Fill buffer with animated gradient
pub fn draw_wallpaper(
    shm: &WlShm,
    surface: &WlSurface,
    qh: &QueueHandle<App>,
) -> Result<()> {
    let width = 1920;
    let height = 1080;

    let (buffer, mut mmap) = create_buffer(shm, width, height, qh)?;

    // ARGB8888 pixels
    for y in 0..height {
        for x in 0..width {
            let offset = ((y * width + x) * 4) as usize;

            let r = (x % 255) as u8;
            let g = (y % 255) as u8;
            let b = 180u8;

            mmap[offset + 0] = b;
            mmap[offset + 1] = g;
            mmap[offset + 2] = r;
            mmap[offset + 3] = 255;
        }
    }

    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(
        0,
        0,
        width as i32,
        height as i32,
    );
    surface.commit();

    Ok(())
}
