use std::convert::TryFrom;

use crate::{
    common::FD,
    libc::read::{
        FAN_EVENT_INFO_TYPE_DFID,
        FAN_EVENT_INFO_TYPE_DFID_NAME,
        FAN_EVENT_INFO_TYPE_FID,
        fanotify_event_file_handle,
    },
};

/// A filesystem id.  It uniquely represents any filesystem object.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct FileSystemId {
    pub(in super::super) fsid: libc::fsid_t,
}

/// TODO there can be multiple of these per event, so need to handle that
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

/// An opaque handle to a file.
/// This is like an absolute [`Path`](std::path::Path), except it is already resolved by the filesystem.
/// But unlike a [`RawFd`](std::os::unix::io::RawFd), it's not opened yet.
/// It can be opened by calling [`Self::open()`].
pub struct FileHandle<'a> {
    pub(in super::super) handle: &'a fanotify_event_file_handle,
}

impl FileHandle<'_> {
    /// Open the resolved file handle.
    /// Not implemented yet.
    pub fn open(&self) -> FD {
        todo!("{:p}", self.handle)
    }
}

/// A [`REPORT_FID`](init::Flags::REPORT_FID) file event.
/// Unlike a normal [`FileFD`] event, which contains an opened [`FD`],
/// it contains a [`FileSystemId`] and an unopened but resolved [`FileHandle`].
pub struct FileFID<'a> {
    pub info_type: InfoType,
    pub fsid: FileSystemId,
    pub handle: FileHandle<'a>,
}
