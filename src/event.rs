use std::{mem, slice};
use std::convert::{TryFrom, TryInto};
use std::os::unix::io::{AsRawFd, FromRawFd};

use nix::errno::Errno;
use nix::unistd::{getpid, gettid, Pid};
use thiserror::Error;

use crate::common::FD;
use crate::descriptor::Fanotify;
use crate::init;
use crate::libc::mark::mask::FAN_Q_OVERFLOW;
use crate::libc::read::{FAN_EVENT_INFO_TYPE_DFID, FAN_EVENT_INFO_TYPE_DFID_NAME, FAN_EVENT_INFO_TYPE_FID, FAN_NOFD, fanotify_event_file_handle, fanotify_event_info_fid, fanotify_event_info_header, fanotify_event_metadata, FANOTIFY_METADATA_VERSION};
use crate::libc::write::{FAN_ALLOW, FAN_DENY, fanotify_response, FAN_AUDIT};
use crate::mark::MarkMask;

use self::PermissionDecision::{Allow, Deny};
use to_trait::To;

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

pub struct EventFileFD {
    pub fd: FD,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct FileSystemId {
    fsid: libc::fsid_t,
}

#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum EventInfoType {
    Fid = FAN_EVENT_INFO_TYPE_FID,
    DFidName = FAN_EVENT_INFO_TYPE_DFID_NAME,
    DFid = FAN_EVENT_INFO_TYPE_DFID,
}

impl TryFrom<u8> for EventInfoType {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use EventInfoType::*;
        let this = match value {
            FAN_EVENT_INFO_TYPE_FID => Fid,
            FAN_EVENT_INFO_TYPE_DFID_NAME => DFidName,
            FAN_EVENT_INFO_TYPE_DFID => DFid,
            _ => return Err(value),
        };
        Ok(this)
    }
}

impl From<EventInfoType> for u8 {
    fn from(this: EventInfoType) -> Self {
        use EventInfoType::*;
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
    // TODO open_by_handle_at()
}

pub struct EventFileFID<'a> {
    pub info_type: EventInfoType,
    pub fsid: FileSystemId,
    pub handle: FileHandle<'a>,
}

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

pub struct EventFilePermission<'a> {
    pub fd: FD,
    pub decision: PermissionDecision,
    pub audit: bool,
    responses: &'a mut Vec<fanotify_response>,
    response_index: usize,
}

impl EventFilePermission<'_> {
    fn response(&self) -> fanotify_response {
        let audit = self.audit as u32 * FAN_AUDIT;
        fanotify_response {
            fd: self.fd.as_raw_fd(),
            response: self.decision.to::<u32>() | audit,
        }
    }
}

// make sure the permission write always goes through
impl Drop for EventFilePermission<'_> {
    fn drop(&mut self) {
        self.responses[self.response_index] = self.response();
    }
}

impl EventFilePermission<'_> {
    pub fn allow(&mut self) {
        self.decision = Allow;
    }

    pub fn deny(&mut self) {
        self.decision = Deny;
    }
}

pub enum EventFile<'a> {
    FD(EventFileFD),
    FID(EventFileFID<'a>),
    Permission(EventFilePermission<'a>),
}

pub struct Event<'a> {
    pub mask: MarkMask,
    pub id: EventId,
    pub file: EventFile<'a>,
}

pub struct Events<'a> {
    fanotify: &'a Fanotify,
    id: Id,
    reads: &'a Vec<u8>,
    read_index: usize,
    writes: &'a mut Vec<fanotify_response>,
    write_index: usize,
}

pub struct EventBuffer {
    reads: Vec<u8>,
    // variable length, so un-typed
    writes: Vec<fanotify_response>,
}

impl EventBuffer {
    pub const fn new() -> Self {
        Self {
            reads: Vec::new(),
            writes: Vec::new(),
        }
    }
}

impl Default for EventBuffer {
    fn default() -> Self {
        let mut this = Self::new();
        this.reads = Vec::with_capacity(4096);
        this
    }
}

impl<'a> Events<'a> {
    pub(crate) fn read(
        fanotify: &'a Fanotify,
        buffer: &'a mut EventBuffer,
    ) -> Result<Self, Errno> {
        let reads = &mut buffer.reads;
        let writes = &mut buffer.writes;

        reads.clear();
        writes.clear();

        // want to use this, but it's unstable
        // reads.spare_capacity_mut()
        let read_buffer = {
            let ptr = reads.as_mut_slice().as_mut_ptr();
            let len = reads.capacity() * mem::size_of::<u8>();
            unsafe { slice::from_raw_parts_mut(ptr, len) }
        };
        let bytes_read = fanotify.fd.read(read_buffer)?;
        unsafe { reads.set_len(bytes_read) };

        // id is read here for two reason
        // 1. it caches it for this set of events
        // 2. it ensures the id is correct, b/c if you read the id later,
        //    it could be different than when the read occurred
        let use_tid = fanotify.init.flags().contains(init::Flags::REPORT_TID);
        let id = Id::current(use_tid);

        let this = Self {
            fanotify,
            id,
            reads,
            read_index: 0,
            writes,
            write_index: 0,
        };
        Ok(this)
    }

    pub fn write(&mut self) -> Result<usize, Errno> {
        let raw_responses = {
            let ptr = self.writes.as_slice().as_ptr() as *const u8;
            let len = mem::size_of_val(self.writes.as_slice());
            unsafe { slice::from_raw_parts(ptr, len) }
        };
        let write_buffer = &raw_responses[self.write_index..];
        let bytes_written = self.fanotify.fd.write(write_buffer)?;
        assert!(bytes_written <= write_buffer.len());
        self.write_index += bytes_written;
        Ok(bytes_written)
    }

    fn write_bytes_remaining(&self) -> usize {
        mem::size_of_val(self.writes.as_slice()) - self.write_index
    }
}

impl Drop for Events<'_> {
    fn drop(&mut self) {
        while self.write_bytes_remaining() > 0 {
            self.write().expect(
                "Events::write() threw in Events::drop().  \
                To handle this, call Events::write() yourself first."
            );
        }
    }
}

#[derive(Error, Debug)]
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

#[derive(Error, Debug)]
pub enum EventError {
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

impl<'a> Events<'a> {
    fn next_unchecked(& mut self) -> Result<Event<'a>, EventError> {
        use WhatIsTooShort::*;
        use EventError::*;

        let remaining = &self.reads.as_slice()[self.read_index..];

        let too_short = |what: WhatIsTooShort, expected: usize| -> Result<(), EventError> {
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
        too_short(BaseEvent, mem::size_of::<fanotify_event_metadata>());
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
        let mask: MarkMask = MarkMask::from_bits_truncate(event.mask);

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
                        + mem::size_of::<fanotify_event_info_header>()
                        + mem::size_of::<libc::fsid_t>(),
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

        let get_fd = || -> Result<FD, EventError> {
            let fd = unsafe { FD::from_raw_fd(event.fd) };
            if !fd.check() {
                return Err(InvalidFd { fd });
            }
            Ok(fd)
        };

        let file = if is_perm {
            let file = EventFilePermission {
                fd: get_fd()?,
                decision: PermissionDecision::default(),
                audit: false,
                responses: self.writes,
                response_index: self.writes.len(),
            };
            self.writes.push(file.response());
            EventFile::Permission(file)
        } else {
            if received_fid {
                // already checked that we have enough bytes for this
                let remaining = &remaining[mem::size_of::<fanotify_event_metadata>()..];
                let ptr = remaining.as_ptr() as *const fanotify_event_info_fid;
                let fid = unsafe { &*ptr };
                let info_type: EventInfoType = fid.hdr.info_type
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
                EventFile::FID(EventFileFID {
                    info_type,
                    fsid: FileSystemId {
                        fsid: fid.fsid,
                    },
                    handle: FileHandle {
                        handle: &fid.handle,
                    },
                })
            } else {
                EventFile::FD(EventFileFD {
                    fd: get_fd()?,
                })
            }
        };

        let raw_id = Pid::from_raw(event.pid);
        let id = match self.id {
            Id::Pid(_) => Id::Pid(raw_id),
            Id::Tid(_) => Id::Tid(raw_id),
        };
        let id = EventId {
            is_generated_by_self: id == self.id,
            id,
        };

        let this = Event {
            mask,
            id,
            file,
        };
        Ok(this)
    }
}

impl<'a> Iterator for Events<'a> {
    type Item = Result<Event<'a>, EventError>;

    fn next(&mut self) -> Option<Self::Item> {
        let reads = self.reads.as_slice();
        if mem::size_of_val(reads) <= self.read_index {
            return None;
        } else {
            Some(self.next_unchecked())
        }
    }
}
