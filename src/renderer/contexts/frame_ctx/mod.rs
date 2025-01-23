mod frame;

use color_eyre::Result;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;
use crate::renderer::contexts::frame_ctx::frame::Frame;

use super::resource_ctx::RenderResourceContext;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

/// Responsibilities:
/// - Manage per-frame command buffers
/// - Manage per-frame resources
/// - Manage synchronization between frames
pub struct RenderFrameContext {
    frames: Vec<Frame>,
}

impl RenderFrameContext {
    pub fn new(
        dev_ctx: &RenderDeviceContext,
        res_ctx: &RenderResourceContext,
    ) -> Result<Self> {
        let mut frames = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            frames.push(Frame::new(dev_ctx, res_ctx)?);
        }
        
        Ok(Self {
            frames,
        })
    }
}
