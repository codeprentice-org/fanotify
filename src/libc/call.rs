use std::fmt::Debug;
use std::ops::Neg;

use nix::errno::Errno;
use thiserror::Error;

/// Defines 0 or 1 for every number type using a macro.
///
/// This allows you to write libc calls generically so that it can
/// handle multiple integer types, e.g. i32, i64, or u32.
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

/// Make a libc call, detecting -1 return values
/// and return an [`Err`] with an [`Errno`] in that case.
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

// /// Make a libc call like [`libc_call`], except throw away the return value.
// pub fn libc_void_call<T: ZeroOne + Copy + Eq + Neg<Output=T>, F: FnOnce() -> T>(f: F) -> Result<(), Errno> {
//     if libc_call(f)? == T::ZERO {
//         Ok(())
//     } else {
//         unreachable!()
//     }
// }

pub trait RawSysCall: Debug {
    type Output: ZeroOne + Copy + Eq + Neg<Output=Self::Output>;
    fn name() -> &'static str;
    
    unsafe fn unsafe_call(&self) -> Self::Output;
    
    fn call(&self) -> Result<Self::Output, Errno> {
        libc_call(|| unsafe { self.unsafe_call() })
    }
}

pub trait SysCall: Debug {
    type Raw: RawSysCall;
    type Output;
    
    fn to_raw(&self) -> Self::Raw;
    
    fn convert_output(output: <Self::Raw as RawSysCall>::Output) -> Self::Output;
    
    fn call(&self) -> Result<Self::Output, SysCallError<Self>> {
        let raw = self.to_raw();
        raw.call()
            .map(Self::convert_output)
            .map_err(|errno| SysCallError {
                syscall: Self::Raw::name(),
                raw_args: raw,
                args: self,
                errno,
            })
    }
}

/// A syscall error containing all the info on the syscall,
/// including the syscall name,
/// the raw named arguments,
/// the higher level arguments,
/// and the [`Errno`] received.
#[derive(Error, Debug)]
#[error("error in syscall {}({:?}; {:?}) = {:?}", .syscall, .raw_args, .args, .errno)]
pub struct SysCallError<'a, Args: SysCall + ?Sized> {
    pub syscall: &'static str,
    pub raw_args: Args::Raw,
    pub args: &'a Args,
    pub errno: Errno,
}

impl<Args: SysCall> SysCallError<'_, Args> {
    /// Panic because a [`SysCallError`] occurred
    /// that should be impossible according to how the syscall is documented.
    /// For example, an [`EINVAL`](Errno::EINVAL) even though all the arguments are indeed valid.
    /// Or an [`Errno`] error that the syscall is not documented as ever returning.
    pub fn impossible(&self) -> ! {
        panic!("impossible {}", self);
    }
}
