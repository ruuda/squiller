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

    pub fn write_indent(&mut self) -> Result {
        self.gen.write_indent()
    }

    // Append comment lines, indented by the current indent.
    pub fn write_comment(&mut self, comment: &str) -> Result {
        for line in comment.lines() {
            self.gen.write_indent();
            writeln!(self.gen.out, "# {}", line)?;
        }
        Ok(())
    }
}