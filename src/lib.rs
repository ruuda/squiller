// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

pub mod ast;
pub mod error;
pub mod lexer {
    pub mod annotation;
    pub mod sql;
}
pub mod parser {
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

/// Marks a location in the source file by byte offset.
#[derive(Copy, Clone, Debug)]
pub struct Span {
    /// Start of the token, inclusive.
    pub start: usize,

    /// End of the token, exclusive.
    pub end: usize,
}

impl Span {
    /// Return the slice from the input that this span spans.
    pub fn resolve<'a>(&self, input: &'a [u8]) -> &'a str {
        use std::str;
        str::from_utf8(&input[self.start..self.end]).expect("Input is not valid UTF-8.")
    }
}
