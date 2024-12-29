use ash::vk;

pub fn copy_image_to_image(
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

pub fn transition_image_layout(
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