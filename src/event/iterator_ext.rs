use std::iter::FilterMap;

use super::error::EventResult;
use super::event::Event;
use super::event::EventOf;
use super::file::fd::FileFD;
use super::file::fid::FileFID;
use super::file::permission::FilePermission;

type UnwrapEventResult<'a> = fn(EventResult<'a>) -> Option<Event<'a>>;
type ProjectEvent<'a, FileT> = fn(Event<'a>) -> Option<EventOf<FileT>>;

pub trait IntoEvents<'a>: IntoIterator<Item=EventResult<'a>> + Sized {
    /// An [`Iterator`] over all [`EventResult`]s, so including errors and [`Event`]s.
    fn all(self) -> Self::IntoIter {
        self.into_iter()
    }
    
    /// An [`Iterator`] over all non-error [`Event`]s.
    ///
    /// Note that the long return type is necessary.
    /// I can't use impl Trait in a trait.
    fn ok(self) -> FilterMap<Self::IntoIter, UnwrapEventResult<'a>> {
        self.all().filter_map(|it| it.ok())
    }
    
    /// An [`Iterator`] over all [`Event`]s containing a [`FileFD`].
    ///
    /// Note that the long return type is necessary.
    /// I can't use impl Trait in a trait.
    fn fds(self) -> FilterMap<FilterMap<Self::IntoIter, UnwrapEventResult<'a>>, ProjectEvent<'a, FileFD>> {
        self.ok().filter_map(|it| it.fd())
    }
    
    /// An [`Iterator`] over all [`Event`]s containing a [`FileFID`].
    ///
    /// Note that the long return type is necessary.
    /// I can't use impl Trait in a trait.
    fn fids(self) -> FilterMap<FilterMap<Self::IntoIter, UnwrapEventResult<'a>>, ProjectEvent<'a, FileFID<'a>>> {
        self.ok().filter_map(|it| it.fid())
    }
    
    /// An [`Iterator`] over all [`Event`]s containing a [`FilePermission`].
    ///
    /// Note that the long return type is necessary.
    /// I can't use impl Trait in a trait.
    fn permissions(self) -> FilterMap<FilterMap<Self::IntoIter, UnwrapEventResult<'a>>, ProjectEvent<'a, FilePermission<'a>>> {
        self.ok().filter_map(|it| it.permission())
    }
}
