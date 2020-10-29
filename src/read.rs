use crate::libc::read::fanotify_event_metadata;

pub struct RawEvent {
    inner: fanotify_event_metadata,
}
