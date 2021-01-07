use std::{
    cell::RefCell,
    mem::size_of,
    slice,
};

use nix::errno::Errno;

use super::super::{
    descriptor::Fanotify,
    libc::write::fanotify_response,
};

impl fanotify_response {
    /// Reinterpret as a byte slice for [`writing`](libc::write) to a [`Fanotify`] instance.
    fn as_bytes(&self) -> &[u8] {
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

// TODO figure out a way to reuse this buffer b/w Events reads

#[derive(Default)]
struct ResponseBuffer {
    buffer: Vec<u8>,
}

impl ResponseBuffer {
    fn new() -> Self {
        Self::default()
    }
    
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
    
    pub fn has_more(&self) -> bool {
        !self.is_empty()
    }
    
    /// Add another raw [`fanotify_response`] to the buffer.
    fn add(&mut self, response: &fanotify_response) {
        self.buffer.extend_from_slice(response.as_bytes());
    }
    
    /// Attempt to [`write`](libc::write) the buffer to the [`Fanotify`] instance.
    /// It also removes what has been written from the buffer,
    /// so this method can be called repeatedly until [`Responses::is_empty`] is true.
    fn write(&mut self, fanotify: &Fanotify) -> Result<usize, Errno> {
        let bytes_written = fanotify.fd.write(self.buffer.as_slice())?;
        // this drain call is O(n) even for small bytes_written, so write_all() is O(n^2)
        // could use a deque instead, but this should be a rare case
        // since the whole buffer should normally be written at once,
        // making write_all() O(n) in practice
        self.buffer.drain(0..bytes_written);
        Ok(bytes_written)
    }
    
    /// Write the entire buffer to the [`Fanotify`] instance.
    ///
    /// This keeps calling [`Responses::write`] until either
    /// all of the responses have been written
    /// or one of the writes throws an error, in which case we exit early with the error.
    fn write_all(&mut self, fanotify: &Fanotify) -> Result<(), Errno> {
        while self.has_more() {
            self.write(fanotify)?;
        }
        Ok(())
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
    responses: RefCell<ResponseBuffer>,
}

impl<'a> Responses<'a> {
    /// Create a [`Responses`] buffer writing to the given [`Fanotify`] instance.
    pub(super) fn new(fanotify: &'a Fanotify) -> Self {
        Self {
            fanotify,
            responses: RefCell::new(ResponseBuffer::new()),
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.responses.borrow().is_empty()
    }
    
    pub fn has_more(&self) -> bool {
        !self.is_empty()
    }
    
    pub(super) fn write_immediately(&self, response: &fanotify_response) -> Result<(), Errno> {
        let bytes_written = self.fanotify.fd.write(response.as_bytes())?;
        // a write this small should definitely succeed, so only try once
        match bytes_written {
            0 => Ok(()),
            _ => return Err(Errno::EAGAIN),
        }
    }
    
    /// Add another raw [`fanotify_response`] to the buffer.
    pub(super) fn write_buffered(&self, response: &fanotify_response) {
        self.responses.borrow_mut().add(response);
    }
    
    /// Attempt to [`write`](libc::write) the buffer to the [`Fanotify`] instance.
    /// It also removes what has been written from the buffer,
    /// so this method can be called repeatedly until [`Responses::is_empty`] is true.
    pub fn flush(&self) -> Result<usize, Errno> {
        self.responses.borrow_mut().write(self.fanotify)
    }
    
    /// Write the entire buffer to the [`Fanotify`] instance.
    ///
    /// This keeps calling [`Responses::write`] until either
    /// all of the responses have been written
    /// or one of the writes throws an error, in which case we exit early with the error.
    pub fn flush_all(&self) -> Result<(), Errno> {
        self.responses.borrow_mut().write_all(self.fanotify)
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
        self.flush_all().expect(
            "Responses::write_all() threw in Responses::drop().  \
                To handle this, call Responses::write_all() yourself first."
        );
    }
}
