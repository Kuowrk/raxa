use std::collections::{hash_map, HashMap};
use std::sync::{Arc, Mutex};
use ash::vk;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use crate::renderer::contexts::device_ctx::command_encoder::CommandEncoder;
use crate::renderer::contexts::device_ctx::queue::{Queue, QueueFamily};

#[repr(transparent)]
pub struct CommandEncoderAllocator(pub Arc<Mutex<CommandEncoderAllocatorInner>>);

impl Clone for CommandEncoderAllocator {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub trait CommandEncoderAllocatorExt<A> {
    fn new(device: Arc<ash::Device>) -> Result<A>;
    fn allocate(&mut self, queue: Arc<Queue>) -> Result<CommandEncoder>;
    fn free(&mut self, command_encoder: &CommandEncoder) -> Result<()>;
}

struct CommandEncoderAllocatorInner {
    command_pools: HashMap<QueueFamily, vk::CommandPool>,
    allocated_command_buffers: HashMap<QueueFamily, Vec<vk::CommandBuffer>>,
    device: Arc<ash::Device>,
}

impl CommandEncoderAllocatorExt<CommandEncoderAllocator> for CommandEncoderAllocator {
    fn new(
        device: Arc<ash::Device>,
    ) -> Result<CommandEncoderAllocator> {
        Ok(CommandEncoderAllocator(
            Arc::new(Mutex::new(CommandEncoderAllocatorInner {
                command_pools: HashMap::new(),
                allocated_command_buffers: HashMap::new(),
                device,
            }
        ))))
    }

    fn allocate(&mut self, queue: Arc<Queue>) -> Result<CommandEncoder> {
        let (command_buffer, device) = {
            let mut guard = self.0
                .lock()
                .map_err(|e| eyre!(e.to_string()))?;

            let device = guard.device.clone();

            let command_pool = match guard.command_pools.entry(queue.family.clone()) {
                hash_map::Entry::Vacant(entry) => {
                    let pool_info = vk::CommandPoolCreateInfo::default()
                        .queue_family_index(queue.family.index)
                        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
                    let pool = unsafe {
                        device.create_command_pool(&pool_info, None)?
                    };
                    entry.insert(pool)
                }
                hash_map::Entry::Occupied(entry) => {
                    entry.into_mut()
                }
            };

            let command_buffer_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(*command_pool)
                .command_buffer_count(1)
                .level(vk::CommandBufferLevel::PRIMARY);
            let command_buffer = unsafe {
                guard.device.allocate_command_buffers(&command_buffer_info)?[0]
            };

            guard.allocated_command_buffers
                .entry(queue.family.clone())
                .or_insert_with(Vec::new)
                .push(command_buffer);

            (command_buffer, device)
        };

        let command_encoder = CommandEncoder::new(
            command_buffer,
            queue,
            device,
            self.clone(),
        );

        Ok(command_encoder)
    }

    fn free(&mut self, command_encoder: &CommandEncoder) -> Result<()> {
        let mut guard = self.0
            .lock()
            .map_err(|e| eyre!(e.to_string()))?;
            
        let command_pool = guard.command_pools.get(&command_encoder.queue.family).unwrap();
        let command_buffer = command_encoder.command_buffer;
        unsafe {
            guard.device.free_command_buffers(*command_pool, &[command_buffer]);
        }
        let command_buffers = guard.allocated_command_buffers
            .get_mut(&command_encoder.queue.family)
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
        let mut guard = self.0
            .lock()
            .map_err(|e| eyre!(e.to_string()))
            .unwrap();

        let command_pools = guard.command_pools
            .drain()
            .collect::<Vec<_>>();
        
        for (queue_family, command_pool) in command_pools {
            let command_buffers = guard.allocated_command_buffers.remove(&queue_family).unwrap();
            unsafe {
                guard.device.free_command_buffers(command_pool, &command_buffers);
                guard.device.destroy_command_pool(command_pool, None);
            }
        }
    }
}
