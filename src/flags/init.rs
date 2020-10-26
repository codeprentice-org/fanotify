use super::libc::*;

use bitflags::bitflags;
use self::NotificationClass::{PreContent, Content, Notify};

use static_assertions::const_assert_eq;
use static_assertions::_core::hint::unreachable_unchecked;
use self::ReadWrite::{Write, Read, ReadAndWrite};

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
    ReadAndWrite = libc::O_RDWR as u32,
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

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct RawInit {
    pub(crate) flags: u32,
    pub(crate) event_flags: u32,
}

impl Init {
    pub fn flags(&self) -> u32 {
        self.notification_class as u32 | self.flags.bits()
    }

    pub fn event_flags(&self) -> u32 {
        self.rw as u32 | self.event_flags.bits()
    }

    pub fn as_raw(&self) -> RawInit {
        RawInit {
            flags: self.flags(),
            event_flags: self.event_flags(),
        }
    }
}

impl RawInit {
    pub fn notification_class(&self) -> NotificationClass {
        const_assert_eq!(PreContent as u32, 0b1000);
        const_assert_eq!(Content as u32, 0b0100);
        const_assert_eq!(Notify as u32, 0b0000);
        match (self.flags & 0b1111) >> 2 {
            0b10 => PreContent,
            0b01 => Content,
            0b00 => Notify,
            0b11 => unsafe { unreachable_unchecked() },
            _ => unsafe { unreachable_unchecked() }, // definitely can't happen
        }
    }

    pub fn flags(&self) -> Flags {
        let bits = self.flags & !0b1100;
        unsafe { Flags::from_bits_unchecked(bits) }
    }

    pub fn rw(&self) -> ReadWrite {
        const_assert_eq!(Read as u32, 0);
        const_assert_eq!(Write as u32, 1);
        const_assert_eq!(ReadAndWrite as u32, 2);
        match self.event_flags & 0b11 {
            0 => Read,
            1 => Write,
            2 => ReadAndWrite,
            3 => unreachable!(), // less sure of this
            _ => unsafe { unreachable_unchecked() }, // definitely can't happen
        }
    }

    pub fn event_flags(&self) -> EventFlags {
        let bits = self.event_flags & !0b11;
        unsafe { EventFlags::from_bits_unchecked(bits) }
    }

    pub fn undo_raw(&self) -> Init {
        Init {
            notification_class: self.notification_class(),
            flags: self.flags(),
            rw: self.rw(),
            event_flags: self.event_flags(),
        }
    }
}
