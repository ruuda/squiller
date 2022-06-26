// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

mod debug;

use std::io;

use clap::ValueEnum;

use crate::ast::Document;
use crate::Span;

/// The different targets that we can generate code for.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Target {
    /// For debugging, run the parser and print a highlighted document.
    Debug,

    /// List all supported targets.
    Help,
}

impl Target {
    pub fn process_file(
        &self,
        raw_input: &str,
        parsed: Document<Span>,
        output: &mut dyn io::Write,
    ) -> io::Result<()> {
        match self {
            Target::Debug => crate::target::debug::process_file(raw_input, parsed, output),
            Target::Help => {
                // We should not get here, the CLI parser handles this case.
                panic!("This pseudo-target should not be used for processing.");
            }
        }
    }
}
