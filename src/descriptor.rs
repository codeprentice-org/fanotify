use crate::flags::init::{Init, RawInit};
use crate::flags::mark::Mark;
use crate::util::{libc_call, libc_void_call};
use libc::{fanotify_init, fanotify_mark};
use nix::errno::Errno;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fanotify(RawFd);

impl Drop for Fanotify {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

impl AsRawFd for Fanotify {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

impl IntoRawFd for Fanotify {
    fn into_raw_fd(self) -> RawFd {
        self.0
    }
}

impl FromRawFd for Fanotify {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(fd)
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum InitError {
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

impl RawInit {
    pub fn run(&self) -> Result<Fanotify, InitError> {
        use Errno::*;
        use InitError::*;
        libc_call(|| unsafe { fanotify_init(self.flags, self.event_flags) })
            .map(Fanotify)
            .map_err(|errno| match errno {
                EMFILE => ExceededFanotifyGroupPerProcessLimit,
                ENFILE => ExceededOpenFileDescriptorPerProcessLimit,
                ENOMEM => OutOfMemory,
                EPERM => PermissionDenied,
                ENOSYS => Unsupported,
                // EINVAL => unreachable!(), // handled below
                _ => panic!(format!(
                    "unexpected error in fanotify_init({}, {}): {}",
                    self.flags,
                    self.event_flags,
                    errno.desc()
                )),
            })
    }
}

impl Init {
    pub fn run(&self) -> Result<Fanotify, InitError> {
        self.as_raw().run()
    }
}

#[derive(Error, Debug)]
pub enum MarkError {
    #[error("TODO")]
    TODO,
}

impl Fanotify {
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
