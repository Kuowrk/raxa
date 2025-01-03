use color_eyre::Result;
use ash::vk;
use crate::renderer::core::device::RenderDevice;
use crate::renderer::internals::buffer_allocator::BufferAllocator;
use crate::renderer::internals::descriptor_set_layout_builder::DescriptorSetLayoutBuilder;

const VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 256; // 256 MB
const INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 64; // 64 MB
const VERTEX_BUFFER_ALIGNMENT: u64 = 16;
const INDEX_BUFFER_ALIGNMENT: u64 = 4;
const STORAGE_BUFFER_ALIGNMENT: u64 = 16;
const UNIFORM_BUFFER_ALIGNMENT: u64 = 256;
const MAX_TEXTURES: u32 = 1024;
const MAX_MATERIALS: u32 = 256;
const MAX_OBJECTS: u32 = 1024;

/// Contains all the resources that the renderer will use like materials, textures, and models
pub struct RenderResources<'a> {
    pub bindless_descriptor_set_layout: vk::DescriptorSetLayout,
    pub vertex_buffer_allocator: BufferAllocator<'a>,
    pub index_buffer_allocator: BufferAllocator<'a>,
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

        let vertex_buffer_allocator = dev.create_buffer_allocator(
            VERTEX_BUFFER_SIZE,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            gpu_allocator::MemoryLocation::GpuOnly,
            VERTEX_BUFFER_ALIGNMENT,
        )?;

        let index_buffer_allocator = dev.create_buffer_allocator(
            INDEX_BUFFER_SIZE,
            vk::BufferUsageFlags::INDEX_BUFFER,
            gpu_allocator::MemoryLocation::GpuOnly,
            INDEX_BUFFER_ALIGNMENT,
        )?;

        vertex_buffer_allocator.update_buffer()?;
        index_buffer_allocator.update_buffer()?;

        Ok(Self {
            bindless_descriptor_set_layout,
            vertex_buffer_allocator,
            index_buffer_allocator,
        })
    }

}
