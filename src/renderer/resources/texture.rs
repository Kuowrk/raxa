use crate::renderer::contexts::device_ctx::transfer_ctx::TransferContext;
use crate::renderer::resources::image::Image;
use color_eyre::Result;
use gpu_allocator::vulkan::Allocator;
use std::sync::{Arc, Mutex};
use crate::renderer::resources::RenderResourceHandle;

pub struct ColorTexture {
    pub image: Image,
    pub sampler_handle: RenderResourceHandle,
}

impl ColorTexture {
    pub fn new_from_bytes(
        data: &[u8],
        width: u32,
        height: u32,
        sampler_handle: RenderResourceHandle,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
        transfer_context: &TransferContext,
    ) -> Result<Self> {
        let image = Image::new_color_image(
            data,
            width,
            height,
            memory_allocator,
            device,
            transfer_context,
        )?;

        Ok(Self {
            image,
            sampler_handle,
        })
    }

    pub fn new_from_image(
        image: &image::DynamicImage,
        sampler_handle: RenderResourceHandle,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
        transfer_context: &TransferContext,
    ) -> Result<Self> {
        let data = image.to_rgba8().into_raw();
        let width = image.width();
        let height = image.height();
        Self::new_from_bytes(
            &data,
            width,
            height,
            sampler_handle,
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
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
    ) -> Result<Self> {
        let image = Image::new_storage_image(
            width,
            height,
            memory_allocator,
            device,
        )?;

        Ok(Self {
            image,
        })
    }
}