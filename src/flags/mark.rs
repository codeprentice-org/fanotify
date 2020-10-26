use bitflags::bitflags;

use super::libc::*;
use self::MarkAction::Flush;
use self::StaticMarkError::EmptyMask;
use std::ffi::CString;
use std::os::raw::c_char;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MarkOneAction {
    Add,
    Remove,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum MarkAction {
    Add = FAN_MARK_ADD,
    Remove = FAN_MARK_REMOVE,
    Flush = FAN_MARK_FLUSH,
}

impl From<MarkOneAction> for MarkAction {
    fn from(it: MarkOneAction) -> Self {
        match it {
            MarkOneAction::Add => Self::Add,
            MarkOneAction::Remove => Self::Remove,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum MarkWhat {
    Inode = FAN_MARK_INODE,
    MountPoint = FAN_MARK_MOUNT,
    FileSystem = FAN_MARK_FILESYSTEM,
}

bitflags! {
    #[derive(Default)]
    pub struct MarkFlags: u32 {
        const DONT_FOLLOW = FAN_MARK_DONT_FOLLOW;
        const ONLY_DIR = FAN_MARK_ONLYDIR;
        const IGNORED_MAKS = FAN_MARK_IGNORED_MASK;
        const IGNORED_SURVIVE_MODIFY = FAN_MARK_IGNORED_SURV_MODIFY;
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct CombinedMarkFlags {
    action: MarkAction,
    what: MarkWhat,
    other: MarkFlags,
}

type RawMarkFlags = u32;

impl CombinedMarkFlags {
    pub fn as_raw(&self) -> RawMarkFlags {
        self.action as u32 | self.what as u32 | self.other.bits()
    }
}

bitflags! {
    pub struct MarkMask: u64 {
        const ACCESS = FAN_ACCESS;
        const MODIFY = FAN_MODIFY;
        const CLOSE_WRITE = FAN_CLOSE_WRITE;
        const CLOSE_NOWRITE = FAN_CLOSE_NOWRITE;
        const OPEN = FAN_OPEN;
        const OPEN_EXEC = FAN_OPEN_EXEC;
        const ATTRIB = FAN_ATTRIB;
        const CREATE = FAN_CREATE;
        const DELETE = FAN_DELETE;
        const DELETE_SELF = FAN_DELETE_SELF;
        const MOVED_FROM = FAN_MOVED_FROM;
        const MOVED_TO = FAN_MOVED_TO;
        const MOVE_SELF = FAN_MOVE_SELF;
        const OPEN_PERM = FAN_OPEN_PERM;
        const OPEN_EXEC_PERM = FAN_OPEN_EXEC_PERM;
        const ACCESS_PERM = FAN_ACCESS_PERM;
        const ON_DIR = FAN_ONDIR;
        const EVENT_ON_CHILD = FAN_EVENT_ON_CHILD;
    }
}

impl MarkMask {
    // combined flags

    pub fn close() -> Self {
        Self::CLOSE_WRITE | Self::CLOSE_NOWRITE
    }

    pub fn moved() -> Self {
        Self::MOVED_FROM | Self::MOVED_TO
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct MarkPath<'a> {
    dir_fd: RawFd,
    path: Option<&'a Path>,
}

impl<'a> MarkPath<'a> {
    pub fn current_working_directory() -> Self {
        Self {
            dir_fd: libc::AT_FDCWD,
            path: None,
        }
    }

    pub fn directory<P: AsRawFd>(dir: &'a P) -> Self {
        Self {
            dir_fd: dir.as_raw_fd(),
            path: None,
        }
    }

    pub fn relative_to<P: AsRawFd>(dir: &'a P, path: &'a Path) -> Self {
        Self {
            dir_fd: dir.as_raw_fd(),
            path: Some(path),
        }
    }

    pub fn absolute(path: &'a Path) -> Self {
        Self {
            dir_fd: 0 as RawFd, // ignored by fanotify_mark()
            path: Some(path),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct MarkOne<'a> {
    action: MarkOneAction,
    what: MarkWhat,
    flags: MarkFlags,
    mask: MarkMask,
    path: MarkPath<'a>,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Mark<'a> {
    flags: CombinedMarkFlags,
    mask: MarkMask,
    path: MarkPath<'a>,
}

#[derive(Error, Debug)]
pub enum StaticMarkError {
    #[error("mask must not be empty for add or remove")]
    EmptyMask,
}

impl<'a> Mark<'a> {
    pub fn one(mark: MarkOne<'a>) -> Result<Self, StaticMarkError> {
        let MarkOne {
            action,
            what,
            flags,
            mask,
            path,
        } = mark;
        if mask.is_empty() {
            return Err(EmptyMask);
        }
        let this = Self {
            flags: CombinedMarkFlags {
                action: action.into(),
                what,
                other: flags,
            },
            mask,
            path,
        };
        Ok(this)
    }

    pub fn flush(what: MarkWhat, path: MarkPath<'a>) -> Self {
        Self {
            flags: CombinedMarkFlags {
                action: Flush,
                what,
                other: MarkFlags::empty(),
            },
            mask: MarkMask::all(), // ignored, but empty is invalid on add/remove
            path,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct RawMark {
    pub(crate) flags: u32,
    pub(crate) mask: u64,
    pub(crate) dir_fd: RawFd,
    pub(crate) path: Option<CString>,
}

impl RawMark {
    pub fn path_ptr(&self) -> *const c_char {
        match &self.path {
            None => std::ptr::null(),
            Some(path) => path.as_ptr(),
        }
    }
}

impl<'a> Mark<'a> {
    pub fn to_raw(&self) -> RawMark {
        RawMark {
            flags: self.flags.as_raw(),
            mask: self.mask.bits(),
            dir_fd: self.path.dir_fd,
            path: self
                .path
                .path
                .map(|path| path.as_os_str().to_os_string().into_vec())
                .map(|bytes| unsafe {
                    // Path can't have null bytes so this is safe
                    CString::from_vec_unchecked(bytes)
                }),
        }
    }
}
