// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

mod debug;
mod rust_sqlite;

use std::io;

use clap::ValueEnum;

use crate::NamedDocument;

/// The different targets that we can generate code for.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Target {
    /// List all supported targets.
    Help,

    /// For debugging, run the parser and print a highlighted document.
    Debug,

    /// Rust with the `sqlite` crate.
    RustSqlite,
}

impl Target {
    pub fn process_files(
        &self,
        output: &mut dyn io::Write,
        documents: &[NamedDocument],
    ) -> io::Result<()> {
        match self {
            Target::Help => {
                // We should not get here, the CLI parser handles this case.
                panic!("This pseudo-target should not be used for processing.");
            }
            Target::Debug => debug::process_documents(output, documents),
            Target::RustSqlite => rust_sqlite::process_documents(output, documents),
        }
    }
}
