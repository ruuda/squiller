mod ast;
mod lex_annotation;
mod lex_sql;
mod parse;

/// Check if a byte is part of an identifier.
///
/// This returns true also for digits, even though identifiers should not start
/// with a digit.
fn is_ascii_identifier(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

#[derive(Copy, Clone, Debug)]
struct Span {
    start: usize,
    end: usize,
}
