use crate::renderer::contexts::resource_ctx::allocator::RenderResourceAllocator;
use crate::renderer::contexts::resource_ctx::storage::RenderResourceStorage;

pub mod allocator;
pub mod storage;

/// Responsibilities:
/// - Manage resources like buffers, images, and samplers
/// - Allocate descriptor sets and memory for resources
/// - Track resource lifetimes
pub struct RenderResourceContext {
    pub allocator: RenderResourceAllocator,
    pub storage: RenderResourceStorage,
}