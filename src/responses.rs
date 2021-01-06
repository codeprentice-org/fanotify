use std::mem::size_of;

use nix::errno::Errno;

use crate::descriptor::Fanotify;
use crate::libc::write::fanotify_response;
use std::slice;

impl fanotify_response {
    /// Reinterpret as a byte slice for [`writing`](libc::write) to a [`Fanotify`] instance.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        // Safe b/c fanotify_response is repr(C)
        // and is meant to be written to a file descriptor as bytes anyways.
        // It also returns an immutable slice,
        // so it cannot put the fanotify_response itself into an undefined state.
        // Even if it could modify it, fanotify_response is just an i32 and u32,
        // so it's always valid no matter the byte representation.
        unsafe {
            slice::from_raw_parts(
                self as *const fanotify_response as *const u8,
                size_of::<Self>(),
            )
        }
    }
}

/// A buffer of responses to fanotify [`Event`](crate::event::Event)s.
///
/// In order to buffer the responses to avoid many [`write`](libc::write)s,
/// it is necessary to keep this [`Responses`] buffer
/// separate from the [`Event`](crate::event::Event)s themselves,
/// due to lifetime, mutability, and [`Iterator`] requirements.
///
/// A [`Responses`] buffer can be explicitly written to its [`Fanotify`] instance
/// using [`Responses::write`] or [`Responses::write_all`],
/// or else the write will be attempted in [`Responses::drop`].
///
///
pub struct Responses<'a> {
    fanotify: &'a Fanotify,
    responses: Vec<u8>,
}

impl<'a> Responses<'a> {
    /// Create a [`Responses`] buffer writing to the given [`Fanotify`] instance.
    pub fn new(fanotify: &'a Fanotify) -> Self {
        Self {
            fanotify,
            responses: Vec::new(),
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.responses.is_empty()
    }
    
    pub fn has_more(&self) -> bool {
        !self.is_empty()
    }
    
    /// Add another raw [`fanotify_response`] to the buffer.
    pub(crate) fn add(&mut self, response: &fanotify_response) {
        self.responses.extend_from_slice(response.as_bytes());
    }
    
    /// Attempt to [`write`](libc::write) the buffer to the [`Fanotify`] instance.
    /// It also removes what has been written from the buffer,
    /// so this method can be called repeatedly until [`Responses::is_empty`] is true.
    pub fn write(&mut self) -> Result<usize, Errno> {
        let bytes_written = self.fanotify.fd.write(self.responses.as_slice())?;
        self.responses.drain(0..bytes_written);
        Ok(bytes_written)
    }
    
    /// Write the entire buffer to the [`Fanotify`] instance.
    ///
    /// This keeps calling [`Responses::write`] until either
    /// all of the responses have been written
    /// or one of the writes throws an error, in which case we exit early with the error.
    pub fn write_all(&mut self) -> Result<(), Errno> {
        while self.has_more() {
            self.write()?;
        }
        Ok(())
    }
}

/// Make sure the responses always get written by calling [`Responses::write_all`].
///
/// See [`Responses::drop`].
impl Drop for Responses<'_> {
    /// Make sure the responses always get written by calling [`Responses::write_all`].
    ///
    /// This panics on error.
    /// To handle the error, first call [`Responses::write_all`] until it returns `Ok(())`.
    fn drop(&mut self) {
        self.write_all().expect(
            "Responses::write_all() threw in Responses::drop().  \
                To handle this, call Responses::write_all() yourself first."
        );
    }
}
