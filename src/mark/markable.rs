use super::Mark;

pub trait Markable {
    /// Add a [`Mark`].
    ///
    /// See [`Mark`] for more details.
    fn mark<'a>(&self, mark: Mark<'a>) -> Result<(), super::Error<'a>>;
}
