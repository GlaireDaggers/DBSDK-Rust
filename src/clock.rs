use core::fmt::Display;

use crate::db_internal::{clock_getTimestamp, clock_timestampToDatetime};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Display for DateTime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}/{}/{} {:02}:{:02}:{:02}", self.month, self.day, self.year, self.hour, self.minute, self.second)
    }
}

/// Get the current console time
pub fn get_time() -> DateTime {
    unsafe {
        let ts = clock_getTimestamp();
        let mut dt = DateTime {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0
        };
        clock_timestampToDatetime(ts, &mut dt);
        return dt;
    }
}