use crate::renderer::contexts::resource_ctx::allocator::RenderResourceAllocator;
use crate::renderer::contexts::resource_ctx::storage::RenderResourceStorage;

pub mod allocator;
pub mod storage;
/// "Internals" refers to low-level objects that are used to implement the "Resources" objects.
/// They should not be used directly by the user.

pub mod descriptor_set_layout_builder;

/// Responsibilities:
/// - Manage resources like buffers, images, and samplers
/// - Allocate descriptor sets and memory for resources
/// - Track resource lifetimes
pub struct RenderResourceContext {
    pub allocator: RenderResourceAllocator,
    pub storage: RenderResourceStorage,
}