use std::io;
use std::path::PathBuf;

use apply::Apply;

use crate::event::file::fd::FileFD;
use crate::event::file::fid::FileFID;
use crate::event::file::permission::FilePermission;
use crate::fd::FD;

pub mod fd;
pub mod fid;
pub mod permission;

pub trait GetFD {
    fn fd(&self) -> &FD;
}

/// An enum of the different kinds of file events.
#[derive(Debug)]
pub enum File<'a> {
    FD(FileFD),
    FID(FileFID<'a>),
    Permission(FilePermission<'a>),
}

impl<'a> File<'a> {
    /// Get the name of the current file variant, `fd`, `fid`, or `permission`.
    pub fn variant_name(&self) -> &'static str {
        match self {
            Self::FD(_) => "fd",
            Self::FID(_) => "fid",
            Self::Permission(_) => "permission",
        }
    }
    
    /// Return the [`FD`](Self::FD) variant if it exists.
    pub fn fd(self) -> Option<FileFD> {
        match self {
            Self::FD(file) => Some(file),
            _ => None,
        }
    }
    
    /// Return the [`FID`](Self::FID) variant if it exists.
    pub fn fid(self) -> Option<FileFID<'a>> {
        match self {
            Self::FID(file) => Some(file),
            _ => None,
        }
    }
    
    /// Return the [`Permission`](Self::Permission) variant if it exists.
    pub fn permission(self) -> Option<FilePermission<'a>> {
        match self {
            Self::Permission(file) => Some(file),
            _ => None,
        }
    }
    
    /// Try to resolve the path of this file event, if it contains a way to resolve it.
    pub fn path(&self) -> Option<io::Result<PathBuf>> {
        match self {
            Self::FD(file) => file.fd(),
            Self::Permission(file) => file.fd(),
            _ => return None,
        }
            .path()
            .apply(Some)
    }
}
