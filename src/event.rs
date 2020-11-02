use std::{mem, slice};
use std::convert::{TryFrom, TryInto};
use std::ops::Range;
use std::os::unix::io::{AsRawFd, FromRawFd};

use nix::errno::Errno;
use nix::unistd::{getpid, gettid, Pid};
use static_assertions::const_assert;
use to_trait::To;

use super::{init, mark};
use super::common::FD;
use super::descriptor::Fanotify;
use super::libc::mark::mask::FAN_Q_OVERFLOW;
use super::libc::read::{FAN_EVENT_INFO_TYPE_DFID, FAN_EVENT_INFO_TYPE_DFID_NAME, FAN_EVENT_INFO_TYPE_FID, FAN_NOFD, fanotify_event_file_handle, fanotify_event_info_fid, fanotify_event_info_header, fanotify_event_metadata, FANOTIFY_METADATA_VERSION};
use super::libc::write::{FAN_ALLOW, FAN_AUDIT, FAN_DENY, fanotify_response};

use self::PermissionDecision::{Allow, Deny};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Id {
    Pid(Pid),
    Tid(Pid),
}

impl Id {
    pub fn pid(&self) -> Option<Pid> {
        match self {
            Self::Pid(pid) => Some(*pid),
            Self::Tid(_) => None,
        }
    }
    
    pub fn tid(&self) -> Option<Pid> {
        match self {
            Self::Pid(_) => None,
            Self::Tid(tid) => Some(*tid),
        }
    }
    
    pub fn current(use_tid: bool) -> Self {
        if use_tid {
            Self::Tid(gettid())
        } else {
            Self::Pid(getpid())
        }
    }
}

pub struct EventId {
    pub is_generated_by_self: bool,
    pub id: Id,
}

impl EventId {
    pub fn pid(&self) -> Option<Pid> {
        self.id.pid()
    }
    
    pub fn tid(&self) -> Option<Pid> {
        self.id.tid()
    }
}

pub struct FileFD {
    pub fd: FD,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct FileSystemId {
    fsid: libc::fsid_t,
}

#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum InfoType {
    Fid = FAN_EVENT_INFO_TYPE_FID,
    DFidName = FAN_EVENT_INFO_TYPE_DFID_NAME,
    DFid = FAN_EVENT_INFO_TYPE_DFID,
}

impl TryFrom<u8> for InfoType {
    type Error = u8;
    
    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        use InfoType::*;
        let this = match value {
            FAN_EVENT_INFO_TYPE_FID => Fid,
            FAN_EVENT_INFO_TYPE_DFID_NAME => DFidName,
            FAN_EVENT_INFO_TYPE_DFID => DFid,
            _ => return Err(value),
        };
        Ok(this)
    }
}

impl From<InfoType> for u8 {
    fn from(this: InfoType) -> Self {
        use InfoType::*;
        match this {
            Fid => FAN_EVENT_INFO_TYPE_FID,
            DFidName => FAN_EVENT_INFO_TYPE_DFID_NAME,
            DFid => FAN_EVENT_INFO_TYPE_DFID,
        }
    }
}

pub struct FileHandle<'a> {
    handle: &'a fanotify_event_file_handle,
}

impl FileHandle<'_> {
    pub fn open(&self) -> FD {
        todo!("{:p}", self.handle)
    }
}

pub struct FileFID<'a> {
    pub info_type: InfoType,
    pub fsid: FileSystemId,
    pub handle: FileHandle<'a>,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum PermissionDecision {
    Allow,
    Deny,
}

// in case the user forgets to make a permission decision,
// we want to allow by default so everything doesn't get blocked by default
impl Default for PermissionDecision {
    fn default() -> Self {
        Allow
    }
}

impl From<PermissionDecision> for u32 {
    fn from(this: PermissionDecision) -> Self {
        match this {
            Allow => FAN_ALLOW,
            Deny => FAN_DENY,
        }
    }
}

pub struct FilePermission<'a> {
    pub fd: FD,
    pub decision: PermissionDecision,
    pub audit: bool,
    response: &'a mut fanotify_response,
}

impl FilePermission<'_> {
    fn response(&self) -> fanotify_response {
        let audit = self.audit as u32 * FAN_AUDIT;
        fanotify_response {
            fd: self.fd.as_raw_fd(),
            response: self.decision.to::<u32>() | audit,
        }
    }
}

// make sure the permission write always goes through
impl Drop for FilePermission<'_> {
    fn drop(&mut self) {
        *self.response = self.response();
    }
}

impl FilePermission<'_> {
    pub fn allow(&mut self) {
        self.decision = Allow;
    }
    
    pub fn deny(&mut self) {
        self.decision = Deny;
    }
}

pub enum File<'a> {
    FD(FileFD),
    FID(FileFID<'a>),
    Permission(FilePermission<'a>),
}

impl<'a> File<'a> {
    pub fn fd(self) -> Option<FileFD> {
        match self {
            Self::FD(file) => Some(file),
            _ => None,
        }
    }
    
    pub fn fid(self) -> Option<FileFID<'a>> {
        match self {
            Self::FID(file) => Some(file),
            _ => None,
        }
    }
    
    pub fn permission(self) -> Option<FilePermission<'a>> {
        match self {
            Self::Permission(file) => Some(file),
            _ => None,
        }
    }
}

pub struct Event<'a> {
    pub mask: mark::Mask,
    pub id: EventId,
    pub file: File<'a>,
}

pub struct Events<'a> {
    fanotify: &'a Fanotify,
    id: Id,
    buffer: &'a mut Vec<u8>,
    read_index: usize,
    write_range: Range<usize>,
}

impl<'a> Events<'a> {
    pub(crate) fn read(
        fanotify: &'a Fanotify,
        buffer: &'a mut Vec<u8>,
    ) -> std::result::Result<Self, Errno> {
        buffer.clear();
        
        // want to use this, but it's unstable
        // reads.spare_capacity_mut()
        let read_buffer = {
            let ptr = buffer.as_mut_slice().as_mut_ptr();
            let len = buffer.capacity() * mem::size_of::<u8>();
            unsafe { slice::from_raw_parts_mut(ptr, len) }
        };
        let bytes_read = fanotify.fd.read(read_buffer)?;
        unsafe { buffer.set_len(bytes_read) };
        
        // id is read here for two reason
        // 1. it caches it for this set of events
        // 2. it ensures the id is correct, b/c if you read the id later,
        //    it could be different than when the read occurred
        let use_tid = fanotify.init.flags().contains(init::Flags::REPORT_TID);
        let id = Id::current(use_tid);
        
        let this = Self {
            fanotify,
            id,
            buffer,
            read_index: 0,
            write_range: (0..0),
        };
        Ok(this)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum WhatIsTooShort {
    #[error("u32 fanotify_event_metadata::event_len field")]
    EventLenField,
    #[error("full event according to fanotify_event_metadata::event_len")]
    FullEvent,
    #[error("fanotify_event_metadata struct")]
    BaseEvent,
    #[error("fanotify_event_metadata struct and the fanotify_event_info_fid struct")]
    BaseAndFidEvent,
    #[error("fanotify_event_info_fid struct")]
    FidEvent,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("the fanotify queue overflowed")]
    QueueOverflowed,
    #[error("the fanotify event has the wrong version so it can't be handled")]
    WrongVersion,
    #[error("the data read ({} bytes) is too short for a full event ({} bytes), specifically, the {}", .found, .expected, .what)]
    TooShort {
        what: WhatIsTooShort,
        found: usize,
        expected: usize,
    },
    #[error("the fanotify queue still overflowed even though {:?} was specified", init::Flags::UNLIMITED_QUEUE)]
    UnlimitedQueueButQueueStillOverflowed,
    #[error("{:?} requested but not received", init::Flags::REPORT_FID)]
    FidRequestedButNotReceived,
    #[error("{:?} not requested but received", init::Flags::REPORT_FID)]
    FidNotRequestedButReceived,
    #[error("a {:?} fanotify event was received for a permission event, meaning it lacks an fd for writing the permission", init::Flags::REPORT_FID)]
    FidReturnedForPermissionEvent,
    #[error("{:?} request but received an invalid or unknown info_type: {}", init::Flags::REPORT_FID, .info_type)]
    InvalidFidInfoType { info_type: u8 },
    #[error("received an invalid fd: {}", .fd)]
    InvalidFd { fd: FD },
}

pub type Result<'a> = std::result::Result<Event<'a>, Error>;

impl<'a> Events<'a> {
    /// Return the next &mut fanotify_response for writing the response to.
    /// This is a reference into self.buffer, which also contains the fanotify_event_metadatas.
    /// But since sizeof(fanotify_response) <= sizeof(fanotify_event_metadata),
    /// I can reuse this space to store the response.
    fn next_response(&mut self) -> &'a mut fanotify_response {
        const_assert!(mem::size_of::<fanotify_response>() <= mem::size_of::<fanotify_event_metadata>());
        let response = self.buffer
            .as_mut_slice()
            [self.write_range.end..]
            .as_mut_ptr()
            as *mut fanotify_response;
        let response = unsafe { &mut *response };
        self.write_range.end += mem::size_of_val(response);
        assert!(self.write_range.end <= self.read_index);
        response
    }
    
    fn next_unchecked(&mut self) -> Result<'a> {
        use WhatIsTooShort::*;
        use Error::*;
        
        let remaining = &self.buffer.as_slice()[self.read_index..];
        
        let too_short = |what: WhatIsTooShort, expected: usize| -> std::result::Result<(), Error> {
            let found = remaining.len();
            if found < expected {
                return Err(TooShort {
                    what,
                    found,
                    expected,
                });
            } else {
                Ok(())
            }
        };
        
        let event_len_size = mem::size_of::<u32>();
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
        too_short(BaseEvent, mem::size_of::<fanotify_event_metadata>())?;
        if event.vers != FANOTIFY_METADATA_VERSION {
            return Err(WrongVersion);
        }
        
        let flags = self.fanotify.init.flags();
        
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
        let requested_fid = self.fanotify.init.flags().contains(init::Flags::REPORT_FID);
        let received_fid = event_len > mem::size_of::<fanotify_event_metadata>();
        let is_perm = mask.includes_permission();
        if requested_fid {
            if !received_fid {
                return Err(FidRequestedButNotReceived);
            } else {
                match (has_no_fd, is_perm) {
                    (true, true) => return Err(FidReturnedForPermissionEvent),
                    (false, false) => return Err(FidRequestedButNotReceived),
                    (true, false) => too_short(BaseAndFidEvent, 0
                        + mem::size_of::<fanotify_event_metadata>()
                        + mem::size_of::<fanotify_event_info_fid>(),
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
        let id = match self.id {
            Id::Pid(_) => Id::Pid(raw_id),
            Id::Tid(_) => Id::Tid(raw_id),
        };
        let id = EventId {
            is_generated_by_self: id == self.id,
            id,
        };
        
        let get_fd = || -> std::result::Result<FD, Error> {
            let fd = unsafe { FD::from_raw_fd(event.fd) };
            if !fd.check() {
                return Err(InvalidFd { fd });
            }
            Ok(fd)
        };
        
        let file = if is_perm {
            let file = FilePermission {
                fd: get_fd()?,
                decision: PermissionDecision::default(),
                audit: false,
                response: self.next_response(),
            };
            File::Permission(file)
        } else {
            if received_fid {
                // already checked that we have enough bytes for this
                let remaining = &remaining[mem::size_of::<fanotify_event_metadata>()..];
                let ptr = remaining.as_ptr() as *const fanotify_event_info_fid;
                let fid = unsafe { &*ptr };
                let info_type: InfoType = fid.hdr.info_type
                    .try_into()
                    .map_err(|info_type| InvalidFidInfoType { info_type })?;
                {
                    let found = fid.hdr.len as usize;
                    let expected = 0 +
                        mem::size_of::<fanotify_event_info_header>()
                        + mem::size_of::<libc::fsid_t>();
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
                    fsid: FileSystemId {
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
            }
        };
        
        let this = Event {
            mask,
            id,
            file,
        };
        Ok(this)
    }
    
    pub fn next(&mut self) -> Option<Result<'a>> {
        if self.buffer.len() <= self.read_index {
            return None;
        } else {
            Some(self.next_unchecked())
        }
    }
    
    pub fn for_each<F: Fn(Result<'a>)>(&mut self, f: F) {
        while let Some(event) = self.next() {
            f(event);
        }
    }
}

impl Events<'_> {
    pub fn write(&mut self) -> std::result::Result<usize, Errno> {
        let buffer = &self.buffer.as_slice()[self.write_range.start..self.write_range.end];
        let bytes_written = self.fanotify.fd.write(buffer)?;
        assert!(bytes_written <= buffer.len());
        self.write_range.start += bytes_written;
        Ok(bytes_written)
    }
}

impl Drop for Events<'_> {
    fn drop(&mut self) {
        while !self.write_range.is_empty() {
            self.write().expect(
                "Events::write() threw in Events::drop().  \
                To handle this, call Events::write() yourself first."
            );
        }
    }
}
