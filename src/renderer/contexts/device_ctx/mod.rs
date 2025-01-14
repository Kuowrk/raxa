pub mod instance;
pub mod device;
pub mod target;

use std::sync::Arc;
use color_eyre::Result;
use crate::renderer::contexts::device_ctx::device::RenderDevice;
use crate::renderer::contexts::device_ctx::instance::RenderInstance;
use crate::renderer::contexts::device_ctx::target::RenderTarget;

/// Responsibilities:
/// - Manage the Vulkan instance, device, and queues
/// - Create and submit command buffers to queues
/// - Create synchronization primitives
pub struct RenderDeviceContext {
    pub instance: RenderInstance,
    pub device: RenderDevice,
    pub target: Option<RenderTarget>,
}

impl RenderDeviceContext {
    pub fn new(
        window: Option<Arc<winit::window::Window>>
    ) -> Result<Self> {
        let instance = RenderInstance::new(window.clone())?;
        let surface = if let Some(window) = window.as_ref() {
            Some(instance.create_surface(window)?)
        } else {
            None
        };
        let device = instance.create_device(surface.as_ref())?;
        let target = if let (
            Some(window),
            Some(surface),
        ) = (window, surface) {
            Some(instance.create_target(window, surface, &device)?)
        } else {
            None
        };

        Ok(Self {
            instance,
            device,
            target,
        })
    }
}