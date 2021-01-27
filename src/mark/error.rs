use std::os::unix::io::RawFd;

use thiserror::Error;

use crate::init;

use super::Flags;
use super::Mark;

#[derive(Error, Debug, Eq, PartialEq, Hash)]
pub enum StaticError {
    #[error("mask must not be empty for add or remove")]
    EmptyMask,
}

#[derive(thiserror::Error, Debug, Eq, PartialEq, Hash)]
pub enum RawError {
    #[error("invalid argument specified")]
    InvalidArgument,
    #[error("bad dir fd specified: {}", .fd)]
    BadDirFd { fd: RawFd },
    #[error("not a directory, but {:?} specified", Flags::ONLY_DIR)]
    NotADirectory,
    #[error("path does not exist")]
    PathDoesNotExist,
    #[error("path is on a filesystem that doesn't support fsid and {:?} has been specified", init::Flags::REPORT_FID)]
    PathDoesNotSupportFSID,
    #[error("path is on a filesystem that doesn't support the encoding of file handles and {:?} has been specified", init::Flags::REPORT_FID)]
    PathNotSupported,
    #[error("path resides on a subvolume that uses a different fsid than its root superblock")]
    PathUsesDifferentFSID,
    #[error("cannot remove mark that doesn't exist yet")]
    CannotRemoveNonExistentMark,
    #[error("exceeded the per-fanotify group mark limit of 8192 and {:?} was not specified", init::Flags::UNLIMITED_MARKS)]
    ExceededMarkLimit,
    #[error("kernel out of memory")]
    OutOfMemory,
    #[error("the kernel does not support a certain feature for fanotify_init()")]
    FeatureUnsupported,
}

#[derive(thiserror::Error, Debug, Eq, PartialEq, Hash)]
#[error("{:?}: {:?}", .error, .mark)]
pub struct Error<'a> {
    pub error: RawError,
    pub mark: Mark<'a>,
}