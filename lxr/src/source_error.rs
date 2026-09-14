//! Preserves source failures in cloneable scan errors.

use std::{error::Error, fmt, io, sync::Arc};

/// An original I/O error shared by cloned scan errors.
///
/// Equality compares error identity, not the error message.
#[derive(Debug, Clone)]
pub struct SourceError(Arc<io::Error>);

impl From<io::Error> for SourceError {
    fn from(error: io::Error) -> Self {
        Self(Arc::new(error))
    }
}

impl std::ops::Deref for SourceError {
    type Target = io::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for SourceError {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for SourceError {}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Error for SourceError {
    /// Gives the cause of the I/O error, and not the I/O error.
    ///
    /// [`Display`](fmt::Display) already writes the message of the I/O error.
    /// A chain reporter thus prints one message one time.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}
