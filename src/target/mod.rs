// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::io;

use clap::ValueEnum;

use crate::Span;
use crate::ast::Document;

/// The different targets that we can generate code for.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Target {
    /// For debugging, run the parser and print a highlighted document.
    Debug,

    /// List all supported targets.
    Help,
}

impl Target {
    fn process_file(
        &self,
        raw_input: &[u8],
        parsed: Document<Span>,
        output: &mut dyn io::Write,
    ) -> io::Result<()> {
        match self {
            // TODO: Dispatch.
            Target::Debug => {
                let _ = raw_input;
                let _ = parsed;
                writeln!(output, "Hello.")?;
                Ok(())
            }
            Target::Help => {
                // We should not get here, the CLI parser handles this case.
                panic!("This pseudo-target should not be used for processing.");
            }
        }
    }
}
