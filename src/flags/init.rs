use super::libc::*;

use bitflags::bitflags;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum NotificationClass {
    PreContent = FAN_CLASS_PRE_CONTENT as u32,
    Content = FAN_CLASS_CONTENT as u32,
    Notify = FAN_CLASS_NOTIF as u32,
}

impl Default for NotificationClass {
    fn default() -> Self {
        Self::Notify
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Flags: u32 {
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

impl Flags {
    pub fn unlimited(self) -> Self {
        self | Self::UNLIMITED_QUEUE | Self::UNLIMITED_MARKS
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum ReadWrite {
    Read = libc::O_RDONLY as u32,
    Write = libc::O_WRONLY as u32,
    ReadWrite = libc::O_RDWR as u32,
}

impl Default for ReadWrite {
    fn default() -> Self {
        Self::Read
    }
}

bitflags! {
    #[derive(Default)]
    pub struct EventFlags: u32 {
        const LARGE_FILE = libc::O_LARGEFILE as u32;
        const CLOSE_ON_EXEC = libc::O_CLOEXEC as u32;
        const APPEND = libc::O_APPEND as u32;
        const DATA_SYNC = libc::O_DSYNC as u32;
        const SYNC = libc::O_SYNC as u32;
        const NO_UPDATE_ACCESS_TIME = libc::O_NOATIME as u32;
        const NON_BLOCKING = libc::O_NONBLOCK as u32;
    }
}

#[derive(Debug, Default, Eq, PartialEq, Hash)]
pub struct Init {
    pub notification_class: NotificationClass,
    pub flags: Flags,
    pub rw: ReadWrite,
    pub event_flags: EventFlags,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct RawInit {
    pub(crate) flags: u32,
    pub(crate) event_flags: u32,
}

impl Init {
    pub fn flags(&self) -> u32 {
        let flags = self.notification_class as u32 | self.flags.bits();
        flags as c_uint
    }

    pub fn event_flags(&self) -> u32 {
        let flags = self.rw as u32 | self.event_flags.bits();
        flags as c_uint
    }

    pub fn as_raw(&self) -> RawInit {
        RawInit {
            flags: self.flags(),
            event_flags: self.event_flags(),
        }
    }
}
