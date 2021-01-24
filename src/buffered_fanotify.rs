use std::io;

use crate::event::buffer::EventBufferSize;
use crate::event::events::Events;

use super::{
    async_fanotify::AsyncFanotify,
    event::buffer::EventBuffer,
    fanotify::Fanotify,
    mark::{
        self,
        Mark,
        Markable,
    },
};
use apply::Apply;

pub struct BufferedFanotify {
    pub fanotify: Fanotify,
    pub buffer: EventBuffer,
}

impl Markable for BufferedFanotify {
    fn mark<'a>(&self, mark: Mark<'a>) -> Result<(), mark::Error<'a>> {
        self.fanotify.mark(mark)
    }
}

impl BufferedFanotify {
    /// See [`Fanotify::read`].
    pub fn read(&mut self) -> io::Result<Events> {
        self.fanotify.read(&mut self.buffer)
    }
}

pub struct AsyncBufferedFanotify {
    pub fanotify: AsyncFanotify,
    pub buffer: EventBuffer,
}

impl Markable for AsyncBufferedFanotify {
    fn mark<'a>(&self, mark: Mark<'a>) -> Result<(), mark::Error<'a>> {
        self.fanotify.mark(mark)
    }
}

impl AsyncBufferedFanotify {
    /// See [`Fanotify::read`].
    pub async fn read(&mut self) -> io::Result<Events<'_>> {
        self.fanotify.read(&mut self.buffer).await
    }
}

pub trait IntoBufferedFanotify: Sized {
    type Buffered;
    fn buffered(self, buffer: EventBuffer) -> Self::Buffered;
    
    fn buffered_with_size(self, size: EventBufferSize) -> Self::Buffered {
        self.buffered(size.into())
    }
    
    fn buffered_default(self) -> Self::Buffered {
        self.buffered_with_size(Default::default())
    }
}

impl IntoBufferedFanotify for Fanotify {
    type Buffered = BufferedFanotify;
    
    fn buffered(self, buffer: EventBuffer) -> Self::Buffered {
        Self::Buffered {
            fanotify: self,
            buffer,
        }
    }
}

impl IntoBufferedFanotify for AsyncFanotify {
    type Buffered = AsyncBufferedFanotify;
    
    fn buffered(self, buffer: EventBuffer) -> Self::Buffered {
        Self::Buffered {
            fanotify: self,
            buffer,
        }
    }
}

impl BufferedFanotify {
    pub fn into_async(self) -> io::Result<AsyncBufferedFanotify> {
        let Self {fanotify, buffer} = self;
        AsyncBufferedFanotify {
            fanotify: fanotify.into_async()?,
            buffer,
        }.apply(Ok)
    }
}
