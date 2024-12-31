use ash::vk;
use color_eyre::Result;
use crate::renderer::vk::queue::Queue;

/// Each CommandBufferAllocator is associated with a single queue
pub struct CommandBufferAllocator<'a> {
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    device: &'a ash::Device,
    queue: &'a Queue,
}

impl CommandBufferAllocator<'_> {
    pub fn new(
        device: &ash::Device,
        queue: &Queue,
    ) -> Result<Self> {
        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue.family.index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe {
            device.create_command_pool(&command_pool_info, None)?
        };

        Ok(Self {
            command_pool,
            command_buffers: Vec::new(),
            device,
            queue,
        })
    }

    pub fn allocate(&mut self) -> Result<vk::CommandBuffer> {
        let command_buffer_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffer = unsafe {
            self.device.allocate_command_buffers(&command_buffer_info)?[0]
        };

        self.command_buffers.push(command_buffer);
        Ok(command_buffer)
    }
}

impl Drop for CommandBufferAllocator<'_> {
    fn drop(&mut self) {
        unsafe {
            self.device.free_command_buffers(self.command_pool, &self.command_buffers);
            self.device.destroy_command_pool(self.command_pool, None);
        }
    }
}