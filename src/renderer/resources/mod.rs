/// "Resources" refers to middle-level objects that created by "Core" objects.
/// They are relatively intuitive and managed by the user.

pub mod mesh;
pub mod vertex;
pub mod model;
pub mod buffer;
pub mod image;
pub mod megabuffer;
pub mod allocator;
pub mod texture;

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use ash::vk;
use color_eyre::Result;

use crate::renderer::internals::descriptor_set_layout_builder::DescriptorSetLayoutBuilder;
use crate::renderer::resources::buffer::Buffer;
use crate::renderer::resources::megabuffer::Megabuffer;
use crate::renderer::resources::texture::{ColorTexture, StorageTexture};

pub struct RenderResourceHandle {
    index: u32,
    ty: RenderResourceType,
}

#[derive(PartialEq)]
pub enum RenderResourceType {
    UniformBuffer,
    StorageBuffer,
    StorageImage,
    SampledImage,
    Sampler,
}

impl RenderResourceType {
    const ALL: &'static [Self] = &[
        Self::UniformBuffer,
        Self::StorageBuffer,
        Self::StorageImage,
        Self::SampledImage,
        Self::Sampler,
    ];

    pub fn descriptor_type(&self) -> vk::DescriptorType {
        match self {
            Self::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
            Self::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
            Self::StorageImage => vk::DescriptorType::STORAGE_IMAGE,
            Self::SampledImage => vk::DescriptorType::SAMPLED_IMAGE,
            Self::Sampler => vk::DescriptorType::SAMPLER,
        }
    }

    pub fn descriptor_count(&self) -> u32 {
        match self {
            Self::UniformBuffer => 1024,
            Self::StorageBuffer => 1024,
            Self::StorageImage => 1024,
            Self::SampledImage => 1024,
            Self::Sampler => 1024,
        }
    }

    pub fn descriptor_binding_flags(&self) -> vk::DescriptorBindingFlags {
        vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
    }
}


#[derive(Default)]
pub struct RenderResourceStorage {
    uniform_buffers: Vec<Buffer>,
    storage_buffers: Vec<Megabuffer>,
    storage_images: Vec<StorageTexture>,
    sampled_images: Vec<ColorTexture>,
    samplers: Vec<vk::Sampler>,
}

pub struct RenderResourceAllocator {
    storage: RenderResourceStorage,

    bindless_descriptor_pool: vk::DescriptorPool,
    bindless_descriptor_set_layout: vk::DescriptorSetLayout,
    bindless_descriptor_set: vk::DescriptorSet,
    device: Arc<ash::Device>,
}

impl RenderResourceAllocator {
    pub fn new(
        device: Arc<ash::Device>,
    ) -> Result<Self> {
        let bindless_descriptor_pool = Self::create_bindless_descriptor_pool(&device)?;
        let bindless_descriptor_set_layout = Self::create_bindless_descriptor_set_layout(&device)?;
        let bindless_descriptor_set = Self::create_bindless_descriptor_set(bindless_descriptor_pool, bindless_descriptor_set_layout, &device)?;
        let storage = RenderResourceStorage::default();

        Ok(Self {
            bindless_descriptor_pool,
            bindless_descriptor_set_layout,
            bindless_descriptor_set,
            device,
        })
    }

    fn create_bindless_descriptor_pool(
        device: &ash::Device,
    ) -> Result<vk::DescriptorPool> {
        let pool_sizes = RenderResourceType::ALL
            .iter()
            .map(|ty| {
                vk::DescriptorPoolSize::default()
                    .ty(ty.descriptor_type())
                    .descriptor_count(ty.descriptor_count())
            })
            .collect::<Vec<_>>();

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .flags(
                vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND
                    | vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET
            );

        let pool = unsafe {
            device.create_descriptor_pool(&pool_info, None)?
        };

        Ok(pool)
    }

    fn create_bindless_descriptor_set_layout(
        device: &ash::Device,
    ) -> Result<vk::DescriptorSetLayout> {
        DescriptorSetLayoutBuilder::new()
            .add_binding_for_resource_type(0, RenderResourceType::UniformBuffer) // Per-frame
            .add_binding_for_resource_type(1, RenderResourceType::StorageBuffer) // Per-material
            .add_binding_for_resource_type(2, RenderResourceType::StorageBuffer) // Per-object
            .add_binding_for_resource_type(3, RenderResourceType::SampledImage)  // Textures
            .add_binding_for_resource_type(4, RenderResourceType::Sampler)       // Samplers
            .build(
                vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
                device,
            )
    }

    fn create_bindless_descriptor_set(
        descriptor_pool: vk::DescriptorPool,
        descriptor_set_layout: vk::DescriptorSetLayout,
        device: &ash::Device,
    ) -> Result<vk::DescriptorSet> {
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&[descriptor_set_layout]);

        let descriptor_set = unsafe {
            device.allocate_descriptor_sets(&descriptor_set_allocate_info)?
        }[0];

        Ok(descriptor_set)
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

    /// Create pipeline layout for the bindless renderer.
    fn create_pipeline_layout(
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        device: &ash::Device,
    ) -> Result<vk::PipelineLayout> {

        let push_constant_size = (PushConstantSlots::ALL.len() * size_of::<u32>()) as u32;
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::ALL)
            .offset(0)
            .size(push_constant_size);
        let push_constant_ranges = [push_constant_range];

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&descriptor_set_layouts)
            .push_constant_ranges(&push_constant_ranges);

        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&pipeline_layout_create_info, None)?
        };

        Ok(pipeline_layout)
    }

    /// Create descriptor set layouts for each render resource type.
    fn create_descriptor_set_layouts(
        immutable_samplers: &[vk::Sampler],
        device: &ash::Device,
    ) -> Result<Vec<vk::DescriptorSetLayout>> {
        RenderResourceType::ALL
            .iter()
            .map(|ty| {
                let mut builder = DescriptorSetLayoutBuilder::new()
                    .add_binding(
                        if *ty == RenderResourceType::Texture {
                            // Set texture binding start at the end of the immutable samplers.
                            immutable_samplers.len() as u32
                        } else {
                            0
                        },
                        ty.descriptor_type(),
                        ty.descriptor_count(),
                        vk::ShaderStageFlags::ALL,
                        vk::DescriptorBindingFlags::PARTIALLY_BOUND
                            | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
                            | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
                        None,
                    );

                if *ty == RenderResourceType::Texture {
                    builder = builder.add_binding(
                        0,
                        vk::DescriptorType::SAMPLER,
                        immutable_samplers.len() as u32,
                        vk::ShaderStageFlags::ALL,
                        vk::DescriptorBindingFlags::empty(),
                        Some(immutable_samplers),
                    );
                }

                builder.build(
                    vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
                    device,
                )
            })
            .collect::<Result<Vec<_>>>()
    }

    /// Create immutable samplers for the texture descriptor set layout.
    fn create_immutable_samplers(device: &ash::Device) -> Result<Vec<vk::Sampler>> {
        let sampler_infos = vec![
            vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::NEAREST)
                .min_filter(vk::Filter::NEAREST)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT),
            vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE),
        ];

        let immutable_samplers = sampler_infos
            .iter()
            .map(|info| unsafe { device.create_sampler(info, None) })
            .collect::<ash::prelude::VkResult<Vec<_>>>()?;

        Ok(immutable_samplers)
    }
     */
}
