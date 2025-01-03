use std::ffi::{c_char, CStr};
use std::sync::{Arc, Mutex};
use ash::vk;
use color_eyre::eyre::OptionExt;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use gpu_descriptor::DescriptorAllocator;
use crate::renderer::internals::megabuffer::Megabuffer;
use crate::renderer::internals::command_buffer_allocator::CommandBufferAllocator;
use crate::renderer::internals::queue::{Queue, QueueFamily};
use crate::renderer::internals::transfer_context::TransferContext;

/// Main structure for the renderer that can create resources
pub struct RenderDevice<'a> {
    pub logical: Arc<ash::Device>,
    pub physical: vk::PhysicalDevice,
    pub instance: &'a ash::Instance,

    // For now, require the graphics queue to support presentation
    pub graphics_queue: Arc<Queue>,
    pub compute_queue: Arc<Queue>,
    pub transfer_queue: Arc<Queue>,

    memory_allocator: Arc<Mutex<Allocator>>,
    command_buffer_allocator: CommandBufferAllocator<'a>,
    descriptor_set_allocator: DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>,

    transfer_context: Arc<TransferContext<'a>>,
}

impl RenderDevice<'_> {
    pub fn new(
        instance: &ash::Instance,
        surface: Option<&vk::SurfaceKHR>,
        surface_loader: Option<&ash::khr::surface::Instance>,
    ) -> Result<Self> {
        let (
            physical_device,
            graphics_queue_family,
            compute_queue_family,
            transfer_queue_family,
        ) = Self::select_physical_device(
            &instance,
            surface,
            surface_loader,
        )?;

        let (
            logical_device,
            graphics_queue,
            compute_queue,
            transfer_queue,
        ) = Self::create_logical_device(
            &instance,
            &physical_device,
            graphics_queue_family,
            compute_queue_family,
            transfer_queue_family,
        )?;

        let memory_allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: logical_device.clone(),
            physical_device: physical_device.clone(),
            debug_settings: gpu_allocator::AllocatorDebugSettings {
                log_memory_information: true,
                log_leaks_on_shutdown: true,
                store_stack_traces: false,
                log_allocations: true,
                log_frees: true,
                log_stack_traces: false,
            },
            buffer_device_address: true,
            allocation_sizes: Default::default(),
        })?;

        let graphics_queue = Arc::new(graphics_queue);
        let compute_queue = Arc::new(compute_queue);
        let transfer_queue = Arc::new(transfer_queue);

        let command_buffer_allocator = CommandBufferAllocator::new(
            &logical_device,
            &graphics_queue,
        )?;

        let descriptor_set_allocator = DescriptorAllocator::new(1024);

        let transfer_context = TransferContext::new(
            &transfer_queue,
            &logical_device,
        )?;

        Ok(Self {
            logical: Arc::new(logical_device),
            physical: physical_device,

            instance,
            graphics_queue,
            compute_queue,
            transfer_queue,

            memory_allocator: Arc::new(Mutex::new(memory_allocator)),
            command_buffer_allocator,
            descriptor_set_allocator,

            transfer_context: Arc::new(transfer_context),
        })
    }

    pub fn immediate_submit<F>(
        &self,
        func: F,
    ) -> Result<()>
    where
        F: FnOnce(vk::CommandBuffer, &ash::Device) -> Result<()>,
    {
        self.transfer_context.immediate_submit(func)
    }

    pub fn create_megabuffer(
        &self,
        size: u64,
        usage: vk::BufferUsageFlags,
        mem_loc: gpu_allocator::MemoryLocation,
        alignment: u64,
    ) -> Result<Megabuffer> {
        Megabuffer::new(
            size,
            usage,
            mem_loc,
            alignment,
            self.memory_allocator.clone(),
            self.logical.clone(),
            self.transfer_context.clone(),
        )
    }

    fn select_physical_device(
        instance: &ash::Instance,
        surface: Option<&vk::SurfaceKHR>,
        surface_loader: Option<&ash::khr::surface::Instance>,
    ) -> Result<(vk::PhysicalDevice, QueueFamily, QueueFamily, QueueFamily)> {
        let req_device_exts = Self::get_required_device_extensions();
        Ok(unsafe {
            instance
                .enumerate_physical_devices()?
                .into_iter()
                // Filter out devices that do not contain the required device extensions
                .filter(|device| {
                    let supported_extensions = instance
                        .enumerate_device_extension_properties(*device)
                        .map_or(Vec::new(), |exts| exts);

                    req_device_exts.iter().all(|req_ext| {
                        supported_extensions
                            .iter()
                            .map(|sup_exts| {
                                sup_exts.extension_name.as_ptr()
                            })
                            .any(|sup_ext| {
                                match (req_ext.to_str(), CStr::from_ptr(sup_ext).to_str()) {
                                    (Ok(req), Ok(sup)) => req == sup,
                                    _ => false,
                                }
                            })
                    })
                })
                // Filter out devices that do not contain the required queues
                .filter_map(|device| {
                    let props = instance
                        .get_physical_device_queue_family_properties(device);

                    let graphics_queue_family_index = props
                        .iter()
                        .enumerate()
                        .position(|(i, q)| {
                            let supports_graphics = q.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                            if let (Some(surface), Some(surface_loader)) = (surface, surface_loader) {
                                let supports_present = {
                                    surface_loader.get_physical_device_surface_support(
                                        device,
                                        i as u32,
                                        *surface,
                                    ).map_or(false, |b| b)
                                };
                                supports_graphics && supports_present
                            } else {
                                supports_graphics
                            }
                        });

                    let compute_queue_family_index = props
                        .iter()
                        .position(|q| {
                            q.queue_flags.contains(vk::QueueFlags::COMPUTE)
                        });

                    let transfer_queue_family_index = props
                        .iter()
                        .position(|q| {
                            q.queue_flags.contains(vk::QueueFlags::TRANSFER)
                        });

                    if let (
                        Some(graphics_queue_family_index),
                        Some(compute_queue_family_index),
                        Some(transfer_queue_family_index)
                    ) = (
                        graphics_queue_family_index,
                        compute_queue_family_index,
                        transfer_queue_family_index
                    ) {
                        Some((
                            device,
                            graphics_queue_family_index as u32,
                            compute_queue_family_index as u32,
                            transfer_queue_family_index as u32
                        ))
                    } else {
                        None
                    }
                })
                .min_by_key(|(device, _, _, _)| {
                    let props = instance.get_physical_device_properties(*device);
                    match props.device_type {
                        vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                        vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                        vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                        vk::PhysicalDeviceType::CPU => 3,
                        vk::PhysicalDeviceType::OTHER => 4,
                        _ => 5,
                    }
                })
                .map(|(
                          device,
                          graphics_queue_family_index,
                          compute_queue_family_index,
                          transfer_queue_family_index,
                      )| {
                    let queue_family_props = instance.get_physical_device_queue_family_properties(device);
                    let graphics_props = queue_family_props.get(graphics_queue_family_index as usize).unwrap();
                    let compute_props = queue_family_props.get(compute_queue_family_index as usize).unwrap();
                    let transfer_props = queue_family_props.get(transfer_queue_family_index as usize).unwrap();
                    (
                        device,
                        QueueFamily::new(graphics_queue_family_index, *graphics_props, true),
                        QueueFamily::new(compute_queue_family_index, *compute_props, false),
                        QueueFamily::new(transfer_queue_family_index, *transfer_props, false),
                    )
                })
                .ok_or_eyre("No suitable physical device found")?
        })
    }

    fn create_logical_device(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        graphics_queue_family: QueueFamily,
        compute_queue_family: QueueFamily,
        transfer_queue_family: QueueFamily,
    ) -> Result<(ash::Device, Queue, Queue, Queue)> {
        let queue_priorities = [1.0];
        let queue_create_infos = [
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(graphics_queue_family.index)
                .queue_priorities(&queue_priorities),
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(compute_queue_family.index)
                .queue_priorities(&queue_priorities),
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(transfer_queue_family.index)
                .queue_priorities(&queue_priorities),
        ];

        let enabled_extension_names = Self::get_required_device_extensions()
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<*const c_char>>();

        let mut enabled_features = RequiredDeviceFeatures::new(physical_device, instance);

        // Check if the device supports the required features
        if !enabled_features.has_all() {
            return Err(eyre!("Required features not supported"));
        }

        // Create device
        let device = {
            let mut features = vk::PhysicalDeviceFeatures2KHR::default()
                .push_next(&mut enabled_features.dynamic_rendering_features)
                .push_next(&mut enabled_features.synchronization2_features)
                .push_next(&mut enabled_features.buffer_device_address_features)
                .push_next(&mut enabled_features.shader_draw_parameters_features)
                .push_next(&mut enabled_features.descriptor_indexing_features)
                .push_next(&mut enabled_features.descriptor_buffer_features);

            let device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&enabled_extension_names)
                .push_next(&mut features);

            unsafe {
                instance.create_device(*physical_device, &device_create_info, None)?
            }
        };

        let graphics_queue = unsafe {
            let queue = device.get_device_queue(graphics_queue_family.index, 0);
            Queue::new(graphics_queue_family, queue)
        };
        let compute_queue = unsafe {
            let queue = device.get_device_queue(compute_queue_family.index, 0);
            Queue::new(compute_queue_family, queue)
        };
        let transfer_queue = unsafe {
            let queue = device.get_device_queue(transfer_queue_family.index, 0);
            Queue::new(transfer_queue_family, queue)
        };

        Ok((device, graphics_queue, compute_queue, transfer_queue))
    }

    fn get_required_device_extensions() -> Vec<&'static CStr> {
        vec![
            ash::khr::swapchain::NAME,
            ash::khr::dynamic_rendering::NAME,
            ash::khr::buffer_device_address::NAME,
            ash::khr::synchronization2::NAME,
            ash::khr::maintenance3::NAME,
            ash::ext::descriptor_indexing::NAME,
            ash::ext::descriptor_buffer::NAME,

            #[cfg(target_os = "macos")]
            ash::khr::portability_subset::NAME,
        ]
    }
}

struct RequiredDeviceFeatures<'a> {
    pub dynamic_rendering_features: vk::PhysicalDeviceDynamicRenderingFeaturesKHR<'a>,
    pub synchronization2_features: vk::PhysicalDeviceSynchronization2FeaturesKHR<'a>,
    pub buffer_device_address_features: vk::PhysicalDeviceBufferDeviceAddressFeatures<'a>,
    pub shader_draw_parameters_features: vk::PhysicalDeviceShaderDrawParametersFeatures<'a>,
    pub descriptor_indexing_features: vk::PhysicalDeviceDescriptorIndexingFeaturesEXT<'a>,
    pub descriptor_buffer_features: vk::PhysicalDeviceDescriptorBufferFeaturesEXT<'a>,
}

impl<'a> RequiredDeviceFeatures<'a> {
    pub fn new(
        physical_device: &vk::PhysicalDevice,
        instance: &ash::Instance,
    ) -> Self {
        let mut dynamic_rendering_features =
            vk::PhysicalDeviceDynamicRenderingFeaturesKHR::default()
                .dynamic_rendering(true);
        let mut synchronization2_features =
            vk::PhysicalDeviceSynchronization2FeaturesKHR::default()
                .synchronization2(true);
        let mut buffer_device_address_features =
            vk::PhysicalDeviceBufferDeviceAddressFeatures::default()
                .buffer_device_address(true);
        let mut shader_draw_parameters_features =
            vk::PhysicalDeviceShaderDrawParametersFeatures::default()
                .shader_draw_parameters(true);
        let mut descriptor_indexing_features =
            vk::PhysicalDeviceDescriptorIndexingFeaturesEXT::default()
                .descriptor_binding_variable_descriptor_count(true);
        let mut descriptor_buffer_features =
            vk::PhysicalDeviceDescriptorBufferFeaturesEXT::default()
                .descriptor_buffer(true);

        {
            let mut features = vk::PhysicalDeviceFeatures2KHR::default()
                .push_next(&mut dynamic_rendering_features)
                .push_next(&mut synchronization2_features)
                .push_next(&mut buffer_device_address_features)
                .push_next(&mut shader_draw_parameters_features)
                .push_next(&mut descriptor_indexing_features)
                .push_next(&mut descriptor_buffer_features);

            // Query physical device features
            unsafe {
                instance.get_physical_device_features2(*physical_device, &mut features);
            }
        }

        Self {
            dynamic_rendering_features,
            synchronization2_features,
            buffer_device_address_features,
            shader_draw_parameters_features,
            descriptor_indexing_features,
            descriptor_buffer_features,
        }
    }

    pub fn has_all(&self) -> bool {
        self.dynamic_rendering_features.dynamic_rendering == vk::TRUE
            && self.synchronization2_features.synchronization2 == vk::TRUE
            && self.buffer_device_address_features.buffer_device_address == vk::TRUE
            && self.shader_draw_parameters_features.shader_draw_parameters == vk::TRUE
            && self.descriptor_indexing_features.descriptor_binding_variable_descriptor_count == vk::TRUE
            && self.descriptor_buffer_features.descriptor_buffer == vk::TRUE
    }
}