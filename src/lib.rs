pub mod common;
pub mod libc;
mod util;
pub mod init;
pub mod mark;
pub mod event;
pub mod descriptor;

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        mem,
        path::Path,
        slice,
    };
    
    use apply::Apply;
    
    use crate::{
        descriptor::Fanotify,
        event::file::GetFD,
        init,
        init::{Flags, Init},
        libc::read::fanotify_event_metadata,
        mark::{
            self,
            Mark,
            OneAction::Add,
            What::MountPoint,
        },
    };
    
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
    
    fn check_is_valid_first_path<P: AsRef<Path>>(path: P) {
        assert_eq!(|| -> Option<&str> {
            path.as_ref().parent()?.file_name()?.to_str()
        }(), Some("bin"));
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
            Path::new("/proc/self/fd")
                .join(buf[0].fd.to_string())
                .read_link()?
                .apply(check_is_valid_first_path);
            Ok(())
        });
    }
    
    #[test]
    fn init_mark_and_read() {
        with_fanotify(|fanotify| {
            fanotify.mark(get_mark())?;
            let mut buf = Vec::with_capacity(4096);
            let events = fanotify.read(&mut buf)?;
            for event in events.fds() {
                event
                    .file()
                    .fd()
                    .path()?
                    .apply(check_is_valid_first_path);
            }
            Ok(())
        });
    }
}
