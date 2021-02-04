use bitflags::bitflags;

use crate::libc::mark::mask;

// TODO find better names for some of these
bitflags! {
    pub struct Mask: u64 {
        /// ACCESS is masked only upon reading. For more information, refer to [FAN_ACCESS](mask::FAN_ACCESS)
        const ACCESS = mask::FAN_ACCESS;
        /// OPEN refers to the [FAN_OPEN](mask::FAN_OPEN) flag
        const OPEN = mask::FAN_OPEN;
        /// OPEN_EXEC refers to the [FAN_OPEN_EXEC](mask::FAN_OPEN_EXEC) flag
        const OPEN_EXEC = mask::FAN_OPEN_EXEC;
        /// CLOSE_NO_WRITE refers to the [FAN_CLOSE_NOWRITE](mask::FAN_CLOSE_NOWRITE) flag
        const CLOSE_NO_WRITE = mask::FAN_CLOSE_NOWRITE;
        /// CLOSE_WRITE refers to the [FAN_CLOSE_WRITE](mask::FAN_CLOSE_WRITE) flag
        const CLOSE_WRITE = mask::FAN_CLOSE_WRITE;
        /// MODIFY refers to the [FAN_MODIFY](mask::FAN_MODIFY) flag
        const MODIFY = mask::FAN_MODIFY;

        /// ATTRIBUTE_CHANGED refers to the [FAN_ATTRIB](mask::FAN_ATTRIB) flag
        const ATTRIBUTE_CHANGED = mask::FAN_ATTRIB;

        /// CREATE refers to the [FAN_CREATE](mask::FAN_CREATE) flag
        const CREATE = mask::FAN_CREATE;
        /// DELETE refers to the [FAN_DELETE](mask::FAN_DELETE) flag
        const DELETE = mask::FAN_DELETE;
        /// DELETE_SELF refers to the [FAN_DELETE_SELF](mask::FAN_DELETE_SELF) flag
        const DELETE_SELF = mask::FAN_DELETE_SELF;
        /// MOVED_FROM refers to the [FAN_MOVED_FROM](mask::FAN_MOVED_FROM) flag
        const MOVED_FROM = mask::FAN_MOVED_FROM;
        /// MOVED_TO refers to the [FAN_MOVED_TO](mask::FAN_MOVED_TO) flag
        const MOVED_TO = mask::FAN_MOVED_TO;
        /// MOVED_SELF refers to the [FAN_MOVED_SELF](mask::FAN_MOVED_SELF) flag
        const MOVE_SELF = mask::FAN_MOVE_SELF;

        /// ACCESS_PERMISSION refers to the [FAN_ACCESS_PERM](mask::FAN_ACCESS_PERM) flag
        const ACCESS_PERMISSION = mask::FAN_ACCESS_PERM;
        /// OPEN_PERMISSION refers to the [FAN_OPEN_PERM](mask::FAN_OPEN_PERM) flag
        const OPEN_PERMISSION = mask::FAN_OPEN_PERM;
        /// OPEN_EXEC_PERMISSION refers to the [FAN_OPEN_EXEC_PERM](mask::FAN_OPEN_EXEC_PERM) flag
        const OPEN_EXEC_PERMISSION = mask::FAN_OPEN_EXEC_PERM;

        /// ON_DIR refers to the [FAN_ONDIR](mask::FAN_ONDIR) flag
        const ON_DIR = mask::FAN_ONDIR;
        /// EVENT_ON_CHILD refers to the [FAN_EVENT_ON_CHILD](mask::FAN_EVENT_ON_CHILD) flag
        const EVENT_ON_CHILD = mask::FAN_EVENT_ON_CHILD;
    }
}

#[allow(clippy::identity_op)]
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
