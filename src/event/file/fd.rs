use crate::fd::FD;
use crate::event::file::GetFD;

/// A regular file event just containing an [`FD`],
/// i.e., it is not a permission event
/// and it's not a [`REPORT_FID`](crate::init::Flags::REPORT_FID) event.
pub struct FileFD {
    pub(in super::super) fd: FD,
}

impl GetFD for FileFD {
    fn fd(&self) -> &FD {
        &self.fd
    }
}
