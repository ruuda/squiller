// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::io;

use crate::NamedDocument;

const PREAMBLE: &'static str = r#"
use sqlite;
use sqlite::Statement;
use std::collections::HashMap;

type Result<T> = sqlite::Result<T>;

pub struct Connection<'a> {
    connection: &'a sqlite::Connection,
    statements: HashMap<u64, Statement<'a>>,
};

pub struct Transaction<'a> {
    connection: &'a sqlite::Connection,
    statements: &'a mut HashMap<u64, Statement<'a>>,
};

impl<'a> Connection<'a> {
    pub fn new(connection: &'a sqlite::Connection) -> Self {
        Self { connection }
    }

    /// Begin a new transaction by executing the `BEGIN` statement.
    pub fn begin(&mut self) -> Result<Transaction> {
        self.connection.execute("BEGIN;")?;
        let result = Transaction {
            connection: &self.connection,
            statements: &mut self.statements,
        };
        Ok(result)
    }
}

impl<'a> Transaction<'a> {
    /// Execute `COMMIT` statement.
    pub fn commit(self) -> Result<()> {
        self.connection.execute("COMMIT;")
    }

    /// Execute `ROLLBACK` statement.
    pub fn rollback(self) -> Result<()> {
        self.connection.execute("ROLLBACK;")
    }

    /// Return the prepared statement for the given SQL.
    ///
    /// This ensures that statements are prepared at most once, at their first
    /// use. The cache key is statically generated; we use a hash map instead of
    /// a vector to ensure a minimal diff in the generated code after altering a
    /// query.
    fn ensure_prepared(&mut self, key: u64, sql: &'static str) -> Result<&Statement> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        match self.statements.entry(key) {
            Occupied(statement) => Ok(statement),
            Vacant(vacancy) => {
                let statement = self.connection.prepare(sql)?;
                let statement = vacancy.insert(statement);
                Ok(statement)
            }
        }
    }
}
"#;

/// Generate Rust code that uses the `sqlite` crate.
pub fn process_documents(out: &mut dyn io::Write, documents: &[NamedDocument]) -> io::Result<()> {
    writeln!(
        out,
        "// This file was generated by Querybinder <TODO: version>."
    )?;
    writeln!(out, "// Input files:")?;
    for doc in documents {
        writeln!(out, "// - {}", doc.fname.to_string_lossy())?;
    }

    out.write_all(PREAMBLE.as_bytes())?;

    writeln!(out, "\n")?;
    // TODO: Process the documents themselves.

    Ok(())
}
