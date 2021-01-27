use bitflags::bitflags;

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
