// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2023 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use crate::ast::{ArgType, Fragment, ResultType};
use crate::codegen::python::PythonCodeGenerator;
use crate::NamedDocument;

use std::io;

const PREAMBLE: &str = r#"
from __future__ import annotations

import contextlib

from typing import Any, Iterator, NamedTuple, Optional

import sqlite3


class Transaction:
    def __init__(self, conn: sqlite3.Connection) -> None:
        self.conn = conn

    def commit(self) -> None:
        self.conn.commit()
        # Ensure we cannot reuse the connection.
        self.conn = None

    def rollback(self) -> None:
        self.conn.rollback()
        self.conn = None

    def cursor(self) -> sqlite3.Cursor:
        return self.conn.cursor()
"#;

/// Generate Python code that uses the `psycopg2` package.
pub fn process_documents(out: &mut dyn io::Write, documents: &[NamedDocument]) -> io::Result<()> {
    use crate::version::{REV, VERSION};

    let mut gen = PythonCodeGenerator::new(out);

    write!(gen, "# This file was generated by Squiller {} ", VERSION)?;
    match REV {
        Some(rev) => writeln!(gen, " (commit {}).", &rev[..10])?,
        None => writeln!(gen, " (unspecified checkout).")?,
    }
    writeln!(gen, "# Input files:")?;
    for doc in documents {
        writeln!(gen, "# - {}", doc.fname.to_string_lossy())?;
    }

    gen.write(PREAMBLE)?;

    for named_document in documents {
        let input = named_document.input;

        for query in named_document.document.iter_queries() {
            let ann = &query.annotation;

            write!(gen, "\n\ndef {}(tx: Transaction", ann.name.resolve(input))?;
        }
    }

    Ok(())
}
