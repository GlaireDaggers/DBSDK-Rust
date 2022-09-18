use std::ffi::CString;
use crate::db_internal::db_log;

/// Prints a message to debug output
pub fn log(str: String) {
    let cstr = CString::new(str.as_str()).expect("Failed creating C string");
    unsafe {
        db_log(cstr.as_ptr());
    }
}