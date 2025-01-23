use color_eyre::Result;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;
use crate::renderer::contexts::resource_ctx::RenderResourceContext;
use crate::renderer::resources::image::Image;
use crate::renderer::resources::megabuffer::{Megabuffer, MegabufferExt};

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024;  // 1 MB

pub struct Frame {
    draw_color_image: Image,
    draw_depth_image: Image,
    vertex_subbuffer: Megabuffer,
    index_subbuffer: Megabuffer,
}

impl Frame {
    pub fn new(
        dev_ctx: &RenderDeviceContext,
        res_ctx: &RenderResourceContext,
    ) -> Result<Self> {
        let target_size = dev_ctx.target.as_ref().unwrap().get_size();
        
        let draw_color_image = dev_ctx.device.create_color_image(target_size.width, target_size.height)?;
        let draw_depth_image = dev_ctx.device.create_depth_image(target_size.width, target_size.height)?;

        let vertex_subbuffer = res_ctx.storage.vertex_megabuffer
            .allocate_subbuffer(FRAME_VERTEX_BUFFER_SIZE)?;
        let index_subbuffer = res_ctx.storage.index_megabuffer
            .allocate_subbuffer(FRAME_INDEX_BUFFER_SIZE)?;

        Ok(Self {
            draw_color_image,
            draw_depth_image,
            vertex_subbuffer,
            index_subbuffer,
        })
    }
}
