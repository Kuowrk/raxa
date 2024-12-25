use color_eyre::Result;
use vulkano::sync;
use vulkano::sync::GpuFuture;
use crate::renderer::core::context::RenderContext;

/// Contains often-mutated flags and other state information
pub struct RenderState {
    pub resize_requested: bool,
    pub previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl RenderState {
    pub fn new(ctx: &RenderContext) -> Result<Self> {
        let previous_frame_end = Some(sync::now(ctx.device.clone()).boxed());

        Ok(Self {
            resize_requested: false,
            previous_frame_end,
        })
    }
}