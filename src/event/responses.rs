use std::cell::RefCell;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::mem::size_of;
use std::rc::Rc;
use std::slice;

use nix::errno::Errno;
use to_trait::To;

use super::file::permission::RawFilePermission;
use super::super::fanotify::Fanotify;
use super::super::libc::write::fanotify_response;

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

/// A buffer of responses to fanotify [`Event`](super::event::Event)s.
///
/// A [`ResponseBuffer`] can be explicitly written to a [`Fanotify`] instance
/// using [`ResponseBuffer::write`] or [`ResponseBuffer::write_all`].
struct ResponseBuffer<'a> {
    buffer: &'a mut Vec<u8>,
}

impl<'a> ResponseBuffer<'a> {
    fn new(buffer: &'a mut Vec<u8>) -> Self {
        buffer.clear();
        Self {
            buffer,
        }
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
    /// so this method can be called repeatedly until [`ResponseBuffer::is_empty`] is true.
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
    /// This keeps calling [`ResponseBuffer::write`] until either
    /// all of the responses have been written
    /// or one of the writes throws an error, in which case we exit early with the error.
    fn write_all(&mut self, fanotify: &Fanotify) -> Result<(), Errno> {
        while self.has_more() {
            self.write(fanotify)?;
        }
        Ok(())
    }
    
    pub fn responses(&self) -> &[fanotify_response] {
        // the buffer could slice a fanotify_response in half when writing
        // but from the back it should be all contiguous
        // so we just skip the offset at the beginning and cast to &[fanotify_response]
        let offset = self.buffer.len() % size_of::<fanotify_response>();
        let len = self.buffer.len() / size_of::<fanotify_response>();
        unsafe {
            let ptr = self.buffer.as_ptr().add(offset) as *const fanotify_response;
            slice::from_raw_parts(ptr, len)
        }
    }
}

impl Debug for ResponseBuffer<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, response) in self.responses().iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            match response.try_to::<RawFilePermission>() {
                Ok(response) => {
                    write!(f, "{:?}", response)?;
                }
                Err(()) => {
                    write!(f, "Err")?;
                }
            };
        }
        write!(f, "]")?;
        Ok(())
    }
}

/// A buffer of responses to fanotify [`Event`](super::event::Event)s.
///
/// A [`fanotify_response`] can be written to this [`Responses`] buffer
/// either using [`Responses::write_immediately`],
/// which immediately writes it to its [`Fanotify`] instance,
/// or using [`Responses::write_buffered`],
/// which writes it to the buffer of responses.
///
/// Then this buffer can be written to its [`Fanotify`] instance,
/// either explicitly with [`Responses::flush`] or [`Responses::flush_all`],
/// or implicitly on [`Responses::drop`].
/// Errors can only be handled when flushing it explicitly, however,
/// since errors can't be returned from [`Drop::drop`].
#[derive(Debug)]
pub struct Responses<'a> {
    fanotify: &'a Fanotify,
    responses: RefCell<ResponseBuffer<'a>>,
}

impl<'a> Responses<'a> {
    /// Create a [`Responses`] buffer writing to the given [`Fanotify`] instance.
    pub(super) fn new(fanotify: &'a Fanotify, buffer: &'a mut Vec<u8>) -> Self {
        Self {
            fanotify,
            responses: RefCell::new(ResponseBuffer::new(buffer)),
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.responses.borrow().is_empty()
    }
    
    pub fn has_more(&self) -> bool {
        !self.is_empty()
    }
    
    /// [`Write`](libc::write) a raw [`fanotify_response`] immediately to the [`Fanotify`] instance.
    pub(super) fn write_immediately(&self, response: &RawFilePermission) -> Result<(), Errno> {
        let bytes_written = self.fanotify.fd.write(response.to::<fanotify_response>().as_bytes())?;
        // a write this small should definitely succeed, so only try once
        match bytes_written {
            0 => Ok(()),
            _ => Err(Errno::EAGAIN),
        }
    }
    
    /// Write a raw [`fanotify_response`] to the buffer.
    pub(super) fn write_buffered(&self, response: &RawFilePermission) {
        self.responses.borrow_mut().add(&response.into());
    }
    
    /// Attempt to [`write`](libc::write) the buffer to the [`Fanotify`] instance.
    /// It also removes what has been written from the buffer,
    /// so this method can be called repeatedly until [`Responses::is_empty`] is true.
    pub fn flush(&self) -> Result<usize, Errno> {
        self.responses.borrow_mut().write(self.fanotify)
    }
    
    /// Write the entire buffer to the [`Fanotify`] instance.
    ///
    /// This keeps calling [`Responses::flush`] until either
    /// all of the responses have been written
    /// or one of the writes throws an error, in which case we exit early with the error.
    pub fn flush_all(&self) -> Result<(), Errno> {
        self.responses.borrow_mut().write_all(self.fanotify)
    }
}

/// Make sure the responses always get written by calling [`Responses::flush_all`].
///
/// See [`Responses::drop`].
impl Drop for Responses<'_> {
    /// Make sure the responses always get written by calling [`Responses::flush_all`].
    ///
    /// This panics on error.
    /// To handle the error, first call [`Responses::flush_all`] until it returns `Ok(())`.
    fn drop(&mut self) {
        self.flush_all().expect(
            "Responses::write_all() threw in Responses::drop().  \
                To handle this, call Responses::write_all() yourself first."
        );
    }
}

/// Parameterized reference counter here just to simplify things a bit.
/// [`Arc`](std::sync::Arc) doesn't work for now.
pub type RC<T> = Rc<T>;
