pub mod camera;
pub mod util;

mod contexts;
mod shader_data;
mod resources;
mod internals;

use color_eyre::Result;
use std::sync::Arc;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;
use crate::renderer::contexts::resource_ctx::RenderResourceContext;
use crate::renderer::contexts::graph_ctx::RenderGraphContext;
use crate::renderer::contexts::frame_ctx::RenderFrameContext;
use crate::renderer::contexts::pipeline_ctx::RenderPipelineContext;

pub struct Renderer {
    dev: RenderDeviceContext,
    res: RenderResourceContext,
    grp: RenderGraphContext,
    frm: RenderFrameContext,
    pip: RenderPipelineContext,

    resize_requested: bool,
}

impl Renderer {
    pub fn new(
        window: Option<Arc<winit::window::Window>>
    ) -> Result<Self> {
    }

    pub fn request_resize(&mut self) {
        self.resize_requested = true;
    }

    pub fn draw(&mut self) -> Result<()> {
        Ok(())
    }
}