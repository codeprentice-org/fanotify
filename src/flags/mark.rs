use bitflags::bitflags;

use super::libc::*;
use self::MarkAction::Flush;
use self::StaticMarkError::EmptyMask;
use std::ffi::CString;
use std::os::raw::c_char;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::{AsRawFd, RawFd, IntoRawFd, FromRawFd};
use std::path::Path;
use thiserror::Error;
use std::borrow::Cow;
use std::fmt::Display;
use static_assertions::_core::fmt::Formatter;
use std::fmt;
use static_assertions::_core::marker::PhantomData;

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
    pub(crate) action: MarkAction,
    pub(crate) what: MarkWhat,
    pub(crate) other: MarkFlags,
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

    pub fn includes_permission(&self) -> bool {
        [Self::OPEN_PERM, Self::OPEN_EXEC_PERM, Self::ACCESS_PERM]
            .iter()
            .any(|perm| self.contains(*perm))
    }
}

// can derive Eq b/c the lifetime ensures the fd survives the DirFd,
// and while that fd is still valid, I can compare by fd
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct DirFd<'a> {
    fd: RawFd,
    phantom: PhantomData<&'a ()>,
}

impl AsRawFd for DirFd<'_> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl IntoRawFd for DirFd<'_> {
    fn into_raw_fd(self) -> RawFd {
        self.fd
    }
}

impl FromRawFd for DirFd<'_> {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self { fd, phantom: PhantomData }
    }
}

impl<'a> DirFd<'a> {
    pub fn current_working_directory() -> Self {
        Self {
            fd: libc::AT_FDCWD,
            phantom: PhantomData,
        }
    }

    pub fn directory<P: AsRawFd>(dir: &'a P) -> Self {
        Self {
            fd: dir.as_raw_fd(),
            phantom: PhantomData,
        }
    }

    pub unsafe fn invalid() -> Self {
        Self::from_raw_fd(-1)
    }

    pub fn resolve(&self) -> Cow<Path> {
        if self.fd == libc::AT_FDCWD {
            Cow::Borrowed(Path::new("."))
        } else {
            let link = Path::new("/proc/self/fd")
                .join(format!("{}", self.fd));
            let link = link.read_link().unwrap_or(link);
            Cow::Owned(link)
        }
    }
}

impl Display for DirFd<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.resolve() {
            Cow::Borrowed(path) => path.display().fmt(f),
            Cow::Owned(path) => path.display().fmt(f),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct MarkPath<'a> {
    pub(crate) dir: DirFd<'a>,
    pub(crate) path: Option<&'a Path>,
}

#[derive(Debug)]
pub struct MarkPathDisplay<'a> {
    dir: Option<DirFd<'a>>,
    path: Option<&'a Path>,
}

impl Display for MarkPathDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<'a> MarkPath<'a> {
    pub fn current_working_directory() -> Self {
        Self {
            dir: DirFd::current_working_directory(),
            path: None,
        }
    }

    pub fn directory<P: AsRawFd>(dir: &'a P) -> Self {
        Self {
            dir: DirFd::directory(dir),
            path: None,
        }
    }

    pub fn relative_to<P: AsRawFd>(dir: &'a P, path: &'a Path) -> Self {
        Self {
            dir: DirFd::directory(dir),
            path: Some(path),
        }
    }

    pub fn absolute(path: &'a Path) -> Self {
        Self {
            dir: unsafe { DirFd::invalid() }, // ignored by fanotify_mark()
            path: Some(path),
        }
    }

    pub fn resolve(&self) -> Cow<Path> {
        match self.path {
            None => self.dir.resolve(),
            Some(path) => if path.is_absolute() {
                Cow::Borrowed(path)
            } else {
                Cow::Owned(self.dir.resolve().to_owned().join(path))
            }
        }
    }

    pub fn display(&self) -> MarkPathDisplay<'a> {
        let dir = Some(self.dir);
        MarkPathDisplay {
            dir: match self.path {
                None => dir,
                Some(path) => dir.filter(|_| path.is_relative())
            },
            path: self.path,
        }
    }
}



impl Display for MarkPath<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.display().fmt(f)
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
    pub(crate) flags: CombinedMarkFlags,
    pub(crate) mask: MarkMask,
    pub(crate) path: MarkPath<'a>,
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

#[derive(Debug, PartialEq, Hash)]
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
            dir_fd: self.path.dir.as_raw_fd(),
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
