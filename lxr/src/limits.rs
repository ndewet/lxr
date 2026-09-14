//! Bounds retained input and mode nesting.

use std::num::NonZeroUsize;

/// Resource bounds for a scanner.
///
/// Defaults are 8 MiB of retained input and 1024 mode frames.
/// Bounds exclude vector capacity, source buffers, token payloads, and
/// diagnostic indexes.
///
/// Each bound is a [`NonZeroUsize`], thus a bound of zero cannot reach the
/// scanner.
///
/// # Examples
///
/// ```
/// use lxr::Limits;
/// use std::num::NonZeroUsize;
///
/// let limits = Limits::new(
///     NonZeroUsize::new(4096).expect("a nonzero byte limit"),
///     NonZeroUsize::new(32).expect("a nonzero mode depth"),
/// );
/// assert_eq!(limits.retained_bytes.get(), 4096);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum retained bytes, including the current lexeme and lookahead.
    pub retained_bytes: NonZeroUsize,
    /// Maximum mode depth, including INITIAL.
    pub mode_depth: NonZeroUsize,
}

impl Limits {
    /// Creates bounds from a byte count and a mode depth.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Limits;
    /// use std::num::NonZeroUsize;
    ///
    /// let depth = NonZeroUsize::new(8).expect("a nonzero mode depth");
    /// let limits = Limits::new(
    ///     NonZeroUsize::new(64).expect("a nonzero byte limit"),
    ///     depth,
    /// );
    /// assert_eq!(limits.mode_depth, depth);
    /// ```
    #[must_use]
    pub const fn new(retained_bytes: NonZeroUsize, mode_depth: NonZeroUsize) -> Self {
        Self {
            retained_bytes,
            mode_depth,
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            retained_bytes: NonZeroUsize::new(8 * 1024 * 1024).expect("the default is nonzero"),
            mode_depth: NonZeroUsize::new(1024).expect("the default is nonzero"),
        }
    }
}
