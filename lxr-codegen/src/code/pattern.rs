use proc_macro2::{Literal, TokenStream};
use quote::quote;

use crate::compiler::ByteRange;

/// Returns the pattern of a `match` arm that matches each byte of `ranges`.
///
/// One range gives one pattern, and the result joins them with `|`. A range of one byte gives that
/// byte alone, thus the source stays short.
///
/// # Panics
///
/// This function panics if `ranges` is empty. An empty pattern matches no byte, thus the arm that
/// holds it is an arm that the caller must not emit.
pub fn patterns(ranges: &[ByteRange]) -> TokenStream {
    assert!(!ranges.is_empty(), "a match arm needs at least one byte");

    let parts = ranges.iter().map(|range| {
        let low = Literal::u8_suffixed(range.low);
        if range.low == range.high {
            quote!(#low)
        } else {
            let high = Literal::u8_suffixed(range.high);
            quote!(#low..=#high)
        }
    });

    quote!(#(#parts)|*)
}

/// Returns the expression that gives `true` for each byte of `ranges`.
///
/// A range gives one subtraction and one comparison, and the result joins the ranges with `|`. The
/// bar is the operator, and not the short circuit of `||`. Thus the test holds one branch for each
/// byte, and not one branch for each range of each byte.
///
/// # Panics
///
/// This function panics if `ranges` is empty. A test of no byte is a test that the caller must not
/// emit.
pub fn test(ranges: &[ByteRange]) -> TokenStream {
    assert!(!ranges.is_empty(), "a test needs at least one byte");

    let parts = ranges.iter().map(|range| {
        let low = Literal::u8_suffixed(range.low);
        if range.low == range.high {
            quote!((byte == #low))
        } else {
            let span = Literal::u8_suffixed(range.high - range.low);
            quote!((byte.wrapping_sub(#low) <= #span))
        }
    });

    quote!((#(#parts)|*))
}

/// Returns `true` if `ranges` hold each byte of the alphabet.
///
/// The labels of one state match no byte in common, thus the count of the bytes of the ranges
/// says whether they cover the alphabet. A `match` of such ranges needs no arm for the rest,
/// because the compiler reports an arm that no byte reaches.
pub fn covers(ranges: &[ByteRange]) -> bool {
    let count: usize = ranges
        .iter()
        .map(|range| usize::from(range.high - range.low) + 1)
        .sum();

    count == 256
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the range from `low` to `high`.
    fn range(low: u8, high: u8) -> ByteRange {
        ByteRange { low, high }
    }

    #[test]
    fn a_range_of_one_byte_gives_that_byte() {
        assert_eq!(
            patterns(&[range(b'a', b'a')]).to_string(),
            quote!(97u8).to_string()
        );
    }

    #[test]
    fn a_range_of_many_bytes_gives_the_two_ends() {
        assert_eq!(
            patterns(&[range(b'a', b'z')]).to_string(),
            quote!(97u8..=122u8).to_string()
        );
    }

    #[test]
    fn two_ranges_join_with_a_bar() {
        assert_eq!(
            patterns(&[range(b'a', b'z'), range(b'_', b'_')]).to_string(),
            quote!(97u8..=122u8 | 95u8).to_string()
        );
    }

    #[test]
    #[should_panic(expected = "a match arm needs at least one byte")]
    fn no_range_panics() {
        let _ = patterns(&[]);
    }

    #[test]
    fn the_ranges_of_the_whole_alphabet_cover_it() {
        assert!(covers(&[range(0, 127), range(128, 255)]));
        assert!(covers(&[range(0, 255)]));
    }

    #[test]
    fn a_range_that_leaves_one_byte_out_covers_nothing() {
        assert!(!covers(&[range(0, 127), range(129, 255)]));
        assert!(!covers(&[]));
    }
}
