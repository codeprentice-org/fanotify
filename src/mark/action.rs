use crate::libc::mark::action;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum OneAction {
    Add,
    Remove,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum Action {
    /// Add refers to the [FAN_MARK_ADD](action::FAN_MARK_ADD) flag
    Add = action::FAN_MARK_ADD,
    /// Remove refers to the [FAN_MARK_REMOVE](action::FAN_MARK_REMOVE) flag
    Remove = action::FAN_MARK_REMOVE,
    /// Flush refers to the [FAN_MARK_FLUSH](action::FAN_MARK_FLUSH) flag
    Flush = action::FAN_MARK_FLUSH,
}

impl OneAction {
    pub const fn const_into(self) -> Action {
        match self {
            Self::Add => Action::Add,
            Self::Remove => Action::Remove,
        }
    }
}

impl From<OneAction> for Action {
    fn from(it: OneAction) -> Self {
        it.const_into()
    }
}
