use std::{
    fmt::{Display, Formatter},
    fmt,
};

use super::{
    Action,
    Flags,
    Mask,
    OneAction,
    Path,
    StaticError,
    What,
    Action::Flush,
    error::StaticError::EmptyMask,
};

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct OneMark<'a> {
    pub action: OneAction,
    pub what: What,
    pub flags: Flags,
    pub mask: Mask,
    pub path: Path<'a>,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Mark<'a> {
    // fields are not pub b/c they maintain invariants
    pub(crate) action: Action,
    pub(crate) what: What,
    pub(crate) flags: Flags,
    pub(crate) mask: Mask,
    pub(crate) path: Path<'a>,
}

impl<'a> Mark<'a> {
    pub fn action(&self) -> Action {
        self.action
    }
    
    pub fn what(&self) -> What {
        self.what
    }
    
    pub fn flags(&self) -> Flags {
        self.flags
    }
    
    pub fn mask(&self) -> Mask {
        self.mask
    }
    
    pub fn path(&self) -> &Path<'a> {
        &self.path
    }
}

impl Display for Mark<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // delegate Debug impl to Init
        write!(f, "{:?}", self)
    }
}

impl<'a> Mark<'a> {
    pub const fn one(mark: OneMark<'a>) -> Result<Self, StaticError> {
        let OneMark {
            action,
            what,
            flags,
            mask,
            path,
        } = mark;
        if mask.is_empty() {
            return Err(EmptyMask);
        }
        let this = Self {
            action: action.const_into(),
            what,
            flags,
            mask,
            path,
        };
        Ok(this)
    }
    
    pub const fn flush(what: What) -> Self {
        Self {
            action: Flush,
            what,
            flags: Flags::empty(),
            mask: Mask::all(), // ignored, but empty is invalid on add/remove
            path: Path::current_working_directory(), // ignored, but good default with 'static lifetime
        }
    }
}
