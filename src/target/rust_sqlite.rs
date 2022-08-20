// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::io;

use crate::NamedDocument;

const PREAMBLE: &'static str = r#"
// use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::hash_map::HashMap;

use sqlite;
use sqlite::Statement;

type Result<T> = sqlite::Result<T>;

pub struct Connection<'a> {
    connection: &'a sqlite::Connection,
    statements: HashMap<u64, Statement<'a>>,
}

pub struct Transaction<'tx, 'a> {
    connection: &'a sqlite::Connection,
    statements: &'tx mut HashMap<u64, Statement<'a>>,
}

impl<'a> Connection<'a> {
    pub fn new(connection: &'a sqlite::Connection) -> Self {
        Self {
            connection,
            // TODO: We could do with_capacity here, because we know the number
            // of queries.
            statements: HashMap::new(),
        }
    }

    /// Begin a new transaction by executing the `BEGIN` statement.
    pub fn begin<'tx>(&'tx mut self) -> Result<Transaction<'tx, 'a>> {
        self.connection.execute("BEGIN;")?;
        let result = Transaction {
            connection: &self.connection,
            statements: &mut self.statements,
        };
        Ok(result)
    }
}

impl<'tx, 'a> Transaction<'tx, 'a> {
    /// Execute `COMMIT` statement.
    pub fn commit(self) -> Result<()> {
        self.connection.execute("COMMIT;")
    }

    /// Execute `ROLLBACK` statement.
    pub fn rollback(self) -> Result<()> {
        self.connection.execute("ROLLBACK;")
    }
}
"#;

// It would be nice if we could make a method for this instead of repeating the
// boilerplate in each method, but I haven't discovered a way to make it work
// lifetime-wise, because the Entry API needs to borrow self as mutable.
const _GET_STATEMENT: &'static str = r#"
        let statement = match self.statements.entry(key) {
            Occupied(entry) => entry.get_mut(),
            Vacant(vacancy) => {
                let statement = self.connection.prepare(sql)?;
                vacancy.insert(statement)
            }
        };
"#;

const MAIN: &'static str = r#"
// A useless main function, included only to make the example compile with
// Cargo’s default settings for examples.
fn main() {
    let raw_connection = sqlite::open(":memory:").unwrap();
    let mut connection = Connection::new(&raw_connection);

    let tx = connection.begin().unwrap();
    tx.rollback().unwrap();

    let tx = connection.begin().unwrap();
    tx.commit().unwrap();
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

    // TODO: Make this configurable.
    out.write_all(MAIN.as_bytes())?;

    writeln!(out, "\n")?;
    // TODO: Process the documents themselves.

    Ok(())
}
