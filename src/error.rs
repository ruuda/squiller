// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use crate::Span;

#[derive(Debug)]
pub struct ParseError {
    pub span: Span,
    pub message: &'static str,
}

/// A parse result, either the parsed value, or a parse error.
pub type PResult<T> = std::result::Result<T, ParseError>;
