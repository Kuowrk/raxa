use std::sync::{Arc, Mutex};
use ash::vk;
use color_eyre::Result;
use gpu_descriptor::DescriptorAllocator;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;
use crate::renderer::contexts::resource_ctx::descriptor_set_layout_builder::DescriptorSetLayoutBuilder;
use crate::renderer::contexts::resource_ctx::resource_type::RenderResourceType;
use crate::renderer::resources::buffer::Buffer;
use crate::renderer::resources::material::{GraphicsMaterialFactoryBuilder, MaterialFactory};
use crate::renderer::resources::megabuffer::{Megabuffer};
use crate::renderer::resources::texture::{ColorTexture, StorageTexture};
use crate::renderer::shader_data::PerDrawData;

const VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 256; // 256 MB
const INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 64; // 64 MB
const VERTEX_BUFFER_ALIGNMENT: u64 = 16;
const INDEX_BUFFER_ALIGNMENT: u64 = 4;
const STORAGE_BUFFER_ALIGNMENT: u64 = 16;
const UNIFORM_BUFFER_ALIGNMENT: u64 = 256;

pub struct RenderResourceStorage {
    uniform_buffers: Vec<Buffer>,
    storage_buffers: Vec<Megabuffer>,
    storage_images: Vec<StorageTexture>,
    sampled_images: Vec<ColorTexture>,
    samplers: Vec<vk::Sampler>,

    vertex_megabuffer: Megabuffer,
    index_megabuffer: Megabuffer,

    bindless_material_factory: MaterialFactory,
}

impl RenderResourceStorage {
    pub fn new(
        dev_ctx: &RenderDeviceContext,
    ) -> color_eyre::Result<Self> {
        let device = &dev_ctx.device;

        let vertex_megabuffer = device.create_megabuffer(
            VERTEX_BUFFER_SIZE,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            VERTEX_BUFFER_ALIGNMENT,
        )?;

        let index_megabuffer = device.create_megabuffer(
            INDEX_BUFFER_SIZE,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            INDEX_BUFFER_ALIGNMENT,
        )?;

        let bindless_material_factory = Self::create_bindless_material_factory(
            device.logical.clone(),
            device.descriptor_allocator.clone(),
        )?;

        Ok(Self {
            uniform_buffers: Vec::new(),
            storage_buffers: Vec::new(),
            storage_images: Vec::new(),
            samplers: Vec::new(),
            sampled_images: Vec::new(),

            vertex_megabuffer,
            index_megabuffer,

            bindless_material_factory,
        })
    }

    fn create_bindless_material_factory(
        device: Arc<ash::Device>,
        descriptor_allocator: Arc<Mutex<DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>>>,
    ) -> Result<MaterialFactory> {
        let bindless_descriptor_set_layout = Self::create_bindless_descriptor_set_layout(
            &device
        )?;
        let bindless_pipeline_layout = Self::create_bindless_pipeline_layout(
            bindless_descriptor_set_layout,
            &device,
        )?;
        let default_shader =
        GraphicsMaterialFactoryBuilder::new(device, descriptor_allocator)
            .with_shader(default_shader)
            .with_pipeline_layout(bindless_pipeline_layout)
            .with_descriptor_set_layout(bindless_descriptor_set_layout)
            .with_color_attachment_format(draw_image)
            .with_depth_attachment_format(depth_image)
            .build()?;
    }
    
    fn create_bindless_descriptor_set_layout(
        device: &ash::Device,
    ) -> Result<vk::DescriptorSetLayout> {
        DescriptorSetLayoutBuilder::new()
            .add_binding( // Per-frame
                0,
                RenderResourceType::UniformBuffer.descriptor_type(),
                RenderResourceType::UniformBuffer.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                RenderResourceType::UniformBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding( // Per-material
                1,
                RenderResourceType::StorageBuffer.descriptor_type(),
                RenderResourceType::StorageBuffer.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                RenderResourceType::StorageBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding( // Per-material
                2,
                RenderResourceType::StorageBuffer.descriptor_type(),
                RenderResourceType::StorageBuffer.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                RenderResourceType::StorageBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding( // Samplers
                3,
                RenderResourceType::Sampler.descriptor_type(),
                RenderResourceType::Sampler.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                RenderResourceType::Sampler.descriptor_binding_flags(),
                None,
            )
            .add_binding( // Textures
                4,
                RenderResourceType::SampledImage.descriptor_type(),
                RenderResourceType::SampledImage.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                RenderResourceType::SampledImage.descriptor_binding_flags(),
                None,
            )
            .build(
                vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
                device,
            )
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
}
