use crate::mark;

use super::{
    file::{
        fd::FileFD,
        fid::FileFID,
        File,
        permission::FilePermission,
    },
    id::EventId,
};

#[derive(Debug)]
pub struct EventOf<FileT> {
    pub(super) mask: mark::Mask,
    pub(super) id: EventId,
    pub(super) file: FileT,
}

impl<FileT> EventOf<FileT> {
    pub fn mask(&self) -> mark::Mask {
        self.mask
    }
    
    pub fn id(&self) -> &EventId {
        &self.id
    }
    
    pub fn file(&self) -> &FileT {
        &self.file
    }
    
    pub fn into_file(self) -> FileT {
        self.file
    }
}

/// A full file event
///
/// It contains:
/// * a [`Mask`](mark::Mask) specifying the type of event
/// * an [`EventId`] of who created the event
/// * the actual [`File`] event
///
/// Most of the [`Event`] is copied from
/// the raw [`fanotify_event_metadata`](crate::libc::read::fanotify_event_metadata)
/// and [`fanotify_event_info_fid`](crate::libc::read::fanotify_event_info_fid) structs,
/// but some fields, namely the [`FileHandle`](super::file::fid::FileHandle),
/// cannot be copied because they are opaque, variable-length fields.
/// Thus, they are the only references in the [`Event`].
pub type Event<'a> = EventOf<File<'a>>;

impl<'a> Event<'a> {
    fn into_variant<FileT>(self, project: impl Fn(File<'a>) -> Option<FileT>) -> Option<EventOf<FileT>> {
        let Self { mask, id, file } = self;
        Some(EventOf {
            mask,
            id,
            file: project(file)?,
        })
    }
    
    /// Return the [`FileFD`] variant if it exists.
    pub fn fd(self) -> Option<EventOf<FileFD>> {
        self.into_variant(|it| it.fd())
    }
    
    /// Return the [`FileFID`] variant if it exists.
    pub fn fid(self) -> Option<EventOf<FileFID<'a>>> {
        self.into_variant(|it| it.fid())
    }
    
    /// Return the [`FilePermission`] variant if it exists.
    pub fn permission(self) -> Option<EventOf<FilePermission<'a>>> {
        self.into_variant(|it| it.permission())
    }
}
