// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2023 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

/// Target Python and `sqlite3` module.

use std::io;

use crate::NamedDocument;
use crate::codegen::python::PythonCodeGenerator;
use crate::codegen::Result;
use crate::target::python;

const PREAMBLE: &str = r#"
from __future__ import annotations

import contextlib

from typing import Any, Iterator, NamedTuple, Optional

import sqlite3


class Transaction:
    def __init__(self, conn: sqlite3.Connection) -> None:
        self.conn = conn
        self.cursor = conn.cursor()
        self.cursor.execute("BEGIN DEFERRED")

    def commit(self) -> None:
        self.conn.commit()
        # Ensure we cannot reuse the connection.
        self.conn = None
        self.cursor = None

    def rollback(self) -> None:
        self.conn.rollback()
        self.conn = None
        self.cursor = None

"#;

/// Generate Python code that uses the `psycopg2` package.
pub fn process_documents(out: &mut dyn io::Write, documents: &[NamedDocument]) -> Result {
    let mut gen = PythonCodeGenerator::new(out);

    python::write_header_comment(&mut gen, documents)?;
    gen.write(PREAMBLE)?;

    for named_document in documents {
        let input = named_document.input;

        for query in named_document.document.iter_queries() {
            let ann = &query.annotation;

            python::write_function_signature(&mut gen, ann, input)?;
            gen.open_scope();
            python::write_docstring(&mut gen, &query.docs, input)?;

            gen.write_indent()?;
            gen.write("return None\n")?;
            gen.close_scope();
        }
    }

    Ok(())
}
