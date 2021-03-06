use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::os::unix::io::FromRawFd;
use std::os::unix::io::RawFd;

use static_assertions::const_assert_eq;

use crate::fd::FD;
use crate::libc::call::{RawSysCall, SysCall};

use super::EventFlags;
use super::Flags;
use super::Init;
use super::NotificationClass;
use super::NotificationClass::Content;
use super::NotificationClass::Notify;
use super::NotificationClass::PreContent;
use super::ReadWrite;
use super::ReadWrite::Read;
use super::ReadWrite::ReadAndWrite;
use super::ReadWrite::Write;

/// [`Init`] flags compressed into the actual flags used in the [`libc::fanotify_init`] call.
/// You can seamlessly convert back and forth between an [`Init`] and a [`RawInit`].
/// A [`RawInit`] takes up less memory, but it field accessor operations are more involved than [`Init`]'s.
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
        
        // unsafe
        [
            Notify,
            Content,
            PreContent,
            Notify,
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
        // unsafe
        [
            Read,
            Write,
            ReadAndWrite,
            Read,
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

impl From<Init> for RawInit {
    fn from(init: Init) -> Self {
        init.as_raw()
    }
}

impl From<RawInit> for Init {
    fn from(raw_init: RawInit) -> Self {
        raw_init.undo_raw()
    }
}

impl Display for RawInit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // delegate Debug impl to Init
        write!(f, "{}", self.undo_raw())
    }
}

impl RawSysCall for RawInit {
    type Output = RawFd;
    
    fn name() -> &'static str {
        "fanotify_init"
    }
    
    unsafe fn unsafe_call(&self) -> Self::Output {
        libc::fanotify_init(self.flags, self.event_flags)
    }
}

impl SysCall for Init {
    type Raw = RawInit;
    type Output = FD;
    
    fn to_raw(&self) -> Self::Raw {
        self.as_raw()
    }
    
    fn convert_output(fd: RawFd) -> FD {
        unsafe { FD::from_raw_fd(fd) }
    }
}
