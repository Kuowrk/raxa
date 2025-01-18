use std::sync::Arc;
use ash::vk;

pub struct Material {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    pipeline_bind_point: vk::PipelineBindPoint,
    descriptor_set: vk::DescriptorSet,
    device: Arc<ash::Device>,
}

impl Material {
    pub fn update_push_constants(
        &self,
        command_buffer: vk::CommandBuffer,
        stage_flags: vk::ShaderStageFlags,
        data: &[u8],
    ) {
        unsafe {
            self.device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                stage_flags,
                0,
                data,
            );
        }
    }

    pub fn bind_pipeline(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device.cmd_bind_pipeline(
                command_buffer,
                self.pipeline_bind_point,
                self.pipeline,
            );
        }
    }

    pub fn bind_descriptor_sets(
        &self,
        command_buffer: vk::CommandBuffer,
        first_set: u32,
        descriptor_sets: &[vk::DescriptorSet],
    ) {
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                self.pipeline_bind_point,
                self.pipeline_layout,
                first_set,
                descriptor_sets,
                &[],
            );
        }
    }
}

pub struct MaterialBuilder {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    pipeline_bind_point: vk::PipelineBindPoint,
    descriptor_set: vk::DescriptorSet,
    device: Arc<ash::Device>,
}