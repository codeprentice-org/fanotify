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
