use crate::flags::init::{
    FanotifyEventFlags, FanotifyFlags, FanotifyNotificationClass, FanotifyReadWrite,
};
use crate::flags::mark::Mark;
use crate::util::{libc_call, libc_void_call};
use libc::{fanotify_init, fanotify_mark};
use nix::errno::Errno;
use std::os::raw::c_uint;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FanotifyDescriptor(RawFd);

impl Drop for FanotifyDescriptor {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

impl AsRawFd for FanotifyDescriptor {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

impl IntoRawFd for FanotifyDescriptor {
    fn into_raw_fd(self) -> RawFd {
        self.0
    }
}

impl FromRawFd for FanotifyDescriptor {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(fd)
    }
}

#[derive(Debug, Default)]
pub struct FanotifyInit {
    pub notification_class: FanotifyNotificationClass,
    pub flags: FanotifyFlags,
    pub rw: FanotifyReadWrite,
    pub event_flags: FanotifyEventFlags,
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum FanotifyError {
    #[error("exceeded the per-process limit on fanotify groups")]
    ExceededFanotifyGroupPerProcessLimit,
    #[error("exceeded the per-process limit on open file descriptors")]
    ExceededOpenFileDescriptorPerProcessLimit,
    #[error("kernel out of memory")]
    OutOfMemory,
    #[error("user does not have the required CAP_SYS_ADMIN capability")]
    PermissionDenied,
    #[error("the kernel does not support fanotify_init()")]
    Unsupported,
}

impl FanotifyInit {
    pub fn flags(&self) -> c_uint {
        let flags = self.notification_class as u32 | self.flags.bits();
        flags as c_uint
    }

    pub fn event_flags(&self) -> c_uint {
        let flags = self.rw as u32 | self.event_flags.bits();
        flags as c_uint
    }

    pub fn run(&self) -> Result<FanotifyDescriptor, FanotifyError> {
        use Errno::*;
        use FanotifyError::*;
        let flags = self.flags();
        let event_flags = self.event_flags();
        libc_call(|| unsafe { fanotify_init(flags, event_flags) })
            .map(FanotifyDescriptor)
            .map_err(|errno| match errno {
                EMFILE => ExceededFanotifyGroupPerProcessLimit,
                ENFILE => ExceededOpenFileDescriptorPerProcessLimit,
                ENOMEM => OutOfMemory,
                EPERM => PermissionDenied,
                ENOSYS => Unsupported,
                // EINVAL => unreachable!(), // handled below
                _ => panic!(format!(
                    "unexpected error in fanotify_init({}, {}): {}",
                    flags,
                    event_flags,
                    errno.desc()
                )),
            })
    }
}

#[derive(Error, Debug)]
pub enum MarkError {
    #[error("TODO")]
    TODO,
}

impl FanotifyDescriptor {
    pub fn mark(&self, mark: Mark) -> Result<(), MarkError> {
        use MarkError::*;
        let raw = mark.to_raw();
        libc_void_call(|| unsafe {
            fanotify_mark(self.0, raw.flags, raw.mask, raw.dir_fd, raw.path_ptr())
        }).map_err(|errno| match errno {
            _ => TODO,
        })
    }
}
