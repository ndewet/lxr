//! Executes generated lexers over blocking buffered input.

use std::{
    io::{self, BufRead, BufReader, Cursor, Read},
    marker::PhantomData,
};

use crate::{Lexer, Limits, Locate, Location, Replay, ScanError, Span, Spanned, Transition};

/// Unconsumed lookahead followed by the remaining buffered source.
pub type Remainder<S> = io::Chain<Cursor<Vec<u8>>, S>;

/// A lexer over blocking input with owned token payloads.
///
/// Spans start at offset zero at construction. Source errors, invalid UTF-8,
/// and resource failures terminate iteration. Lexical errors are recoverable.
/// The scanner retries Interrupted. WouldBlock is a terminal source error.
/// EOF ends this scan, including for sources that later acquire more data.
pub struct Scanner<T: Lexer, S = Replay<Cursor<&'static [u8]>>> {
    source: S,
    buffer: Vec<u8>,
    head: usize,
    offset: u64,
    modes: Vec<ModeFrame>,
    eof: bool,
    finished: bool,
    limits: Limits,
    extras: T::Extras,
    marker: PhantomData<T>,
}

#[derive(Clone)]
struct ModeFrame {
    id: usize,
    opened_at: u64,
}

impl<T: Lexer> Scanner<T>
where
    T::Extras: Default,
{
    /// Creates a scanner over borrowed UTF-8 text with lazy location lookup.
    ///
    /// [`Lexer::scanner`](crate::Lexer::scanner) is the public entry point.
    pub(crate) fn new(input: &str) -> Scanner<T, Replay<Cursor<&[u8]>>> {
        Self::from_bufread(Replay::memory(input.as_bytes()))
    }

    /// Adds buffering to a blocking reader.
    ///
    /// [`Lexer::from_reader`](crate::Lexer::from_reader) is the public entry
    /// point.
    pub(crate) fn from_reader<R: Read>(reader: R) -> Scanner<T, BufReader<R>> {
        Self::from_bufread(BufReader::new(reader))
    }

    /// Uses an existing blocking buffered source.
    ///
    /// [`Lexer::from_bufread`](crate::Lexer::from_bufread) is the public
    /// entry point.
    pub(crate) fn from_bufread<S: BufRead>(source: S) -> Scanner<T, S> {
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
            extras: T::Extras::default(),
            marker: PhantomData,
        }
    }
}

impl<T: Lexer, S: BufRead> Scanner<T, S> {
    /// Returns the caller state that each action reads and writes.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Lexer;
    /// #[derive(Lexer)]
    /// #[lxr(extras = usize)]
    /// enum Token { #[lxr("x", with_extras = count)] X }
    /// fn count(_: &str, total: &mut usize) { *total += 1; }
    /// let mut scanner = Token::scanner("xxx");
    /// assert_eq!(scanner.by_ref().count(), 3);
    /// assert_eq!(*scanner.extras(), 3);
    /// ```
    pub fn extras(&self) -> &T::Extras {
        &self.extras
    }

    /// Returns the caller state for a change before or during a scan.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Lexer;
    /// #[derive(Lexer)]
    /// #[lxr(extras = usize)]
    /// enum Token { #[lxr("x")] X }
    /// let mut scanner = Token::scanner("x");
    /// *scanner.extras_mut() = 7;
    /// assert_eq!(*scanner.extras(), 7);
    /// ```
    pub fn extras_mut(&mut self) -> &mut T::Extras {
        &mut self.extras
    }

    /// Sets the initial caller state.
    ///
    /// Use this for a state that has no [`Default`], or for a state that
    /// starts with content.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Lexer;
    /// #[derive(Lexer)]
    /// #[lxr(extras = usize)]
    /// enum Token { #[lxr("x")] X }
    /// let scanner = Token::scanner("x").with_extras(7);
    /// assert_eq!(*scanner.extras(), 7);
    /// ```
    #[must_use]
    pub fn with_extras(mut self, extras: T::Extras) -> Self {
        self.extras = extras;
        self
    }

    /// Consumes the scanner and returns the caller state.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Lexer;
    /// #[derive(Lexer)]
    /// #[lxr(extras = usize)]
    /// enum Token { #[lxr("x", with_extras = count)] X }
    /// fn count(_: &str, total: &mut usize) { *total += 1; }
    /// let mut scanner = Token::scanner("xx");
    /// assert_eq!(scanner.by_ref().count(), 2);
    /// assert_eq!(scanner.into_extras(), 2);
    /// ```
    pub fn into_extras(self) -> T::Extras {
        self.extras
    }

    /// Sets the resource bounds for this scanner.
    ///
    /// The scanner reads each bound at the point of use, thus a change during
    /// a scan applies to the input that follows it.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::{Lexer, Limits};
    /// use std::num::NonZeroUsize;
    /// #[derive(Lexer)]
    /// enum Token { #[lxr("x")] X }
    /// let scanner = Token::scanner("x").with_limits(Limits::new(
    ///     NonZeroUsize::new(4096).expect("a nonzero byte limit"),
    ///     NonZeroUsize::new(32).expect("a nonzero mode depth"),
    /// ));
    /// # let _ = scanner;
    /// ```
    #[must_use]
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// Returns the byte position of the next token or lexical error.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Lexer;
    /// #[derive(Lexer)]
    /// enum Token { #[lxr("x")] X }
    /// assert_eq!(Token::scanner("x").position(), 0);
    /// ```
    pub fn position(&self) -> u64 {
        self.offset
    }

    /// Returns retained unread bytes followed by the remaining source.
    ///
    /// # Examples
    ///
    /// ```
    /// use lxr::Lexer;
    /// use std::io::Read;
    /// #[derive(Lexer)]
    /// enum Token { #[lxr("x")] X }
    /// let mut remainder = Token::scanner("text").into_remainder();
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
            let available = self.limits.retained_bytes.get().saturating_sub(retained);
            if available == 0 {
                return Err(ScanError::RetentionLimit {
                    offset: self.offset,
                    limit: self.limits.retained_bytes.get(),
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

    fn commit(&mut self, count: usize) -> Result<Span, ScanError> {
        let end = self
            .offset
            .checked_add(count as u64)
            .ok_or(ScanError::PositionOverflow)?;
        let span = Span::new(self.offset, end);
        self.head += count;
        self.offset = end;
        Ok(span)
    }
}

impl<T: Lexer, S: BufRead + Locate> Locate for Scanner<T, S> {
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
                        span: Span::new(frame.opened_at, self.offset),
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
            let action = T::action(rule, text, &mut self.extras);
            let span = self.commit(count)?;
            let (token, transition) = action.map_err(|error| ScanError::InvalidPayload {
                span,
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
                    if self.modes.len() >= self.limits.mode_depth.get() {
                        return Err(ScanError::ModeLimit {
                            span,
                            limit: self.limits.mode_depth.get(),
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
                if error.is_terminal() {
                    self.finished = true;
                }
                Some(Err(error))
            }
        }
    }
}
