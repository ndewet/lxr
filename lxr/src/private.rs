//! Runtime helpers used by generated lexers.

/// Finds the first byte equal to either delimiter, or the end of the input.
#[doc(hidden)]
#[inline]
pub fn until2(bytes: &[u8], start: usize, first: u8, second: u8) -> usize {
    bytes[start..]
        .iter()
        .position(|&byte| byte == first || byte == second)
        .map_or(bytes.len(), |offset| start + offset)
}

/// Finds the first byte equal to `delimiter`, or the end of the input.
#[doc(hidden)]
#[inline]
pub fn until(bytes: &[u8], start: usize, delimiter: u8) -> usize {
    bytes[start..]
        .iter()
        .position(|&byte| byte == delimiter)
        .map_or(bytes.len(), |offset| start + offset)
}

/// Scans an ASCII whitespace run.
#[doc(hidden)]
#[inline]
pub fn whitespace(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .map_or(bytes.len(), |offset| start + offset)
}
