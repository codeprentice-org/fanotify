/// A general buffer for [`Fanotify`] [`Events`].
///
/// It contains raw byte buffers for reading (event_buffer) and writing (response_buffer).
/// These are used by an [`Events::read`] and iteration over its [`Event`]s.
///
/// By storing these in a separate buffer,
/// I can reuse the buffer memory for each [`Fanotify::read`].
///
/// [`Fanotify`]: crate::fanotify::Fanotify
/// [`Events`]: super::events::Events
/// [`Fanotify::read`]: crate::fanotify::Fanotify::read
/// [`Events::read`]: super::events::Events::read
/// [`Event`]: super::event::Event
pub struct EventBuffer {
    pub events: Vec<u8>,
    pub responses: Vec<u8>,
}

impl EventBuffer {
    pub fn clear(&mut self) {
        self.events.clear();
        self.responses.clear();
    }
    
    pub fn shrink_to_fit(&mut self) {
        self.events.shrink_to_fit();
        self.responses.shrink_to_fit();
    }
    
    pub fn reserve(&mut self, additional: EventBufferSize) {
        self.events.reserve(additional.events);
        self.responses.reserve(additional.responses);
    }
    
    pub fn set_capacity(&mut self, capacities: EventBufferSize) {
        self.clear();
        self.reserve(capacities);
    }
}

pub struct EventBufferSize {
    pub events: usize,
    pub responses: usize,
}

impl Default for EventBufferSize {
    fn default() -> Self {
        Self {
            events: 4096,
            responses: 0,
        }
    }
}

impl EventBufferSize {
    /// Create an [`EventBuffer`] with these initial capacities.
    pub fn new_buffer(&self) -> EventBuffer {
        EventBuffer {
            events: Vec::with_capacity(self.events),
            responses: Vec::with_capacity(self.responses),
        }
    }
}

impl From<EventBufferSize> for EventBuffer {
    fn from(size: EventBufferSize) -> Self {
        size.new_buffer()
    }
}
