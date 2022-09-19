use chrono::Local;

use crate::db_internal::{clock_getTimestamp, NativeDateTime, clock_timestampToDatetime};

/// Get the current console time
pub fn get_time() -> chrono::DateTime<Local> {
    unsafe {
        let ts = clock_getTimestamp();
        let mut dt = NativeDateTime {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0
        };
        clock_timestampToDatetime(ts, &mut dt);
        return NativeDateTime::to_chrono(dt);
    }
}