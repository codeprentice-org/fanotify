use std::convert::TryFrom;
use std::convert::TryInto;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;

use apply::Apply;
use nix::errno::Errno;
use static_assertions::const_assert_eq;
use to_trait::To;

use crate::fd::FD;
use crate::libc::write::FAN_ALLOW;
use crate::libc::write::FAN_AUDIT;
use crate::libc::write::FAN_DENY;
use crate::libc::write::fanotify_response;

use super::super::file::GetFD;
use super::super::responses::RC;
use super::super::responses::Responses;

use self::PermissionDecision::Allow;
use self::PermissionDecision::Deny;

/// A permission decision for a file event, either [`Allow`] or [`Deny`].
/// Defaults to [`Allow`].
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum PermissionDecision {
    Allow,
    Deny,
}

/// In case the user forgets to make a permission decision,
/// we want to allow by default so everything doesn't get blocked by default.
impl Default for PermissionDecision {
    fn default() -> Self {
        Allow
    }
}

impl From<PermissionDecision> for u32 {
    fn from(this: PermissionDecision) -> Self {
        match this {
            Allow => FAN_ALLOW,
            Deny => FAN_DENY,
        }
    }
}

impl TryFrom<u32> for PermissionDecision {
    type Error = ();
    
    fn try_from(this: u32) -> Result<Self, Self::Error> {
        const_assert_eq!(FAN_ALLOW, 1);
        const_assert_eq!(FAN_DENY, 2);
        // unsafe
        [
            Err(()),
            Ok(Allow),
            Ok(Deny),
            Err(()),
        ][(this & 0b11) as usize]
    }
}

#[derive(Debug)]
pub(in super::super) struct RawFilePermission {
    pub fd: RawFd,
    pub decision: PermissionDecision,
    pub audit: bool,
}

impl From<&RawFilePermission> for fanotify_response {
    /// The (more) raw [`fanotify_response`] representation of this [`RawFilePermission`].
    fn from(this: &RawFilePermission) -> Self {
        let audit = this.audit as u32 * FAN_AUDIT;
        Self {
            fd: this.fd,
            response: this.decision.to::<u32>() | audit,
        }
    }
}

impl TryFrom<&fanotify_response> for RawFilePermission {
    type Error = ();
    
    fn try_from(this: &fanotify_response) -> Result<Self, Self::Error> {
        // just ignore the other bits
        let audit = this.response & FAN_AUDIT != 0;
        Self {
            fd: this.fd,
            decision: this.response.try_into()?,
            audit,
        }.apply(Ok)
    }
}

/// Like a [`FileFD`](super::fd::FileFD) event, except it is a permission event
/// and thus you must make a permission decision.
///
/// Set [`Self::decision`] for the permission decision (it defaults to [`Allow`]).
/// [`Self::audit`] can also be set to tell the kernel to audit this permission decision.
/// The decision is written once all [`FilePermission`]s
/// from this [`Fanotify::read`](crate::fanotify::Fanotify::read) call are dropped.
#[derive(Debug)]
pub struct FilePermission<'a> {
    fd: FD,
    pub decision: PermissionDecision,
    pub audit: bool,
    written: bool,
    responses: RC<Responses<'a>>,
}

impl GetFD for FilePermission<'_> {
    fn fd(&self) -> &FD {
        &self.fd
    }
}

impl<'a> FilePermission<'a> {
    pub(in super::super) fn new(fd: FD, responses: RC<Responses<'a>>) -> Self {
        Self {
            fd,
            decision: PermissionDecision::default(),
            audit: false,
            written: false,
            responses,
        }
    }
    
    pub fn allow(&mut self) {
        self.decision = Allow;
    }
    
    pub fn deny(&mut self) {
        self.decision = Deny;
    }
    
    pub fn written(&self) -> bool {
        self.written
    }
    
    /// The raw [`RawFilePermission`] of this [`FilePermission`].
    fn response(&self) -> RawFilePermission {
        RawFilePermission {
            fd: self.fd.as_raw_fd(),
            decision: self.decision,
            audit: self.audit,
        }
    }
    
    /// Write the response immediately to the [`Fanotify`](crate::fanotify::Fanotify).
    ///
    /// Return if the response is written (it can only be written successfully once).
    pub fn write_immediately(&mut self) -> std::result::Result<bool, Errno> {
        if self.written {
            return Ok(false);
        }
        self.responses.write_immediately(&self.response())?;
        self.written = true;
        Ok(true)
    }
    
    /// Write the response to the [`Responses`] buffer.
    ///
    /// Return if the response is written (it can only be written successfully once).
    pub fn write_buffered(&mut self) -> bool {
        if self.written {
            return false;
        }
        self.responses.write_buffered(&self.response());
        self.written = true;
        true
    }
}

/// Make sure the permission has always been written
impl Drop for FilePermission<'_> {
    fn drop(&mut self) {
        self.write_buffered();
    }
}
