pub use action::Action;
pub use action::OneAction;
pub use dir_fd::DirFd;
pub use error::Error;
pub use error::RawError;
pub use error::StaticError;
pub use flags::Flags;
pub use mark::Mark;
pub use mark::OneMark as One;
pub use markable::Markable;
pub use mask::Mask;
pub use path::Path;
pub(crate) use raw::FanotifyMark;
pub use raw::RawFlags;
pub use raw::RawMark;
pub use what::What;

mod dir_fd;
mod path;
mod error;
mod raw;
#[allow(clippy::module_inception)]
mod mark;
mod action;
mod what;
mod flags;
mod mask;
mod markable;

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        path::Path,
    };

    use crate::mark::{
        self,
        error,
        mark::Mark,
        OneAction::Add,
        path,
        What::{FileSystem, MountPoint},
    };

    #[test]
    fn mark_static_error() {
        assert_eq!(Mark::one(mark::mark::OneMark {
            action: Add,
            what: FileSystem,
            flags: mark::Flags::empty(),
            mask: mark::Mask::empty(),
            path: path::Path::current_working_directory(),
        }), Err(error::StaticError::EmptyMask));
    }

    #[test]
    fn mark_display_debug_1() {
        let mark = Mark::one(mark::mark::OneMark {
            action: Add,
            what: FileSystem,
            flags: mark::Flags::empty(),
            mask: mark::Mask::OPEN | mark::Mask::close(),
            path: path::Path::current_working_directory(),
        }).unwrap();
        assert_eq!(
            format!("{}", mark),
            "Mark { \
                action: Add, \
                what: FileSystem, \
                flags: (empty), \
                mask: OPEN | CLOSE_NO_WRITE | CLOSE_WRITE, \
                path: { dir: . } \
            }",
        );
    }

    #[test]
    fn mark_display_debug_2() {
        let mark = Mark::one(mark::mark::OneMark {
            action: Add,
            what: FileSystem,
            flags: mark::Flags::ONLY_DIR | mark::Flags::DONT_FOLLOW,
            mask: mark::Mask::CREATE | mark::Mask::DELETE | mark::Mask::moved(),
            path: path::Path::absolute(Path::new("/home")),
        }).unwrap();
        assert_eq!(
            format!("{}", mark),
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
        let mark = Mark::one(mark::mark::OneMark {
            action: Add,
            what: MountPoint,
            flags: mark::Flags::ONLY_DIR | mark::Flags::DONT_FOLLOW,
            mask: mark::Mask::CREATE | mark::Mask::DELETE | mark::Mask::moved(),
            path: path::Path::relative_to(&root, Path::new("proc")),
        }).unwrap();
        assert_eq!(
            format!("{}", mark),
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
