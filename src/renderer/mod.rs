pub mod camera;

mod contexts;
mod shader_data;
mod resources;

use color_eyre::Result;
use std::sync::Arc;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;
use crate::renderer::contexts::resource_ctx::RenderResourceContext;
use crate::renderer::contexts::graph_ctx::RenderGraphContext;
use crate::renderer::contexts::frame_ctx::RenderFrameContext;
use crate::renderer::contexts::pipeline_ctx::RenderPipelineContext;

pub struct Renderer {
    dev_ctx: RenderDeviceContext,
    res_ctx: RenderResourceContext,
    grp_ctx: RenderGraphContext,
    frm_ctx: RenderFrameContext,
    pip_ctx: RenderPipelineContext,

    resize_requested: bool,
}

impl Renderer {
    pub fn new(
        window: Option<Arc<winit::window::Window>>
    ) -> Result<Self> {
        let dev_ctx = RenderDeviceContext::new(window)?;
        let res_ctx = RenderResourceContext::new(&dev_ctx)?;
        let frm_ctx = RenderFrameContext::new(&dev_ctx, &res_ctx)?;
        let grp_ctx = RenderGraphContext::new(&dev_ctx)?;
        let pip_ctx = RenderPipelineContext::new(&dev_ctx)?;

        Ok(Self {
            dev_ctx,
            res_ctx,
            grp_ctx,
            frm_ctx,
            pip_ctx,

            resize_requested: false,
        })
    }

    pub fn request_resize(&mut self) {
        self.resize_requested = true;
    }

    pub fn draw(&mut self) -> Result<()> {
        Ok(())
    }
}
