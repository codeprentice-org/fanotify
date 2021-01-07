use std::{
    mem::size_of,
    rc::Rc,
    slice,
};

use nix::errno::Errno;

use crate::{
    descriptor::Fanotify,
    init,
};

use super::{
    error,
    event::{Event, EventOf},
    file::{
        fd::FileFD,
        fid::FileFID,
        permission::FilePermission,
    },
    id::Id,
    responses::Responses,
};

/// A buffer of [`Event`]s from one [`Fanotify::read`] call.
///
/// The individual [`Event`]s can only be iterated over because they are variable-length.
pub struct Events<'a> {
    fanotify: &'a Fanotify,
    id: Id,
    buffer: &'a mut Vec<u8>,
    responses: Rc<Responses<'a>>,
}

impl<'a> Events<'a> {
    pub fn fanotify(&self) -> &'a Fanotify {
        self.fanotify
    }
    
    pub fn id(&self) -> Id {
        self.id
    }
    
    pub(super) fn responses(&self) -> Rc<Responses<'a>> {
        self.responses.clone()
    }
    
    pub(super) fn as_bytes(&self) -> &[u8] {
        self.buffer.as_slice()
    }
}

impl<'a> Events<'a> {
    /// Construct an [`Events`] by reading from a [`Fanotify`] into a given buffer.
    ///
    /// Returns an error only if the [`FD::read`](crate::common::FD::read) call
    /// returns an [`Errno`], which wraps [`libc::read`].
    pub(in super::super) fn read(
        fanotify: &'a Fanotify,
        buffer: &'a mut Vec<u8>,
    ) -> std::result::Result<Self, Errno> {
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
            responses: Rc::new(Responses::new(fanotify)),
        };
        Ok(this)
    }
}

impl<'a> Events<'a> {
    /// An [`Iterator`] over all [`Result`]s, so including errors and [`Event`]s.
    pub fn all(self) -> impl Iterator<Item=error::EventResult<'a>> {
        self.into_iter()
    }
    
    /// An [`Iterator`] over all non-error [`Event`]s.
    pub fn ok(self) -> impl Iterator<Item=Event<'a>> {
        self.all().filter_map(|it| it.ok())
    }
    
    /// An [`Iterator`] over all [`Event`]s containing a [`FileFD`].
    pub fn fds(self) -> impl Iterator<Item=EventOf<FileFD>> + 'a {
        self.ok().filter_map(|it| it.fd())
    }
    
    /// An [`Iterator`] over all [`Event`]s containing a [`FileFID`].
    pub fn fids(self) -> impl Iterator<Item=EventOf<FileFID<'a>>> {
        self.ok().filter_map(|it| it.fid())
    }
    
    /// An [`Iterator`] over all [`Event`]s containing a [`FilePermission`].
    pub fn permissions(self) -> impl Iterator<Item=EventOf<FilePermission<'a>>> {
        self.ok().filter_map(|it| it.permission())
    }
}
