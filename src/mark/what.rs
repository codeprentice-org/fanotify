use crate::libc::mark::what;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum What {
    Inode = what::FAN_MARK_INODE,
    MountPoint = what::FAN_MARK_MOUNT,
    FileSystem = what::FAN_MARK_FILESYSTEM,
}
