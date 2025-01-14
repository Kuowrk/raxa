use ash::vk;
use crate::renderer::contexts::device::RenderDevice;
use crate::renderer::resources::buffer::Buffer;
use crate::renderer::resources::megabuffer::MegabufferHandle;
use crate::renderer::resources::texture::{ColorTexture, StorageTexture};

const VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 256; // 256 MB
const INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 64; // 64 MB
const VERTEX_BUFFER_ALIGNMENT: u64 = 16;
const INDEX_BUFFER_ALIGNMENT: u64 = 4;
const STORAGE_BUFFER_ALIGNMENT: u64 = 16;
const UNIFORM_BUFFER_ALIGNMENT: u64 = 256;

pub struct RenderResourceStorage {
    uniform_buffers: Vec<Buffer>,
    storage_buffers: Vec<MegabufferHandle>,
    storage_images: Vec<StorageTexture>,
    sampled_images: Vec<ColorTexture>,
    samplers: Vec<vk::Sampler>,

    vertex_megabuffer: MegabufferHandle,
    index_megabuffer: MegabufferHandle,
}

impl RenderResourceStorage {
    pub fn new(
        dev: &RenderDevice,
    ) -> color_eyre::Result<Self> {
        let vertex_megabuffer = dev.create_megabuffer(
            crate::renderer::resources::VERTEX_BUFFER_SIZE,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            crate::renderer::resources::VERTEX_BUFFER_ALIGNMENT,
        )?;

        let index_megabuffer = dev.create_megabuffer(
            crate::renderer::resources::INDEX_BUFFER_SIZE,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            crate::renderer::resources::INDEX_BUFFER_ALIGNMENT,
        )?;

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
}
