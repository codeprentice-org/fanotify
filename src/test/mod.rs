use std::{error::Error, fs, mem, path::Path, slice};
use std::path::PathBuf;

use apply::Apply;
use async_io::block_on;
use to_trait::To;

use driver::Driver;

use crate::{
    event::{
        buffer::EventBufferSize,
        events::Events,
        file::GetFD,
        iterator_ext::IntoEvents,
    },
    fanotify::Fanotify,
    init,
    init::{Flags, Init},
    libc::read::fanotify_event_metadata,
    mark::{
        self,
        Mark,
        Markable,
        OneAction::Add,
        What::MountPoint,
    },
};
use crate::buffered_fanotify::IntoBufferedFanotify;

mod driver;

const fn get_init() -> Init {
    Init {
        flags: Flags::unlimited(),
        ..Init::const_default()
    }
}

fn with_fanotify<F: FnOnce(Fanotify) -> Result<(), Box<dyn Error>>>(f: F) {
    match get_init().to_fanotify() {
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
    let dir = (|| -> Option<&str> {
        path.as_ref().parent()?.file_name()?.to_str()
    })().unwrap();
    assert!(dir == "bin" || dir == "x86_64-linux-gnu");
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

fn check_events(events: Events<'_>) -> Result<(), Box<dyn Error>> {
    for event in events.fds() {
        event
            .file()
            .fd()
            .path()?
            .apply(check_is_valid_first_path);
    }
    Ok(())
}

#[test]
fn init_mark_and_read() {
    with_fanotify(|fanotify| {
        fanotify.mark(get_mark())?;
        let mut buf = EventBufferSize::default().new_buffer();
        let events = fanotify.read(&mut buf)?;
        check_events(events)?;
        Ok(())
    });
}

#[test]
fn init_mark_and_read_async() {
    with_fanotify(|fanotify| {
        let fanotify = fanotify.into_async()?;
        fanotify.mark(get_mark())?;
        let mut buf = EventBufferSize::default().new_buffer();
        let events = block_on(fanotify.read(&mut buf))?;
        check_events(events)?;
        Ok(())
    })
}

#[test]
fn many() {
    with_fanotify(|fanotify| {
        fanotify.mark(Mark::one(mark::One {
            action: Add,
            what: MountPoint,
            flags: mark::Flags::empty(),
            mask: mark::Mask::ACCESS
                | mark::Mask::OPEN
                | mark::Mask::close()
                | mark::Mask::MODIFY,
            path: mark::Path::absolute("/home"),
        }).unwrap())?;
        let mut driver = fanotify.buffered_default().to::<Driver>();
        let path = std::env::var_os("HOME").unwrap();
        let path = PathBuf::new().join(path).join(".bash_history");
        let path = path.as_path();
        {
            fs::read_to_string(path)?;
            let events = driver.read()?.collect::<Vec<_>>();
            println!("{:?}", events);
        }
        let mut driver = driver.into_async()?;
        {
            fs::read_to_string(path)?;
            block_on(driver.read_n(1))?;
        }
        Ok(())
    })
}

#[test]
fn forever() {
    with_fanotify(|fanotify| {
        fanotify.mark(Mark::one(mark::One {
            action: Add,
            what: MountPoint,
            flags: mark::Flags::empty(),
            mask: mark::Mask::ACCESS
                | mark::Mask::OPEN
                | mark::Mask::close()
                | mark::Mask::MODIFY,
            path: mark::Path::absolute("/home"),
        }).unwrap())?;
        let mut fanotify = fanotify.buffered_default();
        loop {
            for event in fanotify.read()?.all() {
                println!("{:?}", event);
                event
                    .ok()
                    .and_then(|it| it.into_file().path())
                    .and_then(|it| it.ok())
                    .map(|path| {
                        println!("{}", path.display());
                    });
            }
        }
    })
}
