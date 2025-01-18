use std::sync::Arc;
use ash::vk;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;
use crate::renderer::contexts::resource_ctx::resource_allocator::RenderResourceAllocator;
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
        allocator: &RenderResourceAllocator,
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
            device.
            device.logical.clone(),
        );

        Ok(Self {
            uniform_buffers: Vec::new(),
            storage_buffers: Vec::new(),
            storage_images: Vec::new(),
            samplers: Vec::new(),
            sampled_images: Vec::new(),

            vertex_megabuffer,
            index_megabuffer,
        })
    }

    fn create_bindless_material_factory(device: Arc<ash::Device>) -> MaterialFactory {
        GraphicsMaterialFactoryBuilder::new(device)
            .
    }
}
