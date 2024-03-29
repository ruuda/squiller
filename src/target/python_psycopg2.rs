// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! Target Python and `psycopg2` package.

use crate::ast::Fragment;
use crate::codegen::Block;
use crate::target::python;
use crate::{NamedDocument, Span};

use std::io;

const PREAMBLE: &str = r#"
from __future__ import annotations

import contextlib

from typing import Any, Iterator, NamedTuple, Optional

import psycopg2.extensions  # type: ignore
import psycopg2.extras  # type: ignore
import psycopg2.pool  # type: ignore


class Transaction:
    def __init__(self, conn: psycopg2.extensions.connection) -> None:
        self.conn = conn

    def commit(self) -> None:
        self.conn.commit()
        # Ensure we cannot reuse the connection.
        self.conn = None

    def rollback(self) -> None:
        self.conn.rollback()
        self.conn = None

    def cursor(self) -> psycopg2.extensions.cursor:
        return self.conn.cursor()


class ConnectionPool(NamedTuple):
    pool: psycopg2.pool.ThreadedConnectionPool

    @contextlib.contextmanager
    def begin(self) -> Iterator[Transaction]:
        conn: Optional[psycopg2.extensions.connection] = None
        try:
            # Use psycopg2 in "no-autocommit" mode, where it implicitly starts a
            # transaction at the first statement, and we need to explicitly
            # commit() or rollback() afterwards.
            conn = self.pool.getconn()
            conn.isolation_level = "SERIALIZABLE"
            conn.autocommit = False
            yield Transaction(conn)

        except:
            if conn is not None:
                self.pool.putconn(conn, close=True)
            raise

        else:
            assert conn is not None
            self.pool.putconn(conn, close=False)
"#;

/// Generate Python code that uses the `psycopg2` package.
pub fn format_documents(documents: &[NamedDocument]) -> Block {
    let mut root = Block::new();
    root.push_block(python::header_comment(documents));
    root.push_line(PREAMBLE.trim_end().to_string());

    for named_document in documents {
        let input = named_document.input;

        for query in named_document.document.iter_queries() {
            let ann = &query.annotation;
            let sig = python::function_signature(ann, input);

            let mut function_body = Block::new();
            function_body.push_block(python::docstring(&query.docs, input));

            for statement in query.statements.iter() {
                // TODO: Include the source file name and line number as a comment.
                function_body.push_line_str("sql =\\");
                function_body.push_block(sql_string(&statement.fragments, input).indent());

                if statement.iter_parameters().next().is_some() {
                    // Write the parameter tuple. We used the counted %s-style
                    // references rather than the named ones (to save a dict lookup),
                    // so we just write out the references in the same order, if the
                    // same parameter is referenced twice, it occurs twice in the tuple.
                    function_body.push_line_str("params = (");
                    let mut param_block = Block::new();
                    for param in statement.iter_parameters() {
                        // Cut off the leading ':' from the parameter name.
                        let variable_name = param.trim_start(1).resolve(input);
                        // TODO: Deal with prefix in case we are accessing a struct.
                        param_block.push_line(format!("{},", variable_name));
                    }
                    function_body.push_block(param_block.indent());
                    function_body.push_line_str(")");
                } else {
                    function_body.push_line_str("params = ()");
                }
            }

            function_body.push_line_str("return None");

            root.push_block(sig);
            root.push_block(function_body.indent());
        }
    }

    root
}

/// Format the SQL string, with parameters substituted with placeholders.
pub fn sql_string(fragments: &[Fragment<Span>], input: &str) -> Block {
    let mut block = Block::new();
    block.push_line_str("\"\"\"");

    let mut sql = String::new();
    for fragment in fragments {
        let span = match fragment {
            Fragment::Verbatim(span) => span.resolve(input),
            Fragment::Param(_span) => "%s",
            // When we put the SQL in the source code, omit the type
            // annotations, it's only a distraction.
            Fragment::TypedIdent(_full_span, ti) => ti.ident.resolve(input),
            Fragment::TypedParam(_full_span, _ti) => "%s",
        };
        sql.push_str(span);
    }
    for line in sql.lines() {
        block.push_line_str(line);
    }

    block.push_line_str("\"\"\"");
    block
}

/// Generate Python code that uses the `psycopg2` package.
pub fn process_documents(out: &mut dyn io::Write, documents: &[NamedDocument]) -> io::Result<()> {
    format_documents(documents).format(out)
}
