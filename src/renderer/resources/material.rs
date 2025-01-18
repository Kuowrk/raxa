use std::ffi::CString;
use std::sync::Arc;
use ash::vk;
use color_eyre::eyre::{eyre, OptionExt};
use color_eyre::Result;
use crate::renderer::resources::shader::{ComputeShader, GraphicsShader};
use crate::renderer::resources::vertex::VertexInputDescription;
pub struct Material {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    pipeline_bind_point: vk::PipelineBindPoint,
    device: Arc<ash::Device>,
}

impl Material {
    pub fn update_push_constants(
        &self,
        command_buffer: vk::CommandBuffer,
        stage_flags: vk::ShaderStageFlags,
        data: &[u8],
    ) {
        unsafe {
            self.device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                stage_flags,
                0,
                data,
            );
        }
    }

    pub fn bind_pipeline(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device.cmd_bind_pipeline(
                command_buffer,
                self.pipeline_bind_point,
                self.pipeline,
            );
        }
    }

    pub fn bind_descriptor_sets(
        &self,
        command_buffer: vk::CommandBuffer,
        first_set: u32,
        descriptor_sets: &[vk::DescriptorSet],
    ) {
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                self.pipeline_bind_point,
                self.pipeline_layout,
                first_set,
                descriptor_sets,
                &[],
            );
        }
    }
}

pub struct GraphicsMaterialBuilder<'a> {
    device: Arc<ash::Device>,

    vertex_input_description: VertexInputDescription,
    vertex_input: vk::PipelineVertexInputStateCreateInfo<'a>,
    input_assembly: vk::PipelineInputAssemblyStateCreateInfo<'a>,
    rasterization: vk::PipelineRasterizationStateCreateInfo<'a>,
    color_blend_attachment: vk::PipelineColorBlendAttachmentState,
    multisample: vk::PipelineMultisampleStateCreateInfo<'a>,
    depth_stencil: vk::PipelineDepthStencilStateCreateInfo<'a>,
    color_attachment_format: vk::Format,
    rendering_info: vk::PipelineRenderingCreateInfo<'a>,
    shader: Option<GraphicsShader>,
    pipeline_layout: Option<vk::PipelineLayout>,
}

impl<'a> GraphicsMaterialBuilder<'a> {
    fn new(device: Arc<ash::Device>) -> Self {
        let vertex_input_desc = VertexInputDescription::default();
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&vertex_input_desc.attributes)
            .vertex_binding_descriptions(&vertex_input_desc.bindings)
            .flags(vertex_input_desc.flags);
        let input_assembly = Self::default_input_assembly_info();
        let rasterization = Self::default_rasterization_info();
        let color_blend_attachment = Self::default_color_blend_state();
        let multisample = Self::default_multisample_info();
        let depth_stencil = Self::default_depth_stencil_info();
        let color_attachment_format = vk::Format::UNDEFINED;
        let rendering_info = vk::PipelineRenderingCreateInfo::default();
        let shader = None;
        let pipeline_layout = None;

        Self {
            device,

            vertex_input_description,
            vertex_input,
            input_assembly,
            rasterization,
            color_blend_attachment,
            multisample,
            depth_stencil,
            color_attachment_format,
            rendering_info,
            shader,
            pipeline_layout,
        }
    }

    pub fn with_shader(mut self, shader: GraphicsShader) -> Self {
        let _ = self.shader.replace(shader);
        self
    }

    pub fn with_pipeline_layout(mut self, layout: vk::PipelineLayout) -> Self {
        let _ = self.pipeline_layout.replace(layout);
        self
    }

    pub fn with_input_topology(mut self, topology: vk::PrimitiveTopology) -> Self {
        self.input_assembly.topology = topology;
        self.input_assembly.primitive_restart_enable = vk::FALSE;
        self
    }

    pub fn with_polygon_mode(mut self, mode: vk::PolygonMode) -> Self {
        self.rasterization.polygon_mode = mode;
        self.rasterization.line_width = 1.0;
        self
    }

    pub fn with_cull_mode(
        mut self,
        cull_mode: vk::CullModeFlags,
        front_face: vk::FrontFace,
    ) -> Self {
        self.rasterization.cull_mode = cull_mode;
        self.rasterization.front_face = front_face;
        self
    }

    pub fn with_multisampling_disabled(mut self) -> Self {
        self.multisample.sample_shading_enable = vk::FALSE;
        // 1 sample per pixel means no multisampling
        self.multisample.rasterization_samples = vk::SampleCountFlags::TYPE_1;
        self.multisample.min_sample_shading = 1.0;
        self.multisample.p_sample_mask = std::ptr::null();
        self.multisample.alpha_to_coverage_enable = vk::FALSE;
        self.multisample.alpha_to_one_enable = vk::FALSE;
        self
    }

    pub fn with_blending_disabled(mut self) -> Self {
        // Default RGBA write mask
        self.color_blend_attachment.color_write_mask =
            vk::ColorComponentFlags::RGBA;
        // No blending
        self.color_blend_attachment.blend_enable = vk::FALSE;
        self
    }

    // Make sure the transparent object is rendered AFTER the opaque ones
    pub fn with_alpha_blending_enabled(mut self) -> Self {
        let blend = &mut self.color_blend_attachment;
        blend.color_write_mask = vk::ColorComponentFlags::RGBA;
        blend.blend_enable = vk::TRUE;
        blend.src_color_blend_factor = vk::BlendFactor::SRC_ALPHA;
        blend.dst_color_blend_factor = vk::BlendFactor::ONE_MINUS_SRC_ALPHA;
        blend.color_blend_op = vk::BlendOp::ADD;
        blend.src_alpha_blend_factor = vk::BlendFactor::ONE;
        blend.dst_alpha_blend_factor = vk::BlendFactor::ZERO;
        blend.alpha_blend_op = vk::BlendOp::ADD;
        self
    }

    pub fn with_additive_blending_enabled(mut self) -> Self {
        let blend = &mut self.color_blend_attachment;
        blend.color_write_mask = vk::ColorComponentFlags::RGBA;
        blend.blend_enable = vk::TRUE;
        blend.src_color_blend_factor = vk::BlendFactor::ONE;
        blend.dst_color_blend_factor = vk::BlendFactor::DST_ALPHA;
        blend.color_blend_op = vk::BlendOp::ADD;
        blend.src_alpha_blend_factor = vk::BlendFactor::ONE;
        blend.dst_alpha_blend_factor = vk::BlendFactor::ZERO;
        blend.alpha_blend_op = vk::BlendOp::ADD;
        self
    }

    pub fn with_color_attachment_format(mut self, format: vk::Format) -> Self {
        self.color_attachment_format = format;
        // Connect the format to the rendering_info struct
        self.rendering_info.color_attachment_count = 1;
        self.rendering_info.p_color_attachment_formats =
            &self.color_attachment_format;
        self
    }

    pub fn with_depth_attachment_format(mut self, format: vk::Format) -> Self {
        self.rendering_info.depth_attachment_format = format;
        self
    }

    pub fn with_depth_test(
        mut self,
        enable: bool,
        compare: Option<vk::CompareOp>,
    ) -> Self {
        self.depth_stencil.depth_test_enable =
            if enable { vk::TRUE } else { vk::FALSE };
        self.depth_stencil.depth_write_enable =
            if enable { vk::TRUE } else { vk::FALSE };
        self.depth_stencil.depth_compare_op = if enable {
            if let Some(compare) = compare {
                compare
            } else {
                vk::CompareOp::LESS_OR_EQUAL
            }
        } else {
            vk::CompareOp::ALWAYS
        };
        self.depth_stencil.min_depth_bounds = 0.0;
        self.depth_stencil.max_depth_bounds = 1.0;
        self
    }

    pub fn with_vertex_input(mut self, description: VertexInputDescription) -> Self {
        self.vertex_input_description = description;
        self.vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&self.vertex_input_description.attributes)
            .vertex_binding_descriptions(&self.vertex_input_description.bindings)
            .flags(self.vertex_input_description.flags);
        self
    }

    pub fn build(mut self) -> Result<Material> {
        let device = self.device;

        let shader = self
            .shader
            .take()
            .ok_or_eyre("No shader provided for GraphicsMaterialBuilder")?;
        let shader_main_fn_name = CString::new("main")?;
        let shader_stages = vec![
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(shader.vert_mod)
                .name(&shader_main_fn_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(shader.frag_mod)
                .name(&shader_main_fn_name),
        ];

        let pipeline_layout = self.pipeline_layout.take().ok_or_eyre(
            "No pipeline layout provided for GraphicsMaterialBuilder",
        )?;

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        };

        let color_blend_info = vk::PipelineColorBlendStateCreateInfo {
            logic_op_enable: vk::FALSE,
            logic_op: vk::LogicOp::COPY,
            attachment_count: 1,
            p_attachments: &self.color_blend_attachment,
            ..Default::default()
        };

        // Use dynamic state for viewport and scissor configuration
        let dynamic_states =
            [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_info = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&dynamic_states);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .push_next(&mut self.rendering_info)
            .stages(&shader_stages)
            .layout(pipeline_layout)
            .vertex_input_state(&self.vertex_input)
            .input_assembly_state(&self.input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&self.rasterization)
            .multisample_state(&self.multisample)
            .color_blend_state(&color_blend_info)
            .depth_stencil_state(&self.depth_stencil)
            .dynamic_state(&dynamic_info);

        let pipeline = unsafe {
            match device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            ) {
                Ok(pipelines) => Ok(pipelines),
                Err(_) => Err(eyre!("Failed to create graphic pipelines")),
            }
        }?[0];

        Ok(Material {
            pipeline,
            pipeline_layout,
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            device,
        })
    }

    fn default_input_assembly_info() -> vk::PipelineInputAssemblyStateCreateInfo
    {
        vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
    }

    fn default_rasterization_info() -> vk::PipelineRasterizationStateCreateInfo
    {
        vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            // Discards all primitives before rasterization stage if true
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            // Backface culling
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            // No depth bias
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0)
    }

    fn default_color_blend_state() -> vk::PipelineColorBlendAttachmentState {
        // Enable alpha blending by default
        vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
    }

    fn default_multisample_info() -> vk::PipelineMultisampleStateCreateInfo {
        vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            // 1 sample per pixel means no multisampling
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false)
    }

    fn default_depth_stencil_info() -> vk::PipelineDepthStencilStateCreateInfo {
        vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0)
            .stencil_test_enable(false)
    }
}

pub struct ComputeMaterialBuilder<'a> {
    device: Arc<ash::Device>,
    shader: Option<ComputeShader>,
    pipeline_layout: Option<vk::PipelineLayout>,
}

impl<'a> ComputeMaterialBuilder<'a> {
    pub fn new(device: Arc<ash::Device>) -> Self {
        Self {
            device,
            shader: None,
            pipeline_layout: None,
        }
    }

    pub fn with_shader(mut self, shader: ComputeShader) -> Self {
        let _ = self.shader.replace(shader);
        self
    }

    pub fn with_pipeline_layout(mut self, layout: vk::PipelineLayout) -> Self {
        let _ = self.pipeline_layout.replace(layout);
        self
    }

    pub fn build(mut self) -> Result<Material> {
        let shader = self
            .shader
            .take()
            .ok_or_eyre("No shader provided for ComputeMaterialBuilder")?;
        let pipeline_layout = self.pipeline_layout.take().ok_or_eyre(
            "No pipeline layout provided for ComputeMaterialBuilder",
        )?;

        let name = CString::new("main")?;
        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader.comp_mod)
            .name(&name);

        let pipeline_info = vk::ComputePipelineCreateInfo::default()
            .layout(pipeline_layout)
            .stage(stage_info);
        let pipeline = unsafe {
            match self.device.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            ) {
                Ok(pipelines) => Ok(pipelines),
                Err(_) => Err(eyre!("Failed to create compute pipeline")),
            }
        }?[0];

        Ok(Material {
            pipeline,
            pipeline_layout,
            pipeline_bind_point: vk::PipelineBindPoint::COMPUTE,
            device: self.device,
        })
    }
}
