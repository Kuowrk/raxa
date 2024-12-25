use crate::renderer::core::context::RenderContext;
use color_eyre::Result;
use std::collections::BTreeMap;
use std::sync::Arc;
use vulkano::descriptor_set::layout::{DescriptorBindingFlags, DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags, DescriptorSetLayoutCreateInfo, DescriptorType};
use vulkano::shader::ShaderStages;

pub struct DescriptorSetLayoutBuilder {
    bindings: Vec<(u32, DescriptorSetLayoutBinding)>
}

impl DescriptorSetLayoutBuilder {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn add_binding(
        mut self,
        binding: u32,
        descriptor_type: DescriptorType,
        descriptor_count: u32,
        stages: ShaderStages,
        binding_flags: DescriptorBindingFlags,
    ) -> Self {
        let mut descriptor_set_layout_binding = DescriptorSetLayoutBinding::descriptor_type(
            descriptor_type
        );
        descriptor_set_layout_binding.descriptor_count = descriptor_count;
        descriptor_set_layout_binding.stages = stages;
        descriptor_set_layout_binding.binding_flags = binding_flags;

        self.bindings.push((
            binding,
            descriptor_set_layout_binding
        ));
        self
    }

    pub fn build(
        self,
        ctx: &RenderContext,
    ) -> Result<Arc<DescriptorSetLayout>> {
        Ok(DescriptorSetLayout::new(
            ctx.device.clone(),
            DescriptorSetLayoutCreateInfo {
                flags: DescriptorSetLayoutCreateFlags::empty(),
                bindings: BTreeMap::from_iter(
                    self.bindings.into_iter()
                ),
                ..Default::default()
            }
        )?)
    }
}