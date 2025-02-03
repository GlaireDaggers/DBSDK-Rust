use std::ffi::CString;
use crate::db_internal::db_log;

/// Print a formatted message to debug output
#[macro_export]
macro_rules! logfmt {
    ($($arg:tt)*) => (log(format!($($arg)*).as_str()));
}

/// Prints a message to debug output
pub fn log(str: &str) {
    let cstr = CString::new(str).expect("Failed creating C string");
    unsafe {
        db_log(cstr.as_ptr());
    }
}

/// Register custom DreamBox-specific panic handler
pub fn register_panic() {
    std::panic::set_hook(Box::new(|panic_info| {
        logfmt!("FATAL ERROR: {}", panic_info);
    }));
}