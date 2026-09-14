//! Exercises streaming, replay, and diagnostic locations.

use std::{
    cell::Cell,
    io::{self, BufRead, BufReader, Cursor, Read, Seek, SeekFrom},
    rc::Rc,
};

use lxr::{Lexer, Limits, Locate, Location, Replay, ScanError, Scanner, Spanned, Tracking};

struct Chunks<'a> {
    bytes: &'a [u8],
    position: usize,
    ends: Vec<usize>,
}

impl Read for Chunks<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let bytes = self.fill_buf()?;
        let count = bytes.len().min(output.len());
        output[..count].copy_from_slice(&bytes[..count]);
        self.consume(count);
        Ok(count)
    }
}

impl BufRead for Chunks<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        let end = self
            .ends
            .iter()
            .copied()
            .find(|&end| end > self.position)
            .unwrap_or(self.bytes.len());
        Ok(&self.bytes[self.position..end])
    }

    fn consume(&mut self, amount: usize) {
        self.position += amount;
    }
}

#[derive(Debug, PartialEq, Lexer)]
#[lxr(skip = r"[ \n]+")]
enum Text {
    #[lxr("[a-zé]+")]
    Word(String),
    #[lxr("[0-9]+")]
    Number(u64),
}

#[test]
fn every_chunk_partition_preserves_tokens_unicode_skips_and_errors() {
    let input = "aa é\n7@";
    let expected = vec![
        Ok(Spanned {
            token: Text::Word("aa".into()),
            span: 0..2,
        }),
        Ok(Spanned {
            token: Text::Word("é".into()),
            span: 3..5,
        }),
        Ok(Spanned {
            token: Text::Number(7),
            span: 6..7,
        }),
        Err(ScanError::Unrecognized { span: 7..8 }),
    ];
    assert_eq!(Text::scanner(input).collect::<Vec<_>>(), expected);
    for mask in 0..1usize << (input.len() - 1) {
        let ends = (1..input.len())
            .filter(|index| mask & (1 << (index - 1)) != 0)
            .collect();
        let source = Chunks {
            bytes: input.as_bytes(),
            position: 0,
            ends,
        };
        assert_eq!(
            Text::from_bufread(source).collect::<Vec<_>>(),
            expected,
            "partition {mask}"
        );
    }
}

#[derive(Debug, PartialEq, Lexer)]
enum Rollback {
    #[lxr("a")]
    A,
    #[lxr("ab*c")]
    Long,
    #[lxr("b+")]
    Bs,
    #[lxr("!")]
    End,
}

#[test]
fn forward_only_input_retains_unbounded_speculation_and_the_remainder() {
    let input = format!("a{}!", "b".repeat(20_000));
    let source = BufReader::with_capacity(3, input.as_bytes());
    let mut scanner = Rollback::from_bufread(source);
    assert_eq!(
        scanner.next(),
        Some(Ok(Spanned {
            token: Rollback::A,
            span: 0..1
        }))
    );
    let mut remainder = String::new();
    scanner
        .into_remainder()
        .read_to_string(&mut remainder)
        .expect("remainder is readable");
    assert_eq!(remainder, input[1..]);
    assert_eq!(
        Rollback::from_reader(input.as_bytes()).collect::<Vec<_>>(),
        vec![
            Ok(Spanned {
                token: Rollback::A,
                span: 0..1
            }),
            Ok(Spanned {
                token: Rollback::Bs,
                span: 1..20_001
            }),
            Ok(Spanned {
                token: Rollback::End,
                span: 20_001..20_002
            }),
        ]
    );
}

#[test]
fn utf8_failures_are_terminal_at_every_chunk_size() {
    for bytes in [
        &b"\xc3"[..],
        &b"\xc0\xaf"[..],
        &b"\xed\xa0\x80"[..],
        &b"\xf4\x90\x80\x80"[..],
        &b"\x80"[..],
    ] {
        for capacity in 1..=5 {
            let mut scanner = Text::from_bufread(BufReader::with_capacity(capacity, bytes));
            assert_eq!(
                scanner.next(),
                Some(Err(ScanError::InvalidEncoding { offset: 0 }))
            );
            assert_eq!(scanner.next(), None);
        }
    }
}

struct Failure {
    bytes: Cursor<&'static [u8]>,
    interrupt: bool,
}

impl Read for Failure {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let bytes = self.fill_buf()?;
        let count = output.len().min(bytes.len());
        output[..count].copy_from_slice(&bytes[..count]);
        self.consume(count);
        Ok(count)
    }
}

impl BufRead for Failure {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.interrupt {
            self.interrupt = false;
            return Err(io::ErrorKind::Interrupted.into());
        }
        let bytes = self.bytes.fill_buf()?;
        if bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "injected failure",
            ));
        }
        Ok(bytes)
    }

    fn consume(&mut self, amount: usize) {
        self.bytes.consume(amount);
    }
}

#[test]
fn input_failure_is_not_eof_and_preserves_its_cause() {
    let mut scanner = Text::from_bufread(Failure {
        bytes: Cursor::new(b"abc"),
        interrupt: true,
    });
    let error = scanner
        .next()
        .expect("one failure")
        .expect_err("identifier is incomplete");
    match error {
        ScanError::Input { offset, error } => {
            assert_eq!(offset, 3);
            assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
            assert_eq!(error.to_string(), "injected failure");
        }
        error => panic!("unexpected error: {error:?}"),
    }
    assert_eq!(scanner.next(), None);
}

#[test]
fn a_terminal_accept_does_not_read_again() {
    let mut scanner = Rollback::from_bufread(Failure {
        bytes: Cursor::new(b"!"),
        interrupt: true,
    });
    assert_eq!(
        scanner.next(),
        Some(Ok(Spanned {
            token: Rollback::End,
            span: 0..1
        }))
    );
    assert!(matches!(
        scanner.next(),
        Some(Err(ScanError::Input { offset: 1, .. }))
    ));
}

#[derive(Debug, PartialEq, Lexer)]
#[lxr(mode = Quoted)]
enum Modes {
    #[lxr("x")]
    X,
    #[lxr("<", push = Quoted)]
    Open,
    #[lxr("<", modes = Quoted, push = Quoted)]
    Nested,
    #[lxr("x+", modes = Quoted)]
    Contents,
    #[lxr(">", modes = Quoted, pop)]
    Close,
}

#[test]
fn checkpoints_restore_modes_lookahead_and_completion() {
    let mut scanner = Modes::scanner("<xx>x");
    assert!(matches!(
        scanner.next(),
        Some(Ok(Spanned {
            token: Modes::Open,
            ..
        }))
    ));
    let checkpoint = scanner.checkpoint().expect("memory supports replay");
    let expected = scanner.by_ref().collect::<Vec<_>>();
    assert_eq!(expected.len(), 3);
    scanner
        .restore(&checkpoint)
        .expect("checkpoint belongs to scanner");
    assert_eq!(scanner.collect::<Vec<_>>(), expected);

    let mut scanner = Rollback::scanner("abbb!");
    assert!(matches!(
        scanner.next(),
        Some(Ok(Spanned {
            token: Rollback::A,
            ..
        }))
    ));
    let checkpoint = scanner.checkpoint().expect("save speculative lookahead");
    let expected = scanner.by_ref().collect::<Vec<_>>();
    scanner.restore(&checkpoint).expect("restore lookahead");
    assert_eq!(scanner.by_ref().collect::<Vec<_>>(), expected);
    let end = scanner.checkpoint().expect("save EOF");
    scanner
        .restore(&checkpoint)
        .expect("restore earlier checkpoint");
    scanner.restore(&end).expect("restore EOF");
    assert_eq!(scanner.next(), None);
}

#[test]
fn foreign_checkpoints_do_not_change_the_scanner() {
    let mut first = Text::scanner("a");
    let mut second = Text::scanner("b");
    let checkpoint = first.checkpoint().expect("save first scanner");
    assert_eq!(
        second
            .restore(&checkpoint)
            .expect_err("foreign checkpoint")
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        second.next().expect("token").expect("valid token").token,
        Text::Word("b".into())
    );
}

#[test]
fn limits_report_failure_instead_of_shortening_a_token() {
    for capacity in 1..=8 {
        let mut scanner =
            Rollback::from_bufread(BufReader::with_capacity(capacity, &b"abbbbb!"[..]))
                .with_limits(Limits {
                    retained_bytes: 4,
                    mode_depth: 4,
                })
                .expect("valid limits");
        assert_eq!(
            scanner.next(),
            Some(Err(ScanError::RetentionLimit {
                offset: 0,
                limit: 4
            }))
        );
        assert_eq!(scanner.next(), None);
    }
    let mut scanner = Modes::scanner("<<")
        .with_limits(Limits {
            retained_bytes: 8,
            mode_depth: 2,
        })
        .expect("valid limits");
    assert!(scanner.next().expect("opening token").is_ok());
    assert_eq!(
        scanner.next(),
        Some(Err(ScanError::ModeLimit {
            span: 1..2,
            limit: 2
        }))
    );
    assert_eq!(scanner.next(), None);
}

struct Counted {
    cursor: Cursor<Vec<u8>>,
    consumed: Rc<Cell<usize>>,
    seeks: Rc<Cell<usize>>,
}

impl Read for Counted {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = self.cursor.read(output)?;
        self.consumed.set(self.consumed.get() + count);
        Ok(count)
    }
}

impl BufRead for Counted {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.cursor.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.consumed.set(self.consumed.get() + amount);
        self.cursor.consume(amount);
    }
}

impl Seek for Counted {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.seeks.set(self.seeks.get() + 1);
        self.cursor.seek(position)
    }
}

#[test]
fn replay_locations_are_lazy_cached_and_preserve_scanning() {
    let consumed = Rc::new(Cell::new(0));
    let seeks = Rc::new(Cell::new(0));
    let source = Counted {
        cursor: Cursor::new(b"a\nb\nc".to_vec()),
        consumed: consumed.clone(),
        seeks: seeks.clone(),
    };
    let source = Replay::new(source).expect("seekable source");
    let initial_seeks = seeks.get();
    let mut scanner = Text::from_bufread(source);
    assert_eq!(
        scanner
            .next()
            .expect("first token")
            .expect("valid token")
            .span,
        0..1
    );
    assert_eq!(
        seeks.get(),
        initial_seeks,
        "normal scanning never seeks for locations"
    );
    let before = consumed.get();
    assert_eq!(
        scanner.locate(2).expect("location"),
        Location { line: 2, column: 1 }
    );
    assert_eq!(
        consumed.get() - before,
        2,
        "index only the requested prefix"
    );
    let cached = consumed.get();
    assert_eq!(
        scanner.locate(1).expect("cached location"),
        Location { line: 1, column: 2 }
    );
    assert_eq!(consumed.get(), cached);
    assert_eq!(
        scanner.locate_span(2..5).expect("span").end,
        Location { line: 3, column: 2 }
    );
    assert!(scanner.locate(99).is_err());
    assert!(
        scanner
            .locate_span(std::ops::Range { start: 3, end: 2 })
            .is_err()
    );
    assert_eq!(
        scanner
            .map(|token| token.expect("valid token").token)
            .collect::<Vec<_>>(),
        vec![Text::Word("b".into()), Text::Word("c".into())]
    );
}

#[test]
fn byte_columns_and_line_boundaries_agree_for_replay_and_tracking() {
    let bytes = "é\t\r\nx\n".as_bytes();
    let mut replay = Replay::new(Cursor::new(bytes)).expect("memory source");
    let mut tracked = Tracking::new(BufReader::with_capacity(1, bytes));
    assert!(
        tracked.locate(1).is_err(),
        "future positions are unavailable"
    );
    io::copy(&mut tracked, &mut io::sink()).expect("consume source");
    let expected = [
        Location { line: 1, column: 1 },
        Location { line: 1, column: 2 },
        Location { line: 1, column: 3 },
        Location { line: 1, column: 4 },
        Location { line: 1, column: 5 },
        Location { line: 2, column: 1 },
        Location { line: 2, column: 2 },
        Location { line: 3, column: 1 },
    ];
    for (offset, expected) in expected.into_iter().enumerate() {
        assert_eq!(
            replay.locate(offset as u64).expect("replay location"),
            expected
        );
        assert_eq!(
            tracked.locate(offset as u64).expect("tracked location"),
            expected
        );
    }
}

#[test]
fn lookup_uses_the_initial_source_position_as_origin() {
    let mut cursor = Cursor::new(b"prefix\nx\ny");
    cursor.set_position(7);
    let source = Replay::new(cursor).expect("source origin");
    let mut scanner = Text::from_bufread(source);
    assert_eq!(
        scanner.locate(2).expect("relative position"),
        Location { line: 2, column: 1 }
    );
    assert_eq!(
        scanner.next().expect("token").expect("valid token").span,
        0..1
    );
}

#[test]
fn tracking_resolves_discarded_input_without_replay() {
    let source = Tracking::new(BufReader::with_capacity(1, &b"a\nb\nc"[..]));
    let mut scanner = Text::from_bufread(source);
    assert_eq!(scanner.by_ref().count(), 3);
    assert_eq!(
        scanner.locate(2).expect("discarded input location"),
        Location { line: 2, column: 1 }
    );
}

#[test]
fn configuration_rejects_invalid_bounds() {
    assert!(
        Scanner::<()>::new("")
            .with_limits(Limits {
                retained_bytes: 0,
                mode_depth: 1
            })
            .is_err()
    );
    assert!(
        Scanner::<()>::new("")
            .with_limits(Limits {
                retained_bytes: 1,
                mode_depth: 0
            })
            .is_err()
    );
}

#[test]
fn already_consumed_adapters_keep_their_original_line_coordinates() {
    let mut source = Replay::new(Cursor::new(b"a\nb")).expect("source");
    source.consume(2);
    let mut scanner = Text::from_bufread(source);
    assert_eq!(
        scanner.locate(0).expect("scanner origin"),
        Location { line: 2, column: 1 }
    );
    assert_eq!(
        scanner.next().expect("token").expect("valid token").span,
        0..1
    );
    assert_eq!(
        scanner.locate(1).expect("scanner EOF"),
        Location { line: 2, column: 2 }
    );

    let mut source = Tracking::new(&b"a\nb"[..]);
    source.fill_buf().expect("expose buffer");
    source.consume(2);
    let mut scanner = Text::from_bufread(source);
    assert_eq!(
        scanner.locate(0).expect("tracked origin"),
        Location { line: 2, column: 1 }
    );
    assert_eq!(scanner.by_ref().count(), 1);
    assert_eq!(
        scanner.locate(1).expect("tracked EOF"),
        Location { line: 2, column: 2 }
    );
}

#[test]
fn files_support_streaming_replay_and_locations() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let input = std::fs::read_to_string(path).expect("read manifest");
    let expected = Text::scanner(&input).collect::<Vec<_>>();
    let file = std::fs::File::open(path).expect("open manifest");
    assert_eq!(Text::from_reader(file).collect::<Vec<_>>(), expected);
    let file = std::fs::File::open(path).expect("reopen manifest");
    let source = Replay::new(BufReader::with_capacity(3, file)).expect("seekable file");
    let mut scanner = Text::from_bufread(source);
    let checkpoint = scanner.checkpoint().expect("file checkpoint");
    let offset = input.find('\n').expect("manifest has multiple lines") as u64 + 1;
    assert_eq!(
        scanner.locate(offset).expect("file location"),
        Location { line: 2, column: 1 }
    );
    assert_eq!(scanner.by_ref().collect::<Vec<_>>(), expected);
    scanner.restore(&checkpoint).expect("file restore");
    assert_eq!(scanner.collect::<Vec<_>>(), expected);
}

struct Faults {
    cursor: Cursor<&'static [u8]>,
    fail_read: Rc<Cell<bool>>,
    fail_seek: Rc<Cell<usize>>,
}

impl Read for Faults {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.fail_read.get() {
            return Err(io::Error::other("injected read failure"));
        }
        self.cursor.read(output)
    }
}

impl BufRead for Faults {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.fail_read.get() {
            return Err(io::Error::other("injected read failure"));
        }
        self.cursor.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.cursor.consume(amount);
    }
}

impl Seek for Faults {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let remaining = self.fail_seek.get();
        if remaining != 0 {
            self.fail_seek.set(remaining - 1);
            if remaining == 1 {
                return Err(io::Error::other("injected seek failure"));
            }
        }
        self.cursor.seek(position)
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        Ok(self.cursor.position())
    }
}

#[test]
fn failed_location_reads_restore_the_source_position() {
    let fail_read = Rc::new(Cell::new(false));
    let source = Faults {
        cursor: Cursor::new(b"a\nb"),
        fail_read: fail_read.clone(),
        fail_seek: Rc::new(Cell::new(0)),
    };
    let mut scanner = Text::from_bufread(Replay::new(source).expect("source"));
    assert_eq!(
        scanner.next().expect("token").expect("valid token").span,
        0..1
    );
    fail_read.set(true);
    assert!(scanner.locate(2).is_err());
    fail_read.set(false);
    assert_eq!(
        scanner.locate(2).expect("retry lookup"),
        Location { line: 2, column: 1 }
    );
    assert_eq!(
        scanner
            .next()
            .expect("next token")
            .expect("valid token")
            .span,
        2..3
    );
}

#[test]
fn failed_checkpoint_restore_terminates_scanning() {
    let fail_seek = Rc::new(Cell::new(0));
    let source = Faults {
        cursor: Cursor::new(b"a\nb"),
        fail_read: Rc::new(Cell::new(false)),
        fail_seek: fail_seek.clone(),
    };
    let mut scanner = Text::from_bufread(Replay::new(source).expect("source"));
    let checkpoint = scanner.checkpoint().expect("checkpoint");
    assert!(scanner.next().expect("token").is_ok());
    fail_seek.set(1);
    assert!(scanner.restore(&checkpoint).is_err());
    assert_eq!(scanner.next(), None);
}

#[test]
fn failed_location_restore_poisoning_prevents_reads_from_the_wrong_position() {
    let fail_seek = Rc::new(Cell::new(0));
    let source = Faults {
        cursor: Cursor::new(b"a\nb"),
        fail_read: Rc::new(Cell::new(false)),
        fail_seek: fail_seek.clone(),
    };
    let mut source = Replay::new(source).expect("source");
    fail_seek.set(2);
    assert!(source.locate(2).is_err());
    assert!(source.fill_buf().is_err());
    assert!(source.read(&mut [0]).is_err());
    assert!(source.locate(0).is_err());
}
