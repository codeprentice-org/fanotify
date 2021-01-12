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
pub enum File<'a> {
    FD(FileFD),
    FID(FileFID<'a>),
    Permission(FilePermission<'a>),
}

impl<'a> File<'a> {
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
}
