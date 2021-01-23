use bitflags::bitflags;

use crate::libc::mark::mask;

// TODO find better names for some of these
bitflags! {
    pub struct Mask: u64 {
        /// ACCESS is masked only upon reading
        const ACCESS = mask::FAN_ACCESS;
        const OPEN = mask::FAN_OPEN;
        const OPEN_EXEC = mask::FAN_OPEN_EXEC;
        const CLOSE_NO_WRITE = mask::FAN_CLOSE_NOWRITE;
        const CLOSE_WRITE = mask::FAN_CLOSE_WRITE;
        const MODIFY = mask::FAN_MODIFY;

        const ATTRIBUTE_CHANGED = mask::FAN_ATTRIB;

        const CREATE = mask::FAN_CREATE;
        const DELETE = mask::FAN_DELETE;
        const DELETE_SELF = mask::FAN_DELETE_SELF;
        const MOVED_FROM = mask::FAN_MOVED_FROM;
        const MOVED_TO = mask::FAN_MOVED_TO;
        const MOVE_SELF = mask::FAN_MOVE_SELF;

        const ACCESS_PERMISSION = mask::FAN_ACCESS_PERM;
        const OPEN_PERMISSION = mask::FAN_OPEN_PERM;
        const OPEN_EXEC_PERMISSION = mask::FAN_OPEN_EXEC_PERM;

        const ON_DIR = mask::FAN_ONDIR;
        const EVENT_ON_CHILD = mask::FAN_EVENT_ON_CHILD;
    }
}

impl Mask {
    // combined flags
    
    pub const fn close() -> Self {
        Self::from_bits_truncate(0
            | Self::CLOSE_NO_WRITE.bits
            | Self::CLOSE_WRITE.bits
        )
    }
    
    pub const fn moved() -> Self {
        Self::from_bits_truncate(0
            | Self::MOVED_FROM.bits
            | Self::MOVED_TO.bits
        )
    }
    
    pub const fn all_permissions() -> Self {
        Self::from_bits_truncate(0
            | Self::ACCESS_PERMISSION.bits
            | Self::OPEN_PERMISSION.bits
            | Self::OPEN_EXEC_PERMISSION.bits
        )
    }
    
    pub const fn includes_permission(&self) -> bool {
        self.contains(Self::all_permissions())
    }
    
    pub const fn path_changed(&self) -> Self {
        Self::from_bits_truncate(0
            | Self::ACCESS.bits
            | Self::OPEN.bits
            | Self::OPEN_EXEC.bits
            | Self::CLOSE_NO_WRITE.bits
            | Self::CLOSE_WRITE.bits
            | Self::MODIFY.bits
        )
    }
    
    pub const fn used(&self) -> Self {
        Self::from_bits_truncate(0
            | Self::CREATE.bits
            | Self::DELETE.bits
            | Self::DELETE_SELF.bits
            | Self::moved().bits
            | Self::MOVE_SELF.bits
        )
    }
}