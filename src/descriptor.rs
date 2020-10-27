use crate::flags::init::{Init, RawInit, Flags};
use crate::flags::mark::Mark;
use crate::util::{libc_call, libc_void_call, ImpossibleError};
use libc::{fanotify_init, fanotify_mark};
use nix::errno::Errno;
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};
use thiserror::Error;
use crate::flags::init::NotificationClass::Notify;

#[derive(Debug)]
pub struct Fanotify {
    fd: RawFd,
    init: RawInit,
}

impl Drop for Fanotify {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

impl AsRawFd for Fanotify {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl IntoRawFd for Fanotify {
    fn into_raw_fd(self) -> RawFd {
        self.fd
    }
}

// can't impl FromRawFd, but this provides equivalent functionality
impl Fanotify {
    pub unsafe fn from_raw_fd(fd: RawFd, init: RawInit) -> Self {
        Self { fd, init }
    }
}

#[derive(Error, Debug, Eq, PartialEq, Hash)]
pub enum InitError {
    #[error("invalid argument specified")]
    InvalidArgument,
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
    pub fn run(self) -> Result<Fanotify, InitError> {
        use Errno::*;
        use InitError::*;

        let init = self.undo_raw();
        if init.flags.contains(Flags::REPORT_FID) && init.notification_class != Notify {
            return Err(InvalidArgument);
        }

        libc_call(|| unsafe { fanotify_init(self.flags, self.event_flags) })
            .map(|fd| Fanotify { fd, init: self })
            .map_err(|errno| match errno {
                EMFILE => ExceededFanotifyGroupPerProcessLimit,
                ENFILE => ExceededOpenFileDescriptorPerProcessLimit,
                ENOMEM => OutOfMemory,
                EPERM => PermissionDenied,
                ENOSYS => Unsupported,
                EINVAL | _ => panic!("{}", ImpossibleError {
                    syscall: "fanotify_init",
                    args: format!("{}, {}", self.flags, self.event_flags),
                    errno,
                }),
            })
    }
}

impl Init {
    pub fn run(&self) -> Result<Fanotify, InitError> {
        self.as_raw().run()
    }
}

#[derive(Error, Debug, Eq, PartialEq, Hash)]
pub enum MarkError {
    #[error("invalid argument specified")]
    InvalidArgument,
    #[error("TODO")]
    PathDoesntSupportFsid,
}

impl Fanotify {
    pub fn mark(&self, mark: Mark) -> Result<(), MarkError> {
        use MarkError::*;
        use Errno::*;
        let init = self.init.undo_raw();
        if mark.mask.includes_permission() && init.notification_class == Notify {
            // man page also says to include || init.flags & Flags::REPORT_FID,
            // but that requires init.notification_class == Notify itself
            return Err(InvalidArgument);
        }
        let raw = mark.to_raw();
        libc_void_call(|| unsafe {
            fanotify_mark(self.fd, raw.flags, raw.mask, raw.dir_fd, raw.path_ptr())
        }).map_err(|errno| match errno {
            EBADF => InvalidArgument,
            EINVAL | _ => panic!("{}", ImpossibleError {
                syscall: "fanotify_mark",
                args: format!("{}, {}, {}, {}, {:?}",
                              self.fd, raw.flags, raw.mask, raw.dir_fd, raw.path_ptr()),
                errno,
            }),
        })
    }
}
