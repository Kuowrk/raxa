mod context;
mod config;
mod resources;
mod viewport;

use color_eyre::Result;
use winit::event_loop::EventLoop;
use config::RenderConfig;
use context::RenderContext;
use resources::RenderResources;
use viewport::RenderViewport;

pub struct Renderer {
    ctx: RenderContext,
    cfg: RenderConfig,
    res: RenderResources,
    vpt: Option<RenderViewport>,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let ctx = RenderContext::new(event_loop)?;

        Ok(Self {
            ctx,
            cfg: RenderConfig::new(),
            res: RenderResources::new(),
            vpt: None,
        })
    }
}