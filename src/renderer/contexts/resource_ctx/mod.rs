pub mod resource_allocator;
pub mod resource_storage;
pub mod descriptor_set_layout_builder;

use color_eyre::Result;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;
use crate::renderer::contexts::resource_ctx::resource_allocator::RenderResourceAllocator;
use crate::renderer::contexts::resource_ctx::resource_storage::RenderResourceStorage;

/// Responsibilities:
/// - Manage resources like buffers, images, and samplers
/// - Allocate descriptor sets and memory for resources
/// - Track resource lifetimes
pub struct RenderResourceContext {
    pub storage: RenderResourceStorage,
}

impl RenderResourceContext {
    pub fn new(
        dev_ctx: &RenderDeviceContext,
    ) -> Result<Self> {
        let storage = RenderResourceStorage::new(dev_ctx)?;

        Ok(Self {
            storage,
        })
    }
}