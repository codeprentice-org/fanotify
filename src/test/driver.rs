use std::io;

use apply::Apply;

use crate::async_fanotify::AsyncFanotify;
use crate::event::buffer::EventBuffer;
use crate::event::event::Event;
use crate::event::iterator_ext::IntoEvents;
use crate::fanotify::Fanotify;

pub struct Driver {
    pub fanotify: Fanotify,
    pub buffer: EventBuffer,
}

impl Driver {
    pub fn read(&mut self) -> io::Result<impl Iterator<Item=Event>> {
        self
            .fanotify
            .read(&mut self.buffer)?
            .all()
            .map(|it| it.expect("event error"))
            .filter(|it| it.id().is_generated_by_self())
            .apply(Ok)
    }
    
    pub fn read_n(&mut self, n: usize) -> io::Result<Vec<Event>> {
        let events = self.read()?.collect::<Vec<_>>();
        assert_eq!(events.len(), n);
        Ok(events)
    }
    
    pub fn into_async(self) -> io::Result<AsyncDriver> {
        AsyncDriver {
            fanotify: self.fanotify.into_async()?,
            buffer: self.buffer,
        }.apply(Ok)
    }
}

pub struct AsyncDriver {
    fanotify: AsyncFanotify,
    buffer: EventBuffer,
}

impl AsyncDriver {
    // the lifetimes are actually required since it's async
    // noinspection RsNeedlessLifetimes
    pub async fn read<'a>(&'a mut self) -> io::Result<impl Iterator<Item=Event<'a>>> {
        self
            .fanotify
            .read(&mut self.buffer)
            .await?
            .all()
            .map(|it| it.expect("event error"))
            .filter(|it| it.id().is_generated_by_self())
            .apply(Ok)
    }
    
    // noinspection RsNeedlessLifetimes
    pub async fn read_n<'a>(&'a mut self, n: usize) -> io::Result<Vec<Event<'a>>> {
        let events = self.read().await?.collect::<Vec<_>>();
        assert_eq!(events.len(), n);
        Ok(events)
    }
}
