use std::ffi::{CString, CStr};
use ash::vk;
use ash::prelude::VkResult;
use ash::extensions::ext::DebugUtils;
use ash::vk::DebugUtilsMessageTypeFlagsEXT;
use ash::vk::{
    KhrGetPhysicalDeviceProperties2Fn, KhrPortabilityEnumerationFn,
};

use super::Window;
use super::VkDebugReporter;

/// Create one of these at startup. It never gets lost/destroyed.
pub struct VkInstance {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub debug_reporter: Option<VkDebugReporter>,
}

#[derive(Debug)]
pub enum VkCreateInstanceError {
    LoadingError(ash::LoadingError),
    InstanceError(vk::Result),
    VkError(vk::Result),
}

impl std::error::Error for VkCreateInstanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            VkCreateInstanceError::LoadingError(ref e) => Some(e),
            VkCreateInstanceError::InstanceError(ref e) => Some(e),
            VkCreateInstanceError::VkError(ref e) => Some(e),
        }
    }
}

impl core::fmt::Display for VkCreateInstanceError {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        match *self {
            VkCreateInstanceError::LoadingError(ref e) => e.fmt(fmt),
            VkCreateInstanceError::InstanceError(ref e) => e.fmt(fmt),
            VkCreateInstanceError::VkError(ref e) => e.fmt(fmt),
        }
    }
}

impl From<ash::LoadingError> for VkCreateInstanceError {
    fn from(result: ash::LoadingError) -> Self {
        VkCreateInstanceError::LoadingError(result)
    }
}

// impl From<ash::InstanceError> for VkCreateInstanceError {
//     fn from(result: ash::InstanceError) -> Self {
//         VkCreateInstanceError::InstanceError(result)
//     }
// }

impl From<vk::Result> for VkCreateInstanceError {
    fn from(result: vk::Result) -> Self {
        VkCreateInstanceError::VkError(result)
    }
}

impl VkInstance {
    /// Creates a vulkan instance.
    pub fn new(
        window: &dyn Window,
        app_name: &CString,
        validation_layer_debug_report_flags: vk::DebugUtilsMessageSeverityFlagsEXT,
    ) -> Result<VkInstance, VkCreateInstanceError> {

        // This loads the dll/so if needed
        info!("Finding vulkan entry point");
        // let entry = unsafe{ ash::Entry::load()? };
        let entry = ash::Entry::linked();

        // Determine the supported version of vulkan that's available
        let vulkan_version = match entry.try_enumerate_instance_version()? {
            // Vulkan 1.1+
            Some(version) => {
                let major = vk::api_version_major(version);
                let minor = vk::api_version_minor(version);
                let patch = vk::api_version_patch(version);

                (major, minor, patch)
            }
            // Vulkan 1.0
            None => (1, 0, 0),
        };

        info!("Found Vulkan version: {:?}", vulkan_version);

        // Get the available layers/extensions
        let layers = entry.enumerate_instance_layer_properties()?;
        debug!("Available Layers: {:#?}", layers);
        let extensions = entry.enumerate_instance_extension_properties(None)?;
        debug!("Available Extensions: {:#?}", extensions);

        // Expected to be 1.1.0 or 1.0.0 depeneding on what we found in try_enumerate_instance_version
        // https://vulkan.lunarg.com/doc/view/1.1.70.1/windows/tutorial/html/16-vulkan_1_1_changes.html
        let api_version = vk::make_api_version(0, vulkan_version.0, vulkan_version.1, 0);

        // Info that's exposed to the driver. In a real shipped product, this data might be used by
        // the driver to make specific adjustments to improve performance
        // https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkApplicationInfo.html
        let appinfo = vk::ApplicationInfo::default()
            .application_name(app_name)
            .application_version(0)
            .engine_name(app_name)
            .engine_version(0)
            .api_version(api_version);

        let mut layer_names = vec![];
        let mut extension_names:Vec<_> = window.extension_names().unwrap().to_vec().iter()
            .map(|x| unsafe{ CStr::from_ptr(*x) } ).collect();

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            extension_names.push(KhrPortabilityEnumerationFn::name());
            // Enabling this extension is a requirement when using `VK_KHR_portability_subset`
            extension_names.push(KhrGetPhysicalDeviceProperties2Fn::name());
        }

        let swapchain_extension_name = CString::new("VK_EXT_swapchain_colorspace").unwrap();
        if extensions.iter().any(|extension| unsafe {
            CStr::from_ptr(extension.extension_name.as_ptr()) == swapchain_extension_name.as_c_str()
        }) {
            extension_names.push(swapchain_extension_name.as_c_str());
        }

        if !validation_layer_debug_report_flags.is_empty() {
            // Find the best validation layer that's available
            let best_validation_layer = VkInstance::find_best_validation_layer(&layers);
            if best_validation_layer.is_none() {
                log::error!("Could not find an appropriate validation layer. Check that the vulkan SDK has been installed or disable validation.");
                return Err(vk::Result::ERROR_LAYER_NOT_PRESENT.into());
            }

            let debug_extension = DebugUtils::name();
            let has_debug_extension = extensions.iter().any(|extension| unsafe {
                debug_extension == CStr::from_ptr(extension.extension_name.as_ptr())
            });

            if has_debug_extension {
                if let Some(best_validation_layer) = best_validation_layer {
                    layer_names.push(best_validation_layer);
                    extension_names.push(DebugUtils::name());
                }
            }else{
                log::error!("Could not find the debug extension. Check that the vulkan SDK has been installed or disable validation.");
                return Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT.into());
            }
        };

        if log_enabled!(log::Level::Debug) {
            log::debug!("Using layers: {:?}", layer_names);
            log::debug!("Using extensions: {:?}", extension_names);
        }

        let layer_names: Vec<_> = layer_names.iter().map(|x| x.as_ptr()).collect();
        let extension_names: Vec<_> = extension_names.iter().map(|x| x.as_ptr()).collect();

        let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::default()
        };

        // Create the instance
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&appinfo)
            .enabled_layer_names(&layer_names)
            .enabled_extension_names(&extension_names)
            .flags(create_flags);

        info!("Creating vulkan instance");
        let instance: ash::Instance = unsafe { entry.create_instance(&create_info, None)? };

        // Setup the debug callback for the validation layer
        let debug_reporter = if !validation_layer_debug_report_flags.is_empty() {
            Some(Self::setup_vulkan_debug_callback(
                &entry,
                &instance,
                validation_layer_debug_report_flags,
            )?)
        } else {
            None
        };

        Ok(VkInstance {
            entry,
            instance,
            debug_reporter,
        })
    }

    fn find_best_validation_layer(layers: &[ash::vk::LayerProperties]) -> Option<&'static CStr> {
        fn khronos_validation_layer_name() -> &'static CStr {
            CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0")
                .expect("Wrong extension string")
        }

        fn lunarg_validation_layer_name() -> &'static CStr {
            CStr::from_bytes_with_nul(b"VK_LAYER_LUNARG_standard_validation\0")
                .expect("Wrong extension string")
        }

        let khronos_validation_layer_name = khronos_validation_layer_name();
        let lunarg_validation_layer_name = lunarg_validation_layer_name();

        // Find the best validation layer that's available
        let mut best_available_layer = None;
        for layer in layers {
            let layer_name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };

            if layer_name == khronos_validation_layer_name {
                best_available_layer = Some(khronos_validation_layer_name);
                break;
            }

            if layer_name == lunarg_validation_layer_name {
                best_available_layer = Some(lunarg_validation_layer_name);
            }
        }

        best_available_layer
    }

    /// This is used to setup a debug callback for logging validation errors
    fn setup_vulkan_debug_callback(
        entry: &ash::Entry,
        instance: &ash::Instance,
        debug_report_flags: vk::DebugUtilsMessageSeverityFlagsEXT,
    ) -> VkResult<VkDebugReporter> {
        log::info!("Seting up vulkan debug callback");

        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(debug_report_flags)
            .message_type(
                DebugUtilsMessageTypeFlagsEXT::GENERAL |
                DebugUtilsMessageTypeFlagsEXT::VALIDATION |
                DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            )
            .pfn_user_callback(Some(super::debug_reporter::vulkan_debug_callback));

        let debug_report_loader = ash::extensions::ext::DebugUtils::new(&entry, instance);
        let debug_callback =
            unsafe { debug_report_loader.create_debug_utils_messenger(&debug_info, None)? };

        Ok(VkDebugReporter {
            debug_report_loader,
            debug_callback,
        })
    }
}



impl Drop for VkInstance {
    fn drop(&mut self) {
        debug!("destroying VkInstance");
        std::mem::drop(self.debug_reporter.take());

        unsafe {
            self.instance.destroy_instance(None);
        }

        debug!("destroyed VkInstance");
    }
}
