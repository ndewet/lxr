//! Converts what a rule converter returns into a token payload.

use crate::PayloadError;
use std::fmt::Display;

/// Accepts the return type of a rule converter.
///
/// A converter names a function with `#[lxr("pattern", with = path)]`. The
/// function can return the payload, an `Option` of the payload, or a `Result`
/// of the payload. A `None` and an `Err` each become a
/// [`ScanError::InvalidPayload`](crate::ScanError::InvalidPayload).
///
/// Only a variant with a payload accepts a converter. A unit variant has no
/// payload, and a `skip` rule emits no token.
///
/// # Examples
///
/// ```
/// use lxr::PayloadResult;
///
/// let direct: u8 = PayloadResult::into_payload(7u8)?;
/// let checked: u8 = PayloadResult::into_payload(Ok::<u8, String>(7))?;
/// assert_eq!((direct, checked), (7, 7));
/// assert!(PayloadResult::<u8>::into_payload(None).is_err());
/// # Ok::<(), lxr::PayloadError>(())
/// ```
pub trait PayloadResult<P> {
    /// Converts this value into a payload, or into a conversion failure.
    ///
    /// # Errors
    ///
    /// Returns an error for a `None`, and for an `Err`. The message of the
    /// error comes from the `Display` of the `Err` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::PayloadResult;
    ///
    /// let payload: u8 = PayloadResult::into_payload(Some(7u8))?;
    /// assert_eq!(payload, 7);
    /// # Ok::<(), lxr::PayloadError>(())
    /// ```
    fn into_payload(self) -> Result<P, PayloadError>;
}

impl<P> PayloadResult<P> for P {
    fn into_payload(self) -> Result<P, PayloadError> {
        Ok(self)
    }
}

impl<P> PayloadResult<P> for Option<P> {
    fn into_payload(self) -> Result<P, PayloadError> {
        self.ok_or_else(|| PayloadError::new("the converter rejected the lexeme"))
    }
}

impl<P, E: Display> PayloadResult<P> for Result<P, E> {
    fn into_payload(self) -> Result<P, PayloadError> {
        self.map_err(|error| PayloadError::new(error.to_string()))
    }
}
