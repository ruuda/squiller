// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use crate::ast::{Annotation, Fragment, PrimitiveType, Type, TypedIdent};
use std::io;

use crate::NamedDocument;

const PREAMBLE: &'static str = r#"
// use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::hash_map::HashMap;

use sqlite;
use sqlite::Statement;

pub type Result<T> = sqlite::Result<T>;

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

#[derive(Copy, Clone)]
enum Ownership {
    Borrow,
    BorrowNamed,
    Owned,
}

fn write_type(
    out: &mut dyn io::Write,
    owned: Ownership,
    type_: &Type<&str>,
) -> io::Result<()> {
    use Ownership::{Borrow, BorrowNamed, Owned};
    match type_ {
        Type::Unit => write!(out, "()"),
        Type::Simple(..) => panic!("Should not occur any more at output time."),
        Type::Primitive(_, prim) => {
            let name = match (prim, owned) {
                (PrimitiveType::Str, Borrow) => "&str",
                (PrimitiveType::Str, BorrowNamed) => "&'a str",
                (PrimitiveType::Str, Owned) => "String",
                (PrimitiveType::Bytes, Borrow) => "&[u8]",
                (PrimitiveType::Bytes, BorrowNamed) => "&'a [u8]",
                (PrimitiveType::Bytes, Owned) => "Vec<u8>",
                (PrimitiveType::I32, _) => "i32",
                (PrimitiveType::I64, _) => "i64",
            };
            out.write_all(name.as_bytes())
        }
        Type::Iterator(_full_span, inner) => {
            // TODO: What to do with generated iterator types?
            write!(out, "impl Iterator<Item = ")?;
            write_type(out, owned, inner)?;
            write!(out, ">")
        }
        Type::Option(_full_span, inner) => {
            write!(out, "Option<")?;
            write_type(out, owned, inner)?;
            write!(out, ">")
        }
        Type::Tuple(_full_span, fields) => {
            write!(out, "(")?;
            let mut is_first = true;
            for field_type in fields {
                if !is_first {
                    write!(out, ", ")?;
                }
                write_type(out, owned, field_type)?;
                is_first = false;
            }
            write!(out, ")")
        }
        Type::Struct(name, _fields) => write!(out, "{}", name),
    }
}

/// Generate Rust code for a struct type.
fn write_struct_definition(
    out: &mut dyn io::Write,
    owned: Ownership,
    name: &str,
    fields: &[TypedIdent<&str>],
) -> io::Result<()> {
    // TODO: Would be nice to generate docs for cross-referencing.
    writeln!(out, "\npub struct {} {{", name)?;
    for field in fields {
        write!(out, "    pub {}: ", field.ident)?;
        write_type(out, owned, &field.type_)?;
        writeln!(out, ",")?;
    }
    writeln!(out, "}}")
}

/// Generate code for all structs that occur in the query's type.
fn write_struct_definitions(
    out: &mut dyn io::Write,
    annotation: Annotation<&str>,
) -> io::Result<()> {
    for param in &annotation.parameters {
        param.type_.traverse(&mut |type_| match type_ {
            Type::Struct(name, fields) =>
                write_struct_definition(out, Ownership::BorrowNamed, name, &fields),
            _ => Ok(()),
        })?;
    }

    annotation.result_type.traverse(&mut |type_| match type_ {
        Type::Struct(name, fields) =>
            write_struct_definition(out, Ownership::Owned, name, fields),
        _ => Ok(()),
    })
}

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

    for named_document in documents {
        let input = named_document.input;

        for query in named_document.document.iter_queries() {
            let ann = &query.annotation;

            // Before the query itself, define any types that it may reference.
            // For now, we put these interspersed with the queries. If we share
            // struct types in the future, we might group all types before the
            // queries.
            write_struct_definitions(out, query.annotation.resolve(input))?;

            writeln!(out)?;

            for doc_line in &query.docs {
                writeln!(out, "///{}", doc_line.resolve(input))?;
            }

            write!(
                out,
                "pub fn {}(tx: &mut Transaction",
                ann.name.resolve(input)
            )?;

            for arg in &ann.parameters {
                write!(out, ", {}: ", arg.ident.resolve(input),)?;
                write_type(out, Ownership::Borrow, &arg.type_.resolve(input))?;
            }

            write!(out, ") -> Result<")?;
            match &ann.result_type {
                Type::Unit => write!(out, "()")?,
                not_unit => write_type(out, Ownership::Owned, &not_unit.resolve(input))?,
            }
            writeln!(out, "> {{")?;

            // TODO: indent the query.
            writeln!(out, "    let sql = r#\"")?;
            // TODO: Include the source file name and line number as a comment.
            for fragment in &query.fragments {
                let span = match fragment {
                    Fragment::Verbatim(span) => span,
                    Fragment::Param(span) => span,
                    // When we put the SQL in the source code, omit the type
                    // annotations, it's only a distraction.
                    Fragment::TypedIdent(_full_span, ti) => &ti.ident,
                    Fragment::TypedParam(_full_span, ti) => &ti.ident,
                };
                out.write_all(span.resolve(input).as_bytes())?;
            }
            writeln!(out, "\n    \"#;")?;
            writeln!(out, "    Ok(())")?;
            writeln!(out, "}}")?;
        }
    }

    // TODO: Make this configurable.
    out.write_all(MAIN.as_bytes())?;

    Ok(())
}
