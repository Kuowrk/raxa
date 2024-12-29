use color_eyre::Result;
use std::sync::{Arc, Mutex};
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use ash::vk;
use crate::renderer::core::context::RenderContext;
use crate::renderer::vk::command_buffer_allocator::CommandBufferAllocator;
use crate::renderer::vk::descriptor_set_allocator::DescriptorSetAllocator;
use crate::renderer::vk::descriptor_set_layout_builder::DescriptorSetLayoutBuilder;

const MAX_TEXTURES: u32 = 1024;
const MAX_MATERIALS: u32 = 256;
const MAX_OBJECTS: u32 = 1024;

/// Contains all the resources that the renderer will use like materials, textures, and models
pub struct RenderResources {
    pub memory_allocator: Arc<Mutex<Allocator>>,

    pub descriptor_set_allocator: Arc<DescriptorSetAllocator>,
    pub command_buffer_allocator: Arc<CommandBufferAllocator>,

    pub bindless_descriptor_set_layout: Arc<vk::DescriptorSetLayout>,
}

impl RenderResources {
    pub fn new(ctx: &RenderContext) -> Result<Self> {
        let mut memory_allocator = Allocator::new(&AllocatorCreateDesc {
            instance: ctx.instance.clone(),
            device: (*ctx.device).clone(),
            physical_device: ctx.physical_device,
            debug_settings: gpu_allocator::AllocatorDebugSettings {
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
