use crate::common::FD;

/// A regular file event just containing an [`FD`],
/// i.e., it is not a permission event and it's not a [`REPORT_FID`](init::Flags::REPORT_FID) event.
pub struct FileFD {
    pub fd: FD,
}
