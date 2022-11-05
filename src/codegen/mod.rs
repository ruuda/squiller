// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! Utilities for generating code in various languages.

mod python;

use std::io;
use std::collections::HashSet;

/// Helper for generating nicely formatted code, and avoiding name collisions.
struct CodeGenerator<'a> {
    /// The output to write to.
    out: &'a mut dyn io::Write,

    /// Current indent, in number of spaces.
    indent: u32,

    /// The currently open scopes.
    scopes: Vec<Scope>
}

struct Scope {
    /// Names that are defined in this scope (and therefore cannot be used as
    /// identifiers for new things).
    idents: HashSet<String>
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
            idents: HashSet::new(),
        };
        self.scopes.push(scope);
        self.indent += 4;
    }

    /// Close the innermost scope.
    ///
    /// This does not write any tokens, it only does the bookkeeping.
    pub fn close_scope(&mut self) {
        self.scopes.pop().expect("Scope must be open in order to close it.");
        assert!(self.indent >= 4, "Indent must be large enough to dedent.");
        self.indent -= 4;
    }
}
