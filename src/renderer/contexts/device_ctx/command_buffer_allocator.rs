use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use ash::vk;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use crate::renderer::contexts::device_ctx::command_encoder::CommandEncoder;
use crate::renderer::contexts::device_ctx::queue::{Queue, QueueFamily};

type CommandEncoderAllocatorHandle = Arc<Mutex<CommandEncoderAllocator>>;

pub struct CommandEncoderAllocator {
    command_pools: HashMap<QueueFamily, vk::CommandPool>,
    allocated_command_buffers: HashMap<QueueFamily, Vec<vk::CommandBuffer>>,
    device: Arc<ash::Device>,
}

impl CommandEncoderAllocator {
    pub fn new(
        device: Arc<ash::Device>,
    ) -> Result<Self> {
        Ok(Self {
            command_pools: HashMap::new(),
            allocated_command_buffers: HashMap::new(),
            device,
        })
    }

    pub fn allocate(&mut self, queue: Arc<Queue>) -> Result<CommandEncoder> {
        let command_pool = self
            .command_pools
            .entry(queue.family.clone())
            .or_insert_with(|| {
                let command_pool_info = vk::CommandPoolCreateInfo::default()
                    .queue_family_index(queue.family.index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
                unsafe {
                    self.device.create_command_pool(&command_pool_info, None)?
                }
            });

        let command_buffer_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(*command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffer = unsafe {
            self.device.allocate_command_buffers(&command_buffer_info)?[0]
        };

        self.allocated_command_buffers
            .entry(queue.family.clone())
            .or_insert_with(Vec::new)
            .push(command_buffer);

        let command_encoder = CommandEncoder::new(
            command_buffer,
            queue,
            self.device.clone(),
            self.clone(),
        );

        Ok(command_encoder)
    }

    pub fn free(&mut self, command_encoder: &CommandEncoder) -> Result<()> {
        let command_pool = self.command_pools.get(&command_encoder.queue.family).unwrap();
        let command_buffer = command_encoder.command_buffer;
        unsafe {
            self.device.free_command_buffers(*command_pool, &[command_buffer]);
        }
        let mut command_buffers = self.allocated_command_buffers
            .get(&command_encoder.queue.family)
            .ok_or_eyre(format!("Failed to get command buffers for queue family: {}", command_encoder.queue.family.index))?;
        let index = command_buffers
            .iter()
            .position(|&cb| cb == command_buffer)
            .ok_or_eyre(format!("Failed to find command buffer in vec for queue family: {}", command_encoder.queue.family.index))?;
        let _ = command_buffers.swap_remove(index);
        Ok(())
    }
}

impl Drop for CommandEncoderAllocator {
    fn drop(&mut self) {
        for (queue_family, command_pool) in self.command_pools.drain() {
            let command_buffers = self.allocated_command_buffers.remove(&queue_family).unwrap();
            unsafe {
                self.device.free_command_buffers(command_pool, &command_buffers);
            }
            unsafe {
                self.device.destroy_command_pool(*command_pool, None);
            }
        }
    }
}