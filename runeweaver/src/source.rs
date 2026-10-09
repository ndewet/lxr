//! Byte sources and adapters for [`crate::Scanner`].

use std::convert::Infallible;
use std::io::{self, Read};

/// Supplies UTF-8 bytes to a scanner.
///
/// A source only provides bytes. The scanner owns buffering, lookahead, UTF-8
/// validation, and byte-position tracking, so implementations do not need to
/// support seeking or expose their storage.
pub trait Source {
    /// The error produced while reading input.
    type Error;

    /// Reads bytes into `buffer` and returns the number written.
    ///
    /// Returning `Ok(0)` marks the end of the source. An implementation must
    /// not return a number greater than `buffer.len()`.
    ///
    /// # Errors
    ///
    /// Returns the source's error when input cannot be read.
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
}

impl<S: Source + ?Sized> Source for &mut S {
    type Error = S::Error;

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        (**self).read(buffer)
    }
}

/// An in-memory byte source.
///
/// This adapter accepts either UTF-8 strings or arbitrary byte slices. Invalid
/// UTF-8 is reported by the scanner with its byte range.
#[derive(Debug, Clone, Copy)]
pub struct Slice<'input> {
    remaining: &'input [u8],
}

impl<'input> Slice<'input> {
    /// Creates a source over `bytes`.
    pub const fn new(bytes: &'input [u8]) -> Self {
        Self { remaining: bytes }
    }

    /// Returns the bytes that have not been read from this source.
    pub const fn remaining(&self) -> &'input [u8] {
        self.remaining
    }
}

impl<'input> From<&'input str> for Slice<'input> {
    fn from(input: &'input str) -> Self {
        Self::new(input.as_bytes())
    }
}

impl<'input> From<&'input [u8]> for Slice<'input> {
    fn from(input: &'input [u8]) -> Self {
        Self::new(input)
    }
}

impl Source for Slice<'_> {
    type Error = Infallible;

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        let length = buffer.len().min(self.remaining.len());
        buffer[..length].copy_from_slice(&self.remaining[..length]);
        self.remaining = &self.remaining[length..];
        Ok(length)
    }
}

/// Adapts a standard [`Read`] implementation into a lexer [`Source`].
///
/// `Reader<File>` scans a file directly. Wrapping a network stream or another
/// reader works the same way; the scanner performs its own buffering.
#[derive(Debug)]
pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R> {
    /// Creates a source backed by `reader`.
    pub const fn new(reader: R) -> Self {
        Self { inner: reader }
    }

    /// Returns a shared reference to the underlying reader.
    pub const fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns a mutable reference to the underlying reader.
    pub const fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Returns the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read> Source for Reader<R> {
    type Error = io::Error;

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        loop {
            match self.inner.read(buffer) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                result => return result,
            }
        }
    }
}

/// A source returned when a scanner gives back its unread input.
///
/// The scanner may have read beyond its latest token to implement longest
/// match. This source yields those buffered bytes before reading its original
/// source again.
#[derive(Debug)]
pub struct Remainder<S> {
    buffered: Vec<u8>,
    position: usize,
    source: S,
}

impl<S> Remainder<S> {
    pub(crate) fn new(buffered: Vec<u8>, position: usize, source: S) -> Self {
        Self {
            buffered,
            position,
            source,
        }
    }

    /// Returns a shared reference to the original source.
    pub const fn get_ref(&self) -> &S {
        &self.source
    }

    /// Returns the original source and discards any bytes buffered ahead of it.
    pub fn into_inner(self) -> S {
        self.source
    }
}

impl<S: Source> Source for Remainder<S> {
    type Error = S::Error;

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        if self.position == self.buffered.len() {
            return self.source.read(buffer);
        }
        let available = &self.buffered[self.position..];
        let length = buffer.len().min(available.len());
        buffer[..length].copy_from_slice(&available[..length]);
        self.position += length;
        Ok(length)
    }
}
