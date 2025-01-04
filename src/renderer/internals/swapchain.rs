use ash::prelude::VkResult;
use ash::vk;
use color_eyre::Result;
use winit::window::Window;
use crate::renderer::core::device::RenderDevice;
use crate::renderer::core::instance::RenderInstance;

pub struct Swapchain {
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain_present_mode: vk::PresentModeKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_count: u32,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub swapchain_image_extent: vk::Extent2D,
    pub swapchain_image_format: vk::Format,
    pub swapchain_image_color_space: vk::ColorSpaceKHR,
    pub swapchain_image_usage: vk::ImageUsageFlags,
    pub swapchain_image_sharing_mode: vk::SharingMode,
}

impl Swapchain {
    pub fn new(
        surface: &vk::SurfaceKHR,
        surface_loader: &ash::khr::surface::Instance,
        surface_format: &vk::SurfaceFormatKHR,
        surface_present_mode: &vk::PresentModeKHR,
        window: &Window,
        ins: &RenderInstance,
        dev: &RenderDevice,
    ) -> Result<Self> {
        let surface_capabilities = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(dev.physical, *surface)?
        };

        let image_extent = {
            if surface_capabilities.current_extent.width != u32::MAX {
                surface_capabilities.current_extent
            } else {
                let window_size = window.inner_size();
                vk::Extent2D {
                    width: window_size.width.clamp(
                        surface_capabilities.min_image_extent.width,
                        surface_capabilities.max_image_extent.width,
                    ),
                    height: window_size.height.clamp(
                        surface_capabilities.min_image_extent.height,
                        surface_capabilities.max_image_extent.height,
                    ),
                }
            }
        };

        let min_image_count = {
            let min = surface_capabilities.min_image_count;
            let max = surface_capabilities.max_image_count;
            // Recommended to request at least one more image than the minimum
            // to prevent having to wait on driver to complete internal operations
            // before another image can be acquired
            if max > 0 && min + 1 > max {
                max
            } else {
                min + 1
            }
        };
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let image_usage = vk::ImageUsageFlags::COLOR_ATTACHMENT
            | vk::ImageUsageFlags::TRANSFER_DST;
        let image_sharing_mode = vk::SharingMode::EXCLUSIVE;

        let swapchain_loader = ash::khr::swapchain::Device::new(
            &ins.instance,
            &dev.logical,
        );
        let swapchain_info = vk::SwapchainCreateInfoKHR::default()
            .surface(*surface)
            .min_image_count(min_image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(image_extent)
            .image_usage(image_usage)
            .image_sharing_mode(image_sharing_mode)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(*surface_present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain = unsafe {
            swapchain_loader.create_swapchain(&swapchain_info, None)?
        };

        let (
            swapchain_images,
            swapchain_image_views,
        ) = Self::create_swapchain_images(
            &swapchain,
            &swapchain_loader,
            &surface_format.format,
            dev,
        )?;

        let swapchain_image_count = swapchain_images.len() as u32;

        Ok(Self {
            swapchain,
            swapchain_loader,
            swapchain_present_mode: *surface_present_mode,
            swapchain_images,
            swapchain_image_count,
            swapchain_image_views,
            swapchain_image_extent: image_extent,
            swapchain_image_format: surface_format.format,
            swapchain_image_color_space: surface_format.color_space,
            swapchain_image_usage: image_usage,
            swapchain_image_sharing_mode: image_sharing_mode,
        })
    }

    fn create_swapchain_images(
        swapchain: &vk::SwapchainKHR,
        swapchain_loader: &ash::khr::swapchain::Device,
        swapchain_image_format: &vk::Format,
        dev: &RenderDevice,
    ) -> Result<(Vec<vk::Image>, Vec<vk::ImageView>)> {
        let swapchain_images = unsafe {
            swapchain_loader.get_swapchain_images(*swapchain)?
        };
        let swapchain_image_views = swapchain_images
            .iter()
            .map(|image| {
                let view_info = vk::ImageViewCreateInfo::default()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(*swapchain_image_format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(*image);
                unsafe {
                    dev.logical.create_image_view(&view_info, None)
                }
            })
            .collect::<VkResult<Vec<vk::ImageView>>>()?;

        Ok((
            swapchain_images,
            swapchain_image_views,
        ))
    }
}