pub mod camera;
pub mod util;

mod core;
mod shader_data;
mod resources;
mod vk;

use color_eyre::Result;
use std::sync::Arc;

use core::config::RenderConfig;
use core::instance::RenderInstance;
use core::resources::RenderResources;
use core::state::RenderState;
use core::target::RenderTarget;
use crate::renderer::core::device::RenderDevice;

pub struct Renderer<'a> {
    ins: RenderInstance<'a>,
    tgt: Option<RenderTarget>,
    dev: RenderDevice<'a>,
    cfg: RenderConfig,
    res: RenderResources,
    ste: RenderState,
}

impl Renderer {
    pub fn new(
        window: Option<Arc<winit::window::Window>>
    ) -> Result<Self> {
        let (ins, tgt) = RenderInstance::new(window)?;
        let dev = ins.create_device(tgt.as_ref())?;
        let cfg = RenderConfig::default();
        let res = RenderResources::new(&dev)?;
        let ste = RenderState::new()?;

        Ok(Self {
            ins,
            tgt,
            dev,
            cfg,
            res,
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