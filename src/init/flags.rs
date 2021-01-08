use bitflags::bitflags;

use crate::libc::init::flag;

bitflags! {
    pub struct Flags: u32 {
        const CLOSE_ON_EXEC = flag::FAN_CLOEXEC;
        const NON_BLOCKING = flag::FAN_NONBLOCK;
        const UNLIMITED_QUEUE = flag::FAN_UNLIMITED_QUEUE;
        const UNLIMITED_MARKS = flag::FAN_UNLIMITED_MARKS;
        const REPORT_TID = flag::FAN_REPORT_TID;
        const REPORT_FID = flag::FAN_REPORT_FID;
        const REPORT_DIR_FID = flag::FAN_REPORT_DIR_FID;
        const REPORT_NAME = flag::FAN_REPORT_NAME;
    }
}

impl Flags {
    pub const fn const_default() -> Self {
        Self::empty()
    }
    
    pub const fn unlimited() -> Self {
        Self::from_bits_truncate(Self::UNLIMITED_QUEUE.bits | Self::UNLIMITED_MARKS.bits)
    }
}

impl Default for Flags {
    fn default() -> Self {
        Self::const_default()
    }
}
