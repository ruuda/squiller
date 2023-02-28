// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! Utilities for generating Python code.

use std::io;

use crate::codegen::{CodeGenerator, Result};

/// Helper for generating Python code.
pub struct PythonCodeGenerator<'a> {
    /// The underlying generic code generator.
    gen: CodeGenerator<'a>,
}

impl<'a> PythonCodeGenerator<'a> {
    pub fn new(out: &'a mut dyn io::Write) -> PythonCodeGenerator<'a> {
        Self {
            gen: CodeGenerator::new(out),
        }
    }

    /// Append a string verbatim to the output.
    pub fn write(&mut self, s: &str) -> Result {
        self.gen.out.write_all(s.as_bytes())
    }

    /// Method to support writing to the generator with the `write!` macro.
    pub fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> Result {
        self.gen.out.write_fmt(fmt)
    }

    pub fn open_scope(&mut self) {
        self.gen.open_scope()
    }

    pub fn close_scope(&mut self) {
        self.gen.close_scope()
    }

    pub fn increase_indent(&mut self) {
        self.gen.indent += 4;
    }

    pub fn decrease_indent(&mut self) {
        assert!(self.gen.indent >= 4, "Cannot decrease indent below zero.");
        self.gen.indent -= 4;
    }

    pub fn write_indent(&mut self) -> Result {
        self.gen.write_indent()
    }
}
