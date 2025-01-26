use crate::renderer::contexts::device_ctx::transfer_ctx::TransferContext;
use crate::renderer::resources::image::Image;
use color_eyre::Result;
use std::sync::{Arc, Mutex};

pub struct ColorTexture {
    pub image: Image,
}

impl ColorTexture {
    pub fn new_from_bytes(
        width: u32,
        height: u32,
        data: Option<&[u8]>,
        use_dedicated_memory: bool,
        
        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
        transfer_context: &TransferContext,
    ) -> Result<Self> {
        let image = Image::new_color_image(
            width,
            height,
            data,
            use_dedicated_memory,
            
            memory_allocator,
            device,
            transfer_context,
        )?;

        Ok(Self {
            image,
        })
    }

    pub fn new_from_image(
        image: &image::DynamicImage,
        use_dedicated_memory: bool,
        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
        transfer_context: &TransferContext,
    ) -> Result<Self> {
        let data = image.to_rgba8().into_raw();
        let width = image.width();
        let height = image.height();
        Self::new_from_bytes(
            width,
            height,
            Some(&data),
            use_dedicated_memory,
            memory_allocator,
            device,
            transfer_context,
        )
    }
}

pub struct StorageTexture {
    pub image: Image,
}

impl StorageTexture {
    pub fn new(
        width: u32,
        height: u32,
        use_dedicated_memory: bool,
        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
    ) -> Result<Self> {
        let image = Image::new_storage_image(
            width,
            height,
            use_dedicated_memory,
            memory_allocator,
            device,
        )?;

        Ok(Self {
            image,
        })
    }
}
