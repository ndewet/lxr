//! Bounds retained input and mode nesting.

/// Resource bounds for a scanner.
///
/// Defaults are 8 MiB of retained input and 1024 mode frames.
/// Bounds exclude vector capacity, source buffers, token payloads,
/// checkpoints, and diagnostic indexes.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum retained bytes, including the current lexeme and lookahead.
    pub retained_bytes: usize,
    /// Maximum mode depth, including INITIAL.
    pub mode_depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            retained_bytes: 8 * 1024 * 1024,
            mode_depth: 1024,
        }
    }
}
