/// The run of bytes that one node of a lexer read last.
///
/// A node that reads a run of one class reads the same bytes at each offset inside that run,
/// because the class decides each byte on its own. Thus the run from an offset inside a run that
/// the scan already read stops where that run stopped, and the scan needs no second read.
///
/// A region that no rule ends makes the scan read the region again at each start position. This
/// record turns each of those reads into one comparison, thus such a region costs its length and
/// not the square of its length.
///
/// The emitted source of a lexer writes it, thus the fields are public. One record holds the last
/// run alone. A scan that moves between two runs of two nodes reads each one again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    /// The node that read the run, or [`u32::MAX`] if the scan read no run.
    pub node: u32,
    /// The offset at which the run starts.
    pub low: usize,
    /// The offset at which the run stops.
    pub high: usize,
}

impl Run {
    /// The node of a record that holds no run.
    ///
    /// A graph holds at most [`MAX_NODES`] nodes, thus no node carries this number.
    ///
    /// [`MAX_NODES`]: crate::syntax
    pub const NONE: u32 = u32::MAX;

    /// Creates a record that holds no run.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Run;
    ///
    /// assert_eq!(Run::new().node, Run::NONE);
    /// ```
    pub const fn new() -> Self {
        Self {
            node: Self::NONE,
            low: 0,
            high: 0,
        }
    }
}

impl Default for Run {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_record_holds_no_run() {
        let run = Run::new();

        assert_eq!(run.node, Run::NONE);
        assert_eq!((run.low, run.high), (0, 0));
    }

    #[test]
    fn the_default_record_holds_no_run() {
        assert_eq!(Run::default(), Run::new());
    }
}
