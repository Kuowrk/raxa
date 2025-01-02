use color_eyre::Result;
use ash::vk;
use crate::renderer::core::device::RenderDevice;
use crate::renderer::core::instance::RenderInstance;
use crate::renderer::vk::buffer::AllocatedBuffer;
use crate::renderer::vk::descriptor_set_layout_builder::DescriptorSetLayoutBuilder;

const MAX_TEXTURES: u32 = 1024;
const MAX_MATERIALS: u32 = 256;
const MAX_OBJECTS: u32 = 1024;

/// Contains all the resources that the renderer will use like materials, textures, and models
pub struct RenderResources {
    pub bindless_descriptor_set_layout: vk::DescriptorSetLayout,
    //pub vertex_buffer: AllocatedBuffer,
    //pub index_buffer: AllocatedBuffer,
}

impl RenderResources {
    pub fn new(dev: &RenderDevice) -> Result<Self> {
        let bindless_descriptor_set_layout = DescriptorSetLayoutBuilder::new()
            // Per-frame uniform buffer
            .add_binding(
                0,
                vk::DescriptorType::UNIFORM_BUFFER,
                1,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                vk::DescriptorBindingFlags::empty(),
            )
            // Per-material storage buffer
            .add_binding(
                1,
                vk::DescriptorType::STORAGE_BUFFER,
                MAX_MATERIALS,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT,
            )
            // Per-object storage buffer
            .add_binding(
                2,
                vk::DescriptorType::STORAGE_BUFFER,
                MAX_OBJECTS,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT,
            )
            // Textures
            .add_binding(
                3,
                vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                MAX_TEXTURES,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT,
            )
            .build(&dev.logical)?;

        Ok(Self {
            bindless_descriptor_set_layout,
        })
    }

}
