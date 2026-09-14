//! Replays stable seekable input.

use std::io::{self, BufRead, Cursor, Read, Seek, SeekFrom};

use crate::{Locate, Location, ReplaySource, location::resolve};

/// A stable seekable source with lazy location lookup.
///
/// The caller must keep the source content unchanged while this adapter exists.
/// Construction records the current position as the source origin.
/// Normal reads do not build a line index. Lookup caches line starts through
/// the requested offset, using memory proportional to the number of lines.
/// A failed seek poisons this adapter; later reads return errors.
pub struct Replay<S> {
    inner: S,
    origin: u64,
    indexed: u64,
    lines: Vec<u64>,
    poisoned: bool,
}

impl<S: BufRead + Seek> Replay<S> {
    /// Wraps a source whose content remains unchanged.
    ///
    /// # Errors
    ///
    /// Returns an error if the initial position cannot be read.
    ///
    /// # Examples
    ///
    /// ```
    /// let source = lxr::Replay::new(std::io::Cursor::new(b"text"))?;
    /// # let _ = source;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn new(mut source: S) -> io::Result<Self> {
        let origin = source.stream_position()?;
        Ok(Self {
            inner: source,
            origin,
            indexed: 0,
            lines: vec![0],
            poisoned: false,
        })
    }

    fn check(&self) -> io::Result<()> {
        if self.poisoned {
            Err(io::Error::other(
                "source position is unavailable after a failed seek",
            ))
        } else {
            Ok(())
        }
    }

    fn seek_to(&mut self, position: u64) -> io::Result<()> {
        self.check()?;
        match self.inner.seek(SeekFrom::Start(position)) {
            Ok(actual) if actual == position => Ok(()),
            Ok(_) => {
                self.poisoned = true;
                Err(io::Error::other("source restored an incorrect position"))
            }
            Err(error) => {
                self.poisoned = true;
                Err(error)
            }
        }
    }

    fn index_to(&mut self, offset: u64) -> io::Result<()> {
        let position = self
            .origin
            .checked_add(self.indexed)
            .ok_or_else(|| io::Error::other("source position exceeds u64"))?;
        self.seek_to(position)?;
        while self.indexed < offset {
            let bytes = match self.inner.fill_buf() {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => result?,
            };
            if bytes.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "position exceeds source length",
                ));
            }
            let count = usize::try_from(offset - self.indexed)
                .unwrap_or(usize::MAX)
                .min(bytes.len());
            for (index, &byte) in bytes[..count].iter().enumerate() {
                if byte == b'\n' {
                    self.lines.push(self.indexed + index as u64 + 1);
                }
            }
            self.indexed += count as u64;
            self.inner.consume(count);
        }
        Ok(())
    }
}

impl<'a> Replay<Cursor<&'a [u8]>> {
    pub(crate) fn memory(bytes: &'a [u8]) -> Self {
        Self {
            inner: Cursor::new(bytes),
            origin: 0,
            indexed: 0,
            lines: vec![0],
            poisoned: false,
        }
    }
}

impl<S: BufRead + Seek> Read for Replay<S> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.check()?;
        self.inner.read(bytes)
    }
}

impl<S: BufRead + Seek> BufRead for Replay<S> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.check()?;
        self.inner.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.inner.consume(amount);
    }
}

impl<S: BufRead + Seek> ReplaySource for Replay<S> {
    type Mark = u64;

    fn mark(&mut self) -> io::Result<u64> {
        self.check()?;
        self.inner.stream_position()
    }

    fn restore(&mut self, mark: &u64) -> io::Result<()> {
        self.seek_to(*mark)
    }
}

impl<S: BufRead + Seek> Locate for Replay<S> {
    fn current_offset(&mut self) -> io::Result<u64> {
        self.mark()?
            .checked_sub(self.origin)
            .ok_or_else(|| io::Error::other("position precedes source origin"))
    }

    fn locate(&mut self, offset: u64) -> io::Result<Location> {
        self.check()?;
        if offset > self.indexed {
            let saved = self.mark()?;
            let result = self.index_to(offset);
            // Restore even when indexing fails, so diagnostics preserve scanning.
            self.restore(&saved)?;
            result?;
        }
        resolve(&self.lines, offset)
    }
}
