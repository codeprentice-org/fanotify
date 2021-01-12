use std::os::unix::io::AsRawFd;

use nix::errno::Errno;
use to_trait::To;

use crate::{
    fd::FD,
    libc::write::{FAN_ALLOW, FAN_AUDIT, FAN_DENY, fanotify_response},
};

use super::super::{
    file::GetFD,
    responses::{RC, Responses},
};

use self::PermissionDecision::{Allow, Deny};

/// A permission decision for a file event, either [`Allow`] or [`Deny`].
/// Defaults to [`Allow`].
#[derive(Eq, PartialEq, Copy, Clone)]
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

/// Like a [`FileFD`](super::fd::FileFD) event, except it is a permission event
/// and thus you must make a permission decision.
///
/// Set [`Self::decision`] for the permission decision (it defaults to [`Allow`]).
/// [`Self::audit`] can also be set to tell the kernel to audit this permission decision.
/// The decision is written once all [`FilePermission`]s
/// from this [`Fanotify::read`](crate::fanotify::Fanotify::read) call are dropped.
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
    
    /// The raw [`fanotify_response`] representation of this [`FilePermission`].
    fn response(&self) -> fanotify_response {
        let audit = self.audit as u32 * FAN_AUDIT;
        fanotify_response {
            fd: self.fd.as_raw_fd(),
            response: self.decision.to::<u32>() | audit,
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
