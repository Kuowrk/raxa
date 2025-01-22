mod frame;

use color_eyre::Result;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;
use crate::renderer::contexts::frame_ctx::frame::Frame;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

/// Responsibilities:
/// - Manage per-frame command buffers
/// - Manage per-frame resources
/// - Manage synchronization between frames
pub struct RenderFrameContext {
    frames: Vec<Frame>
}

impl RenderFrameContext {
    pub fn new(dev_ctx: &RenderDeviceContext) -> Result<Self> {
        Ok(Self {
            frames: vec![Frame::new(dev_ctx)?; MAX_FRAMES_IN_FLIGHT]
        })
    }
}