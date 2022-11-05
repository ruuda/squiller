// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! Utilities for generating Python code.

use std::io;

use crate::codegen::CodeGenerator;

/// Helper for generating Python code.
pub struct PythonCodeGenerator<'a> {
    /// The underlying generic code generator.
    gen: CodeGenerator<'a>,
}

impl<'a> PythonCodeGenerator<'a> {
    pub fn new(out: &'a mut dyn io::Write) -> PythonCodeGenerator<'a> {
        Self {
            gen: CodeGenerator::new(out)
        }
    }
}
