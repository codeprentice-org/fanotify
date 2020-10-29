use std::fmt::{Debug, Display};
use std::fmt;

use bitflags::bitflags;
use static_assertions::_core::fmt::Formatter;
use static_assertions::const_assert_eq;

use super::libc::init::{flag, notification_class};

use self::NotificationClass::{Content, Notify, PreContent};
use self::ReadWrite::{Read, ReadAndWrite, Write};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum NotificationClass {
    PreContent = notification_class::FAN_CLASS_PRE_CONTENT,
    Content = notification_class::FAN_CLASS_CONTENT,
    Notify = notification_class::FAN_CLASS_NOTIF,
}

impl NotificationClass {
    pub const fn const_default() -> Self {
        Self::Notify
    }
}

impl Default for NotificationClass {
    fn default() -> Self {
        Self::const_default()
    }
}

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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum ReadWrite {
    Read = libc::O_RDONLY as u32,
    Write = libc::O_WRONLY as u32,
    ReadAndWrite = libc::O_RDWR as u32,
}

impl ReadWrite {
    pub const fn const_default() -> Self {
        Self::Read
    }
}

impl Default for ReadWrite {
    fn default() -> Self {
        Self::const_default()
    }
}

bitflags! {
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

impl EventFlags {
    pub const fn const_default() -> Self {
        Self::empty()
    }
}

impl Default for EventFlags {
    fn default() -> Self {
        Self::const_default()
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Init {
    pub notification_class: NotificationClass,
    pub flags: Flags,
    pub rw: ReadWrite,
    pub event_flags: EventFlags,
}

impl Init {
    pub const fn const_default() -> Self {
        Self {
            notification_class: NotificationClass::const_default(),
            flags: Flags::const_default(),
            rw: ReadWrite::const_default(),
            event_flags: EventFlags::const_default(),
        }
    }
}

impl Default for Init {
    fn default() -> Self {
        Self::const_default()
    }
}

impl Display for Init {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // delegate Debug impl to Init
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct RawInit {
    pub(crate) flags: u32,
    pub(crate) event_flags: u32,
}

impl Init {
    pub const fn flags(&self) -> u32 {
        self.notification_class as u32 | self.flags.bits()
    }

    pub const fn event_flags(&self) -> u32 {
        self.rw as u32 | self.event_flags.bits()
    }

    pub const fn as_raw(&self) -> RawInit {
        RawInit {
            flags: self.flags(),
            event_flags: self.event_flags(),
        }
    }
}

impl RawInit {
    pub const fn notification_class(&self) -> NotificationClass {
        const_assert_eq!(PreContent as u32, 0b1000);
        const_assert_eq!(Content as u32, 0b0100);
        const_assert_eq!(Notify as u32, 0b0000);

        const_assert_eq!(PreContent as u32, 2 << 2);
        const_assert_eq!(Content as u32, 1 << 2);
        const_assert_eq!(Notify as u32, 0 << 2);

        [
            Notify,
            Content,
            PreContent,
            Notify, // unsafe
        ][((self.flags & 0b1111) >> 2) as usize]
    }

    pub const fn flags(&self) -> Flags {
        let bits = self.flags & !0b1100;
        Flags::from_bits_truncate(bits)
    }

    pub const fn rw(&self) -> ReadWrite {
        const_assert_eq!(Read as u32, 0);
        const_assert_eq!(Write as u32, 1);
        const_assert_eq!(ReadAndWrite as u32, 2);
        [
            Read,
            Write,
            ReadAndWrite,
            Read, // unsafe
        ][(self.event_flags & 0b11) as usize]
    }

    pub const fn event_flags(&self) -> EventFlags {
        let bits = self.event_flags & !0b11;
        EventFlags::from_bits_truncate(bits)
    }

    pub const fn undo_raw(&self) -> Init {
        Init {
            notification_class: self.notification_class(),
            flags: self.flags(),
            rw: self.rw(),
            event_flags: self.event_flags(),
        }
    }
}

impl Display for RawInit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // delegate Debug impl to Init
        write!(f, "{}", self.undo_raw())
    }
}

#[cfg(test)]
mod tests {
    use crate::init::{Flags, Init};

    #[test]
    fn init_display_debug() {
        let args = Init {
            flags: Flags::unlimited() | Flags::REPORT_FID,
            ..Default::default()
        };
        assert_eq!(
            format!("{}", args.as_raw()),
            "Init { \
                notification_class: Notify, \
                flags: UNLIMITED_QUEUE | UNLIMITED_MARKS | REPORT_FID, \
                rw: Read, \
                event_flags: LARGE_FILE \
            }",
        );
    }
}
