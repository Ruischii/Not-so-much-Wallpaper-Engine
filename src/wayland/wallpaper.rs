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

/// =============================
/// Application State
/// =============================
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

/// =============================
/// Registry Handling
/// =============================
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

/// =============================
/// Layer Surface Events
/// =============================
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

/// =============================
/// SHM Buffer Creation
/// =============================
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

/// =============================
/// Software Renderer
/// =============================
fn draw(app: &mut App) {
    let t = app.start.elapsed().as_secs_f32();

    let buf = app.mmap.as_mut().unwrap();

    for y in 0..app.height {
        for x in 0..app.width {
            let i = ((y * app.width + x) * 4) as usize;

            let r = ((x as f32 * 0.2 + t * 50.0) as u8);
            let g = ((y as f32 * 0.2) as u8);
            let b = ((t.sin() * 127.0 + 128.0) as u8);

            buf[i] = b;
            buf[i + 1] = g;
            buf[i + 2] = r;
            buf[i + 3] = 255;
        }
    }
}

/// =============================
/// Run
/// =============================
pub fn run() -> Result<()> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init::<App>(&conn)?;

    let qh = event_queue.handle();
    let mut app = App::new();

    event_queue.blocking_dispatch(&mut app)?;

    let compositor = app.compositor.as_ref().unwrap();
    let layer_shell = app.layer_shell.as_ref().unwrap();

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

    app.surface = Some(surface);
    app.layer_surface = Some(layer_surface);

    event_queue.blocking_dispatch(&mut app)?;

    let (buffer, mmap) =
        create_buffer(app.shm.as_ref().unwrap(), app.width, app.height, &qh)?;

    app.buffer = Some(buffer);
    app.mmap = Some(mmap);

    loop {
        draw(&mut app);

        let surface = app.surface.as_ref().unwrap();
        surface.attach(app.buffer.as_ref(), 0, 0);
        surface.damage_buffer(
            0,
            0,
            app.width as i32,
            app.height as i32,
        );
        surface.commit();

        event_queue.blocking_dispatch(&mut app)?;
    }
}
