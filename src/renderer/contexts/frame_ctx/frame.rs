use color_eyre::Result;
use crate::renderer::contexts::device_ctx::RenderDeviceContext;
use crate::renderer::contexts::resource_ctx::RenderResourceContext;
use crate::renderer::resources::image::Image;
use crate::renderer::resources::megabuffer::{AllocatedMegabufferRegion, MegabufferExt};

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024;  // 1 MB

pub struct Frame {
    draw_image: Image,
    draw_depth_image: Image,
    draw_resolve_image: Image,
    vertex_buffer_region: AllocatedMegabufferRegion,
    index_buffer_region: AllocatedMegabufferRegion,
}

impl Frame {
    pub fn new(
        dev_ctx: &RenderDeviceContext,
        res_ctx: &RenderResourceContext,
    ) -> Result<Self> {
        let draw_depth_image = dev_ctx.device.create_depth_image();
        let vertex_buffer_region = res_ctx.storage.vertex_megabuffer
            .allocate_region(FRAME_VERTEX_BUFFER_SIZE)?;
        let index_buffer_region = res_ctx.storage.index_megabuffer
            .allocate_region(FRAME_INDEX_BUFFER_SIZE)?;
    }
}
