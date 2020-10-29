use std::os::raw::c_int;

use nix::errno::Errno;
use thiserror::Error;

pub fn libc_call<F: FnOnce() -> c_int>(f: F) -> Result<c_int, Errno> {
    Errno::clear();
    match f() {
        -1 => {
            let errno = Errno::last();
            Errno::clear();
            Err(errno)
        },
        result if result >= 0 => Ok(result),
        _ => unreachable!(),
    }
}

pub fn libc_void_call<F: FnOnce() -> c_int>(f: F) -> Result<(), Errno> {
    match libc_call(f)? {
        0 => Ok(()),
        _ => unreachable!(),
    }
}

#[derive(Error, Debug)]
#[error("impossible error in syscall {}({}): {:?}", .syscall, .args, .errno)]
pub struct ImpossibleError {
    pub syscall: &'static str,
    pub args: String,
    pub errno: Errno,
}
