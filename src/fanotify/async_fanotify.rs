use std::io;

use async_io::Async;

use crate::event::buffer::EventBuffer;
use crate::event::events::Events;
use crate::fanotify::Fanotify;
use crate::mark;
use crate::mark::Mark;
use crate::mark::Markable;

/// An async version of [`Fanotify`].
pub struct AsyncFanotify {
    inner: Async<Fanotify>,
}

impl AsyncFanotify {
    pub fn new(fanotify: Fanotify) -> io::Result<Self> {
        let this = Self {
            inner: Async::new(fanotify)?,
        };
        Ok(this)
    }
}

impl Fanotify {
    pub fn into_async(self) -> io::Result<AsyncFanotify> {
        AsyncFanotify::new(self)
    }
}

impl AsyncFanotify {
    pub fn fanotify(&self) -> &Fanotify {
        self.inner.get_ref()
    }
    
    pub fn fanotify_mut(&mut self) -> &mut Fanotify {
        self.inner.get_mut()
    }
    
    pub fn into_fanotify(self) -> io::Result<Fanotify> {
        self.inner.into_inner()
    }
    
    pub fn into_sync(self) -> io::Result<Fanotify> {
        self.into_fanotify()
    }
    
    pub fn into_inner(self) -> io::Result<Fanotify> {
        self.into_fanotify()
    }
}

impl Markable for AsyncFanotify {
    fn mark<'a>(&self, mark: Mark<'a>) -> Result<(), mark::Error<'a>> {
        self.fanotify().mark(mark)
    }
}

impl AsyncFanotify {
    /// Read file events from the wrapped [`Fanotify`] group into the given buffer.
    ///
    /// Return an [`Events`] iterator over the individual events.
    ///
    /// This method does not block.
    ///
    /// However, while reads are handled asynchronously,
    /// writes (permission responses) are not.
    /// This is because there is no `AsyncDrop`, so I cannot flush writes in [`Drop::drop`].
    /// However, the fanotify file descriptor
    /// is still placed in non-blocking mode by [`AsyncFanotify::new`],
    /// meaning the writes must happen without blocking,
    /// or else [`Errno::EAGAIN`](nix::errno::Errno::EAGAIN) will be thrown.
    /// This likely won't happen though,
    /// since writing permission responses to a fanotify file descriptor shouldn't normally block.
    pub async fn read<'a>(&'a self, buffer: &'a mut EventBuffer) -> io::Result<Events<'a>> {
        self.inner.readable().await?;
        let events = self.fanotify().read(buffer)?;
        Ok(events)
    }
}