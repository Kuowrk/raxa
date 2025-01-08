use std::collections::HashMap;

use ash::vk;
use color_eyre::Result;

use super::descriptor_set_layout_builder::DescriptorSetLayoutBuilder;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct RenderResourceHandle(u32);

#[derive(PartialEq)]
pub enum BindlessTableType {
    Buffer,
    Texture,
    RwTexture,
    Tlas, // Top-level acceleration structure
}

impl BindlessTableType {
    const ALL_TABLES: &'static [Self] = &[
        Self::Buffer,
        Self::Texture,
        Self::RwTexture,
        Self::Tlas,
    ];

    pub fn descriptor_type(&self) -> vk::DescriptorType {
        match self {
            Self::Buffer => vk::DescriptorType::STORAGE_BUFFER,
            Self::Texture => vk::DescriptorType::SAMPLED_IMAGE,
            Self::RwTexture => vk::DescriptorType::STORAGE_IMAGE,
            Self::Tlas => vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
        }
    }

    pub fn descriptor_count(&self) -> u32 {
        match self {
            Self::Buffer => 1000,
            Self::Texture => 1000,
            Self::RwTexture => 1000,
            Self::Tlas => 1000,
        }
    }
    
    pub fn descriptor_pool_sizes(
        immutable_sampler_count: u32,
    ) -> Vec<vk::DescriptorPoolSize> {
        let mut type_histogram = HashMap::new();
        
        // For each descriptor type, retrieve the bindless size.
        for table in Self::ALL_TABLES {
            type_histogram
                .entry(table.descriptor_type())
                .and_modify(|count| *count += table.descriptor_count())
                .or_insert(table.descriptor_count());
        }
        
        // Add immutable sampler descriptors to texture descriptor pool size.
        type_histogram
            .entry(Self::Texture.descriptor_type())
            .and_modify(|v| *v += immutable_sampler_count);
        
        type_histogram
            .iter()
            .map(|(ty, descriptor_count)| vk::DescriptorPoolSize {
                ty: *ty,
                descriptor_count: *descriptor_count,
            })
            .collect::<Vec<vk::DescriptorPoolSize>>()
    }
}

pub fn create_bindless_layout(
    device: &ash::Device,
    immutable_samplers: &[vk::Sampler],
) -> Result<(
    Vec<vk::DescriptorSetLayout>,
    vk::PipelineLayout,
)> {
    let descriptor_set_layouts = BindlessTableType::ALL_TABLES
        .iter()
        .map(|table| {
            let mut builder = DescriptorSetLayoutBuilder::new()
                .add_binding(
                    if *table == BindlessTableType::Texture {
                        // Set texture binding start at the end of the immutable samplers.
                        immutable_samplers.len() as u32
                    } else {
                        0
                    },
                    table.descriptor_type(),
                    table.descriptor_count(),
                    vk::ShaderStageFlags::ALL,
                    vk::DescriptorBindingFlags::PARTIALLY_BOUND
                        | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
                        | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
                    None,
                );
            
            if *table == BindlessTableType::Texture {
                builder = builder.add_binding(
                    0,
                    vk::DescriptorType::SAMPLER,
                    immutable_samplers.len() as u32,
                    vk::ShaderStageFlags::ALL,
                    vk::DescriptorBindingFlags::empty(),
                    Some(immutable_samplers),
                );
            }

            builder.build(
                vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
                device,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    
    let num_push_constants = PushConstantSlots::ALL_SLOTS.len() as u32;
    let num_push_constants_size = num_push_constants * std::mem::size_of::<u32>() as u32;
    let push_constant_range = vk::PushConstantRange::default()
        .stage_flags(vk::ShaderStageFlags::ALL)
        .offset(0)
        .size(num_push_constants_size);
    let push_constant_ranges = [push_constant_range];
    
    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&descriptor_set_layouts)
        .push_constant_ranges(&push_constant_ranges);

    let pipeline_layout = unsafe {
        device.create_pipeline_layout(&pipeline_layout_create_info, None)?
    };
    
    Ok((descriptor_set_layouts, pipeline_layout))

}

pub enum PushConstantSlots {
    ObjectIndex,
    MaterialIndex,
    VertexOffset,
}

impl PushConstantSlots {
    const ALL_SLOTS: &'static [Self] = &[
        Self::ObjectIndex,
        Self::MaterialIndex,
        Self::VertexOffset,
    ];
}