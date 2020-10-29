use std::{cmp, mem};
use std::os::raw::c_void;
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};

use nix::errno::Errno;
use thiserror::Error;

use crate::read::RawEvent;

use super::init::{Flags, Init, NotificationClass::Notify, RawInit};
use super::mark::{Mark, MarkAction::{Add, Remove}, MarkFlags};
use super::util::{ImpossibleError, libc_call, libc_void_call};

#[derive(Debug)]
pub struct Fanotify {
    fd: RawFd,
    init: RawInit,
}

impl Drop for Fanotify {
    fn drop(&mut self) {
        // Note that errors are ignored when closing a file descriptor. The
        // reason for this is that if an error occurs we don't actually know if
        // the file descriptor was closed or not, and if we retried (for
        // something like EINTR), we might close another valid file descriptor
        // opened after we closed ours.
        let _ = unsafe { libc::close(self.fd) };
    }
}

impl AsRawFd for Fanotify {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl IntoRawFd for Fanotify {
    fn into_raw_fd(self) -> RawFd {
        let fd = self.fd;
        mem::forget(self); // need to skip closing the fd
        fd
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
    #[error("the kernel does not support the fanotify_init() syscall")]
    FanotifyUnsupported,
    #[error("the kernel does not support a certain feature for fanotify_init()")]
    FeatureUnsupported,
}

impl RawInit {
    pub fn run(self) -> Result<Fanotify, InitError> {
        use Errno::*;
        use InitError::*;

        let init = self.undo_raw();
        if init.flags.contains(Flags::REPORT_FID) && init.notification_class != Notify {
            return Err(InvalidArgument);
        }

        libc_call(|| unsafe { libc::fanotify_init(self.flags, self.event_flags) })
            .map(|fd| Fanotify { fd, init: self })
            .map_err(|errno| match errno {
                EMFILE => ExceededFanotifyGroupPerProcessLimit,
                ENFILE => ExceededOpenFileDescriptorPerProcessLimit,
                ENOMEM => OutOfMemory,
                EPERM => PermissionDenied,
                ENOSYS => FanotifyUnsupported,
                // ruled out EINVAL for fully supported kernel
                // and ENOSYS is returned if fanotify_init() is not supported at all
                // so this must mean only certain features are supported,
                // like on WSL 2, where Flags::REPORT_FID results in an EINVAL
                EINVAL => FeatureUnsupported,
                _ => panic!("{}", ImpossibleError {
                    syscall: "fanotify_init",
                    args: format!(
                        "flags = {}, event_flags = {}; init = {}",
                        self.flags, self.event_flags,
                        self,
                    ),
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
pub enum RawMarkError {
    #[error("invalid argument specified")]
    InvalidArgument,
    #[error("bad dir fd specified: {}", .fd)]
    BadDirFd { fd: RawFd },
    #[error("not a directory, but {:?} specified", MarkFlags::ONLY_DIR)]
    NotADirectory,
    #[error("path does not exist")]
    PathDoesNotExist,
    #[error("path is on a filesystem that doesn't support fsid and {:?} has been specified", Flags::REPORT_FID)]
    PathDoesNotSupportFSID,
    #[error("path is on a filesystem that doesn't support the encoding of file handles and {:?} has been specified", Flags::REPORT_FID)]
    PathNotSupported,
    #[error("path resides on a subvolume that uses a different fsid than its root superblock")]
    PathUsesDifferentFSID,
    #[error("cannot remove mark that doesn't exist yet")]
    CannotRemoveNonExistentMark,
    #[error("exceeded the per-fanotify group mark limit of 8192 and {:?} was not specified", Flags::UNLIMITED_MARKS)]
    ExceededMarkLimit,
    #[error("kernel out of memory")]
    OutOfMemory,
    #[error("the kernel does not support a certain feature for fanotify_init()")]
    FeatureUnsupported,
}

#[derive(Error, Debug, Eq, PartialEq, Hash)]
#[error("{:?}: {:?}", .error, .mark)]
pub struct MarkError<'a> {
    pub error: RawMarkError,
    pub mark: Mark<'a>,
}

impl Fanotify {
    fn mark_raw_error(&self, mark: &Mark) -> Result<(), RawMarkError> {
        use RawMarkError::*;
        use Errno::*;
        let init = self.init.undo_raw();
        if mark.mask.includes_permission() && init.notification_class == Notify {
            // man page also says to include || init.flags & Flags::REPORT_FID,
            // but that requires init.notification_class == Notify itself
            return Err(InvalidArgument);
        }
        let raw = mark.to_raw();
        libc_void_call(|| unsafe {
            libc::fanotify_mark(self.fd, raw.flags, raw.mask, raw.dir_fd, raw.path_ptr())
        }).map_err(|errno| match errno {
            EBADF => BadDirFd { fd: mark.path.dir.as_raw_fd() },
            ENOTDIR => NotADirectory,
            ENOENT if mark.action == Add => PathDoesNotExist,
            ENODEV => PathDoesNotSupportFSID,
            EOPNOTSUPP => PathUsesDifferentFSID,
            ENOENT if mark.action == Remove => CannotRemoveNonExistentMark,
            ENOSPC => ExceededMarkLimit,
            ENOMEM => OutOfMemory,
            // ruled out EINVAL for fully supported kernel
            // and ENOSYS is returned if fanotify_init() is not supported at all
            // so this must mean only certain features are supported
            EINVAL => FeatureUnsupported,
            // ENOSYS should be caught be init
            ENOSYS | _ => panic!("{}", ImpossibleError {
                syscall: "fanotify_mark",
                args: format!(
                    "fd = {}, flags = {}, mask = {}, dir_fd = {}, path = {:?}; mark = {}",
                    self.fd, raw.flags, raw.mask, raw.dir_fd, raw.path_ptr(),
                    mark,
                ),
                errno,
            }),
        })
    }

    pub fn mark<'a>(&self, mark: Mark<'a>) -> Result<(), MarkError<'a>> {
        self.mark_raw_error(&mark)
            .map_err(|error| MarkError { error, mark })
    }
}

impl Fanotify {
    pub fn read_raw(&self, buf: &mut [u8]) -> Result<usize, Errno> {
        let len = cmp::min(buf.len(), libc::ssize_t::MAX as usize) as libc::size_t;
        let buf = buf.as_mut_ptr() as *mut c_void;
        let bytes_read = libc_call(|| unsafe { libc::read(self.fd, buf, len) })?;
        Ok(bytes_read as usize)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::mem;
    use std::path::Path;

    use static_assertions::_core::ptr::slice_from_raw_parts_mut;

    use crate::descriptor::{Fanotify, InitError};
    use crate::init::{Flags, Init};
    use crate::libc::read::fanotify_event_metadata;
    use crate::mark::{Mark, MarkFlags, MarkMask, MarkOne, MarkPath};
    use crate::mark::MarkOneAction::Add;
    use crate::mark::MarkWhat::MountPoint;

    const fn get_init() -> Init {
        Init {
            flags: Flags::unlimited(),
            ..Init::const_default()
        }
    }

    fn with_fanotify<F: FnOnce(Fanotify) -> Result<(), Box<dyn Error>>>(f: F) {
        match get_init().run() {
            Ok(fanotify) => f(fanotify).unwrap(),
            Err(e) => {
                assert_eq!(e, InitError::FanotifyUnsupported);
            }
        }
    }

    #[test]
    fn init_or_catches_unsupported() {
        with_fanotify(|_| Ok(()));
    }

    fn get_mark() -> Mark<'static> {
        Mark::one(MarkOne {
            action: Add,
            what: MountPoint,
            flags: MarkFlags::empty(),
            mask: MarkMask::OPEN | MarkMask::close(),
            path: MarkPath::absolute("/home"),
        }).unwrap()
    }

    #[test]
    fn init_and_mark() {
        with_fanotify(|fanotify| {
            fanotify.mark(get_mark())?;
            Ok(())
        });
    }

    #[test]
    fn init_mark_and_read() {
        with_fanotify(|fanotify| {
            fanotify.mark(get_mark())?;
            let mut buf = [fanotify_event_metadata::default(); 1];
            fanotify.read_raw(unsafe {
                &mut *slice_from_raw_parts_mut(
                    buf.as_mut_ptr() as *mut u8,
                    mem::size_of::<fanotify_event_metadata>() * buf.len(),
                )
            })?;
            let path = Path::new("/proc/self/fd")
                .join(buf[0].fd.to_string())
                .read_link()?;
            assert_eq!(format!("{}", path.display()), "");
            Ok(())
        });
    }
}
