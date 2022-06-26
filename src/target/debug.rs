// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::io;

use crate::ast::{Document, Fragment, Section};
use crate::Span;

/// Pretty-print the parsed file, for debugging purposes.
pub fn process_file(
    input: &str,
    parsed: Document<Span>,
    out: &mut dyn io::Write,
) -> io::Result<()> {
    let red = "\x1b[31m";
    let green = "\x1b[32m";
    let yellow = "\x1b[33m";
    let blue = "\x1b[34;1m";
    let white = "\x1b[37;1m";
    let reset = "\x1b[0m";

    for section in &parsed.sections {
        match section {
            Section::Verbatim(s) => {
                write!(out, "{}", s.resolve(input))?;
            }
            Section::Query(query) => {
                let annotation = &query.annotation;

                for doc_line in &query.docs {
                    writeln!(out, "{}--{}", red, doc_line.resolve(input))?;
                }

                writeln!(
                    out,
                    "{}-- {}@query{} {}",
                    reset,
                    green,
                    reset,
                    annotation.name.resolve(input)
                )?;

                for param in &annotation.parameters {
                    writeln!(
                        out,
                        "{}-- {}: {}{:?}",
                        reset,
                        param.ident.resolve(input),
                        yellow,
                        param.type_.resolve(input),
                    )?;
                }
                writeln!(
                    out,
                    "{}-- -> {}{:?}{}",
                    reset,
                    yellow,
                    annotation.result_type.resolve(input),
                    reset,
                )?;

                for fragment in &query.fragments {
                    match fragment {
                        Fragment::Verbatim(s) => {
                            write!(out, "{}", s.resolve(input))?;
                        }
                        Fragment::TypedIdent(raw, _parsed) => {
                            write!(out, "{}{}{}", blue, raw.resolve(input), reset)?;
                        }
                        Fragment::Param(s) => {
                            write!(out, "{}{}{}", white, s.resolve(input), reset)?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
