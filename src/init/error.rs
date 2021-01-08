use crate::common::FD;

#[derive(thiserror::Error, Debug, Eq, PartialEq, Hash)]
pub enum Error {
    #[error("invalid argument specified")]
    InvalidArgument,
    #[error("exceeded the per-process limit on fanotify groups")]
    ExceededFanotifyGroupPerProcessLimit,
    #[error("exceeded the per-process limit on open file descriptors")]
    ExceededOpenFileDescriptorPerProcessLimit,
    #[error("kernel out of memory")]
    OutOfMemory,
    #[error("user does not have the required CAP_SYS_ADMIN capability")]
    PermissionDenied,
    #[error("the kernel does not support the fanotify_init() syscall")]
    FanotifyUnsupported,
    #[error("the kernel does not support a certain feature for fanotify_init()")]
    FeatureUnsupported,
    #[error("received an invalid fd: {}", .fd)]
    InvalidFd { fd: FD },
}
