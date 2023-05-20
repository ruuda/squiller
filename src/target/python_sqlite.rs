// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2023 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! Target Python and `sqlite3` module.

use std::io;

use crate::codegen::Block;
use crate::target::python;
use crate::NamedDocument;

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

/// Generate Python code that uses the `sqlite` module.
fn format_documents(documents: &[NamedDocument]) -> Block {
    let mut root = Block::new();
    root.push_block(python::header_comment(documents));
    root.push_line(PREAMBLE.to_string());

    for named_document in documents {
        let input = named_document.input;

        for query in named_document.document.iter_queries() {
            let ann = &query.annotation;
            let sig = python::function_signature(ann, input);

            let mut function_body = Block::new();
            function_body.push_block(python::docstring(&query.docs, input));

            root.push_block(sig);
            root.push_block(function_body.indent());
        }
    }

    root
}

/// Generate Python code that uses the `sqlite` module.
pub fn process_documents(
    out: &mut dyn io::Write,
    documents: &[NamedDocument],
) -> std::io::Result<()> {
    format_documents(documents).format(out)
}
