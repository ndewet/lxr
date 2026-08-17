//! Reads a JSON document into tokens.
//!
//! This example shows that one regular expression reads a whole string, and that it needs no start
//! condition. The pattern holds the escapes of JSON, thus a quote inside a string does not end it.
//!
//! Run it with `cargo run -p lxr --example json`.

use lxr::Lexer;

/// The tokens of JSON, as RFC 8259 gives them.
///
/// The pattern of [`Text`](Json::Text) reads a character that is not a quote and not a backslash,
/// an escape of one character, or an escape of a codepoint. The pattern of
/// [`Number`](Json::Number) reads an optional sign, an integer with no leading zero, an optional
/// fraction, and an optional exponent.
#[derive(Debug, PartialEq, Eq, Lexer)]
#[lxr(skip = r"[ \t\r\n]+")]
enum Json {
    #[lxr(token = "{")]
    OpenObject,
    #[lxr(token = "}")]
    CloseObject,
    #[lxr(token = "[")]
    OpenArray,
    #[lxr(token = "]")]
    CloseArray,
    #[lxr(token = ":")]
    Colon,
    #[lxr(token = ",")]
    Comma,
    #[lxr(token = "true")]
    True,
    #[lxr(token = "false")]
    False,
    #[lxr(token = "null")]
    Null,
    #[lxr(regex = r#""([^"\\]|\\["\\/bfnrt]|\\u[0-9a-fA-F]{4})*""#)]
    Text,
    #[lxr(regex = r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    Number,
}

const DOCUMENT: &str = r#"{
  "name": "lxr",
  "version": [0, 1, 1],
  "generated": true,
  "authors": null,
  "ratio": -1.5e-3,
  "note": "a quote \" and a newline \n stay inside the string"
}"#;

fn main() {
    println!("{DOCUMENT}\n");

    let mut counted = 0;
    for found in Json::scan(DOCUMENT).located() {
        match found {
            Ok(found) => {
                counted += 1;
                let kind = format!("{:?}", found.token);

                println!("{kind:<12} {:?}", &DOCUMENT[found.span]);
            }
            Err(error) => println!("error: {error}"),
        }
    }

    println!("\n{counted} tokens");
}
