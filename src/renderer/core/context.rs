use std::ffi::{c_char, c_void, CStr, CString};
use ash::vk;
use ash::vk::QueueFlags;
use color_eyre::eyre::OptionExt;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use std::sync::Arc;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::event_loop::EventLoop;
use winit::window::Window;

/// Contains Vulkan objects
pub struct RenderContext {
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub physical_device: vk::PhysicalDevice,

    pub graphics_queue: Arc<vk::Queue>,
    pub compute_queue: Arc<vk::Queue>,
    pub transfer_queue: Arc<vk::Queue>,

    pub graphics_queue_family: u32,
    pub compute_queue_family: u32,
    pub transfer_queue_family: u32,

    pub surface: Option<Arc<vk::SurfaceKHR>>,
    pub surface_loader: Option<Arc<ash::khr::surface::Instance>>,

    entry: ash::Entry,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    debug_utils_loader: ash::ext::debug_utils::Instance,
}

impl RenderContext {
    const ENABLE_VALIDATION_LAYERS: bool = cfg!(debug_assertions);
    const REQUIRED_VALIDATION_LAYERS: &'static [&'static CStr] = &[
        CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap(),
    ];

    pub fn new(
        event_loop: &EventLoop<()>,
        window: Option<&Arc<Window>>,
    ) -> Result<Self> {
        let entry = ash::Entry::linked();

        let instance = Self::create_instance(&entry, event_loop)?;
        let (
            debug_utils_messenger,
            debug_utils_loader,
        ) = Self::create_debug_utils_messenger(&entry, &instance)?;
        let surface = if window.is_some() {
            Some(Self::create_surface(&entry, &instance, window.unwrap())?)
        } else {
            None
        };
        let (
            physical_device,
            graphics_queue_family,
            compute_queue_family,
            transfer_queue_family,
        ) = Self::select_physical_device(&instance, surface.as_ref())?;

        let (
            device,
            graphics_queue,
            compute_queue,
            transfer_queue,
        ) = Self::create_device(
            &instance,
            &physical_device,
            graphics_queue_family,
            compute_queue_family,
            transfer_queue_family,
        )?;

        let (
            surface,
            surface_loader,
        ) = if let Some((s, l)) = surface {
            (Some(Arc::new(s)), Some(Arc::new(l)))
        } else {
            (None, None)
        };

        Ok(Self {
            instance,
            device,
            physical_device,
            graphics_queue: Arc::new(graphics_queue),
            compute_queue: Arc::new(compute_queue),
            transfer_queue: Arc::new(transfer_queue),
            graphics_queue_family,
            compute_queue_family,
            transfer_queue_family,
            surface,
            surface_loader,
            entry,
            debug_utils_messenger,
            debug_utils_loader,
        })
    }

    fn create_instance(
        entry: &ash::Entry,
        event_loop: &EventLoop<()>,
    ) -> Result<ash::Instance> {
        if Self::ENABLE_VALIDATION_LAYERS {
            Self::check_validation_layers_supported(entry)?;
        }

        let application_info = vk::ApplicationInfo::default()
            .api_version(vk::API_VERSION_1_3);
        let enabled_layer_names = if Self::ENABLE_VALIDATION_LAYERS {
            Self::REQUIRED_VALIDATION_LAYERS
                .iter()
                .map(|layer| layer.as_ptr())
                .collect::<Vec<*const c_char>>()
        } else {
            Vec::new()
        };
        let enabled_extension_names = Self::get_required_instance_extensions(event_loop)?
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<*const c_char>>();
        let mut debug_info = debug_utils_messenger_create_info();
        let instance_info = vk::InstanceCreateInfo::default()
            .application_info(&application_info)
            .enabled_layer_names(&enabled_layer_names)
            .enabled_extension_names(&enabled_extension_names)
            .push_next(&mut debug_info);

        #[cfg(target_os = "macos")]
        let instance_info = instance_info
            .flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);

        Ok(unsafe {
            entry.create_instance(&instance_info, None)?
        })
    }

    fn create_debug_utils_messenger(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<(vk::DebugUtilsMessengerEXT, ash::ext::debug_utils::Instance)> {
        let debug_utils_loader = ash::ext::debug_utils::Instance::new(entry, instance);
        let debug_utils_info = debug_utils_messenger_create_info();
        let debug_utils_messenger = unsafe {
            debug_utils_loader.create_debug_utils_messenger(&debug_utils_info, None)?
        };
        Ok((debug_utils_messenger, debug_utils_loader))
    }

    fn create_surface(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &winit::window::Window,
    ) -> Result<(vk::SurfaceKHR, ash::khr::surface::Instance)> {
        let surface = unsafe {
            ash_window::create_surface(
                entry,
                instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )?
        };
        let surface_loader = ash::khr::surface::Instance::new(entry, instance);
        Ok((surface, surface_loader))
    }

    fn select_physical_device(
        instance: &ash::Instance,
        surface: Option<&(vk::SurfaceKHR, ash::khr::surface::Instance)>
    ) -> Result<(vk::PhysicalDevice, u32, u32, u32)> {
        let req_device_exts = Self::get_required_device_extensions();
        Ok(unsafe {
            instance
                .enumerate_physical_devices()?
                .into_iter()
                // Filter out devices that do not contain the required device extensions
                .filter(|device| {
                    let supported_extensions = unsafe {
                        instance.enumerate_device_extension_properties(*device)
                    }.map_or(Vec::new(), |exts| exts);

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
                    let props = unsafe {
                        instance.get_physical_device_queue_family_properties(device)
                    };

                    let graphics_queue_family_index = props
                        .iter()
                        .enumerate()
                        .position(|(i, q)| {
                            let supports_graphics = q.queue_flags.contains(QueueFlags::GRAPHICS);
                            if let Some((surface, surface_loader)) = surface {
                                let supports_present = unsafe {
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
                            q.queue_flags.contains(QueueFlags::COMPUTE)
                        });

                    let transfer_queue_family_index = props
                        .iter()
                        .position(|q| {
                            q.queue_flags.contains(QueueFlags::TRANSFER)
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
                .ok_or_eyre("No suitable physical device found")?
        })
    }

    fn create_device(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        graphics_queue_family: u32,
        compute_queue_family: u32,
        transfer_queue_family: u32,
    ) -> Result<(ash::Device, vk::Queue, vk::Queue, vk::Queue)> {
        let queue_priorities = [1.0];
        let queue_create_infos = [
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(graphics_queue_family)
                .queue_priorities(&queue_priorities),
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(compute_queue_family)
                .queue_priorities(&queue_priorities),
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(transfer_queue_family)
                .queue_priorities(&queue_priorities),
        ];
        let enabled_extension_names = Self::get_required_device_extensions()
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<*const c_char>>();
        let enabled_features = vk::PhysicalDeviceFeatures::default();

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&enabled_extension_names)
            .enabled_features(&enabled_features);

        let device = unsafe {
            instance.create_device(*physical_device, &device_create_info, None)?
        };

        let graphics_queue = unsafe {
            device.get_device_queue(graphics_queue_family, 0)
        };
        let compute_queue = unsafe {
            device.get_device_queue(compute_queue_family, 0)
        };
        let transfer_queue = unsafe {
            device.get_device_queue(transfer_queue_family, 0)
        };

        Ok((device, graphics_queue, compute_queue, transfer_queue))
    }

    fn get_required_instance_extensions(
        event_loop: &EventLoop<()>
    ) -> Result<Vec<&'static CStr>> {
        let mut exts = ash_window::enumerate_required_extensions(
            event_loop.display_handle()?.as_raw()
        )?
            .iter()
            .map(|ext| unsafe {
                CStr::from_ptr(*ext)
            })
            .collect::<Vec<_>>();

        if Self::ENABLE_VALIDATION_LAYERS {
            exts.push(ash::ext::debug_utils::NAME);
        }

        #[cfg(target_os = "macos")]
        {
            exts.push(ash::khr::portability_enumeration::NAME);
            exts.push(ash::khr::get_physical_device_properties2::NAME);
        }

        Ok(exts)
    }

    fn get_required_device_extensions() -> Vec<&'static CStr> {
        vec![
            ash::khr::swapchain::NAME,
            ash::khr::dynamic_rendering::NAME,

            #[cfg(target_os = "macos")]
            ash::khr::portability_subset::NAME,
        ]
    }

    fn check_validation_layers_supported(entry: &ash::Entry) -> Result<()> {
        let supported_layers = unsafe {
            entry
                .enumerate_instance_layer_properties()?
                .iter()
                .map(|props| {
                    props.layer_name_as_c_str()
                })
                .collect::<Result<Vec<_>>>()?
        };

        for layer in Self::REQUIRED_VALIDATION_LAYERS {
            if !supported_layers.contains(layer) {
                return Err(eyre!(
                    "Validation layer {:?} not supported",
                    layer
                ));
            }
        }

        Ok(())
    }
}
fn debug_utils_messenger_create_info(
) -> vk::DebugUtilsMessengerCreateInfoEXT {
    let message_severity = vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
        | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR;
    let message_type = vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE;
    vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(message_severity)
        .message_type(message_type)
        .pfn_user_callback(Some(debug_callback))
}

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let msg_type = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };
    let msg = CStr::from_ptr((*p_callback_data).p_message);
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            log::trace!("[Verbose]{} {:?}", msg_type, msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log::warn!("[Warning]{} {:?}", msg_type, msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            log::error!("[Error]{} {:?}", msg_type, msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log::info!("[Info]{} {:?}", msg_type, msg);
        }
        _ => {
            log::warn!("[Unknown]{} {:?}", msg_type, msg);
        }
    }

    vk::FALSE
}