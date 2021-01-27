use std::convert::TryFrom;
use std::io;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::FromRawFd;
use std::os::unix::io::IntoRawFd;
use std::os::unix::io::RawFd;

use nix::errno::Errno;

use crate::event::buffer::EventBuffer;
use crate::event::events::Events;
use crate::fd::FD;
use crate::init;
use crate::init::Flags;
use crate::init::Init;
use crate::init::NotificationClass::Notify;
use crate::init::RawInit;
use crate::libc::call::SysCall;
use crate::mark;
use crate::mark::Action::Add;
use crate::mark::Action::Remove;
use crate::mark::FanotifyMark;
use crate::mark::Mark;
use crate::mark::Markable;

pub mod buffered_fanotify;
pub mod async_fanotify;

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
    ///
    /// # Safety
    /// See [`FromRawFd`].
    pub unsafe fn from_raw_fd(fd: RawFd, init: RawInit) -> Self {
        Self {
            fd: FD::from_raw_fd(fd),
            init,
        }
    }
}

impl Init {
    /// Create a [`Fanotify`] using the flags in this [`Init`].
    pub fn to_fanotify(&self) -> Result<Fanotify, init::Error> {
        use Errno::*;
        use init::Error::*;
        
        // check for argument errors
        if self.flags.contains(Flags::REPORT_FID) && self.notification_class != Notify {
            return Err(InvalidArgument);
        }
        
        self.call()
            .map_err(|error| match error.errno {
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
                _ => error.impossible(),
            })
            .and_then(|fd| if fd.check() { Ok(fd) } else { Err(InvalidFd { fd }) })
            .map(|fd| Fanotify { fd, init: self.as_raw() })
    }
}

impl TryFrom<Init> for Fanotify {
    type Error = init::Error;
    
    fn try_from(this: Init) -> Result<Self, Self::Error> {
        this.to_fanotify()
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
        FanotifyMark {
            fanotify: self,
            mark,
        }.call().map_err(|error| match error.errno {
            EBADF => BadDirFd { fd: error.raw_args.dir_fd },
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
            // ENOSYS is possible, but should be caught by init
            _ => error.impossible(),
        })
    }
}

impl Markable for Fanotify {
    fn mark<'a>(&self, mark: Mark<'a>) -> Result<(), mark::Error<'a>> {
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
