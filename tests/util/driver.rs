use std::io;

use apply::Apply;

use fanotify::buffered_fanotify::AsyncBufferedFanotify;
use fanotify::buffered_fanotify::BufferedFanotify;
use fanotify::event::event::Event;
use fanotify::event::iterator_ext::IntoEvents;

pub struct Driver {
    pub fanotify: BufferedFanotify,
}

impl From<BufferedFanotify> for Driver {
    fn from(this: BufferedFanotify) -> Self {
        Self { fanotify: this }
    }
}

impl Driver {
    pub fn read(&mut self) -> io::Result<impl Iterator<Item=Event>> {
        self
            .fanotify
            .read()?
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
    
    pub fn read1(&mut self) -> io::Result<Event> {
        let events = self.read_n(1)?;
        Ok(events.into_iter().next().unwrap())
    }
}

pub struct AsyncDriver {
    pub fanotify: AsyncBufferedFanotify,
}

impl From<AsyncBufferedFanotify> for AsyncDriver {
    fn from(this: AsyncBufferedFanotify) -> Self {
        Self { fanotify: this }
    }
}

impl AsyncDriver {
    // the lifetimes are actually required since it's async
    // noinspection RsNeedlessLifetimes
    pub async fn read<'a>(&'a mut self) -> io::Result<impl Iterator<Item=Event<'a>>> {
        self
            .fanotify
            .read()
            .await?
            .all()
            .map(|it| it.expect("event error"))
            .filter(|it| it.id().is_generated_by_self())
            .apply(Ok)
    }
    
    pub async fn read_n(&mut self, n: usize) -> io::Result<Vec<Event<'_>>> {
        let events = self.read().await?.collect::<Vec<_>>();
        assert_eq!(events.len(), n);
        Ok(events)
    }
    
    pub async fn read1(&mut self) -> io::Result<Event<'_>> {
        let events = self.read_n(1).await?;
        Ok(events.into_iter().next().unwrap())
    }
}

impl Driver {
    pub fn into_async(self) -> io::Result<AsyncDriver> {
        AsyncDriver {
            fanotify: self.fanotify.into_async()?
        }.apply(Ok)
    }
}