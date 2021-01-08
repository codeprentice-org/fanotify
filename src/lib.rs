pub mod common;
pub mod libc;
mod util;
pub mod init;
pub mod mark;
pub mod event;
pub mod descriptor;

#[cfg(test)]
mod tests {
    use std::{mem, slice};
    use std::error::Error;
    use std::path::Path;
    
    use crate::{init, mark};
    use crate::descriptor::Fanotify;
    use crate::event::file::GetFD;
    use crate::init::{Flags, Init};
    use crate::libc::read::fanotify_event_metadata;
    use crate::mark::Mark;
    use crate::mark::OneAction::Add;
    use crate::mark::What::MountPoint;
    
    const fn get_init() -> Init {
        Init {
            flags: Flags::unlimited(),
            ..Init::const_default()
        }
    }
    
    fn with_fanotify<F: FnOnce(Fanotify) -> Result<(), Box<dyn Error>>>(f: F) {
        match get_init().run() {
            Ok(fanotify) => f(fanotify).unwrap(),
            Err(e) => {
                assert_eq!(e, init::Error::FanotifyUnsupported);
            }
        }
    }
    
    #[test]
    fn init_or_catches_unsupported() {
        with_fanotify(|_| Ok(()));
    }
    
    fn get_mark() -> Mark<'static> {
        Mark::one(mark::One {
            action: Add,
            what: MountPoint,
            flags: mark::Flags::empty(),
            mask: mark::Mask::OPEN | mark::Mask::close(),
            path: mark::Path::absolute("/home"),
        }).unwrap()
    }
    
    #[test]
    fn init_and_mark() {
        with_fanotify(|fanotify| {
            fanotify.mark(get_mark())?;
            Ok(())
        });
    }
    
    #[test]
    fn init_mark_and_raw_read() {
        with_fanotify(|fanotify| {
            fanotify.mark(get_mark())?;
            let mut buf = [fanotify_event_metadata {
                event_len: 0,
                vers: 0,
                reserved: 0,
                metadata_len: 0,
                mask: 0,
                fd: 0,
                pid: 0,
            }; 1];
            fanotify.fd.read(unsafe {
                slice::from_raw_parts_mut(
                    buf.as_mut_ptr() as *mut u8,
                    mem::size_of::<fanotify_event_metadata>() * buf.len(),
                )
            })?;
            let path = Path::new("/proc/self/fd")
                .join(buf[0].fd.to_string())
                .read_link()?;
            assert_eq!(path.parent().unwrap(), Path::new("/usr/bin"));
            Ok(())
        });
    }
    
    #[test]
    fn init_mark_and_read() {
        with_fanotify(|fanotify| {
            fanotify.mark(get_mark())?;
            let mut buf = Vec::with_capacity(4096);
            let events = fanotify.read(&mut buf)?;
            assert!(events
                .fds()
                .map(|it| it.file().fd().path().expect("/proc doesn't work"))
                .any(|it| it.parent().unwrap() == Path::new("/usr/bin"))
            );
            Ok(())
        });
    }
}
