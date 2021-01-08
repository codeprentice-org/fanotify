use crate::libc::init::notification_class;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum NotificationClass {
    PreContent = notification_class::FAN_CLASS_PRE_CONTENT,
    Content = notification_class::FAN_CLASS_CONTENT,
    Notify = notification_class::FAN_CLASS_NOTIF,
}

impl NotificationClass {
    pub const fn const_default() -> Self {
        Self::Notify
    }
}

impl Default for NotificationClass {
    fn default() -> Self {
        Self::const_default()
    }
}
