use std::collections::BTreeMap;
use color_eyre::Result;
use vulkano::descriptor_set::layout::{DescriptorBindingFlags, DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags, DescriptorSetLayoutCreateInfo, DescriptorType};
use vulkano::shader::ShaderStages;
use crate::renderer::core::context::RenderContext;

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
        self.bindings.push((
            binding,
            DescriptorSetLayoutBinding {
                descriptor_type,
                descriptor_count,
                stages,
                binding_flags,
                ..Default::default()
            }
        ));
        self
    }

    pub fn build(
        self,
        ctx: &RenderContext,
    ) -> Result<DescriptorSetLayout> {
        DescriptorSetLayout::new(
            ctx.device.clone(),
            DescriptorSetLayoutCreateInfo {
                flags: DescriptorSetLayoutCreateFlags::empty(),
                bindings: BTreeMap::from(self.bindings),
                ..Default::default()
            }
        ).into()
    }
}