// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

pub mod ast;
pub mod error;
pub mod lexer {
    pub mod annotation;
    pub mod document;
}
pub mod parser {
    pub mod annotation;
    pub mod document;
}
pub mod target;

/// Check if a byte is part of an identifier.
///
/// This returns true also for digits, even though identifiers should not start
/// with a digit.
fn is_ascii_identifier(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// As `str::from_utf8`, but map errors to a type that we can print.
pub fn str_from_utf8(input: &[u8]) -> error::PResult<&str> {
    use std::str;
    str::from_utf8(input).map_err(|err| error::ParseError {
        span: Span {
            start: err.valid_up_to(),
            end: err.valid_up_to() + err.error_len().unwrap_or(0),
        },
        message: "This input is not valid UTF-8.",
        note: None,
    })
}

/// Marks a location in the source file by byte offset.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Span {
    /// Start of the token, inclusive.
    pub start: usize,

    /// End of the token, exclusive.
    pub end: usize,
}

impl Span {
    /// Return the slice from the input that this span spans.
    pub fn resolve<'a>(&self, input: &'a str) -> &'a str {
        &input[self.start..self.end]
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }
}
