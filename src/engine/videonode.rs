// src/engine/videonode.rs

use std::any::Any;

use anyhow::Result;
use memmap2::MmapMut;
use wayland_client::{
    protocol::{wl_buffer::WlBuffer, wl_shm::WlShm},
    QueueHandle,
};

// Changed: Use engine's own wayland module instead of crate::wayland
use super::wayland::WaylandBackend;
use super::media::VideoFrame;
use super::render::{RenderContext, RenderNode};

// We will define a local App type alias or placeholder
// For now, we assume the App type is re-exported or defined in engine::wayland
// If App is only in top-level wayland, we'll create a simple placeholder.

pub type App = (); // Temporary placeholder until you have real App type

pub struct VideoNode {
    backend: WaylandBackend,
    shm: WlShm,
    qh: QueueHandle<App>,

    buffer: Option<WlBuffer>,
    mmap: Option<MmapMut>,

    width: u32,
    height: u32,

    pending_frame: Option<VideoFrame>,
}

impl VideoNode {
    pub fn new(backend: WaylandBackend, shm: WlShm, qh: QueueHandle<App>) -> Self {
        Self {
            backend,
            shm,
            qh,
            buffer: None,
            mmap: None,
            width: 0,
            height: 0,
            pending_frame: None,
        }
    }

    fn ensure_buffer(&mut self, w: u32, h: u32) {
        if self.buffer.is_some() && self.width == w && self.height == h {
            return;
        }

        if let Ok((buf, mmap)) = self.backend.create_buffer(&self.shm, w, h, &self.qh) {
            self.buffer = Some(buf);
            self.mmap = Some(mmap);
            self.width = w;
            self.height = h;
        }
    }

    pub fn submit_frame(&mut self, frame: VideoFrame) {
        self.pending_frame = Some(frame);
    }
}

impl RenderNode for VideoNode {
    fn render(&mut self, _ctx: &mut RenderContext) {
        if let Some(frame) = self.pending_frame.take() {
            self.ensure_buffer(frame.width, frame.height);

            if let Some(mem) = &mut self.mmap {
                mem[..frame.pixels.len()].copy_from_slice(&frame.pixels);
            }
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
