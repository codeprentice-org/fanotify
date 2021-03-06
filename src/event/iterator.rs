use std::convert::TryInto;
use std::mem::size_of;
use std::os::unix::io::FromRawFd;

use nix::unistd::Pid;

use crate::fd::FD;
use crate::init;
use crate::libc::mark::mask::FAN_Q_OVERFLOW;
use crate::libc::read::FAN_NOFD;
use crate::libc::read::fanotify_event_info_fid;
use crate::libc::read::fanotify_event_info_header;
use crate::libc::read::fanotify_event_metadata;
use crate::libc::read::FANOTIFY_METADATA_VERSION;
use crate::mark;

use super::error::EventError;
use super::error::EventResult;
use super::error::TooShortError;
use super::event::Event;
use super::events::Events;
use super::file::fd::FileFD;
use super::file::fid::FileFID;
use super::file::fid::FileHandle;
use super::file::fid::FileSystemId;
use super::file::fid::InfoType;
use super::file::File;
use super::file::permission::FilePermission;
use super::id::EventId;
use super::id::Id;
use super::iterator_ext::IntoEvents;

/// A consuming [`Iterator`] over [`Events`].
pub struct EventIterator<'a> {
    events: Events<'a>,
    read_index: usize,
}

impl<'a> EventIterator<'a> {
    /// Like [`Self::next`] except it doesn't check if there is still more room in the events buffer
    /// so it returns a plain [`Result`] instead of an [`Option<Result>`].
    ///
    /// This is only called from [`next`](EventIterator::next) so it's safe.
    /// It's just used to avoid nesting the [`Option`] and [`Result`].
    fn next_unchecked(&mut self) -> EventResult<'a> {
        use EventError::*;
        use TooShortError::*;
        
        let remaining = &self.events.as_bytes()[self.read_index..];
        
        let too_short = |what: TooShortError, expected: usize| -> std::result::Result<(), EventError> {
            let found = remaining.len();
            if found < expected {
                Err(TooShort {
                    what,
                    found,
                    expected,
                })
            } else {
                Ok(())
            }
        };
        
        let event_len_size = size_of::<u32>();
        // in case we error here, we want read_index to be at the end,
        // so None is returned from next() next time
        self.read_index += event_len_size;
        too_short(EventLenField, event_len_size)?;
        self.read_index -= event_len_size;
        let ptr = remaining.as_ptr() as *const fanotify_event_metadata;
        let event = unsafe { &*ptr };
        let event_len = event.event_len as usize;
        self.read_index += event_len;
        too_short(FullEvent, event_len)?;
        too_short(BaseEvent, size_of::<fanotify_event_metadata>())?;
        if event.vers != FANOTIFY_METADATA_VERSION {
            return Err(WrongVersion);
        }
        
        let flags = self.events.fanotify().init.flags();
        
        if event.mask & FAN_Q_OVERFLOW != 0 {
            let has_unlimited_queue = flags.contains(init::Flags::UNLIMITED_QUEUE);
            return Err(if has_unlimited_queue {
                UnlimitedQueueButQueueStillOverflowed
            } else {
                QueueOverflowed
            });
        }
        // type annotated for IDE, since from_bits_truncate is generated
        let mask: mark::Mask = mark::Mask::from_bits_truncate(event.mask);
        
        let has_no_fd = event.fd == FAN_NOFD;
        let requested_fid = flags.contains(init::Flags::REPORT_FID);
        let received_fid = event_len > size_of::<fanotify_event_metadata>();
        let is_perm = mask.includes_permission();
        if requested_fid {
            if !received_fid {
                return Err(FidRequestedButNotReceived);
            } else {
                match (has_no_fd, is_perm) {
                    (true, true) => return Err(FidReturnedForPermissionEvent),
                    (false, false) => return Err(FidRequestedButNotReceived),
                    #[allow(clippy::identity_op)]
                    (true, false) => too_short(BaseAndFidEvent, 0
                        + size_of::<fanotify_event_metadata>()
                        + size_of::<fanotify_event_info_fid>(),
                    )?,
                    (false, true) => {}
                }
            }
        } else {
            if has_no_fd {
                return Err(QueueOverflowed);
            }
            if received_fid {
                return Err(FidNotRequestedButReceived);
            }
        }
        
        let raw_id = Pid::from_raw(event.pid);
        let own_id = self.events.id();
        let id = match own_id {
            Id::Pid(_) => Id::Pid(raw_id),
            Id::Tid(_) => Id::Tid(raw_id),
        };
        let id = EventId {
            is_generated_by_self: id == own_id,
            id,
        };
        
        let get_fd = || -> std::result::Result<FD, EventError> {
            let fd = unsafe { FD::from_raw_fd(event.fd) };
            if !fd.check() {
                return Err(InvalidFd { fd });
            }
            Ok(fd)
        };
        
        let file = if is_perm {
            File::Permission(FilePermission::new(get_fd()?, self.events.responses()))
        } else if received_fid {
            // already checked that we have enough bytes for this
            let remaining = &remaining[size_of::<fanotify_event_metadata>()..];
            let ptr = remaining.as_ptr() as *const fanotify_event_info_fid;
            let fid = unsafe { &*ptr };
            let info_type: InfoType = fid.hdr.info_type
                .try_into()
                .map_err(|info_type| InvalidFidInfoType { info_type })?;
            {
                let found = fid.hdr.len as usize;
                #[allow(clippy::identity_op)]
                    let expected = 0
                    + size_of::<fanotify_event_info_header>()
                    + size_of::<libc::fsid_t>();
                if found != expected {
                    return Err(TooShort {
                        what: FidEvent,
                        found,
                        expected,
                    });
                }
            }
            File::FID(FileFID {
                info_type,
                file_system_id: FileSystemId {
                    fsid: fid.fsid,
                },
                handle: FileHandle {
                    handle: &fid.handle,
                },
            })
        } else {
            File::FD(FileFD {
                fd: get_fd()?,
            })
        };
        
        let this = Event {
            mask,
            id,
            file,
        };
        Ok(this)
    }
}

impl<'a> Iterator for EventIterator<'a> {
    type Item = EventResult<'a>;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.events.as_bytes().len() <= self.read_index {
            None
        } else {
            Some(self.next_unchecked())
        }
    }
}

impl<'a> IntoIterator for Events<'a> {
    type Item = EventResult<'a>;
    type IntoIter = EventIterator<'a>;
    
    fn into_iter(self) -> Self::IntoIter {
        EventIterator { events: self, read_index: 0 }
    }
}

impl<'a> IntoEvents<'a> for Events<'a> {}