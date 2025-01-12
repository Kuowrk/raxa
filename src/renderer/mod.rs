pub mod camera;
pub mod util;

mod core;
mod shader_data;
mod resources;
mod internals;

use color_eyre::Result;
use std::sync::Arc;
use core::config::RenderConfig;
use core::instance::RenderInstance;
use core::state::RenderState;
use core::target::RenderTarget;
use crate::renderer::core::device::RenderDevice;

pub struct Renderer {
    ins: RenderInstance,
    tgt: Option<RenderTarget>,
    dev: RenderDevice,
    cfg: RenderConfig,
    ste: RenderState,
}

impl Renderer {
    pub fn new(
        window: Option<Arc<winit::window::Window>>
    ) -> Result<Self> {
        let ins = RenderInstance::new(window.clone())?;
        let surface = if let Some(window) = window.as_ref() {
            Some(ins.create_surface(window)?)
        } else {
            None
        };
        let dev = ins.create_device(surface.as_ref())?;
        let tgt = if let (
            Some(window),
            Some(surface),
        ) = (window, surface) {
            Some(ins.create_target(window, surface, &dev)?)
        } else {
            None
        };
        let cfg = RenderConfig::default();
        let ste = RenderState::new()?;

        Ok(Self {
            ins,
            tgt,
            dev,
            cfg,
            ste,
        })
    }

    pub fn request_resize(&mut self) {
        self.ste.resize_requested = true;
    }

    pub fn draw(&mut self) -> Result<()> {
        Ok(())
    }
}