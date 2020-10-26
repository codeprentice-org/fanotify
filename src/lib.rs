pub mod flags;
pub mod descriptor;
mod util;

#[cfg(test)]
mod tests {
    use crate::flags::init::{Init, Flags};
    use crate::descriptor::InitError;

    #[test]
    fn it_works() {
        let args = Init {
            flags: Flags::unlimited() | Flags::REPORT_FID,
            ..Default::default()
        };
        assert_eq!(
            format!("{:?}", args.as_raw()),
            "Init { \
                notification_class: Notify, \
                flags: UNLIMITED_QUEUE | UNLIMITED_MARKS | REPORT_FID, \
                rw: Read, \
                event_flags: LARGE_FILE \
            }",
        );
        match args.run() {
            Ok(_fd) => {}
            Err(e) => {
                assert_eq!(e, InitError::Unsupported);
            }
        }
        assert_eq!(2 + 2, 4);
    }
}
