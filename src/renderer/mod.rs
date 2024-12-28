pub mod camera;
pub mod util;

mod core;
mod shader_data;
mod resources;
mod vk;

use std::ops::Deref;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use winit::event_loop::EventLoop;
use std::sync::Arc;

use core::config::RenderConfig;
use core::context::RenderContext;
use core::resources::RenderResources;
use core::state::RenderState;
use core::viewport::RenderViewport;

pub struct Renderer {
    ctx: RenderContext,
    vpt: Option<RenderViewport>,
    cfg: RenderConfig,
    res: RenderResources,
    ste: RenderState,
}

impl Renderer {
    pub fn new(
        event_loop: &EventLoop<()>,
        window: Option<Arc<winit::window::Window>>
    ) -> Result<Self> {
        let ctx = RenderContext::new(event_loop, window.as_ref())?;
        let vpt = if window.is_some() {
            Some(RenderViewport::new(window.unwrap(), &ctx)?)
        } else {
            None
        };
        let cfg = RenderConfig::default();
        let res = RenderResources::new(&ctx)?;
        let ste = RenderState::new(&ctx)?;

        Ok(Self {
            ctx,
            vpt,
            cfg,
            res,
            ste,
        })
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

        let mut vertex_buffers = Vec::new();
        for model in &self.res.models {
            vertex_buffers.push(model.vertex_buffer.clone());
        }

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
                self.res.vertex_buffer_allocator
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