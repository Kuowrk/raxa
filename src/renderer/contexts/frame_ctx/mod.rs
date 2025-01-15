use color_eyre::Result;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;

/// Responsibilities:
/// - Manage per-frame command buffers
/// - Manage per-frame resources
/// - Manage synchronization between frames
pub struct RenderFrameContext;

impl RenderFrameContext {
    pub fn new(_dev_ctx: &RenderDeviceContext) -> Result<Self> {
        Ok(Self)
    }
}