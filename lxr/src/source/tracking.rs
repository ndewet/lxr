//! Records line starts for input that cannot replay.

use std::io::{self, BufRead, Read};

use crate::{Locate, Location, location::resolve};

/// An optional line index for a forward-only buffered source.
///
/// The index retains one u64 per line and no source text. Locations are
/// available through the consumed position, including discarded input.
/// Columns count bytes. The current position at construction is offset zero.
pub struct Tracking<S> {
    inner: S,
    offset: u64,
    lines: Vec<u64>,
    indexed: u64,
    exposed: usize,
}

impl<S: BufRead> Tracking<S> {
    /// Enables line tracking for a buffered source.
    ///
    /// # Examples
    ///
    /// ```
    /// let source = lxr::Tracking::new(&b"text"[..]);
    /// # let _ = source;
    /// ```
    pub fn new(source: S) -> Self {
        Self {
            inner: source,
            offset: 0,
            lines: vec![0],
            indexed: 0,
            exposed: 0,
        }
    }
}

impl<S: BufRead> Read for Tracking<S> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        let bytes = self.fill_buf()?;
        let count = bytes.len().min(output.len());
        output[..count].copy_from_slice(&bytes[..count]);
        self.consume(count);
        Ok(count)
    }
}

impl<S: BufRead> BufRead for Tracking<S> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        let bytes = self.inner.fill_buf()?;
        let end = self
            .offset
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| io::Error::other("tracked source position exceeds u64"))?;
        let skip = usize::try_from(self.indexed - self.offset)
            .unwrap_or(usize::MAX)
            .min(bytes.len());
        for (index, &byte) in bytes.iter().enumerate().skip(skip) {
            if byte == b'\n' {
                self.lines.push(self.offset + index as u64 + 1);
            }
        }
        self.indexed = self.indexed.max(end);
        self.exposed = bytes.len();
        Ok(bytes)
    }

    fn consume(&mut self, amount: usize) {
        assert!(amount <= self.exposed, "consume exceeds the exposed buffer");
        self.offset += amount as u64;
        self.exposed -= amount;
        self.inner.consume(amount);
    }
}

impl<S: BufRead> Locate for Tracking<S> {
    fn current_offset(&mut self) -> io::Result<u64> {
        Ok(self.offset)
    }

    fn locate(&mut self, offset: u64) -> io::Result<Location> {
        if offset > self.offset {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "position has not been tracked",
            ));
        }
        resolve(&self.lines, offset)
    }
}
