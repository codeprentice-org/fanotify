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
