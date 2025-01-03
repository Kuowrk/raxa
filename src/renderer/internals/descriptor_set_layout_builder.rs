use color_eyre::Result;
use ash::vk;

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
    ) -> Self {
        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(binding)
            .descriptor_type(descriptor_type)
            .descriptor_count(descriptor_count)
            .stage_flags(stages);

        self.bindings.push(binding);
        self.binding_flags.push(binding_flags);
        self
    }

    pub fn build(
        self,
        device: &ash::Device,
    ) -> Result<vk::DescriptorSetLayout> {
        let mut binding_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::default()
            .binding_flags(&self.binding_flags);
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&self.bindings)
            .push_next(&mut binding_flags_info);
        Ok(unsafe {
            device.create_descriptor_set_layout(&layout_info, None)?
        })
    }
}