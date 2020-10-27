pub mod flags;
pub mod descriptor;
mod util;

pub use descriptor::{Fanotify, InitError, RawMarkError, MarkError};

#[cfg(test)]
mod tests {}
