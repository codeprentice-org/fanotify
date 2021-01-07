use bitflags::bitflags;

use crate::libc::mark::flag;

bitflags! {
    pub struct Flags: u32 {
        const DONT_FOLLOW = flag::FAN_MARK_DONT_FOLLOW;
        const ONLY_DIR = flag::FAN_MARK_ONLYDIR;
        const IGNORED_MASK = flag::FAN_MARK_IGNORED_MASK;
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
