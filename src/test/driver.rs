use std::io;

use apply::Apply;

use crate::{
    buffered_fanotify::{AsyncBufferedFanotify, BufferedFanotify},
    event::{
        event::Event,
        iterator_ext::IntoEvents,
    },
};

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
    
    // noinspection RsNeedlessLifetimes
    pub async fn read_n<'a>(&'a mut self, n: usize) -> io::Result<Vec<Event<'a>>> {
        let events = self.read().await?.collect::<Vec<_>>();
        assert_eq!(events.len(), n);
        Ok(events)
    }
}

impl Driver {
    pub fn into_async(self) -> io::Result<AsyncDriver> {
        AsyncDriver {
            fanotify: self.fanotify.into_async()?
        }.apply(Ok)
    }
}
