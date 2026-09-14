//! Adapts blocking byte sources for replay and location lookup.

mod replay;
mod tracking;

pub use replay::Replay;
pub use tracking::Tracking;

/// A buffered source that can reproduce bytes at a saved position.
///
/// Implementations must preserve source content for the lifetime of a mark.
/// Marks belong to their source. Scanner checkpoints enforce this ownership.
pub trait ReplaySource: std::io::BufRead {
    /// The source's saved position.
    type Mark;

    /// Saves the current source position.
    ///
    /// # Errors
    ///
    /// Returns the source's position error.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Replay, ReplaySource};
    /// let mut source = Replay::new(std::io::Cursor::new(b"x"))?;
    /// let mark = source.mark()?;
    /// source.restore(&mark)?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn mark(&mut self) -> std::io::Result<Self::Mark>;

    /// Restores a position saved by this source.
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot restore the position.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Replay, ReplaySource};
    /// let mut source = Replay::new(std::io::Cursor::new(b"x"))?;
    /// let mark = source.mark()?;
    /// source.restore(&mark)?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn restore(&mut self, mark: &Self::Mark) -> std::io::Result<()>;
}
