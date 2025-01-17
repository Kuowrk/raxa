use std::sync::{Arc, Mutex};
use color_eyre::Result;
use ash::vk;
use color_eyre::eyre::eyre;
use crate::renderer::contexts::device_ctx::command_buffer_allocator::CommandEncoderAllocator;
use crate::renderer::contexts::device_ctx::queue::Queue;

pub struct CommandEncoder {
    pub command_buffer: vk::CommandBuffer,
    pub queue: Arc<Queue>,

    is_recording: bool,

    device: Arc<ash::Device>,
    allocator: Arc<Mutex<CommandEncoderAllocator>>,
}

impl CommandEncoder {
    pub fn new(
        command_buffer: vk::CommandBuffer,
        queue: Arc<Queue>,
        device: Arc<ash::Device>,
        allocator: Arc<Mutex<CommandEncoderAllocator>>,
    ) -> Self {
        Self {
            command_buffer,
            queue,
            device,
            allocator,
            is_recording: false,
        }
    }

    pub fn begin_recording(&mut self) -> Result<()> {
        if self.is_recording {
            return Err(eyre!("Command buffer is already recording"));
        }

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            self.device.begin_command_buffer(*self.command_buffer, &begin_info)?;
        }

        self.is_recording = true;

        Ok(())
    }

    pub fn end_recording(&mut self) -> Result<()> {
        if !self.is_recording {
            return Err(eyre!("Command buffer is not recording"));
        }

        unsafe {
            self.device.end_command_buffer(*self.command_buffer)?
        }

        self.is_recording = false;

        Ok(())
    }
}

impl Drop for CommandEncoder {
    fn drop(&mut self) {
        if self.is_recording {
            log::warn!("Dropping CommandEncoder while still recording");
        }

        let mut allocator = self.allocator.lock().unwrap();
        allocator.free(self).unwrap();
    }
}