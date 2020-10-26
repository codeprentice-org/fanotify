use super::libc::*;

use bitflags::bitflags;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum FanotifyNotificationClass {
    PreContent = FAN_CLASS_PRE_CONTENT as u32,
    Content = FAN_CLASS_CONTENT as u32,
    Notify = FAN_CLASS_NOTIF as u32,
}

impl Default for FanotifyNotificationClass {
    fn default() -> Self {
        Self::Notify
    }
}

bitflags! {
    #[derive(Default)]
    pub struct FanotifyFlags: u32 {
        const CLOSE_ON_EXEC = FAN_CLOEXEC;
        const NON_BLOCKING = FAN_NONBLOCK;
        const UNLIMITED_QUEUE = FAN_UNLIMITED_QUEUE;
        const UNLIMITED_MARKS = FAN_UNLIMITED_MARKS;
        const REPORT_TID = FAN_REPORT_TID;
        const REPORT_FID = FAN_REPORT_FID;
        const REPORT_DIR_FID = FAN_REPORT_DIR_FID;
        const REPORT_NAME = FAN_REPORT_NAME;
    }
}

impl FanotifyFlags {
    pub fn unlimited(self) -> Self {
        self | Self::UNLIMITED_QUEUE | Self::UNLIMITED_MARKS
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum FanotifyReadWrite {
    Read = libc::O_RDONLY as u32,
    Write = libc::O_WRONLY as u32,
    ReadWrite = libc::O_RDWR as u32,
}

impl Default for FanotifyReadWrite {
    fn default() -> Self {
        Self::Read
    }
}

bitflags! {
    #[derive(Default)]
    pub struct FanotifyEventFlags: u32 {
        const LARGE_FILE = libc::O_LARGEFILE as u32;
        const CLOSE_ON_EXEC = libc::O_CLOEXEC as u32;
        const APPEND = libc::O_APPEND as u32;
        const DATA_SYNC = libc::O_DSYNC as u32;
        const SYNC = libc::O_SYNC as u32;
        const NO_UPDATE_ACCESS_TIME = libc::O_NOATIME as u32;
        const NON_BLOCKING = libc::O_NONBLOCK as u32;
    }
}
