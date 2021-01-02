use std::{cmp, fmt, mem, io};
use std::fmt::{Display, Formatter};
use std::os::raw::c_void;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use libc;
use nix::errno::Errno;

use super::util::libc_call;
use std::path::{PathBuf, Path};

/// A wrapper around an open [`RawFd`] file descriptor with RAII semantics
/// and generic file descriptor related functions
/// like [`read`](FD::read) and [`write`](FD::write).
#[derive(Eq, PartialEq, Hash, Debug)]
pub struct FD {
    fd: RawFd,
}

impl Drop for FD {
    fn drop(&mut self) {
        // Note that errors are ignored when closing a file descriptor. The
        // reason for this is that if an error occurs we don't actually know if
        // the file descriptor was closed or not, and if we retried (for
        // something like EINTR), we might close another valid file descriptor
        // opened after we closed ours.
        let _ = unsafe { libc::close(self.fd) };
    }
}

impl AsRawFd for FD {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl IntoRawFd for FD {
    fn into_raw_fd(self) -> RawFd {
        let fd = self.fd;
        mem::forget(self); // need to skip closing the fd
        fd
    }
}

impl FromRawFd for FD {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self { fd }
    }
}

impl FD {
    /// Check if the file descriptor is at least possibly valid, i.e. non-negative.
    ///
    /// If this returns `false`, then the file descriptor is definitely invalid.
    ///
    /// If this returns `true`, then the file descriptor might be valid.
    pub fn check(&self) -> bool {
        self.fd >= 0
    }
    
    /// Read from this file descriptor into the given buffer as much as possible.
    ///
    /// Return the number of bytes read like [`libc::read`]
    /// or the libc [`Errno`] if there was an error.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Errno> {
        if buf.is_empty() {
            return Ok(0);
        }
        let len = cmp::min(buf.len(), libc::ssize_t::MAX as usize) as libc::size_t;
        let buf = buf.as_mut_ptr() as *mut c_void;
        let bytes_read = libc_call(|| unsafe { libc::read(self.fd, buf, len) })?;
        Ok(bytes_read as usize)
    }
    
    /// Write from given buffer to this file descriptor as much as possible.
    ///
    /// Return the number of bytes written like [`libc::write`]
    /// or the libc [`Errno`] if there was an error.
    pub fn write(&self, buf: &[u8]) -> Result<usize, Errno> {
        if buf.is_empty() {
            return Ok(0);
        }
        let len = buf.len();
        let buf = buf.as_ptr() as *const c_void;
        let bytes_written = libc_call(|| unsafe { libc::write(self.fd, buf, len) })?;
        Ok(bytes_written as usize)
    }
    
    /// Resolve this file descriptor to its path using the `/proc` filesystem.
    pub fn path(&self) -> io::Result<PathBuf> {
        Path::new("/proc/self/fd")
            .join(self.fd.to_string())
            .read_link()
    }
}

impl Display for FD {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
