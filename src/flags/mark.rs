use bitflags::bitflags;

use super::libc::mark::{action, what, flag, mask};
use self::MarkAction::Flush;
use self::StaticMarkError::EmptyMask;
use std::ffi::CString;
use std::os::raw::c_char;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::{AsRawFd, RawFd, IntoRawFd, FromRawFd};
use std::path::Path;
use thiserror::Error;
use std::borrow::Cow;
use std::fmt::{Display, Formatter, Debug};
use std::fmt;
use std::marker::PhantomData;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MarkOneAction {
    Add,
    Remove,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum MarkAction {
    Add = action::FAN_MARK_ADD,
    Remove = action::FAN_MARK_REMOVE,
    Flush = action::FAN_MARK_FLUSH,
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
    Inode = what::FAN_MARK_INODE,
    MountPoint = what::FAN_MARK_MOUNT,
    FileSystem = what::FAN_MARK_FILESYSTEM,
}

bitflags! {
    #[derive(Default)]
    pub struct MarkFlags: u32 {
        const DONT_FOLLOW = flag::FAN_MARK_DONT_FOLLOW;
        const ONLY_DIR = flag::FAN_MARK_ONLYDIR;
        const IGNORED_MASK = flag::FAN_MARK_IGNORED_MASK;
        const IGNORED_SURVIVE_MODIFY = flag::FAN_MARK_IGNORED_SURV_MODIFY;
    }
}

bitflags! {
    pub struct MarkMask: u64 {
        const ACCESS = mask::FAN_ACCESS;
        const MODIFY = mask::FAN_MODIFY;
        const CLOSE_WRITE = mask::FAN_CLOSE_WRITE;
        const CLOSE_NOWRITE = mask::FAN_CLOSE_NOWRITE;
        const OPEN = mask::FAN_OPEN;
        const OPEN_EXEC = mask::FAN_OPEN_EXEC;
        const ATTRIB = mask::FAN_ATTRIB;
        const CREATE = mask::FAN_CREATE;
        const DELETE = mask::FAN_DELETE;
        const DELETE_SELF = mask::FAN_DELETE_SELF;
        const MOVED_FROM = mask::FAN_MOVED_FROM;
        const MOVED_TO = mask::FAN_MOVED_TO;
        const MOVE_SELF = mask::FAN_MOVE_SELF;
        const OPEN_PERM = mask::FAN_OPEN_PERM;
        const OPEN_EXEC_PERM = mask::FAN_OPEN_EXEC_PERM;
        const ACCESS_PERM = mask::FAN_ACCESS_PERM;
        const ON_DIR = mask::FAN_ONDIR;
        const EVENT_ON_CHILD = mask::FAN_EVENT_ON_CHILD;
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

impl DirFd<'static> {
    pub fn current_working_directory() -> Self {
        Self {
            fd: libc::AT_FDCWD,
            phantom: PhantomData,
        }
    }
}

impl<'a> DirFd<'a> {
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
            Cow::Borrowed(path) => write!(f, "{}", path.display()),
            Cow::Owned(path) => write!(f, "{}", path.display()),
        }
    }
}

#[derive(Eq, PartialEq, Hash)]
pub struct MarkPath<'a> {
    pub(crate) dir: DirFd<'a>,
    pub(crate) path: Option<&'a Path>,
}

impl MarkPath<'static> {
    pub fn current_working_directory() -> Self {
        Self {
            dir: DirFd::current_working_directory(),
            path: None,
        }
    }
}

impl<'a> MarkPath<'a> {
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
}

impl Display for MarkPath<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.path {
            None => write!(f, "{{ dir: {} }}", self.dir),
            Some(path) => {
                if path.is_absolute() {
                    write!(f, "{{ absolute: {} }}", path.display())
                } else {
                    write!(f, "{{ dir: {}, relative: {}, path: {} }}",
                           self.dir, path.display(), self.resolve().to_owned().display())
                }
            }
        }
    }
}

impl Debug for MarkPath<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct MarkOne<'a> {
    pub action: MarkOneAction,
    pub what: MarkWhat,
    pub flags: MarkFlags,
    pub mask: MarkMask,
    pub path: MarkPath<'a>,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Mark<'a> {
    pub(crate) action: MarkAction,
    pub(crate) what: MarkWhat,
    pub(crate) flags: MarkFlags,
    pub(crate) mask: MarkMask,
    pub(crate) path: MarkPath<'a>,
}

#[derive(Error, Debug, Eq, PartialEq, Hash)]
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
            action: action.into(),
            what,
            flags,
            mask,
            path,
        };
        Ok(this)
    }

    pub fn flush(what: MarkWhat) -> Self {
        Self {
            action: Flush,
            what,
            flags: MarkFlags::empty(),
            mask: MarkMask::all(), // ignored, but empty is invalid on add/remove
            path: MarkPath::current_working_directory(), // ignored, but good default with 'static lifetime
        }
    }
}

type RawMarkFlags = u32;

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
    pub fn flags(&self) -> RawMarkFlags {
        self.action as u32 | self.what as u32 | self.flags.bits()
    }

    pub fn to_raw(&self) -> RawMark {
        RawMark {
            flags: self.flags(),
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
