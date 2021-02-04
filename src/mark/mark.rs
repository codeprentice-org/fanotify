use std::convert::TryFrom;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use super::Action;
use super::Action::Flush;
use super::error::StaticError::EmptyMask;
use super::Flags;
use super::Mask;
use super::OneAction;
use super::Path;
use super::StaticError;
use super::What;

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
    /// Turn a [`OneMark`] into a [`Mark`].
    ///
    /// This can only fail if the [`OneMark`] contains an empty [`Mask`],
    /// since that's not allowed.
    ///
    /// Also available as [`Mark::try_from`].
    ///
    /// ```
    /// use fanotify::mark;
    /// use fanotify::mark::Mark;
    /// use fanotify::mark::OneAction::Add;
    /// use fanotify::mark::What::FileSystem;
    ///
    /// assert_eq!(Mark::one(mark::One {
    ///     action: Add,
    ///     what: FileSystem,
    ///     flags: mark::Flags::empty(),
    ///     mask: mark::Mask::empty(),
    ///     path: mark::Path::current_working_directory(),
    /// }), Err(mark::StaticError::EmptyMask));
    /// ```
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

impl<'a> TryFrom<OneMark<'a>> for Mark<'a> {
    type Error = StaticError;

    /// See [`Mark::one`].
    fn try_from(this: OneMark<'a>) -> Result<Self, Self::Error> {
        Mark::one(this)
    }
}
