use std::ffi::CStr;
use std::os::raw::{c_void};

use ash::extensions::ext::DebugUtils;
use ash::vk;

const ERRORS_TO_IGNORE: [&'static str; 0] = [];

/// Callback for vulkan validation layer logging
pub extern "system" fn vulkan_debug_callback(
    flags: vk::DebugUtilsMessageSeverityFlagsEXT,
    _: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> u32 {
    if !data.is_null() {
        let data_ptr = unsafe { &(*data) };
        if !data_ptr.p_message.is_null() {
            let msg_ptr = (*data_ptr).p_message;
            let msg = unsafe { CStr::from_ptr(msg_ptr) };
            if flags.intersects(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
                let mut ignored = false;
                for ignored_error in &ERRORS_TO_IGNORE {
                    if msg.to_string_lossy().contains(ignored_error) {
                        ignored = true;
                        break;
                    }
                }

                if !ignored {
                    log::error!("{:?}", msg);
                    //panic!();
                }
            } else if flags.intersects(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
                log::warn!("{:?}", msg);
            } else if flags.intersects(vk::DebugUtilsMessageSeverityFlagsEXT::INFO) {
                log::info!("{:?}", msg);
            } else {
                log::debug!("{:?}", msg);
            }
        } else {
            log::error!("Received null message pointer in vulkan_debug_callback");
        }
    } else {
        log::error!("Received null data pointer in vulkan_debug_callback");
    }

    vk::FALSE
}

/// Handles dropping vulkan debug reporting
pub struct VkDebugReporter {
    pub debug_report_loader: DebugUtils,
    pub debug_callback: vk::DebugUtilsMessengerEXT,
}

impl Drop for VkDebugReporter {
    fn drop(&mut self) {
        unsafe {
            log::trace!("destroying VkDebugReporter");
            self.debug_report_loader
                .destroy_debug_utils_messenger(self.debug_callback, None);
            log::trace!("destroyed VkDebugReporter");
        }
    }
}
