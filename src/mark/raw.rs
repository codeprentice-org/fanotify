use std::{
    ffi::CString,
    os::{
        raw::c_char,
        unix::{
            ffi::OsStringExt,
            io::{AsRawFd, RawFd},
        },
    },
};
use std::os::raw::c_int;

use static_assertions::_core::marker::PhantomData;

use crate::fanotify::Fanotify;
use crate::libc::call::{RawSysCall, SysCall};

use super::Mark;

pub type RawFlags = u32;

#[derive(Debug, PartialEq, Hash)]
pub struct RawMark {
    pub(crate) flags: u32,
    pub(crate) mask: u64,
    pub(crate) dir_fd: RawFd,
    pub(crate) path: Option<CString>,
}

impl RawMark {
    pub fn path_ptr(&self) -> *const c_char {
        match &self.path {
            None => std::ptr::null(),
            Some(path) => path.as_ptr(),
        }
    }
}

impl<'a> Mark<'a> {
    pub const fn raw_flags(&self) -> RawFlags {
        self.action as u32 | self.what as u32 | self.flags.bits()
    }
    
    pub fn to_raw(&self) -> RawMark {
        RawMark {
            flags: self.raw_flags(),
            mask: self.mask.bits(),
            dir_fd: self.path.dir.as_raw_fd(),
            path: self
                .path
                .path
                .map(|path| path.as_os_str().to_os_string().into_vec())
                .map(|bytes| unsafe {
                    // Path can't have null bytes so this is safe
                    CString::from_vec_unchecked(bytes)
                }),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RawFanotifyMark<'a> {
    pub fd: RawFd,
    pub flags: u32,
    pub mask: u64,
    pub dir_fd: RawFd,
    pub path: *const c_char,
    phantom: PhantomData<&'a ()>,
}

impl RawSysCall for RawFanotifyMark<'_> {
    type Output = c_int;
    
    fn name() -> &'static str {
        "fanotify_mark"
    }
    
    unsafe fn unsafe_call(&self) -> Self::Output {
        libc::fanotify_mark(self.fd, self.flags, self.mask, self.dir_fd, self.path)
    }
}

#[derive(Debug)]
pub(crate) struct FanotifyMark<'a> {
    pub fanotify: &'a Fanotify,
    pub mark: &'a Mark<'a>,
}

impl<'a> SysCall for FanotifyMark<'a> {
    type Raw = RawFanotifyMark<'a>;
    type Output = ();
    
    fn to_raw(&self) -> Self::Raw {
        let raw = self.mark.to_raw();
        Self::Raw {
            fd: self.fanotify.fd.as_raw_fd(),
            flags: raw.flags,
            mask: raw.mask,
            dir_fd: raw.dir_fd,
            path: raw.path_ptr(),
            phantom: PhantomData,
        }
    }
    
    fn convert_output(output: c_int) {
        assert_eq!(output, 0);
    }
}
