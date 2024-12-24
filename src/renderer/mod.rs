mod context;
mod config;
mod resources;
mod viewport;
mod state;

use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use winit::event_loop::EventLoop;
use winit::window::Window;
use context::RenderContext;
use config::RenderConfig;
use resources::RenderResources;
use state::RenderState;
use std::sync::Arc;
use vulkano::swapchain::{acquire_next_image, SwapchainPresentInfo};
use vulkano::{sync, Validated, VulkanError};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::sync::GpuFuture;
use viewport::RenderViewport;

pub struct Renderer {
    ctx: RenderContext,
    cfg: RenderConfig,
    res: RenderResources,
    ste: RenderState,
    vpt: Option<RenderViewport>,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let ctx = RenderContext::new(event_loop)?;
        let cfg = RenderConfig::default();
        let res = RenderResources::default();
        let ste = RenderState::default();

        Ok(Self {
            ctx,
            cfg,
            res,
            ste,
            vpt: None,
        })
    }

    pub fn set_window(&mut self, window: Arc<Window>) -> Result<()> {
        self.vpt = Some(RenderViewport::new(window, &self.ctx)?);

        Ok(())
    }

    pub fn request_resize(&mut self) {
        self.ste.resize_requested = true;
    }

    pub fn draw(&mut self) -> Result<()> {
        let window_size = self.vpt
            .as_ref()
            .ok_or_eyre("No viewport to get size from")?
            .window
            .inner_size();

        // Do not draw the frame if the window size is 0.
        // This can happen when the window is minimized.
        if window_size.width == 0 || window_size.height == 0 {
            return Ok(());
        }

        if self.ste.resize_requested {
            self.vpt.as_mut().ok_or_eyre("No viewport to resize")?.resize()?;
            self.ste.resize_requested = false;
        }

        // Calling this function polls various fences in order to determine what the GPU has
        // already processed and frees the resources that are no longer needed.
        self.ctx.previous_frame_end
            .as_mut()
            .ok_or_eyre("No previous frame end")?
            .cleanup_finished();

        let (
            image_index,
            suboptimal,
            acquire_future,
        ) = match acquire_next_image(
            self.vpt
                .as_ref()
                .ok_or_eyre("No viewport to acquire next image from")?
                .swapchain
                .clone(),
            None,
        )
        .map_err(Validated::unwrap) {
            Ok(r) => r,
            Err(VulkanError::OutOfDate) => {
                self.request_resize();
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        if suboptimal {
            self.request_resize();
        }

        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            self.ctx.command_buffer_allocator.clone(),
            self.ctx.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        let command_buffer = command_buffer_builder.build()?;

        let future_result = self.ctx
            .previous_frame_end
            .take()
            .ok_or_eyre("No previous frame end")?
            .join(acquire_future)
            .then_execute(self.ctx.queue.clone(), command_buffer)?
            .then_swapchain_present(
                self.ctx.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.vpt
                        .as_ref()
                        .ok_or_eyre("No viewport to present to")?
                        .swapchain
                        .clone(),
                    image_index,
                )
            )
            .then_signal_fence_and_flush();

        match future_result.map_err(Validated::unwrap) {
            Ok(future) => {
                self.ctx.previous_frame_end = Some(future.boxed());
            }
            Err(VulkanError::OutOfDate) => {
                self.request_resize();
                self.ctx.previous_frame_end = Some(sync::now(self.ctx.device.clone()).boxed());
            }
            Err(e) => {
                self.ctx.previous_frame_end = Some(sync::now(self.ctx.device.clone()).boxed());
                return Err(e.into())
            }
        }

        Ok(())
    }
}