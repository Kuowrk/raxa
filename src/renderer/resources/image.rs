use std::sync::{Arc, Mutex};
use ash::vk;
use color_eyre::eyre::Result;
use color_eyre::eyre::eyre;
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator},
    MemoryLocation,
};
use crate::renderer::resources::buffer::Buffer;
use crate::renderer::contexts::device_ctx::transfer_ctx::TransferContext;
use crate::renderer::internals::util;

pub struct ImageCreateInfo {
    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub usage: vk::ImageUsageFlags,
    pub aspect: vk::ImageAspectFlags,
    pub name: String,
}

pub struct Image {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub aspect: vk::ImageAspectFlags,

    allocation: Option<Allocation>, // GPU-only memory block
    memory_allocator: Arc<Mutex<Allocator>>,
    device: Arc<ash::Device>,
}

impl Image {
    // NOTE: The `allocation` field of the Image this function returns is GPU-only
    // and is NOT yet populated with any data.
    // This means that unless you are making a depth image or storage image, you will need to call
    // `Image::upload()`
    fn new(
        create_info: &ImageCreateInfo,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
    ) -> Result<Self> {
        let image = {
            let info = vk::ImageCreateInfo::default()
                .format(create_info.format)
                .usage(create_info.usage)
                .extent(create_info.extent)
                .image_type(vk::ImageType::TYPE_2D)
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL);
            unsafe { device.create_image(&info, None)? }
        };
        let reqs = unsafe { device.get_image_memory_requirements(image) };
        let allocation = memory_allocator
            .lock()
            .map_err(|e| eyre!(e.to_string()))?
            .allocate(&AllocationCreateDesc {
                name: &create_info.name,
                requirements: reqs,
                location: MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: AllocationScheme::DedicatedImage(image),
            })?;
        unsafe {
            device.bind_image_memory(image, allocation.memory(), 0)?;
        }
        let view = {
            let info = vk::ImageViewCreateInfo::default()
                .view_type(vk::ImageViewType::TYPE_2D)
                .image(image)
                .format(create_info.format)
                .subresource_range(vk::ImageSubresourceRange {
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                    aspect_mask: create_info.aspect,
                });
            unsafe { device.create_image_view(&info, None)? }
        };

        Ok(Self {
            image,
            view,
            format: create_info.format,
            extent: create_info.extent,
            aspect: create_info.aspect,

            allocation: Some(allocation),
            memory_allocator,
            device,
        })
    }

    /// Create a 32-bit shader-readable image from a byte array
    pub fn new_color_image(
        data: &[u8],
        width: u32,
        height: u32,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
        transfer_context: &TransferContext,
    ) -> Result<Self> {
        let image = {
            let create_info = ImageCreateInfo {
                format: vk::Format::R8G8B8A8_SRGB,
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                aspect: vk::ImageAspectFlags::COLOR,
                name: "Color Image".into(),
            };
            let mut image = Self::new(&create_info, memory_allocator, device)?;
            image.upload(data, transfer_context)?;
            image
        };

        Ok(image)
    }

    /// Create a special type of image used for depth buffer
    pub fn new_depth_image(
        width: u32,
        height: u32,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
    ) -> Result<Self> {
        let create_info = ImageCreateInfo {
            format: vk::Format::D32_SFLOAT,
            extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            aspect: vk::ImageAspectFlags::DEPTH,
            name: "Depth Image".into(),
        };
        Self::new(&create_info, memory_allocator, device)
    }

    /// Create a special type of image likely used by compute shaders
    pub fn new_storage_image(
        width: u32,
        height: u32,
        memory_allocator: Arc<Mutex<Allocator>>,
        device: Arc<ash::Device>,
    ) -> Result<Self> {
        let image = {
            let extent = vk::Extent3D {
                width,
                height,
                depth: 1,
            };
            let usage = vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::STORAGE;
            let create_info = ImageCreateInfo {
                format: vk::Format::R16G16B16A16_SFLOAT,
                extent,
                usage,
                aspect: vk::ImageAspectFlags::COLOR,
                name: "Storage Image".into(),
            };
            Image::new(&create_info, memory_allocator, device)?
        };

        Ok(image)
    }

    pub fn transition_layout(
        &mut self,
        cmd: vk::CommandBuffer,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        transition_image_layout(
            cmd,
            self.image,
            self.aspect,
            old_layout,
            new_layout,
            self.device.as_ref(),
        );
    }

    pub fn copy_to_vkimage(
        &self,
        cmd: vk::CommandBuffer,
        dst_image: vk::Image,
        dst_image_extent: vk::Extent2D,
    ) {
        copy_image_to_image(
            cmd,
            self.image,
            dst_image,
            vk::Extent2D {
                width: self.extent.width,
                height: self.extent.height,
            },
            dst_image_extent,
            self.device.as_ref(),
        );
    }

    pub fn copy_to_image(
        &self,
        cmd: vk::CommandBuffer,
        dst_image: &Image,
    ) {
        self.copy_to_vkimage(
            cmd,
            dst_image.image,
            vk::Extent2D {
                width: dst_image.extent.width,
                height: dst_image.extent.height,
            },
        );
    }

    fn upload(
        &mut self,
        data: &[u8],
        transfer_context: &TransferContext,
    ) -> Result<()> {
        let mut staging_buffer = Buffer::new(
            data.len() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            "Image staging buffer",
            MemoryLocation::CpuToGpu,
            self.memory_allocator.clone(),
            self.device.clone(),
        )?;
        staging_buffer.write(data, 0)?;
        transfer_context.immediate_submit(
            |cmd: vk::CommandBuffer, device: &ash::Device| {
                let range = vk::ImageSubresourceRange {
                    aspect_mask: self.aspect,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                };

                let img_barrier_to_transfer = vk::ImageMemoryBarrier {
                    old_layout: vk::ImageLayout::UNDEFINED,
                    new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    image: self.image,
                    subresource_range: range,
                    src_access_mask: vk::AccessFlags::empty(),
                    dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    ..Default::default()
                };

                unsafe {
                    // Create a pipeline barrier that blocks from
                    // VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT to VK_PIPELINE_STAGE_TRANSFER_BIT
                    // Read more: https://gpuopen.com/learn/vulkan-barriers-explained/
                    device.cmd_pipeline_barrier(
                        cmd,
                        vk::PipelineStageFlags::TOP_OF_PIPE,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[img_barrier_to_transfer],
                    );
                }

                let copy_region = vk::BufferImageCopy {
                    buffer_offset: 0,
                    buffer_row_length: 0,
                    buffer_image_height: 0,
                    image_subresource: vk::ImageSubresourceLayers {
                        aspect_mask: self.aspect,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    image_extent: self.extent,
                    ..Default::default()
                };

                unsafe {
                    // Copy staging buffer into image
                    device.cmd_copy_buffer_to_image(
                        cmd,
                        staging_buffer.buffer,
                        self.image,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[copy_region],
                    );
                }

                let mut img_barrier_to_readable = img_barrier_to_transfer;
                img_barrier_to_readable.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
                img_barrier_to_readable.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                img_barrier_to_readable.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                img_barrier_to_readable.dst_access_mask = vk::AccessFlags::SHADER_READ;

                // Barrier the image into the shader-readable layout
                unsafe {
                    device.cmd_pipeline_barrier(
                        cmd,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::FRAGMENT_SHADER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[img_barrier_to_readable],
                    )
                }

                Ok(())
            },
        )?;

        Ok(())
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.view, None);
            self.memory_allocator
                .lock()
                .unwrap()
                .free(self.allocation.take().unwrap())
                .unwrap();
            self.device.destroy_image(self.image, None);
        }
    }
}

fn copy_image_to_image(
    cmd: vk::CommandBuffer,
    src: vk::Image,
    dst: vk::Image,
    src_size: vk::Extent2D,
    dst_size: vk::Extent2D,
    device: &ash::Device,
) {
    let blit_region = vk::ImageBlit2 {
        src_offsets: [
            vk::Offset3D { x: 0, y: 0, z: 0 },
            vk::Offset3D {
                x: src_size.width as i32,
                y: src_size.height as i32,
                z: 1,
            },
        ],
        dst_offsets: [
            vk::Offset3D { x: 0, y: 0, z: 0 },
            vk::Offset3D {
                x: dst_size.width as i32,
                y: dst_size.height as i32,
                z: 1,
            },
        ],
        src_subresource: vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_array_layer: 0,
            layer_count: 1,
            mip_level: 0,
        },
        dst_subresource: vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_array_layer: 0,
            layer_count: 1,
            mip_level: 0,
        },
        ..Default::default()
    };

    let blit_info = vk::BlitImageInfo2 {
        dst_image: dst,
        dst_image_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        src_image: src,
        src_image_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        filter: vk::Filter::LINEAR,
        region_count: 1,
        p_regions: &blit_region,
        ..Default::default()
    };

    unsafe {
        device.cmd_blit_image2(cmd, &blit_info);
    }
}

fn transition_image_layout(
    cmd: vk::CommandBuffer,
    image: vk::Image,
    image_aspect: vk::ImageAspectFlags,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    device: &ash::Device,
) {
    if old_layout == new_layout {
        return;
    }

    let image_barrier = vk::ImageMemoryBarrier2 {
        src_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
        src_access_mask: vk::AccessFlags2::MEMORY_WRITE,
        dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
        dst_access_mask: vk::AccessFlags2::MEMORY_WRITE
            | vk::AccessFlags2::MEMORY_READ,
        old_layout,
        new_layout,
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: image_aspect,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        },
        image,
        ..Default::default()
    };

    let dep_info = vk::DependencyInfo {
        image_memory_barrier_count: 1,
        p_image_memory_barriers: &image_barrier,
        ..Default::default()
    };

    unsafe {
        device.cmd_pipeline_barrier2(cmd, &dep_info);
    }
}
