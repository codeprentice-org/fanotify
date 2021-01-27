use std::convert::TryInto;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::{Path, PathBuf};

use async_io::block_on;
use tempfile::NamedTempFile;
use tempfile::tempfile;
use to_trait::To;

use fanotify::buffered_fanotify::IntoBufferedFanotify;
use fanotify::event::iterator_ext::IntoEvents;
use fanotify::init;
use fanotify::init::Flags;
use fanotify::init::Init;
use fanotify::mark;
use fanotify::mark::Markable;
use fanotify::mark::Mask;
use fanotify::mark::OneAction::Add;
use fanotify::mark::What::FileSystem;
use fanotify::mark::What::MountPoint;

use crate::util::driver::Driver;
use crate::util::get_init;
use crate::util::supported::Supported;
use crate::util::supported::Supported::Full;
use crate::util::supported::Supported::Partial;
use crate::util::supported::supports;

mod util;

type AnyResult<T = ()> = anyhow::Result<T>;

#[test]
fn init_unsupported() {
    let init = Init {
        flags: Flags::unlimited(),
        ..Init::const_default()
    };
    assert_eq!(
        init.to_fanotify().err(),
        Some(init::Error::FanotifyUnsupported)
            .filter(|_| !supports(Partial)),
    );
}

#[test]
fn report_fid_unsupported() {
    if !supports(Partial) {
        return;
    }
    let init = Init {
        flags: Flags::unlimited() | Flags::REPORT_FID,
        ..Init::const_default()
    };
    assert_eq!(
        init.to_fanotify().err(),
        Some(init::Error::FeatureUnsupported)
            .filter(|_| !supports(Full)),
    );
}

fn mark_unsupported(error: mark::RawError, mark: mark::One) -> AnyResult {
    if !supports(Partial) {
        return Ok(());
    }
    let fanotify = get_init().to_fanotify()?;
    let e = fanotify.mark(mark.try_into()?);
    assert_eq!(
        e.err().map(|it| it.error),
        Some(error)
            .filter(|_| !supports(Full)),
    );
    Ok(())
}

#[test]
fn filesystem_mark_unsupported() -> AnyResult {
    mark_unsupported(mark::RawError::FeatureUnsupported, mark::One {
        action: Add,
        what: FileSystem,
        flags: mark::Flags::empty(),
        mask: Mask::ACCESS
            | Mask::OPEN
            | Mask::close()
            | Mask::MODIFY,
        path: mark::Path::absolute("/home"),
    })
}

#[test]
fn create_mask_unsupported() -> AnyResult {
    mark_unsupported(mark::RawError::FeatureUnsupported, mark::One {
        action: Add,
        what: MountPoint,
        flags: mark::Flags::empty(),
        mask: Mask::CREATE
            | Mask::ACCESS
            | Mask::OPEN
            | Mask::close()
            | Mask::MODIFY,
        path: mark::Path::absolute("/home"),
    })
}

fn mark_and_read(read1: impl Fn(Driver) -> io::Result<(Mask, Option<io::Result<PathBuf>>)>) -> AnyResult {
    if !supports(Partial) {
        return Ok(());
    }
    let driver = get_init()
        .to_fanotify()?
        .buffered_default()
        .to::<Driver>();
    driver.fanotify.mark(mark::One {
        action: Add,
        what: MountPoint,
        flags: mark::Flags::empty(),
        mask: Mask::ACCESS
            | Mask::OPEN
            | Mask::close()
            | Mask::MODIFY,
        path: mark::Path::absolute("/etc"),
    }.try_into()?)?;
    let path = Path::new("/etc/passwd");
    let _ = fs::read(path)?;
    let (mask, event_path) = read1(driver)?;
    assert_eq!(mask, Mask::OPEN | Mask::ACCESS | Mask::CLOSE_NO_WRITE);
    assert_eq!(event_path.unwrap()?.as_path(), path);
    Ok(())
}

#[test]
fn sync_api() -> AnyResult {
    mark_and_read(|mut driver| {
        let event = driver.read1()?;
        Ok((event.mask(), event.file().path()))
    })
}

#[test]
fn async_api() -> AnyResult {
    mark_and_read(|driver| {
        let mut driver = driver.into_async()?;
        let event = block_on(driver.read1())?;
        let mask = event.mask();
        let path = event.file().path();
        drop(event);
        let _ = driver.into_sync()?;
        Ok((mask, path))
    })
}

fn tmp_file(driver: &mut Driver, text: &str, mut file: impl Read) -> AnyResult {
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    assert_eq!(text, buf);
    drop(file);
    let event = driver.read1()?;
    println!("tmp: {:?}", event);
    assert_eq!(event.mask(), Mask::OPEN | Mask::ACCESS | Mask::MODIFY | Mask::CLOSE_WRITE);
    Ok(())
}

#[test]
fn tmp() -> AnyResult {
    if !supports(Partial) {
        return Ok(());
    }
    let mut driver = get_init()
        .to_fanotify()?
        .buffered_default()
        .to::<Driver>();
    driver.fanotify.mark(mark::One {
        action: Add,
        what: MountPoint,
        flags: mark::Flags::empty(),
        mask: Mask::ACCESS
            | Mask::OPEN
            | Mask::close()
            | Mask::MODIFY,
        path: mark::Path::absolute("/tmp"),
    }.try_into()?)?;
    let text = "test";
    // un-named
    tmp_file(&mut driver, text, {
        let mut f = tempfile()?;
        f.write_all(text.as_bytes())?;
        f.seek(SeekFrom::Start(0))?;
        f
    })?;
    // named
    tmp_file(&mut driver, text, {
        let mut temp_file = NamedTempFile::new()?;
        let f = temp_file.reopen()?;
        temp_file.write_all(text.as_bytes())?;
        f
    })?;
    Ok(())
}

#[test]
#[ignore]
fn forever() -> AnyResult {
    let (what, mask) = match Supported::get() {
        Supported::None => return Ok(()),
        Partial => (MountPoint, Mask::empty()
            | Mask::ACCESS
            | Mask::OPEN
            | Mask::close()
            | Mask::MODIFY
        ),
        Full => (FileSystem, Mask::all() & !Mask::all_permissions()),
    };
    let mut fanotify = get_init().to_fanotify()?.buffered_default();
    fanotify.mark(mark::One {
        action: Add,
        what,
        flags: mark::Flags::empty(),
        mask,
        path: mark::Path::absolute("/home"),
    }.try_into()?)?;
    loop {
        for event in fanotify.read()?.all() {
            let event = event?;
            println!("{}", &event.display());
        }
    }
}
