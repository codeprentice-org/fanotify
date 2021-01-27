use crate::fd::FD;
use crate::init;

use super::event::Event;

/// An error where the buffer for an [`Event`] field or struct is too short,
/// so the full field or struct cannot be read.
///
/// TODO document each error
#[derive(thiserror::Error, Debug)]
pub enum TooShortError {
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

/// An error from reading an [`Event`] from the buffer.
///
/// TODO document each error
#[derive(thiserror::Error, Debug)]
pub enum EventError {
    #[error("the fanotify queue overflowed")]
    QueueOverflowed,
    #[error("the fanotify event has the wrong version so it can't be handled")]
    WrongVersion,
    #[error("the data read ({} bytes) is too short for a full event ({} bytes), specifically, the {}", .found, .expected, .what)]
    TooShort {
        what: TooShortError,
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

pub type EventResult<'a> = Result<Event<'a>, EventError>;