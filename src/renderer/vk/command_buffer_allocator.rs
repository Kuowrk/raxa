use std::sync::Arc;
use ash::vk;

pub struct CommandBufferAllocator {
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    device: Arc<ash::Device>,
}