use std::borrow::Cow;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::os::unix::io::AsRawFd;

use super::DirFd;

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
