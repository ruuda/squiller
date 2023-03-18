// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! Utilities for generating code in various languages.

pub mod python;

use std::collections::HashSet;
use std::io;

type Result = io::Result<()>;

/// Helper for generating nicely formatted code, and avoiding name collisions.
///
/// The base state for the code generator is to be at the start of a new line,
/// no indent applied yet. Most appends should append one logical segment (e.g.
/// a function call, or a function signature) and terminate that with a newline.
struct CodeGenerator<'a> {
    /// The output to write to.
    out: &'a mut dyn io::Write,

    /// Current indent, in number of spaces.
    indent: u32,

    /// The currently open scopes.
    scopes: Vec<Scope>,
}

struct Scope {
    /// Names that are defined in this scope (and therefore cannot be used as
    /// identifiers for new things).
    _idents: HashSet<String>,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(out: &'a mut dyn io::Write) -> CodeGenerator<'a> {
        Self {
            out,
            indent: 0,
            scopes: Vec::new(),
        }
    }

    /// Begin a new scope.
    ///
    /// This does not write any tokens, it only does the bookkeeping.
    pub fn open_scope(&mut self) {
        let scope = Scope {
            _idents: HashSet::new(),
        };
        self.scopes.push(scope);
        self.indent += 4;
    }

    /// Close the innermost scope.
    ///
    /// This does not write any tokens, it only does the bookkeeping.
    pub fn close_scope(&mut self) {
        self.scopes
            .pop()
            .expect("Scope must be open in order to close it.");
        assert!(self.indent >= 4, "Indent must be large enough to dedent.");
        self.indent -= 4;
    }

    /// Write as many spaces as the current indent.
    pub fn write_indent(&mut self) -> Result {
        assert!(self.indent <= 32, "Indent is too big.");
        let thirty_two_spaces = "                                ";
        self.out
            .write_all(&thirty_two_spaces.as_bytes()[..self.indent as usize])
    }
}
