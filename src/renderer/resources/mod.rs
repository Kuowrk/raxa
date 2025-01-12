/// "Resources" refers to middle-level objects that created by "Core" objects.
/// They are relatively intuitive and managed by the user.

pub mod mesh;
pub mod vertex;
pub mod model;
pub mod buffer;
pub mod image;
pub mod megabuffer;
pub mod texture;

use crate::renderer::core::device::RenderDevice;
use crate::renderer::internals::descriptor_set_layout_builder::DescriptorSetLayoutBuilder;
use crate::renderer::resources::buffer::Buffer;
use crate::renderer::resources::megabuffer::MegabufferHandle;
use crate::renderer::resources::texture::{ColorTexture, StorageTexture};
use ash::vk;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use gpu_descriptor::{DescriptorAllocator, DescriptorSetLayoutCreateFlags, DescriptorTotalCount};

const VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 256; // 256 MB
const INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 64; // 64 MB
const VERTEX_BUFFER_ALIGNMENT: u64 = 16;
const INDEX_BUFFER_ALIGNMENT: u64 = 4;
const STORAGE_BUFFER_ALIGNMENT: u64 = 16;
const UNIFORM_BUFFER_ALIGNMENT: u64 = 256;
const MAX_TEXTURES: u32 = 1024;
const MAX_MATERIALS: u32 = 256;
const MAX_OBJECTS: u32 = 1024;


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


pub struct RenderResourceStorage {
    uniform_buffers: Vec<Buffer>,
    storage_buffers: Vec<MegabufferHandle>,
    storage_images: Vec<StorageTexture>,
    sampled_images: Vec<ColorTexture>,
    samplers: Vec<vk::Sampler>,

    vertex_megabuffer: MegabufferHandle,
    index_megabuffer: MegabufferHandle,
}

pub struct RenderResourceAllocator {
    storage: RenderResourceStorage,

    bindless_descriptor_set_layout: vk::DescriptorSetLayout,
    bindless_descriptor_set: gpu_descriptor::DescriptorSet<vk::DescriptorSet>,

    descriptor_allocator: DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>,
}

impl RenderResourceAllocator {
    pub fn new(
        dev: &RenderDevice,
    ) -> Result<Self> {
        let mut descriptor_allocator: DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>
            = DescriptorAllocator::new(1024);
        let bindless_descriptor_set_layout = DescriptorSetLayoutBuilder::new()
            .add_binding_for_resource_type(0, RenderResourceType::UniformBuffer) // Per-frame
            .add_binding_for_resource_type(1, RenderResourceType::StorageBuffer) // Per-material
            .add_binding_for_resource_type(2, RenderResourceType::StorageBuffer) // Per-object
            .add_binding_for_resource_type(3, RenderResourceType::SampledImage)  // Textures
            .add_binding_for_resource_type(4, RenderResourceType::Sampler)       // Samplers
            .build(
                vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
                &dev.logical,
            )?;

        let bindless_descriptor_set = unsafe {
            descriptor_allocator
                .allocate(
                    dev,
                    &bindless_descriptor_set_layout,
                    DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND,
                    &DescriptorTotalCount {
                        sampler: RenderResourceType::Sampler.descriptor_count(),
                        combined_image_sampler: 0,
                        sampled_image: RenderResourceType::SampledImage.descriptor_count(),
                        storage_image: RenderResourceType::StorageImage.descriptor_count(),
                        uniform_texel_buffer: 0,
                        storage_texel_buffer: 0,
                        uniform_buffer: RenderResourceType::UniformBuffer.descriptor_count(),
                        storage_buffer: RenderResourceType::StorageBuffer.descriptor_count(),
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

        let vertex_megabuffer = dev.create_megabuffer(
            VERTEX_BUFFER_SIZE,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            VERTEX_BUFFER_ALIGNMENT,
        )?;

        let index_megabuffer = dev.create_megabuffer(
            INDEX_BUFFER_SIZE,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            INDEX_BUFFER_ALIGNMENT,
        )?;

        let storage = RenderResourceStorage {
            uniform_buffers: Vec::new(),
            storage_buffers: Vec::new(),
            storage_images: Vec::new(),
            sampled_images: Vec::new(),
            samplers: Vec::new(),
            vertex_megabuffer,
            index_megabuffer,
        };

        Ok(Self {
            storage,
            bindless_descriptor_set_layout,
            bindless_descriptor_set,
            descriptor_allocator,
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
        let set_layouts = [descriptor_set_layout];
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts);

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

     */
}