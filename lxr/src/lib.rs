//! Provides runtime support for generated lxr lexers.
//!
//! Derive [`Lexer`] for a unit enum and place one `#[lxr("pattern")]`
//! attribute on each variant.

#![deny(dead_code)]

pub use lxr_derive::Lexer;

/// Scans the longest prefix accepted by a generated lexer.
pub trait Lexer: Sized {
    /// Returns the matching token and the number of UTF-8 bytes it consumed.
    fn scan(input: &str) -> Option<(Self, usize)>;
}
