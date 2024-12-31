use std::sync::Arc;
use ash::prelude::VkResult;
use ash::vk;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use winit::window::Window;

use crate::renderer::core::context::RenderContext;

/// Presentation target of the renderer, encapsulating the window, surface, and swapchain
pub struct RenderTarget {
    pub window: Arc<Window>,

    pub surface: vk::SurfaceKHR,
    pub surface_loader: ash::khr::surface::Instance,
    pub surface_format: vk::SurfaceFormatKHR,

    pub swapchain: vk::SwapchainKHR,
    pub swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,
}

impl RenderTarget {
    pub fn new(
        window: Arc<Window>,
        surface: vk::SurfaceKHR,
        surface_loader: ash::khr::surface::Instance,
        ctx: &RenderContext,
    ) -> Result<Self> {
        let (
            swapchain,
            swapchain_loader,
            surface_format,
        ) = Self::create_swapchain(
            &surface,
            &surface_loader,
            &window,
            ctx,
        )?;

        let (
            swapchain_images,
            swapchain_image_views,
        ) = Self::get_swapchain_images(
            &swapchain,
            &swapchain_loader,
            &surface_format.format,
            ctx,
        )?;

        Ok(Self {
            window,
            surface,
            surface_loader,
            surface_format,
            swapchain,
            swapchain_loader,
            swapchain_images,
            swapchain_image_views,
        })
    }

    fn create_swapchain(
        surface: &vk::SurfaceKHR,
        surface_loader: &ash::khr::surface::Instance,
        window: &Window,
        ctx: &RenderContext,
    ) -> Result<(
        vk::SwapchainKHR,
        ash::khr::swapchain::Device,
        vk::SurfaceFormatKHR,
    )> {
        let physical_device = ctx.device.physical;

        let surface_capabilities = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(physical_device, *surface)?
        };

        let surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_device, *surface)?
        };

        let surface_present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(physical_device, *surface)?
        };

        let surface_format = surface_formats
            .iter()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_SRGB
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .ok_or_eyre("No suitable surface format found")?;

        let present_mode = surface_present_modes
            .iter()
            .find(|mode| **mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(&vk::PresentModeKHR::FIFO);

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

        let swapchain_loader = ash::khr::swapchain::Device::new(
            &ctx.instance, &ctx.device.logical);
        let swapchain_info = vk::SwapchainCreateInfoKHR::default()
            .surface(*surface)
            .min_image_count(min_image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(image_extent)
            .image_usage(
                vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_DST
            )
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(*present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain = unsafe {
            swapchain_loader.create_swapchain(&swapchain_info, None)?
        };

        Ok((
            swapchain,
            swapchain_loader,
            *surface_format
        ))
    }

    fn get_swapchain_images(
        swapchain: &vk::SwapchainKHR,
        swapchain_loader: &ash::khr::swapchain::Device,
        swapchain_image_format: &vk::Format,
        ctx: &RenderContext,
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
                    ctx.device.logical.create_image_view(&view_info, None)
                }
            })
            .collect::<VkResult<Vec<vk::ImageView>>>()?;

        Ok((
            swapchain_images,
            swapchain_image_views,
        ))
    }

    /*
    pub fn resize(&mut self) -> Result<()> {
        let (
            new_swapchain,
            new_swapchain_images,
        ) = self.swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: self.window.inner_size().into(),
                ..self.swapchain.create_info()
            })?;

        self.swapchain = new_swapchain;
        self.swapchain_images = new_swapchain_images;
        self.swapchain_image_views = Self::create_swapchain_image_views(&self.swapchain_images)?;
        self.viewport.extent = self.window.inner_size().into();

        Ok(())
    }

    fn create_swapchain_image_views(
        swapchain_images: &[Arc<Image>],
    ) -> Result<Vec<Arc<ImageView>>> {
        swapchain_images
            .iter()
            .map(|image| {
                ImageView::new_default(image.clone())
                    .map_err(Into::into)
            })
            .collect::<Result<Vec<_>>>()
    }

    pub fn get_size(&self) -> PhysicalSize<u32> {
        self.window.inner_size()
    }
     */
}

