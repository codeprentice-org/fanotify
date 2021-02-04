use bitflags::bitflags;

use crate::libc::mark::flag;

bitflags! {
    pub struct Flags: u32 {
        /// DONT_FOLLOW refers to the [FAN_MARK_DONT_FOLLOW](flag::FAN_MARK_DONT_FOLLOW) flag
        const DONT_FOLLOW = flag::FAN_MARK_DONT_FOLLOW;
        /// ONLY_DIR refers to the [FAN_MARK_ONLYDIR](flag::FAN_MARK_ONLYDIR) flag
        const ONLY_DIR = flag::FAN_MARK_ONLYDIR;
        /// IGNORED_MASK refers to the [FAN_MARK_IGNORED_MASK](flag::FAN_MARK_IGNORED_MASK) flag
        const IGNORED_MASK = flag::FAN_MARK_IGNORED_MASK;
        /// IGNORED_SURVIVE_MODIFY refers to the [FAN_MARK_IGNORED_SURV_MODIFY](flag::FAN_MARK_IGNORED_SURV_MODIFY) flag
        const IGNORED_SURVIVE_MODIFY = flag::FAN_MARK_IGNORED_SURV_MODIFY;
    }
}

impl Flags {
    pub const fn const_default() -> Self {
        Self::empty()
    }
}

impl Default for Flags {
    fn default() -> Self {
        Self::const_default()
    }
}
