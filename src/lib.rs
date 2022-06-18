// TODO: Enable once this is finished.
#![allow(dead_code)]

mod ast;
mod error;
mod lexer {
    pub mod annotation;
    pub mod sql;
}
mod parser {
    pub mod annotation;
    pub mod document;
}

/// Check if a byte is part of an identifier.
///
/// This returns true also for digits, even though identifiers should not start
/// with a digit.
fn is_ascii_identifier(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

#[derive(Copy, Clone, Debug)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Return the slice from the input that this span spans.
    pub fn resolve<'a>(&self, input: &'a [u8]) -> &'a str {
        use std::str;
        str::from_utf8(&input[self.start..self.end]).expect("Input is not valid UTF-8.")
    }
}
