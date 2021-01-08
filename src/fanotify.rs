use std::io;
/// Contains main syscalls and the main [`Fanotify`] struct.

use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use nix::errno::Errno;

use crate::mark::Mark;

use super::{
    common::FD,
    event::{
        buffer::EventBuffer,
        events::Events,
    },
    init,
    init::{Flags, Init, NotificationClass::Notify, RawInit},
    mark,
    mark::{Action::{Add, Remove}},
    util::{ImpossibleSysCallError, libc_call, libc_void_call},
};

/// The main [`Fanotify`] struct, the primary entry point to the fanotify API.
#[derive(Debug)]
pub struct Fanotify {
    /// The fanotify descriptor/group.
    pub(super) fd: FD,
    
    /// The flags used to initialize it.
    pub(super) init: RawInit,
}

impl AsRawFd for Fanotify {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl IntoRawFd for Fanotify {
    fn into_raw_fd(self) -> RawFd {
        self.fd.into_raw_fd()
    }
}

// can't impl FromRawFd, but this provides equivalent functionality
impl Fanotify {
    /// We can't `impl `[`FromRawFd`]` for `[`Fanotify`] because [`Fanotify`] also contains a [`RawInit`].
    /// Thus, we provide this analogous unsafe API for constructing a [`Fanotify`] from a [`RawFd`]
    /// and the corresponding [`RawInit`] flags used to create the [`RawFd`].
    pub unsafe fn from_raw_fd(fd: RawFd, init: RawInit) -> Self {
        Self {
            fd: FD::from_raw_fd(fd),
            init,
        }
    }
}

impl RawInit {
    /// Create a [`Fanotify`] using the flags in this [`RawInit`].
    pub fn run(self) -> Result<Fanotify, init::Error> {
        use Errno::*;
        use init::Error::*;
        
        // construct init object with raw init and check for argument errors
        let init = self.undo_raw();
        if init.flags.contains(Flags::REPORT_FID) && init.notification_class != Notify {
            return Err(InvalidArgument);
        }
        
        // Try to initialize Fanotify with flag then catch and return status
        libc_call(|| unsafe { libc::fanotify_init(self.flags, self.event_flags) })
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
                _ => panic!("{}", ImpossibleSysCallError {
                    syscall: "fanotify_init",
                    args: format!(
                        "flags = {}, event_flags = {}; init = {}",
                        self.flags, self.event_flags,
                        self,
                    ),
                    errno,
                }),
            })
            .map(|fd| unsafe { FD::from_raw_fd(fd) })
            .and_then(|fd| if fd.check() { Ok(fd) } else { Err(InvalidFd { fd }) })
            .map(|fd| Fanotify { fd, init: self })
    }
}

impl Init {
    /// Create a [`Fanotify`] using the flags in this [`Init`].
    pub fn run(&self) -> Result<Fanotify, init::Error> {
        self.as_raw().run()
    }
}

impl Fanotify {
    /// The main method that adds a [`Mark`], only it returns just a [`mark::RawError`].
    /// The below [`mark`](Fanotify::mark) function wraps this into a full [`mark::Error`].
    fn mark_raw_error(&self, mark: &Mark) -> Result<(), mark::RawError> {
        use crate::mark::RawError::*;
        use Errno::*;
        let init = self.init.undo_raw();
        if mark.mask.includes_permission() && init.notification_class == Notify {
            // man page also says to include || init.flags & Flags::REPORT_FID,
            // but that requires init.notification_class == Notify itself
            return Err(InvalidArgument);
        }
        let raw = mark.to_raw();
        libc_void_call(|| unsafe {
            libc::fanotify_mark(self.fd.as_raw_fd(), raw.flags, raw.mask, raw.dir_fd, raw.path_ptr())
        }).map_err(|errno| match errno {
            EBADF => BadDirFd { fd: raw.dir_fd },
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
            ENOSYS | _ => panic!("{}", ImpossibleSysCallError {
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
    
    /// Add a [`Mark`] to this [`Fanotify`] group.
    ///
    /// See [`Mark`] for more details.
    pub fn mark<'a>(&self, mark: Mark<'a>) -> Result<(), mark::Error<'a>> {
        self.mark_raw_error(&mark)
            .map_err(|error| mark::Error { error, mark })
    }
}

impl Fanotify {
    /// Read file events from this [`Fanotify`] group into the given buffer.
    ///
    /// Return an [`Events`] iterator over the individual events.
    ///
    /// This method blocks.
    pub fn read<'a>(&'a self, buffer: &'a mut EventBuffer) -> io::Result<Events<'a>> {
        let events = Events::read(self, buffer)?;
        Ok(events)
    }
}

#[cfg(test)]
mod tests {}
