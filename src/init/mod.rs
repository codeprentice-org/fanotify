pub use error::Error;
pub use event_flags::EventFlags;
pub use flags::Flags;
pub use init::Init;
pub use notification_class::NotificationClass;
pub use raw::RawInit;
pub use rw::ReadWrite;

mod notification_class;
mod flags;
mod rw;
mod event_flags;
#[allow(clippy::module_inception)]
mod init;
mod raw;
mod error;

#[cfg(test)]
mod tests {
    use crate::init::{Flags, Init};
    
    #[test]
    fn init_display_debug() {
        let args = Init {
            flags: Flags::unlimited() | Flags::REPORT_FID,
            ..Default::default()
        };
        assert_eq!(
            format!("{}", args.as_raw()),
            "Init { \
                notification_class: Notify, \
                flags: UNLIMITED_QUEUE | UNLIMITED_MARKS | REPORT_FID, \
                rw: Read, \
                event_flags: LARGE_FILE \
            }",
        );
    }
}
