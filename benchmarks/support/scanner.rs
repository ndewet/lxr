use criterion::{Criterion, Throughput};
use runeweaver::{Lexer, Reader};
use std::hint::black_box;
use std::io::Cursor;

const MINIMUM_INPUT_BYTES: usize = 512 * 1024;

fn input(source: &str) -> String {
    assert!(
        !source.is_empty(),
        "a benchmark source file cannot be empty"
    );
    let repetitions = MINIMUM_INPUT_BYTES.div_ceil(source.len());
    source.repeat(repetitions)
}

fn scan_slice<T: Lexer>(input: &str) -> usize {
    T::scanner(black_box(input))
        .try_fold(0usize, |count, result| result.map(|_| count + 1))
        .expect("the benchmark source is valid for its token definition")
}

fn scan_reader<T: Lexer>(input: &[u8]) -> usize {
    let source = Reader::new(Cursor::new(black_box(input)));
    T::scanner_from(source)
        .try_fold(0usize, |count, result| result.map(|_| count + 1))
        .expect("the benchmark source is valid for its token definition")
}

fn scan_slice_with_lexemes<T: Lexer>(input: &str) -> usize {
    T::scanner(black_box(input))
        .with_lexemes()
        .try_fold(0usize, |bytes, result| {
            result.map(|token| bytes + token.lexeme.len())
        })
        .expect("the benchmark source is valid for its token definition")
}

fn scan_reader_with_lexemes<T: Lexer>(input: &[u8]) -> usize {
    let source = Reader::new(Cursor::new(black_box(input)));
    T::scanner_from(source)
        .with_lexemes()
        .try_fold(0usize, |bytes, result| {
            result.map(|token| bytes + token.lexeme.len())
        })
        .expect("the benchmark source is valid for its token definition")
}

pub fn benchmarks<T: Lexer>(criterion: &mut Criterion, name: &str, source: &str) {
    let input = input(source);
    let mut group = criterion.benchmark_group(format!("scanning/{name}"));
    group.throughput(Throughput::Bytes(input.len() as u64));

    group.bench_function("slice", |bencher| {
        bencher.iter(|| black_box(scan_slice::<T>(input.as_str())));
    });
    group.bench_function("reader", |bencher| {
        bencher.iter(|| black_box(scan_reader::<T>(input.as_bytes())));
    });
    group.bench_function("slice-with-lexemes", |bencher| {
        bencher.iter(|| black_box(scan_slice_with_lexemes::<T>(input.as_str())));
    });
    group.bench_function("reader-with-lexemes", |bencher| {
        bencher.iter(|| black_box(scan_reader_with_lexemes::<T>(input.as_bytes())));
    });

    group.finish();
}
