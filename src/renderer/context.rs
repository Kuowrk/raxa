use std::sync::Arc;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use vulkano::device::{DeviceExtensions, QueueFlags};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::swapchain::Surface;
use vulkano::{Version, VulkanLibrary};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use winit::event_loop::EventLoop;

/// Contains Vulkan objects like the instance, devices, and queues
pub struct RenderContext {
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
        let (physical_device, queue_family_index) = Self::select_physical_device(event_loop, &instance)?;

        Ok(Self {
        })
    }

    fn select_physical_device(
        event_loop: &EventLoop<()>,
        instance: &Instance,
    ) -> Result<(Arc<PhysicalDevice>, u32)> {
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
        // extension manually. This extension is guaranteed to supported by the physical device
        // because of the filtering above.
        if physical_device.api_version() < Version::V1_3 {
            device_extensions.khr_dynamic_rendering = true;
        }

        Ok((physical_device, queue_family_index))
    }
}
