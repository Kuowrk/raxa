use std::sync::Arc;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures, Queue, QueueCreateInfo, QueueFlags};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::swapchain::Surface;
use vulkano::{sync, Version, VulkanLibrary};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::sync::GpuFuture;
use winit::event_loop::EventLoop;

/// Contains Vulkan objects like the instance, devices, and queues
pub struct RenderContext {
    pub instance: Arc<Instance>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub previous_frame_end: Option<Box<dyn sync::GpuFuture>>,
}

impl RenderContext {
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self> {
        let library = VulkanLibrary::new()?;

        let instance_extensions = Surface::required_extensions(event_loop)?;
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: instance_extensions,
                ..Default::default()
            },
        )?;

        let (
            physical_device,
            queue_family_index,
            device_extensions
        ) = Self::select_physical_device(event_loop, instance.clone())?;

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                queue_create_infos: vec![
                    QueueCreateInfo {
                        queue_family_index,
                        ..Default::default()
                    },
                ],
                enabled_extensions: device_extensions,
                enabled_features: DeviceFeatures {
                    dynamic_rendering: true,
                    ..DeviceFeatures::empty()
                },
                ..Default::default()
            },
        )?;

        // Only one queue was requested, so it should be the first and only one in the iterator
        let queue = queues.next().ok_or_eyre("No queues found")?;

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let previous_frame_end = Some(sync::now(device.clone()).boxed());

        Ok(Self {
            instance,
            device,
            queue,
            memory_allocator,
            command_buffer_allocator,
            previous_frame_end,
        })
    }

    fn select_physical_device(
        event_loop: &EventLoop<()>,
        instance: Arc<Instance>,
    ) -> Result<(Arc<PhysicalDevice>, u32, DeviceExtensions)> {
        let mut device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()?
            .filter(|p| {
                p.api_version() >= Version::V1_3 || p.supported_extensions().khr_dynamic_rendering
            })
            .filter(|p| {
                p.supported_extensions().contains(&device_extensions)
            })
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.intersects(QueueFlags::GRAPHICS | QueueFlags::COMPUTE)
                            && p.presentation_support(i as u32, event_loop).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| {
                match p.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                }
            })
            .ok_or_eyre("No suitable physical device found")?;

        // If the physical device does not support Vulkan 1.3, enable the `khr_dynamic_rendering`
        // extension manually. This extension is guaranteed to be supported by the physical device
        // because of the filtering above.
        if physical_device.api_version() < Version::V1_3 {
            device_extensions.khr_dynamic_rendering = true;
        }

        Ok((physical_device, queue_family_index, device_extensions))
    }
}
