use color_eyre::Result;
use std::sync::Arc;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use vulkano::buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo};
use vulkano::buffer::{Buffer, BufferUsage};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::layout::{DescriptorBindingFlags, DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags, DescriptorSetLayoutCreateInfo, DescriptorType};
use vulkano::memory::allocator::{MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::shader::ShaderStages;
use crate::renderer::core::context::RenderContext;
use crate::renderer::vk::descriptor_set_layout_builder::DescriptorSetLayoutBuilder;

const MAX_TEXTURES: u32 = 1024;
const MAX_MATERIALS: u32 = 256;
const MAX_OBJECTS: u32 = 1024;

/// Contains all the resources that the renderer will use like materials, textures, and models
pub struct RenderResources {
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub vertex_buffer_allocator: Arc<SubbufferAllocator>,
    pub index_buffer_allocator: Arc<SubbufferAllocator>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,

    pub bindless_descriptor_set_layout: Arc<DescriptorSetLayout>,
}

impl RenderResources {
    pub fn new(ctx: &RenderContext) -> Result<Self> {
        let mut memory_allocator = Allocator::new(&AllocatorCreateDesc {
            instance: ctx.instance.clone(),
            device: ctx.device.clone(),
            physical_device: ctx.physical_device,
            debug_settings: AllocatorDebugSettings {
                log_memory_information: true,
                log_leaks_on_shutdown: true,
                store_stack_traces: false,
                log_allocations: true,
                log_frees: true,
                log_stack_traces: false,
            },
            buffer_device_address: true,
            allocation_sizes: Default::default(),
        })?;

        let memory_allocator = Arc::new(
            StandardMemoryAllocator::new_default(ctx.device.clone())
        );

        let vertex_buffer_allocator = Arc::new(
            SubbufferAllocator::new(
                memory_allocator.clone(),
                SubbufferAllocatorCreateInfo {
                    buffer_usage: BufferUsage::VERTEX_BUFFER,
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                }
            )
        );

        let index_buffer_allocator = Arc::new(
            SubbufferAllocator::new(
                memory_allocator.clone(),
                SubbufferAllocatorCreateInfo {
                    buffer_usage: BufferUsage::INDEX_BUFFER,
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                }
            )
        );

        let descriptor_set_allocator = Arc::new(
            StandardDescriptorSetAllocator::new(
                ctx.device.clone(),
                Default::default(),
            )
        );

        let command_buffer_allocator = Arc::new(
            StandardCommandBufferAllocator::new(
                ctx.device.clone(),
                Default::default(),
            )
        );

        let bindless_descriptor_set_layout = DescriptorSetLayoutBuilder::new()
            // Per-frame uniform buffer
            .add_binding(
                0,
                DescriptorType::UniformBuffer,
                1,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                DescriptorBindingFlags::empty(),
            )
            // Per-material storage buffer
            .add_binding(
                1,
                DescriptorType::StorageBuffer,
                MAX_MATERIALS,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT | DescriptorBindingFlags::Desc,
            )
            // Per-object storage buffer
            .add_binding(
                2,
                DescriptorType::StorageBuffer,
                MAX_OBJECTS,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT,
            )
            // Textures
            .add_binding(
                3,
                DescriptorType::CombinedImageSampler,
                MAX_TEXTURES,
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT,
            )
            .build(ctx)?;

        Ok(Self {
            memory_allocator,
            vertex_buffer_allocator,
            index_buffer_allocator,
            descriptor_set_allocator,
            command_buffer_allocator,
            bindless_descriptor_set_layout,
        })
    }

}
