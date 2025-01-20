use std::sync::Arc;
use ash::vk;
use gpu_descriptor::DescriptorAllocator;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;
use crate::renderer::contexts::resource_ctx::descriptor_set_layout_builder::DescriptorSetLayoutBuilder;
use crate::renderer::resources::buffer::Buffer;
use crate::renderer::resources::material::{GraphicsMaterialFactoryBuilder, MaterialFactory};
use crate::renderer::resources::megabuffer::{Megabuffer};
use crate::renderer::resources::texture::{ColorTexture, StorageTexture};

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
        descriptor_allocator: Arc<DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>>,
    ) -> Result<MaterialFactory> {
        let bindless_descriptor_set_layout = Self::create_bindless_descriptor_set_layout(
            &device
        )?;
        let bindless_pipeline_layout = Self::create_bindless_pipeline_layout(
            bindless_descriptor_set_layout,
            &device,
        )?;
        GraphicsMaterialFactoryBuilder::new(device, descriptor_allocator)
            .with_descriptor_set_layouts(vec![bindless_descriptor_set_layout])
            .with_pipeline_layout(bindless_pipeline_layout)
    }
    
    fn create_bindless_descriptor_set_layout(
        device: &ash::Device,
    ) -> Result<vk::DescriptorSetLayout> {
        DescriptorSetLayoutBuilder::new()
            .add_binding_for_resource_type(0, RenderResourceType::UniformBuffer) // Per-frame
            .add_binding_for_resource_type(1, RenderResourceType::StorageBuffer) // Per-material
            .add_binding_for_resource_type(2, RenderResourceType::StorageBuffer) // Per-object
            .add_binding_for_resource_type(3, RenderResourceType::Sampler)       // Samplers
            .add_binding_for_resource_type(4, RenderResourceType::SampledImage)  // Textures
            .build(
                vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
                device,
            )?
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
