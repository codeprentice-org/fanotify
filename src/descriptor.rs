use super::flags::init::{Init, RawInit, Flags, NotificationClass::Notify};
use super::flags::mark::{Mark, DirFd, MarkPath, MarkFlags, MarkAction::{Add, Remove}};
use super::util::{libc_call, libc_void_call, ImpossibleError};
use libc::{fanotify_init, fanotify_mark};
use nix::errno::Errno;
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};
use thiserror::Error;
use crate::flags::mark::{MarkOne, MarkMask, MarkOneAction};
use crate::flags::mark::MarkWhat::FileSystem;

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

        libc_call(|| unsafe { fanotify_init(self.flags, self.event_flags) })
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

#[test]
fn init_or_catches_unsupported() {
    let args = Init {
        flags: Flags::unlimited(),
        ..Default::default()
    };
    match args.run() {
        Ok(_fd) => {}
        Err(e) => {
            assert_eq!(e, InitError::FanotifyUnsupported);
        }
    }
}

#[derive(Error, Debug, Eq, PartialEq, Hash)]
pub enum RawMarkError {
    #[error("invalid argument specified")]
    InvalidArgument,
    #[error("bad dir fd specified")]
    BadDirFd,
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
            fanotify_mark(self.fd, raw.flags, raw.mask, raw.dir_fd, raw.path_ptr())
        }).map_err(|errno| match errno {
            EBADF => BadDirFd,
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

#[test]
fn init_and_mark() {
    let args = Init {
        flags: Flags::unlimited(),
        ..Default::default()
    };
    let fanotify = match args.run() {
        Ok(fanotify) => fanotify,
        Err(e) => {
            assert_eq!(e, InitError::FanotifyUnsupported);
            return;
        }
    };
    let mark = Mark::one(MarkOne {
        action: MarkOneAction::Add,
        what: FileSystem,
        flags: MarkFlags::empty(),
        mask: MarkMask::OPEN | MarkMask::close(),
        path: MarkPath::current_working_directory(),
    }).unwrap();
    fanotify.mark(mark).unwrap();
}
