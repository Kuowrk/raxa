pub mod camera;
pub mod util;

mod core;
mod shader_data;
mod resources;
mod vk;

use color_eyre::Result;
use std::sync::Arc;

use core::config::RenderConfig;
use core::context::RenderContext;
use core::resources::RenderResources;
use core::state::RenderState;
use core::target::RenderTarget;

pub struct Renderer {
    ctx: RenderContext,
    tgt: Option<RenderTarget>,
    cfg: RenderConfig,
    res: RenderResources,
    ste: RenderState,
}

impl Renderer {
    pub fn new(
        window: Option<Arc<winit::window::Window>>
    ) -> Result<Self> {
        let (ctx, tgt) = RenderContext::new(window)?;
        let cfg = RenderConfig::default();
        let res = RenderResources::new(&ctx)?;
        let ste = RenderState::new(&ctx)?;

        Ok(Self {
            ctx,
            tgt,
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