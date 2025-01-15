use color_eyre::Result;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;

pub mod graph;

/// Responsibilities:
/// - Manage the RenderGraph object
/// - Build and schedule passes based on dependencies
/// - Record command buffers in the correct order
pub struct RenderGraphContext;

impl RenderGraphContext {
    pub fn new(_dev_ctx: &RenderDeviceContext) -> Result<Self> {
        Ok(Self)
    }
}