pub mod camera;
pub mod util;

mod core;
mod shader_data;
mod resources;

use std::ops::Deref;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use winit::event_loop::EventLoop;
use winit::window::Window;
use std::sync::Arc;
use vulkano::swapchain::{acquire_next_image, SwapchainAcquireFuture, SwapchainPresentInfo};
use vulkano::{sync, NonExhaustive, Validated, VulkanError};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo};
use vulkano::render_pass::{AttachmentLoadOp, AttachmentStoreOp};
use vulkano::sync::GpuFuture;

use core::config::RenderConfig;
use core::context::RenderContext;
use core::resources::RenderResources;
use core::state::RenderState;
use core::viewport::RenderViewport;

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
        let res = RenderResources::new(&ctx)?;
        let ste = RenderState::new(&ctx)?;

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
        let (
            swapchain_image_index,
            swapchain_acquire_future
        ) = match self.pre_draw()? {
            Some(r) => r,
            None => return Ok(()),
        };

        let vpt = self.vpt.as_ref().ok_or_eyre("No viewport")?;

        let mut cmd = AutoCommandBufferBuilder::primary(
            self.res.command_buffer_allocator.clone(),
            self.ctx.graphics_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        cmd.begin_rendering(RenderingInfo {
            color_attachments: vec![
                Some(RenderingAttachmentInfo::image_view(
                    vpt.swapchain_image_views[swapchain_image_index as usize].clone()
                ))
            ],
            ..Default::default()
        })?
            .set_viewport(
                0,
                [vpt.viewport.clone()].into_iter().collect(),
            )?
            .bind_vertex_buffers(
                0,
                &self.res.vertex_buffer_allocator
            )?;

        let cmd = cmd.build()?;

        let future_result = self.ste
            .previous_frame_end
            .take()
            .ok_or_eyre("No previous frame end")?
            .join(swapchain_acquire_future)
            .then_execute(self.ctx.graphics_queue.clone(), cmd)?
            .then_swapchain_present(
                self.ctx.graphics_queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.vpt
                        .as_ref()
                        .ok_or_eyre("No viewport to present to")?
                        .swapchain
                        .clone(),
                    swapchain_image_index,
                )
            )
            .then_signal_fence_and_flush();

        match future_result.map_err(Validated::unwrap) {
            Ok(future) => {
                self.ste.previous_frame_end = Some(future.boxed());
            }
            Err(VulkanError::OutOfDate) => {
                self.request_resize();
                self.ste.previous_frame_end = Some(sync::now(self.ctx.device.clone()).boxed());
            }
            Err(e) => {
                self.ste.previous_frame_end = Some(sync::now(self.ctx.device.clone()).boxed());
                return Err(e.into())
            }
        }

        Ok(())
    }

    fn pre_draw(
        &mut self
    ) -> Result<Option<(u32, SwapchainAcquireFuture)>> {
        let window_size = self.vpt
            .as_ref()
            .ok_or_eyre("No viewport to get size from")?
            .window
            .inner_size();

        // Do not draw the frame if the window size is 0.
        // This can happen when the window is minimized.
        if window_size.width == 0 || window_size.height == 0 {
            return Ok(None);
        }

        if self.ste.resize_requested {
            self.vpt.as_mut().ok_or_eyre("No viewport to resize")?.resize()?;
            self.ste.resize_requested = false;
        }

        // Calling this function polls various fences in order to determine what the GPU has
        // already processed and frees the resources that are no longer needed.
        self.ste.previous_frame_end
            .as_mut()
            .ok_or_eyre("No previous frame end")?
            .cleanup_finished();

        let (
            swapchain_image_index,
            suboptimal,
            swapchain_acquire_future,
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
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };

        if suboptimal {
            self.request_resize();
        }

        Ok(Some((swapchain_image_index, swapchain_acquire_future)))
    }
}