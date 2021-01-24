use std::{
    fs,
    io::{
        Read,
        Seek,
        SeekFrom,
        Write,
    },
    mem,
    path::Path,
    slice,
};

use apply::Apply;
use async_io::block_on;
use tempfile::NamedTempFile;
use tempfile::tempfile;
use to_trait::To;

use driver::Driver;

use crate::{
    buffered_fanotify::IntoBufferedFanotify,
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
        Mask,
        OneAction::Add,
        What::MountPoint,
    },
};

mod driver;
mod supported;

const fn get_init() -> Init {
    Init {
        flags: Flags::unlimited(),
        ..Init::const_default()
    }
}

fn with_fanotify<F: FnOnce(Fanotify) -> anyhow::Result<()>>(f: F) {
    match get_init().to_fanotify() {
        Ok(fanotify) => match f(fanotify) {
            Ok(()) => {}
            Err(e) => panic!("{}", e),
        }
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
        mask: Mask::OPEN | Mask::close(),
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

fn check_events(events: Events<'_>) -> anyhow::Result<()> {
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
fn async_api() {
    with_fanotify(|fanotify| {
        block_on(async {
            let mut driver = fanotify.buffered_default().to::<Driver>().into_async()?;
            let path = Path::new("/bin/ls");
            fs::read_to_string(path)?;
            let event = driver.read1().await?;
            assert_eq!(event.mask(), Mask::OPEN | Mask::ACCESS);
            assert_eq!(event.into_file().path().unwrap().unwrap().as_path(), path);
            Ok(())
        })
    })
}

#[test]
fn many() {
    with_fanotify(|fanotify| {
        fanotify.mark(Mark::one(mark::One {
            action: Add,
            what: MountPoint,
            flags: mark::Flags::empty(),
            mask: Mask::ACCESS
                | Mask::OPEN
                | Mask::close()
                | Mask::MODIFY,
            path: mark::Path::absolute("/tmp"),
        }).unwrap())?;
        let mut driver = fanotify.buffered_default().to::<Driver>();
        test_unnamed_temp_file(&mut driver)?;
        test_named_temp_file(&mut driver)?;
        Ok(())
    })
}

fn test_unnamed_temp_file(driver: &mut Driver) -> anyhow::Result<()> {
    {
        let text = "test";
        let mut temp_file = tempfile()?;
        temp_file.write_all(text.as_bytes())?;
        temp_file.seek(SeekFrom::Start(0))?;
        let mut buf = String::new();
        temp_file.read_to_string(&mut buf)?;
        assert_eq!(text, buf);
    }
    let event = driver.read1()?;
    println!("unnamed_temp_file event: {:?}", event);
    assert!(event.mask().contains(Mask::OPEN | Mask::ACCESS | Mask::MODIFY | Mask::CLOSE_WRITE));
    Ok(())
}

fn test_named_temp_file(driver: &mut Driver) -> anyhow::Result<()> {
    {
        let text = "test";
        let mut temp_file = NamedTempFile::new()?;
        let mut file = temp_file.reopen()?;
        temp_file.write_all(text.as_bytes())?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        assert_eq!(text, buf);
    }
    let event = driver.read1()?;
    println!("named_temp_file event: {:?}", event);
    assert!(event.mask().contains(Mask::OPEN | Mask::ACCESS | Mask::MODIFY | Mask::CLOSE_WRITE));
    Ok(())
}

#[test]
fn forever() {
    with_fanotify(|fanotify| {
        fanotify.mark(Mark::one(mark::One {
            action: Add,
            what: MountPoint,
            flags: mark::Flags::empty(),
            mask: Mask::ACCESS
                | Mask::OPEN
                | Mask::close()
                | Mask::MODIFY,
            path: mark::Path::absolute("/home"),
        }).unwrap())?;
        let mut fanotify = fanotify.buffered_default();
        loop {
            for event in fanotify.read()?.all() {
                let event = event?;
                print!("{}, {:05?}, {:20?}", event.file().variant_name(), event.id().id(), event.mask());
                if let Some(path) = event.into_file().path() {
                    println!(": {}", path?.display());
                }
            }
        }
    })
}
