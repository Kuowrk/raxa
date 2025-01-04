use std::sync::Arc;
use ash::vk;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use winit::window::Window;
use crate::renderer::core::device::RenderDevice;
use crate::renderer::internals::swapchain::Swapchain;

/// Presentation target of the renderer, encapsulating the window, surface, and swapchain
pub struct RenderTarget {
    pub window: Arc<Window>,

    pub surface: vk::SurfaceKHR,
    pub surface_loader: ash::khr::surface::Instance,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_present_mode: vk::PresentModeKHR,

    pub swapchain: Swapchain,
}

impl RenderTarget {
    pub fn new(
        window: Arc<Window>,
        surface: vk::SurfaceKHR,
        surface_loader: ash::khr::surface::Instance,
        dev: &RenderDevice,
    ) -> Result<Self> {
        let surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(dev.physical, surface)?
        };

        let surface_present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(dev.physical, surface)?
        };

        let surface_format = surface_formats
            .iter()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_SRGB
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .ok_or_eyre("No suitable surface format found")?;

        let surface_present_mode = surface_present_modes
            .iter()
            .find(|mode| **mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(&vk::PresentModeKHR::FIFO);

        let swapchain = Swapchain::new(
            &surface,
            &surface_loader,
            surface_format,
            surface_present_mode,
            &window,
            dev,
        )?;

        Ok(Self {
            window,
            surface,
            surface_loader,
            surface_format: *surface_format,
            surface_present_mode: *surface_present_mode,
            swapchain,
        })
    }

    pub fn resize(
        &mut self,
        device: &RenderDevice,
    ) -> Result<()> {
        unsafe {
            device.logical.device_wait_idle()?;
        }

        self.swapchain = Swapchain::new(
            &self.surface,
            &self.surface_loader,
            &self.surface_format,
            &self.surface_present_mode,
            &self.window,
            device,
        )?;

        Ok(())
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

