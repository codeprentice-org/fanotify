use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use nix::errno::Errno;

use super::{init, mark};
use super::common::FD;
use super::event::Events;
use super::init::{Flags, Init, NotificationClass::Notify, RawInit};
use super::mark::{Action::{Add, Remove}, Mark};
use super::util::{ImpossibleSysCallError, libc_call, libc_void_call};

#[derive(Debug)]
pub struct Fanotify {
    pub(crate) fd: FD,
    pub(crate) init: RawInit,
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
    pub unsafe fn from_raw_fd(fd: RawFd, init: RawInit) -> Self {
        Self {
            fd: FD::from_raw_fd(fd),
            init,
        }
    }
}

impl RawInit {
    pub fn run(self) -> Result<Fanotify, init::Error> {
        use Errno::*;
        use init::Error::*;
        
        let init = self.undo_raw();
        if init.flags.contains(Flags::REPORT_FID) && init.notification_class != Notify {
            return Err(InvalidArgument);
        }
        
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
    pub fn run(&self) -> Result<Fanotify, init::Error> {
        self.as_raw().run()
    }
}

impl Fanotify {
    fn mark_raw_error(&self, mark: &Mark) -> Result<(), mark::RawError> {
        use mark::RawError::*;
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
    
    pub fn mark<'a>(&self, mark: Mark<'a>) -> Result<(), mark::Error<'a>> {
        self.mark_raw_error(&mark)
            .map_err(|error| mark::Error { error, mark })
    }
}

impl Fanotify {
    pub fn read<'a>(&'a self, buffer: &'a mut Vec<u8>) -> Result<Events<'a>, Errno> {
        Events::read(self, buffer)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::mem;
    use std::path::Path;
    
    use static_assertions::_core::ptr::slice_from_raw_parts_mut;
    
    use crate::init::{Flags, Init};
    use crate::libc::read::fanotify_event_metadata;
    use crate::{mark, init};
    use crate::mark::Mark;
    use crate::mark::OneAction::Add;
    use crate::mark::What::MountPoint;
    use crate::descriptor::Fanotify;
    
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
                assert_eq!(e, init::Error::FanotifyUnsupported);
            }
        }
    }
    
    #[test]
    fn init_or_catches_unsupported() {
        with_fanotify(|_| Ok(()));
    }
    
    fn get_mark() -> Mark<'static> {
        Mark::one(mark::One {
            action: Add,
            what: MountPoint,
            flags: mark::Flags::empty(),
            mask: mark::Mask::OPEN | mark::Mask::close(),
            path: mark::Path::absolute("/home"),
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
            let mut buf = [fanotify_event_metadata {
                event_len: 0,
                vers: 0,
                reserved: 0,
                metadata_len: 0,
                mask: 0,
                fd: 0,
                pid: 0
            }; 1];
            fanotify.fd.read(unsafe {
                &mut *slice_from_raw_parts_mut(
                    buf.as_mut_ptr() as *mut u8,
                    mem::size_of::<fanotify_event_metadata>() * buf.len(),
                )
            })?;
            let path = Path::new("/proc/self/fd")
                .join(buf[0].fd.to_string())
                .read_link()?;
            assert_eq!(format!("{}", path.display()), "/usr/bin/ls");
            Ok(())
        });
    }
}
