use crate::renderer::contexts::device_ctx::RenderDeviceContext;
use crate::renderer::contexts::resource_ctx::descriptor_set_layout_builder::DescriptorSetLayoutBuilder;
use crate::renderer::shader_data::PerDrawData;
use ash::vk;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use gpu_descriptor::{DescriptorAllocator, DescriptorSetLayoutCreateFlags, DescriptorTotalCount};

const MAX_SAMPLED_IMAGES: u32 = 1024;
const MAX_SAMPLERS: u32 = 16;

pub struct RenderResourceHandle {
    index: u32,
    ty: RenderResourceType,
}

#[derive(PartialEq)]
pub enum RenderResourceType {
    UniformBuffer,
    StorageBuffer,
    StorageImage,
    Sampler,
    SampledImage,
}

impl RenderResourceType {
    const ALL: &'static [Self] = &[
        Self::UniformBuffer,
        Self::StorageBuffer,
        Self::StorageImage,
        Self::Sampler,
        Self::SampledImage,
    ];

    pub fn descriptor_type(&self) -> vk::DescriptorType {
        match self {
            Self::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
            Self::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
            Self::StorageImage => vk::DescriptorType::STORAGE_IMAGE,
            Self::Sampler => vk::DescriptorType::SAMPLER,
            Self::SampledImage => vk::DescriptorType::SAMPLED_IMAGE,
        }
    }

    pub fn descriptor_count(&self) -> u32 {
        match self {
            Self::UniformBuffer => 1,
            Self::StorageBuffer => 1,
            Self::StorageImage => 1,
            Self::Sampler => MAX_SAMPLERS,
            Self::SampledImage => MAX_SAMPLED_IMAGES,
        }
    }

    pub fn descriptor_binding_flags(&self) -> vk::DescriptorBindingFlags {
        match self {
            Self::UniformBuffer => vk::DescriptorBindingFlags::PARTIALLY_BOUND
                | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            Self::StorageBuffer => vk::DescriptorBindingFlags::PARTIALLY_BOUND
                | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            Self::StorageImage => vk::DescriptorBindingFlags::PARTIALLY_BOUND
                | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            Self::Sampler => vk::DescriptorBindingFlags::PARTIALLY_BOUND
                | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            Self::SampledImage => vk::DescriptorBindingFlags::PARTIALLY_BOUND
                | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
                | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT,
        }
    }

    pub fn descriptor_pool_count(&self) -> u32 {
        match self {
            Self::UniformBuffer => 16,
            Self::StorageBuffer => 16,
            Self::StorageImage => 16,
            Self::Sampler => 16,
            Self::SampledImage => 16,
        }
    }
}

pub struct RenderResourceAllocator {
    bindless_descriptor_set_layout: vk::DescriptorSetLayout,
    bindless_descriptor_set: gpu_descriptor::DescriptorSet<vk::DescriptorSet>,
    bindless_pipeline_layout: vk::PipelineLayout,
    descriptor_allocator: DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>,
}

impl RenderResourceAllocator {
    pub fn new(
        dev_ctx: &RenderDeviceContext,
    ) -> Result<Self> {
        let device = &dev_ctx.device;

        let mut descriptor_allocator: DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>
            = DescriptorAllocator::new(1024);
        let bindless_descriptor_set_layout = DescriptorSetLayoutBuilder::new()
            .add_binding_for_resource_type(0, RenderResourceType::UniformBuffer) // Per-frame
            .add_binding_for_resource_type(1, RenderResourceType::StorageBuffer) // Per-material
            .add_binding_for_resource_type(2, RenderResourceType::StorageBuffer) // Per-object
            .add_binding_for_resource_type(3, RenderResourceType::Sampler)       // Samplers
            .add_binding_for_resource_type(4, RenderResourceType::SampledImage)  // Textures
            .build(
                vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
                &device.logical,
            )?;

        let bindless_descriptor_set = unsafe {
            descriptor_allocator
                .allocate(
                    device,
                    &bindless_descriptor_set_layout,
                    DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND,
                    &DescriptorTotalCount {
                        sampler: RenderResourceType::Sampler.descriptor_pool_count(),
                        combined_image_sampler: 0,
                        sampled_image: RenderResourceType::SampledImage.descriptor_pool_count(),
                        storage_image: RenderResourceType::StorageImage.descriptor_pool_count(),
                        uniform_texel_buffer: 0,
                        storage_texel_buffer: 0,
                        uniform_buffer: RenderResourceType::UniformBuffer.descriptor_pool_count(),
                        storage_buffer: RenderResourceType::StorageBuffer.descriptor_pool_count(),
                        uniform_buffer_dynamic: 0,
                        storage_buffer_dynamic: 0,
                        input_attachment: 0,
                        acceleration_structure: 0,
                        inline_uniform_block_bytes: 0,
                        inline_uniform_block_bindings: 0,
                    },
                    1,
                )?
                .drain(..)
                .next()
                .ok_or_eyre("Failed to allocate bindless descriptor set")?
        };

        let bindless_pipeline_layout = Self::create_bindless_pipeline_layout(
            bindless_descriptor_set_layout,
            &device.logical,
        )?;

        Ok(Self {
            bindless_descriptor_set_layout,
            bindless_descriptor_set,
            bindless_pipeline_layout,
            descriptor_allocator,
        })
    }

    fn create_bindless_pipeline_layout(
        bindless_descriptor_set_layout: vk::DescriptorSetLayout,
        device: &ash::Device,
    ) -> Result<vk::PipelineLayout> {

        let push_constant_size = size_of::<PerDrawData>() as u32;
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::ALL)
            .offset(0)
            .size(push_constant_size);
        let push_constant_ranges = [push_constant_range];

        let set_layouts = [bindless_descriptor_set_layout];
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_constant_ranges);

        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&pipeline_layout_create_info, None)?
        };

        Ok(pipeline_layout)
    }

    /*
    pub fn allocate_buffer_handle(
        &self,
        buffer: vk::Buffer,
    ) -> Result<RenderResourceHandle> {
        let handle = self.fetch_available_handle(RenderResourceType::Buffer)?;

        let buffer_info = [
            vk::DescriptorBufferInfo::default()
                .buffer(buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE)
        ];

        let write = [
            vk::WriteDescriptorSet::default()
                .dst_set(self.descriptor_sets[RenderResourceType::Buffer.descriptor_set_index()])
                .dst_binding(0)
                .descriptor_count(1)
                .dst_array_element(handle.0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&buffer_info)
        ];

        unsafe {
            self.device.update_descriptor_sets(&write, &[]);
        }

        Ok(handle)
    }

    pub fn retire_handle(&self, handle: RenderResourceHandle) -> Result<()> {
        self.available_recycled_descriptors
            .lock()?
            .push_back(handle);

        Ok(())
    }

    pub fn fetch_available_handle(&self, ty: RenderResourceType) -> Result<RenderResourceHandle> {
        self.available_recycled_descriptors
            .lock()?
            .pop_front()
            .map_or_else(
                || RenderResourceHandle::new(ty),
                |recycled_handle| {
                    recycled_handle.bump_version_and_update_type(ty);
                    recycled_handle
                },
            )
    }

     */
}
