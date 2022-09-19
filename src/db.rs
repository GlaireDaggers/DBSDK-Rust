use std::{ffi::CString};
use crate::db_internal::db_log;

/// Prints a message to debug output
pub fn log(str: String) {
    let cstr = CString::new(str.as_str()).expect("Failed creating C string");
    unsafe {
        db_log(cstr.as_ptr());
    }
}

/// Register custom DreamBox-specific panic handler
pub fn register_panic() {
    std::panic::set_hook(Box::new(|panic_info| {
        log(format!("FATAL ERROR: {}", panic_info));
    }));
}