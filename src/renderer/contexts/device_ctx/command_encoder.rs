use std::sync::{Arc, Mutex};
use color_eyre::Result;
use ash::vk;
use color_eyre::eyre::eyre;
use crate::renderer::contexts::device_ctx::command_buffer_allocator::CommandEncoderAllocator;
use crate::renderer::contexts::device_ctx::queue::Queue;
use crate::renderer::resources::image::Image;

pub struct CommandEncoder {
    pub command_buffer: vk::CommandBuffer,
    pub queue: Arc<Queue>,

    is_recording: bool,

    device: Arc<ash::Device>,
    allocator: CommandEncoderAllocator,
}

impl CommandEncoder {
    pub fn new(
        command_buffer: vk::CommandBuffer,
        queue: Arc<Queue>,
        device: Arc<ash::Device>,
        allocator: CommandEncoderAllocator,
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
            self.device.begin_command_buffer(self.command_buffer, &begin_info)?;
        }

        self.is_recording = true;

        Ok(())
    }

    pub fn end_recording(&mut self) -> Result<()> {
        if !self.is_recording {
            return Err(eyre!("Command buffer is not recording"));
        }

        unsafe {
            self.device.end_command_buffer(self.command_buffer)?
        }

        self.is_recording = false;

        Ok(())
    }

    pub fn transition_image_layout(
        &self,
        image: &mut Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        image.transition_layout(
            self.command_buffer,
            old_layout,
            new_layout,
        )
    }

    pub fn copy_image_to_image(
        &self,
        src_image: &Image,
        dst_image: &Image,
    ) {
        src_image.copy_to_image(
            self.command_buffer,
            dst_image,
        )
    }
}

impl Drop for CommandEncoder {
    fn drop(&mut self) {
        if self.is_recording {
            log::warn!("Dropping CommandEncoder while still recording");
        }

        let mut allocator = self.allocator.0.lock().unwrap();
        allocator.free(self).unwrap();
    }
}
