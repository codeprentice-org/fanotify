use std::{
    borrow::Cow,
    ffi::CString,
    fmt::{Debug, Display, Formatter},
    fmt,
    marker::PhantomData,
    os::{
        raw::c_char,
        unix::{
            ffi::OsStringExt,
            io::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
        },
    },
};

use bitflags::bitflags;
use thiserror::Error;

use super::{
    init,
    libc::mark::{action, flag, mask, what},
};

use self::{
    Action::Flush,
    StaticError::EmptyMask,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum OneAction {
    Add,
    Remove,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum Action {
    Add = action::FAN_MARK_ADD,
    Remove = action::FAN_MARK_REMOVE,
    Flush = action::FAN_MARK_FLUSH,
}

impl OneAction {
    pub const fn const_into(self) -> Action {
        match self {
            Self::Add => Action::Add,
            Self::Remove => Action::Remove,
        }
    }
}

impl From<OneAction> for Action {
    fn from(it: OneAction) -> Self {
        it.const_into()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum What {
    Inode = what::FAN_MARK_INODE,
    MountPoint = what::FAN_MARK_MOUNT,
    FileSystem = what::FAN_MARK_FILESYSTEM,
}

bitflags! {
    pub struct Flags: u32 {
        const DONT_FOLLOW = flag::FAN_MARK_DONT_FOLLOW;
        const ONLY_DIR = flag::FAN_MARK_ONLYDIR;
        const IGNORED_MASK = flag::FAN_MARK_IGNORED_MASK;
        const IGNORED_SURVIVE_MODIFY = flag::FAN_MARK_IGNORED_SURV_MODIFY;
    }
}

impl Flags {
    pub const fn const_default() -> Self {
        Self::empty()
    }
}

impl Default for Flags {
    fn default() -> Self {
        Self::const_default()
    }
}

// TODO find better names for some of these
bitflags! {
    pub struct Mask: u64 {
        const ACCESS = mask::FAN_ACCESS;
        const OPEN = mask::FAN_OPEN;
        const OPEN_EXEC = mask::FAN_OPEN_EXEC;
        const CLOSE_NO_WRITE = mask::FAN_CLOSE_NOWRITE;
        const CLOSE_WRITE = mask::FAN_CLOSE_WRITE;
        const MODIFY = mask::FAN_MODIFY;

        const ATTRIBUTE_CHANGED = mask::FAN_ATTRIB;

        const CREATE = mask::FAN_CREATE;
        const DELETE = mask::FAN_DELETE;
        const DELETE_SELF = mask::FAN_DELETE_SELF;
        const MOVED_FROM = mask::FAN_MOVED_FROM;
        const MOVED_TO = mask::FAN_MOVED_TO;
        const MOVE_SELF = mask::FAN_MOVE_SELF;

        const ACCESS_PERMISSION = mask::FAN_ACCESS_PERM;
        const OPEN_PERMISSION = mask::FAN_OPEN_PERM;
        const OPEN_EXEC_PERMISSION = mask::FAN_OPEN_EXEC_PERM;

        const ON_DIR = mask::FAN_ONDIR;
        const EVENT_ON_CHILD = mask::FAN_EVENT_ON_CHILD;
    }
}

impl Mask {
    // combined flags
    
    pub const fn close() -> Self {
        Self::from_bits_truncate(0
            | Self::CLOSE_NO_WRITE.bits
            | Self::CLOSE_WRITE.bits
        )
    }
    
    pub const fn moved() -> Self {
        Self::from_bits_truncate(0
            | Self::MOVED_FROM.bits
            | Self::MOVED_TO.bits
        )
    }
    
    pub const fn all_permissions() -> Self {
        Self::from_bits_truncate(0
            | Self::ACCESS_PERMISSION.bits
            | Self::OPEN_PERMISSION.bits
            | Self::OPEN_EXEC_PERMISSION.bits
        )
    }
    
    pub const fn includes_permission(&self) -> bool {
        self.contains(Self::all_permissions())
    }
    
    pub const fn path_changed(&self) -> Self {
        Self::from_bits_truncate(0
            | Self::ACCESS.bits
            | Self::OPEN.bits
            | Self::OPEN_EXEC.bits
            | Self::CLOSE_NO_WRITE.bits
            | Self::CLOSE_WRITE.bits
            | Self::MODIFY.bits
        )
    }
    
    pub const fn used(&self) -> Self {
        Self::from_bits_truncate(0
            | Self::CREATE.bits
            | Self::DELETE.bits
            | Self::DELETE_SELF.bits
            | Self::moved().bits
            | Self::MOVE_SELF.bits
        )
    }
}

/// A borrowed directory file descriptor with lifetime `'a`.
///
/// It contains a [`RawFd`] for the directory file descriptor, which outlives this [`DirFd`].
/// Thus, the [`RawFd`] is never closed by [`DirFd`].
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
        Self::const_from_raw_fd(fd)
    }
}

impl DirFd<'static> {
    /// A pseudo [`DirFd`] representing the current working directory
    /// (at use time, not at the time of this call necessarily).
    pub const fn current_working_directory() -> Self {
        unsafe { Self::const_from_raw_fd(libc::AT_FDCWD) }
    }
    
    pub const unsafe fn invalid() -> Self {
        Self::const_from_raw_fd(-1)
    }
}

impl<'a> DirFd<'a> {
    pub const unsafe fn const_from_raw_fd(fd: RawFd) -> Self {
        Self {
            fd,
            phantom: PhantomData,
        }
    }
    
    /// Create a [`DirFd`] from an existing [`AsRawFd`].
    /// The [`AsRawFd`] given must point to a directory for things to work correctly.
    pub fn directory<P: AsRawFd>(dir: &'a P) -> Self {
        Self {
            fd: dir.as_raw_fd(),
            phantom: PhantomData,
        }
    }
    
    /// Check if this [`DirFd`] represents the special current working directory file descriptor.
    ///
    /// It could be the case that this [`DirFd`] represents the current working directory as `open(".")`,
    /// but this only checks if this [`DirFd`] is the special file descriptor for the current working directory.
    pub const fn is_current_working_directory(&self) -> bool {
        self.fd == libc::AT_FDCWD
    }
    
    /// Resolve this [`DirFd`] to its absolute path,
    /// attempting to use the `/proc` filesystem to resolve the file descriptor.
    pub fn resolve(&self) -> Cow<std::path::Path> {
        if self.is_current_working_directory() {
            Cow::Borrowed(std::path::Path::new("."))
        } else {
            let link = std::path::Path::new("/proc/self/fd")
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

/// A path that is either absolute or relative to a directory file descriptor ([`DirFd`]).
#[derive(Eq, PartialEq, Hash)]
pub struct Path<'a> {
    pub(super) dir: DirFd<'a>,
    pub(super) path: Option<&'a std::path::Path>,
}

impl Path<'static> {
    /// A pseudo [`Path<'static>`](Path) representing the current working directory
    /// (at use time, not at the time of this call necessarily).
    ///
    /// It has a `'static` lifetime because there is always a current working directory.
    /// It always refers to whatever the current working directory is, even after a [`libc::chdir`].
    ///
    /// See [`DirFd::current_working_directory`].
    pub const fn current_working_directory() -> Self {
        Self {
            dir: DirFd::current_working_directory(),
            path: None,
        }
    }
}

impl<'a> Path<'a> {
    /// Create a [`Path`] referring to a directory by its file descriptor ([`DirFd`]).
    pub fn directory<FD: AsRawFd>(dir: &'a FD) -> Self {
        Self {
            dir: DirFd::directory(dir),
            path: None,
        }
    }
    
    /// Create a [`Path`] relative to the given [`DirFd`] directory.
    pub fn relative_to<FD: AsRawFd, P: AsRef<std::path::Path> + 'a + ?Sized>(dir: &'a FD, path: &'a P) -> Self {
        Self {
            dir: DirFd::directory(dir),
            path: Some(path.as_ref()),
        }
    }
    
    /// Create a [`Path`] using an absolute path.
    pub fn absolute<P: AsRef<std::path::Path> + 'a + ?Sized>(path: &'a P) -> Self {
        Self {
            dir: unsafe { DirFd::invalid() }, // ignored by fanotify_mark()
            path: Some(path.as_ref()),
        }
    }
    
    /// Resolve this [`Path`] to its absolute path,
    /// attempting to use the `/proc` filesystem to resolve the [`DirFd`] directory.
    ///
    /// See [`DirFd::resolve`].
    pub fn resolve(&self) -> Cow<std::path::Path> {
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

impl Display for Path<'_> {
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

impl Debug for Path<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct One<'a> {
    pub action: OneAction,
    pub what: What,
    pub flags: Flags,
    pub mask: Mask,
    pub path: Path<'a>,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Mark<'a> {
    pub(super) action: Action,
    pub(super) what: What,
    pub(super) flags: Flags,
    pub(super) mask: Mask,
    pub(super) path: Path<'a>,
}

impl Display for Mark<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // delegate Debug impl to Init
        write!(f, "{:?}", self)
    }
}

#[derive(Error, Debug, Eq, PartialEq, Hash)]
pub enum StaticError {
    #[error("mask must not be empty for add or remove")]
    EmptyMask,
}

impl<'a> Mark<'a> {
    pub const fn one(mark: One<'a>) -> Result<Self, StaticError> {
        let One {
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
            action: action.const_into(),
            what,
            flags,
            mask,
            path,
        };
        Ok(this)
    }
    
    pub const fn flush(what: What) -> Self {
        Self {
            action: Flush,
            what,
            flags: Flags::empty(),
            mask: Mask::all(), // ignored, but empty is invalid on add/remove
            path: Path::current_working_directory(), // ignored, but good default with 'static lifetime
        }
    }
}

type RawFlags = u32;

#[derive(Debug, PartialEq, Hash)]
pub struct RawMark {
    pub(super) flags: u32,
    pub(super) mask: u64,
    pub(super) dir_fd: RawFd,
    pub(super) path: Option<CString>,
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
    pub const fn flags(&self) -> RawFlags {
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

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::path::Path;
    
    use crate::mark;
    use crate::mark::Mark;
    use crate::mark::OneAction::Add;
    use crate::mark::What::{FileSystem, MountPoint};
    
    #[test]
    fn mark_static_error() {
        assert_eq!(Mark::one(mark::One {
            action: Add,
            what: FileSystem,
            flags: mark::Flags::empty(),
            mask: mark::Mask::empty(),
            path: mark::Path::current_working_directory(),
        }), Err(mark::StaticError::EmptyMask));
    }
    
    #[test]
    fn mark_display_debug_1() {
        let mark = Mark::one(mark::One {
            action: Add,
            what: FileSystem,
            flags: mark::Flags::empty(),
            mask: mark::Mask::OPEN | mark::Mask::close(),
            path: mark::Path::current_working_directory(),
        }).unwrap();
        assert_eq!(
            format!("{}", mark),
            "Mark { \
                action: Add, \
                what: FileSystem, \
                flags: (empty), \
                mask: OPEN | CLOSE_NO_WRITE | CLOSE_WRITE, \
                path: { dir: . } \
            }",
        );
    }
    
    #[test]
    fn mark_display_debug_2() {
        let mark = Mark::one(mark::One {
            action: Add,
            what: FileSystem,
            flags: mark::Flags::ONLY_DIR | mark::Flags::DONT_FOLLOW,
            mask: mark::Mask::CREATE | mark::Mask::DELETE | mark::Mask::moved(),
            path: mark::Path::absolute(Path::new("/home")),
        }).unwrap();
        assert_eq!(
            format!("{}", mark),
            "Mark { \
                action: Add, \
                what: FileSystem, \
                flags: DONT_FOLLOW | ONLY_DIR, \
                mask: CREATE | DELETE | MOVED_FROM | MOVED_TO, \
                path: { absolute: /home } \
            }",
        );
    }
    
    #[test]
    fn mark_display_debug_3() {
        let root = File::open(Path::new("/")).unwrap();
        let mark = Mark::one(mark::One {
            action: Add,
            what: MountPoint,
            flags: mark::Flags::ONLY_DIR | mark::Flags::DONT_FOLLOW,
            mask: mark::Mask::CREATE | mark::Mask::DELETE | mark::Mask::moved(),
            path: mark::Path::relative_to(&root, Path::new("proc")),
        }).unwrap();
        assert_eq!(
            format!("{}", mark),
            "Mark { \
                action: Add, \
                what: MountPoint, \
                flags: DONT_FOLLOW | ONLY_DIR, \
                mask: CREATE | DELETE | MOVED_FROM | MOVED_TO, \
                path: { dir: /, relative: proc, path: /proc } \
            }",
        );
    }
}
