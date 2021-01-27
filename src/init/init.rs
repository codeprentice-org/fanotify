use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use super::EventFlags;
use super::Flags;
use super::NotificationClass;
use super::ReadWrite;

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