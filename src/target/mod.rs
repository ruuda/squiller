// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

mod debug;
mod rust_sqlite;

use std::io;

use crate::NamedDocument;

pub struct Target {
    pub name: &'static str,
    pub help: &'static str,
    pub handler: fn(&mut dyn io::Write, &[NamedDocument]) -> io::Result<()>,
}

/// The different targets that we can generate code for.
pub const TARGETS: &[Target] = &[
    Target {
        name: "help",
        help: "List all supported targets.",
        handler: |_output, _documents| {
            // We should not get here, the CLI parser handles this case.
            panic!("This pseudo-target should not be used for processing.");
        },
    },
    Target {
        name: "debug",
        help: "For debugging, run the parser and print a highlighted document.",
        handler: debug::process_documents,
    },
    Target {
        name: "rust-sqlite",
        help: "Rust with the 'sqlite' crate.",
        handler: rust_sqlite::process_documents,
    },
];

impl Target {
    /// Get a target by name.
    pub fn from_name(name: &str) -> Option<&'static Target> {
        for t in TARGETS.iter() {
            if t.name == name {
                return Some(t);
            }
        }
        None
    }

    pub fn process_files(
        &self,
        output: &mut dyn io::Write,
        documents: &[NamedDocument],
    ) -> io::Result<()> {
        (self.handler)(output, documents)
    }
}
