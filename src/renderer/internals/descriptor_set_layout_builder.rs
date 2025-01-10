use color_eyre::Result;
use ash::vk;
use crate::renderer::resources::allocator::RenderResourceType;

pub struct DescriptorSetLayoutBuilder<'a> {
    bindings: Vec<vk::DescriptorSetLayoutBinding<'a>>,
    binding_flags: Vec<vk::DescriptorBindingFlags>,
}

impl DescriptorSetLayoutBuilder<'_> {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            binding_flags: Vec::new(),
        }
    }

    pub fn add_binding(
        mut self,
        binding: u32,
        descriptor_type: vk::DescriptorType,
        descriptor_count: u32,
        stages: vk::ShaderStageFlags,
        binding_flags: vk::DescriptorBindingFlags,
        immutable_samplers: Option<&[vk::Sampler]>,
    ) -> Self {
        let mut binding = vk::DescriptorSetLayoutBinding::default()
            .binding(binding)
            .descriptor_type(descriptor_type)
            .descriptor_count(descriptor_count)
            .stage_flags(stages);

        if let Some(immutable_samplers) = immutable_samplers {
            binding = binding.immutable_samplers(immutable_samplers);
        }

        self.bindings.push(binding);
        self.binding_flags.push(binding_flags);
        self
    }

    pub fn add_binding_for_resource_type(
        mut self,
        binding: u32,
        resource_type: RenderResourceType,
    ) -> Self {
        self.add_binding(
            binding,
            resource_type.descriptor_type(),
            resource_type.descriptor_count(),
            vk::ShaderStageFlags::ALL,
            resource_type.descriptor_binding_flags(),
            None,
        )
    }

    pub fn build(
        self,
        flags: vk::DescriptorSetLayoutCreateFlags,
        device: &ash::Device,
    ) -> Result<vk::DescriptorSetLayout> {
        let mut binding_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfoEXT::default()
            .binding_flags(&self.binding_flags);
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&self.bindings)
            .flags(flags)
            .push_next(&mut binding_flags_info);
        Ok(unsafe {
            device.create_descriptor_set_layout(&layout_info, None)?
        })
    }
}