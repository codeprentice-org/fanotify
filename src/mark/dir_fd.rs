use std::borrow::Cow;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::FromRawFd;
use std::os::unix::io::IntoRawFd;
use std::os::unix::io::RawFd;

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

    /// # Safety
    /// This will create an invalid directory file descriptor.
    /// Only use it where the [`DirFd`] will be ignored or you want it to cause an error.
    pub(in super) const unsafe fn invalid() -> Self {
        Self::const_from_raw_fd(-1)
    }
}

impl<'a> DirFd<'a> {
    /// # Safety
    /// See [`FromRawFd`].  This is just a `const` version of that.
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
