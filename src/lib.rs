pub mod flags;
pub mod descriptor;
mod util;

#[cfg(test)]
mod tests {
    use crate::flags::init::{Init, Flags};
    use crate::descriptor::InitError;
    use crate::flags::mark::{Mark, MarkOne, MarkPath, MarkMask, MarkFlags, StaticMarkError};
    use crate::flags::mark::MarkWhat::{FileSystem, MountPoint};
    use crate::flags::mark::MarkOneAction::Add;
    use std::path::Path;
    use std::fs::File;

    #[test]
    fn catches_unsupported() {
        let args = Init {
            flags: Flags::unlimited() | Flags::REPORT_FID,
            ..Default::default()
        };
        match args.run() {
            Ok(_fd) => {}
            Err(e) => {
                assert_eq!(e, InitError::Unsupported);
            }
        }
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn init_display_debug() {
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
    }

    #[test]
    fn mark_static_error() {
        assert_eq!(Mark::one(MarkOne {
            action: Add,
            what: FileSystem,
            flags: MarkFlags::empty(),
            mask: MarkMask::empty(),
            path: MarkPath::current_working_directory(),
        }), Err(StaticMarkError::EmptyMask));
    }

    #[test]
    fn mark_display_debug_1() {
        let mark = Mark::one(MarkOne {
            action: Add,
            what: FileSystem,
            flags: MarkFlags::empty(),
            mask: MarkMask::OPEN | MarkMask::close(),
            path: MarkPath::current_working_directory(),
        }).unwrap();
        assert_eq!(
            format!("{:?}", mark),
            "Mark { \
                action: Add, \
                what: FileSystem, \
                flags: (empty), \
                mask: CLOSE_WRITE | CLOSE_NOWRITE | OPEN, \
                path: { dir: . } \
            }",
        );
    }

    #[test]
    fn mark_display_debug_2() {
        let mark = Mark::one(MarkOne {
            action: Add,
            what: FileSystem,
            flags: MarkFlags::ONLY_DIR | MarkFlags::DONT_FOLLOW,
            mask: MarkMask::CREATE | MarkMask::DELETE | MarkMask::moved(),
            path: MarkPath::absolute(Path::new("/home")),
        }).unwrap();
        assert_eq!(
            format!("{:?}", mark),
            "Mark { \
                action: Add, \
                what: FileSystem, \
                flags: DONT_FOLLOW | ONLY_DIR, \
                mask: CREATE | DELETE | MOVED_FROM | MOVED_TO, \
                path: { absolute: /home } \
            }",
        );
    }

    #[test]
    fn mark_display_debug_3() {
        let root = File::open(Path::new("/")).unwrap();
        let mark = Mark::one(MarkOne {
            action: Add,
            what: MountPoint,
            flags: MarkFlags::ONLY_DIR | MarkFlags::DONT_FOLLOW,
            mask: MarkMask::CREATE | MarkMask::DELETE | MarkMask::moved(),
            path: MarkPath::relative_to(&root, Path::new("proc")),
        }).unwrap();
        assert_eq!(
            format!("{:?}", mark),
            "Mark { \
                action: Add, \
                what: MountPoint, \
                flags: DONT_FOLLOW | ONLY_DIR, \
                mask: CREATE | DELETE | MOVED_FROM | MOVED_TO, \
                path: { dir: /, relative: proc, path: /proc } \
            }",
        );
    }
}
