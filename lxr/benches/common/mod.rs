//! Inputs and Criterion configuration shared by the two benchmark executables.

use criterion::Criterion;
use std::time::Duration;

pub const SIZE: usize = 128 * 1024;

pub fn configured() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .sample_size(50)
}

pub fn repeat(seed: &str) -> String {
    seed.repeat(SIZE.div_ceil(seed.len()))
}

pub fn compact_json() -> String {
    repeat(
        r#"{"id":8342,"active":true,"score":-12.75e3,"owner":null,"tags":["lexer","rust","fast"],"escaped":"quote: \\"}"#,
    )
}

pub fn pretty_json() -> String {
    repeat(
        "{\n  \"id\": 8342,\n  \"active\": true,\n  \"score\": -12.75e3,\n  \"owner\": null,\n  \"tags\": [\"lexer\", \"rust\", \"fast\"]\n}\n",
    )
}

pub fn rust_source() -> String {
    repeat(
        r#"
// A representative mixture of code, comments, strings, and whitespace.
fn accumulate(values: [number]) -> number {
    let mut total = 0;
    while total == 0 { total = total + values[0]; }
    if total == 42 { return "forty-two"; } else { return "other"; }
}
"#,
    )
}

pub const SYNTHETIC: [(&str, &str); 7] = [
    ("dense_punctuation", "()+-*/=;"),
    ("short_identifiers", "a b c d e f g h "),
    (
        "long_identifiers",
        "a_single_identifier_with_enough_bytes_to_stress_long_matches ",
    ),
    (
        "keyword_prefixes",
        "trans transformation transform transformed transformer ",
    ),
    (
        "mostly_whitespace",
        "                                name\n",
    ),
    ("unicode_strings", "\"København 東京 🚲 naïve café\" "),
    ("error_recovery", "valid @ valid € valid ? "),
];
