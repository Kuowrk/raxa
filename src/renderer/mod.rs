mod context;
mod config;
mod resources;
mod viewport;

use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use config::RenderConfig;
use context::RenderContext;
use resources::RenderResources;
use std::sync::Arc;
use viewport::RenderViewport;
use winit::event_loop::EventLoop;
use winit::window::Window;

pub struct Renderer {
    ctx: RenderContext,
    cfg: RenderConfig,
    res: RenderResources,
    vpt: Option<RenderViewport>,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let ctx = RenderContext::new(event_loop)?;
        let cfg = RenderConfig::default();
        let res = RenderResources::default();

        Ok(Self {
            ctx,
            cfg,
            res,
            vpt: None,
        })
    }

    pub fn set_window(&mut self, window: Arc<Window>) -> Result<()> {
        self.vpt = Some(RenderViewport::new(window, &self.ctx)?);

        Ok(())
    }
}