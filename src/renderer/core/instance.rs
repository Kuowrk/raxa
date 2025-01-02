use std::ffi::{c_char, c_void, CStr};
use ash::vk;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use std::sync::Arc;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;
use crate::renderer::core::target::RenderTarget;
use crate::renderer::core::device::RenderDevice;

/// Initializes Vulkan and keeps the Vulkan instance alive
pub struct RenderInstance<'a> {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    debug_utils_loader: ash::ext::debug_utils::Instance,
}

impl RenderInstance {
    const ENABLE_VALIDATION_LAYERS: bool = cfg!(debug_assertions);
    const REQUIRED_VALIDATION_LAYERS: &'static [&'static CStr] = unsafe { &[
        CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0")
    ] };

    pub fn new(
        window: Option<Arc<Window>>,
    ) -> Result<(Self, Option<RenderTarget>)> {
        let entry = ash::Entry::linked();

        let instance = Self::create_instance(&entry, window.as_ref())?;

        let (
            debug_utils_messenger,
            debug_utils_loader,
        ) = Self::create_debug_utils_messenger(&entry, &instance)?;

        let (
            surface,
            surface_loader
        ) = if let Some(window) = window.as_ref() {
            let (surface, surface_loader) = Self::create_surface(&entry, &instance, window)?;
            (Some(surface), Some(surface_loader))
        } else {
            (None, None)
        };

        let ctx = Self {
            instance,
            entry,
            debug_utils_messenger,
            debug_utils_loader,
        };

        let tgt = if let Some(window) = window {
            Some(RenderTarget::new(
                window,
                surface.unwrap(),
                surface_loader.unwrap(),
                &ctx)?
            )
        } else {
            None
        };

        Ok((
            ctx,
            tgt,
        ))
    }

    pub fn create_device(
        &self,
        tgt: Option<&RenderTarget>,
    ) -> Result<RenderDevice> {
        let (surface, surface_loader) = if let Some(tgt) = tgt {
            (Some(&tgt.surface), Some(&tgt.surface_loader))
        } else {
            (None, None)
        };
        RenderDevice::new(
            &self.instance,
            surface,
            surface_loader,
        )
    }

    fn create_instance(
        entry: &ash::Entry,
        window: Option<&Arc<Window>>,
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
        let enabled_extension_names = Self::get_required_instance_extensions(window)?
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
        window: &Window,
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

    fn get_required_instance_extensions(
        window: Option<&Arc<Window>>,
    ) -> Result<Vec<&'static CStr>> {
        let mut exts = if let Some(window) = window {
            ash_window::enumerate_required_extensions(
                window.display_handle()?.as_raw()
            )?
                .iter()
                .map(|ext| unsafe {
                    CStr::from_ptr(*ext)
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

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
) -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
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
    let msg = unsafe {
        CStr::from_ptr((*p_callback_data).p_message)
    };
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