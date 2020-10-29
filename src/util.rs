use std::ops::Neg;
use std::os::raw::c_int;

use nix::errno::Errno;
use thiserror::Error;

pub trait ZeroOne {
    const ZERO: Self;
    const ONE: Self;
}

macro_rules! impl_zero_one {
    ($($t:ident)*) => ($(impl ZeroOne for $t {
        const ZERO: Self = 0 as $t;
        const ONE: Self = 1 as $t;
    })*)
}

impl_zero_one! { u8 i8 u16 i16 u32 i32 u64 i64 usize isize f32 f64 }

pub fn libc_call<T: ZeroOne + Copy + Eq + Neg<Output=T>, F: FnOnce() -> T>(f: F) -> Result<T, Errno> {
    Errno::clear();
    let result = f();
    if result == T::ONE.neg() {
        let errno = Errno::last();
        Errno::clear();
        Err(errno)
    } else {
        Ok(result)
    }
}

pub fn libc_void_call<T: ZeroOne + Copy + Eq + Neg<Output=T>, F: FnOnce() -> T>(f: F) -> Result<(), Errno> {
    if libc_call(f)? == T::ZERO {
        Ok(())
    } else {
        unreachable!()
    }
}

#[derive(Error, Debug)]
#[error("impossible error in syscall {}({}): {:?}", .syscall, .args, .errno)]
pub struct ImpossibleError {
    pub syscall: &'static str,
    pub args: String,
    pub errno: Errno,
}
