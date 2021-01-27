use std::mem::size_of;
use std::slice;

use nix::errno::Errno;

use crate::event::buffer::EventBuffer;
use crate::fanotify::Fanotify;
use crate::init;

use super::id::Id;
use super::responses::RC;
use super::responses::Responses;

/// A buffer of [`Event`]s from one [`Fanotify::read`] call.
///
/// The individual [`Event`]s can only be iterated over because they are variable-length.
///
/// [`Event`]: super::event::Event
pub struct Events<'a> {
    fanotify: &'a Fanotify,
    id: Id,
    buffer: &'a mut Vec<u8>,
    responses: RC<Responses<'a>>,
}

impl<'a> Events<'a> {
    pub fn fanotify(&self) -> &'a Fanotify {
        self.fanotify
    }
    
    pub fn id(&self) -> Id {
        self.id
    }
    
    pub(super) fn responses(&self) -> RC<Responses<'a>> {
        self.responses.clone()
    }
    
    pub(super) fn as_bytes(&self) -> &[u8] {
        self.buffer.as_slice()
    }
}

impl<'a> Events<'a> {
    /// Construct an [`Events`] by reading from a [`Fanotify`] into a given buffer.
    ///
    /// Returns an error only if the [`FD::read`](crate::fd::FD::read) call
    /// returns an [`Errno`], which wraps [`libc::read`].
    pub(in super::super) fn read(
        fanotify: &'a Fanotify,
        buffer: &'a mut EventBuffer,
    ) -> std::result::Result<Self, Errno> {
        let EventBuffer {
            events: buffer,
            responses: response_buffer,
        } = buffer;
        buffer.clear();
        
        // want to use this, but it's unstable
        // reads.spare_capacity_mut()
        let read_buffer = {
            let ptr = buffer.as_mut_slice().as_mut_ptr();
            let len = buffer.capacity() * size_of::<u8>();
            unsafe { slice::from_raw_parts_mut(ptr, len) }
        };
        let bytes_read = fanotify.fd.read(read_buffer)?;
        unsafe { buffer.set_len(bytes_read) };
        
        // id is read here for two reason
        // 1. it caches it for this set of events
        // 2. it ensures the id is correct, b/c if you read the id later,
        //    it could be different than when the read occurred
        let use_tid = fanotify.init.flags().contains(init::Flags::REPORT_TID);
        let id = Id::current(use_tid);
        
        let this = Self {
            fanotify,
            id,
            buffer,
            responses: RC::new(Responses::new(fanotify, response_buffer)),
        };
        Ok(this)
    }
}
