use std::sync::Arc;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use vulkano::image::{Image, ImageUsage};
use vulkano::image::view::ImageView;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::swapchain::{ColorSpace, CompositeAlpha, Surface, Swapchain, SwapchainCreateInfo};
use winit::window::Window;

use crate::renderer::core::context::RenderContext;

/// Target of the renderer, where the renderer will draw to
pub struct RenderViewport {
    pub window: Arc<Window>,
    pub surface: Arc<Surface>,
    pub swapchain: Arc<Swapchain>,
    pub swapchain_images: Vec<Arc<Image>>,
    pub swapchain_image_views: Vec<Arc<ImageView>>,
    pub viewport: Viewport,
}

impl RenderViewport {
    pub fn new(window: Arc<Window>, ctx: &RenderContext) -> Result<Self> {
        let surface = Surface::from_window(
            ctx.instance.clone(),
            window.clone(),
        )?;
        let window_size = window.inner_size();

        let (
            swapchain,
            swapchain_images,
        ) = {
            let surface_capabilities = ctx
                .device
                .physical_device()
                .surface_capabilities(&surface, Default::default())?;

            let surface_formats = ctx
                .device
                .physical_device()
                .surface_formats(&surface, Default::default())?;

            let (
                image_format,
                _color_space
            ) = surface_formats
                .iter()
                .find(|(_format, space)| {
                    *space == ColorSpace::SrgbNonLinear
                })
                .ok_or_eyre("No suitable surface format found")?;

            Swapchain::new(
                ctx.device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    // Some drivers report an `min_image_count` of 1, but fullscreen mode requires
                    // at least 2. Therefore, we must ensure the count is at least 2 otherwise the
                    // program would crash when entering fullscreen mode on those drivers.
                    min_image_count: surface_capabilities.min_image_count.max(2),
                    image_format: *image_format,
                    image_extent: window_size.into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .into_iter()
                        .find(|&composite_alpha| {
                            composite_alpha == CompositeAlpha::Inherit
                        })
                        .ok_or_eyre("No suitable composite alpha found")?,
                    ..Default::default()
                }
            )
        }?;

        let swapchain_image_views =
            Self::create_swapchain_image_views(&swapchain_images)?;

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window_size.into(),
            depth_range: 0.0..=1.0,
        };

        Ok(Self {
            window,
            surface,
            swapchain,
            swapchain_images,
            swapchain_image_views,
            viewport,
        })
    }

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
}

