//! Executes generated lexers over blocking buffered input.

use std::{
    io::{self, BufRead, BufReader, Cursor, Read},
    marker::PhantomData,
    ops::Range,
    sync::Arc,
};

use crate::{
    Lexer, Limits, Locate, LocatedSpan, Location, Replay, ReplaySource, ScanError, Spanned,
    Transition,
};

/// Unconsumed lookahead followed by the remaining buffered source.
pub type Remainder<S> = io::Chain<Cursor<Vec<u8>>, S>;

/// A complete scanner position, including modes and retained lookahead.
///
/// A checkpoint belongs to its originating scanner. Restoring repeats payload
/// conversion and does not undo converter side effects. Each checkpoint owns
/// a copy of retained lookahead and the mode stack.
pub struct Checkpoint<M> {
    owner: Arc<()>,
    source: M,
    buffer: Vec<u8>,
    offset: u64,
    modes: Vec<ModeFrame>,
    eof: bool,
    finished: bool,
}

/// A lexer over blocking input with owned token payloads.
///
/// Spans start at offset zero at construction. Source errors, invalid UTF-8,
/// and resource failures terminate iteration. Lexical errors are recoverable.
/// The scanner retries Interrupted. WouldBlock is a terminal source error.
/// EOF ends this scan, including for sources that later acquire more data.
pub struct Scanner<T, S = Replay<Cursor<&'static [u8]>>> {
    source: S,
    buffer: Vec<u8>,
    head: usize,
    offset: u64,
    modes: Vec<ModeFrame>,
    eof: bool,
    finished: bool,
    limits: Limits,
    owner: Arc<()>,
    marker: PhantomData<T>,
}

#[derive(Clone)]
struct ModeFrame {
    id: usize,
    opened_at: u64,
}

impl<T> Scanner<T> {
    /// Creates a scanner over borrowed UTF-8 text with lazy location lookup.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Lexer, Scanner};
    /// #[derive(Lexer)]
    /// enum Token { #[lxr("x")] X }
    /// assert_eq!(Scanner::<Token>::new("x").count(), 1);
    /// ```
    pub fn new(input: &str) -> Scanner<T, Replay<Cursor<&[u8]>>> {
        Self::from_bufread(Replay::memory(input.as_bytes()))
    }

    /// Adds buffering to a blocking reader.
    ///
    /// # Examples
    ///
    /// ```
    /// let scanner = lxr::Scanner::<()>::from_reader(&b"text"[..]);
    /// # let _ = scanner;
    /// ```
    pub fn from_reader<R: Read>(reader: R) -> Scanner<T, BufReader<R>> {
        Self::from_bufread(BufReader::new(reader))
    }

    /// Uses an existing blocking buffered source.
    ///
    /// # Examples
    ///
    /// ```
    /// let scanner = lxr::Scanner::<()>::from_bufread(&b"text"[..]);
    /// # let _ = scanner;
    /// ```
    pub fn from_bufread<S: BufRead>(source: S) -> Scanner<T, S> {
        Scanner {
            source,
            buffer: Vec::new(),
            head: 0,
            offset: 0,
            modes: vec![ModeFrame {
                id: 0,
                opened_at: 0,
            }],
            eof: false,
            finished: false,
            limits: Limits::default(),
            owner: Arc::new(()),
            marker: PhantomData,
        }
    }
}

impl<T, S: BufRead> Scanner<T, S> {
    /// Sets bounds before scanning begins.
    ///
    /// # Errors
    ///
    /// Returns an error for zero limits or a scanner that has read input.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Limits, Scanner};
    /// let scanner = Scanner::<()>::new("x").with_limits(Limits {
    ///     retained_bytes: 4096, mode_depth: 32,
    /// })?;
    /// # let _ = scanner;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn with_limits(mut self, limits: Limits) -> io::Result<Self> {
        if limits.retained_bytes == 0
            || limits.mode_depth == 0
            || self.offset != 0
            || !self.buffer.is_empty()
            || self.eof
            || self.finished
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "limits require a fresh scanner and nonzero bounds",
            ));
        }
        self.limits = limits;
        Ok(self)
    }

    /// Returns the byte position of the next token or lexical error.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(lxr::Scanner::<()>::new("x").position(), 0);
    /// ```
    pub fn position(&self) -> u64 {
        self.offset
    }

    /// Returns retained unread bytes followed by the remaining source.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Read;
    /// let mut remainder = lxr::Scanner::<()>::new("text").into_remainder();
    /// let mut text = String::new();
    /// remainder.read_to_string(&mut text)?;
    /// assert_eq!(text, "text");
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn into_remainder(self) -> Remainder<S> {
        Cursor::new(self.buffer[self.head..].to_vec()).chain(self.source)
    }

    fn ensure(&mut self, count: usize) -> Result<bool, ScanError> {
        while self.buffer.len() - self.head < count && !self.eof {
            let retained = self.buffer.len() - self.head;
            let bytes = match self.source.fill_buf() {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    return Err(ScanError::Input {
                        offset: self
                            .offset
                            .checked_add(retained as u64)
                            .ok_or(ScanError::PositionOverflow)?,
                        error: error.into(),
                    });
                }
                Ok(bytes) => bytes,
            };
            if bytes.is_empty() {
                self.eof = true;
                break;
            }
            let available = self.limits.retained_bytes.saturating_sub(retained);
            if available == 0 {
                return Err(ScanError::RetentionLimit {
                    offset: self.offset,
                    limit: self.limits.retained_bytes,
                });
            }
            let amount = bytes.len().min(8192).min(available);
            self.offset
                .checked_add(retained as u64)
                .and_then(|offset| offset.checked_add(amount as u64))
                .ok_or(ScanError::PositionOverflow)?;
            self.buffer.extend_from_slice(&bytes[..amount]);
            self.source.consume(amount);
        }
        Ok(self.buffer.len() - self.head >= count)
    }

    fn scalar(&mut self, index: usize) -> Result<Option<usize>, ScanError> {
        let count = index.checked_add(1).ok_or(ScanError::PositionOverflow)?;
        if !self.ensure(count)? {
            return Ok(None);
        }
        let offset = self
            .offset
            .checked_add(index as u64)
            .ok_or(ScanError::PositionOverflow)?;
        let width = match self.buffer[self.head + index] {
            0..=0x7f => 1,
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => return Err(ScanError::InvalidEncoding { offset }),
        };
        let end = index
            .checked_add(width)
            .ok_or(ScanError::PositionOverflow)?;
        if !self.ensure(end)?
            || std::str::from_utf8(&self.buffer[self.head + index..self.head + end]).is_err()
        {
            return Err(ScanError::InvalidEncoding { offset });
        }
        Ok(Some(width))
    }

    fn commit(&mut self, count: usize) -> Result<Range<u64>, ScanError> {
        let end = self
            .offset
            .checked_add(count as u64)
            .ok_or(ScanError::PositionOverflow)?;
        let span = self.offset..end;
        self.head += count;
        self.offset = end;
        Ok(span)
    }
}

impl<T, S: ReplaySource> Scanner<T, S> {
    /// Saves input, modes, lookahead, and completion state between tokens.
    ///
    /// # Errors
    ///
    /// Returns the source's position error.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut scanner = lxr::Scanner::<()>::new("text");
    /// let checkpoint = scanner.checkpoint()?;
    /// scanner.restore(&checkpoint)?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn checkpoint(&mut self) -> io::Result<Checkpoint<S::Mark>> {
        Ok(Checkpoint {
            owner: self.owner.clone(),
            source: self.source.mark()?,
            buffer: self.buffer[self.head..].to_vec(),
            offset: self.offset,
            modes: self.modes.clone(),
            eof: self.eof,
            finished: self.finished,
        })
    }

    /// Restores a checkpoint from this scanner.
    ///
    /// A source restoration failure terminates scanning.
    ///
    /// # Errors
    ///
    /// Returns an error for a foreign checkpoint or a source restoration failure.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut scanner = lxr::Scanner::<()>::new("text");
    /// let checkpoint = scanner.checkpoint()?;
    /// scanner.restore(&checkpoint)?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn restore(&mut self, checkpoint: &Checkpoint<S::Mark>) -> io::Result<()> {
        if !Arc::ptr_eq(&self.owner, &checkpoint.owner) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "checkpoint belongs to another scanner",
            ));
        }
        if let Err(error) = self.source.restore(&checkpoint.source) {
            self.finished = true;
            return Err(error);
        }
        self.buffer.clone_from(&checkpoint.buffer);
        self.head = 0;
        self.offset = checkpoint.offset;
        self.modes.clone_from(&checkpoint.modes);
        self.eof = checkpoint.eof;
        self.finished = checkpoint.finished;
        Ok(())
    }
}

impl<T, S: BufRead + Locate> Locate for Scanner<T, S> {
    fn current_offset(&mut self) -> io::Result<u64> {
        Ok(self.offset)
    }

    fn locate(&mut self, offset: u64) -> io::Result<Location> {
        let consumed = self
            .offset
            .checked_add((self.buffer.len() - self.head) as u64)
            .ok_or_else(|| io::Error::other("source position exceeds u64"))?;
        let origin = self
            .source
            .current_offset()?
            .checked_sub(consumed)
            .ok_or_else(|| io::Error::other("source position precedes scanner origin"))?;
        let position = origin
            .checked_add(offset)
            .ok_or_else(|| io::Error::other("source position exceeds u64"))?;
        self.source.locate(position)
    }

    fn locate_span(&mut self, span: Range<u64>) -> io::Result<LocatedSpan> {
        if span.start > span.end {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "reversed span"));
        }
        let start = self.locate(span.start)?;
        let end = self.locate(span.end)?;
        Ok(LocatedSpan { start, end })
    }
}

impl<T: Lexer, S: BufRead> Scanner<T, S> {
    fn scan(&mut self) -> Result<Option<Spanned<T>>, ScanError> {
        loop {
            if self.head > 0 && self.head >= self.buffer.len() / 2 {
                self.buffer.drain(..self.head);
                self.head = 0;
            }
            let Some(first_width) = self.scalar(0)? else {
                self.finished = true;
                if let Some(frame) = self.modes.get(1) {
                    return Err(ScanError::UnterminatedMode {
                        span: frame.opened_at..self.offset,
                        mode: T::mode_name(frame.id),
                    });
                }
                return Ok(None);
            };
            let mode = self.modes.last().expect("INITIAL exists").id;
            let mut state = T::start(mode);
            let mut latest = None;
            let mut index = 0;
            'execution: while let Some(width) = self.scalar(index)? {
                for position in index..index + width {
                    let Some(next) = T::step(state, self.buffer[self.head + position]) else {
                        break 'execution;
                    };
                    state = next;
                }
                index += width;
                if let Some(rule) = T::accept(state) {
                    latest = Some((rule, index));
                }
                if !T::continues(state) {
                    break;
                }
            }
            let Some((rule, count)) = latest else {
                let span = self.commit(first_width)?;
                return Err(ScanError::Unrecognized { span });
            };
            let text = std::str::from_utf8(&self.buffer[self.head..self.head + count])
                .expect("execution validates complete UTF-8 scalars");
            let action = T::action(rule, text);
            let span = self.commit(count)?;
            let (token, transition) = action.map_err(|error| ScanError::InvalidPayload {
                span: span.clone(),
                message: error.to_string(),
            })?;
            match transition {
                Transition::Stay => {}
                Transition::Begin(id) => {
                    let frame = self.modes.last_mut().expect("INITIAL exists");
                    frame.id = id;
                    frame.opened_at = span.start;
                }
                Transition::Push(id) => {
                    if self.modes.len() >= self.limits.mode_depth {
                        return Err(ScanError::ModeLimit {
                            span,
                            limit: self.limits.mode_depth,
                        });
                    }
                    self.modes.push(ModeFrame {
                        id,
                        opened_at: span.start,
                    });
                }
                Transition::Pop => {
                    if self.modes.len() > 1 {
                        self.modes.pop();
                    }
                }
            }
            if let Some(token) = token {
                return Ok(Some(Spanned { token, span }));
            }
        }
    }
}

impl<T: Lexer, S: BufRead> Iterator for Scanner<T, S> {
    type Item = Result<Spanned<T>, ScanError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        match self.scan() {
            Ok(token) => token.map(Ok),
            Err(error) => {
                if !matches!(
                    error,
                    ScanError::Unrecognized { .. } | ScanError::InvalidPayload { .. }
                ) {
                    self.finished = true;
                }
                Some(Err(error))
            }
        }
    }
}
