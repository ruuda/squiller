// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use crate::ast::{
    Annotation, ArgType, ComplexType, Fragment, PrimitiveType, ResultType, SimpleType, TypedIdent,
};
use crate::NamedDocument;

use std::collections::hash_set::HashSet;
use std::io;

const PREAMBLE: &'static str = r#"
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::hash_map::HashMap;

use sqlite;
use sqlite::{State::{Row, Done}, Statement};

pub type Result<T> = sqlite::Result<T>;

pub struct Connection<'a> {
    connection: &'a sqlite::Connection,
    statements: HashMap<*const u8, Statement<'a>>,
}

pub struct Transaction<'tx, 'a> {
    connection: &'a sqlite::Connection,
    statements: &'tx mut HashMap<*const u8, Statement<'a>>,
}

pub struct Iter<'i, 'a, T> {
    statement: &'i mut Statement<'a>,
    decode_row: fn(&Statement<'a>) -> Result<T>,
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

impl<'i, 'a, T> Iterator for Iter<'i, 'a, T> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Result<T>> {
        match self.statement.next() {
            Ok(Row) => Some((self.decode_row)(self.statement)),
            Ok(Done) => None,
            Err(err) => Some(Err(err)),
        }
    }
}
"#;

// It would be nice if we could make a method for this instead of repeating the
// boilerplate in each method, but I haven't discovered a way to make it work
// lifetime-wise, because the Entry API needs to borrow self as mutable.
const GET_STATEMENT: &'static str = r#"
    let statement = match tx.statements.entry(sql.as_ptr()) {
        Occupied(entry) => entry.into_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
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

#[derive(Copy, Clone, Eq, PartialEq)]
enum Ownership {
    Borrow,
    BorrowNamed,
    Owned,
}

fn write_primitive_type(
    out: &mut dyn io::Write,
    owned: Ownership,
    type_: PrimitiveType,
) -> io::Result<()> {
    use Ownership::{Borrow, BorrowNamed, Owned};
    let name = match (type_, owned) {
        (PrimitiveType::Str, Borrow) => "&str",
        (PrimitiveType::Str, BorrowNamed) => "&'a str",
        (PrimitiveType::Str, Owned) => "String",
        (PrimitiveType::Bytes, Borrow) => "&[u8]",
        (PrimitiveType::Bytes, BorrowNamed) => "&'a [u8]",
        (PrimitiveType::Bytes, Owned) => "Vec<u8>",
        (PrimitiveType::I32, _) => "i32",
        (PrimitiveType::I64, _) => "i64",
        // TODO: Convert to f64 under the hood.
        (PrimitiveType::F32, _) => panic!("f32 is not supported for rust-sqlite right now."),
        (PrimitiveType::F64, _) => "f64",
    };
    out.write_all(name.as_bytes())
}

fn write_simple_type(
    out: &mut dyn io::Write,
    owned: Ownership,
    type_: &SimpleType<&str>,
) -> io::Result<()> {
    match type_ {
        SimpleType::Primitive { type_: t, .. } => write_primitive_type(out, owned, *t)?,
        SimpleType::Option { type_: t, .. } => {
            write!(out, "Option<")?;
            write_primitive_type(out, owned, *t)?;
            write!(out, ">")?;
        }
    }
    Ok(())
}

fn write_complex_type(
    out: &mut dyn io::Write,
    owned: Ownership,
    type_: &ComplexType<&str>,
) -> io::Result<()> {
    match type_ {
        ComplexType::Simple(t) => write_simple_type(out, owned, t),
        ComplexType::Struct(name, _fields) => write!(out, "{}", name),
        ComplexType::Tuple(_full_span, fields) => {
            write!(out, "(")?;
            let mut is_first = true;
            for field_type in fields {
                if !is_first {
                    write!(out, ", ")?;
                }
                write_simple_type(out, owned, field_type)?;
                is_first = false;
            }
            write!(out, ")")
        }
    }
}

/// Generate Rust code for a struct type.
fn write_struct_definition(
    out: &mut dyn io::Write,
    owned: Ownership,
    name: &str,
    fields: &[TypedIdent<&str>],
) -> io::Result<()> {
    // TODO: This all feels a bit ad-hoc. I should probably parametrize the AST
    // over the type type, then add a pass that translates the language-agnostic
    // types into Rust types, and then have some helper methods on those for this
    // kind of stuff.
    let has_lifetime_types = fields.iter().any(|field| match field.type_.inner_type() {
        PrimitiveType::Str => true,
        PrimitiveType::Bytes => true,
        _ => false,
    });

    // TODO: Would be nice to generate docs for cross-referencing.
    writeln!(out, "\n#[derive(Debug)]")?;
    write!(out, "pub struct {}", name)?;

    if has_lifetime_types && owned == Ownership::BorrowNamed {
        write!(out, "<'a>")?;
    }

    writeln!(out, " {{")?;

    for field in fields {
        write!(out, "    pub {}: ", field.ident)?;
        write_simple_type(out, owned, &field.type_)?;
        writeln!(out, ",")?;
    }
    writeln!(out, "}}")
}

/// Generate code for all structs that occur in the query's type.
fn write_struct_definitions(
    out: &mut dyn io::Write,
    annotation: Annotation<&str>,
) -> io::Result<()> {
    match &annotation.arguments {
        ArgType::Struct {
            type_name, fields, ..
        } => {
            write_struct_definition(out, Ownership::BorrowNamed, type_name, &fields)?;
        }
        ArgType::Args(..) => {}
    }

    match annotation.result_type.get() {
        Some(ComplexType::Struct(name, fields)) => {
            write_struct_definition(out, Ownership::Owned, name, fields)
        }
        _ => Ok(()),
    }
}

/// Generate code that calls `.read` on the statement, and constructs a return value.
fn write_return_value(
    out: &mut dyn io::Write,
    index: usize,
    type_: ComplexType<&str>,
) -> io::Result<()> {
    match type_ {
        ComplexType::Simple(..) => {
            write!(out, "statement.read({})?", index)?;
        }
        ComplexType::Tuple(_, fields) => {
            writeln!(out, "(")?;
            for (i, _field_type) in (index..).zip(fields) {
                writeln!(out, "        statement.read({})?,", i)?;
            }
            write!(out, ")")?;
        }
        ComplexType::Struct(name, fields) => {
            writeln!(out, "{} {{", name)?;
            // TODO: Once we unify types across multiple queries, the index of
            // the fields may not be the order in which they occur.
            for (i, field) in (index..).zip(fields) {
                writeln!(out, "        {}: statement.read({})?,", field.ident, i)?;
            }
            write!(out, "    }}")?;
        }
    }

    Ok(())
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

            write!(out, "pub fn {}", ann.name.resolve(input))?;
            match &ann.result_type {
                ResultType::Iterator(..) => {
                    write!(out, "<'i, 't, 'a>(tx: &'i mut Transaction<'t, 'a>")?;
                }
                _ => {
                    write!(out, "(tx: &mut Transaction")?;
                }
            }

            match &ann.arguments {
                ArgType::Args(args) => {
                    for arg in args {
                        write!(out, ", {}: ", arg.ident.resolve(input),)?;
                        write_simple_type(out, Ownership::Borrow, &arg.type_.resolve(input))?;
                    }
                }
                ArgType::Struct {
                    var_name,
                    type_name,
                    ..
                } => {
                    write!(
                        out,
                        ", {}: {}",
                        var_name.resolve(input),
                        type_name.resolve(input)
                    )?;
                }
            }

            write!(out, ") -> Result<")?;
            match &ann.result_type {
                ResultType::Unit => write!(out, "()")?,
                ResultType::Option(t) => {
                    write!(out, "Option<")?;
                    write_complex_type(out, Ownership::Owned, &t.resolve(input))?;
                    write!(out, ">")?;
                }
                ResultType::Single(t) => {
                    write_complex_type(out, Ownership::Owned, &t.resolve(input))?;
                }
                ResultType::Iterator(t) => {
                    write!(out, "Iter<'i, 'a, ")?;
                    write_complex_type(out, Ownership::Owned, &t.resolve(input))?;
                    write!(out, ">")?;
                }
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

            // The literal starts with a newline that we don't want here.
            // TODO: For now we use the address of the literal as the cache key.
            // But we should instead use a precomputed hash of the query, so that
            // LLVM can constant-fold the hash function.
            out.write_all(&GET_STATEMENT.as_bytes()[1..])?;

            // Next we bind all query parameters.
            let prefix = &match query.annotation.arguments {
                ArgType::Struct { var_name, .. } => {
                    let mut prefix = var_name.resolve(input).to_string();
                    prefix.push('.');
                    prefix
                }
                _ => String::new(),
            };
            writeln!(out, "    statement.reset()?;")?;
            let mut param_nr = 1;
            let mut params_seen = HashSet::new();
            for param in query.iter_parameters() {
                // Cut off the leading ':' from the parameter name.
                let variable_name = param.trim_start(1).resolve(input);

                // SQLite numbers parameters by unique name, so if the same
                // name occurs twice, we should only bind it once.
                // TODO: Add a golden test for this, because we failed this in
                // the past.
                let first_seen = params_seen.insert(variable_name);
                if first_seen {
                    writeln!(out, "    statement.bind({}, {}{})?;", param_nr, prefix, variable_name)?;
                    param_nr += 1;
                };
            }

            if let Some(type_) = query.annotation.result_type.get() {
                write!(out, "    let decode_row = |statement: &Statement| Ok(")?;
                write_return_value(out, 0, type_.resolve(input))?;
                writeln!(out, ");")?;
            }

            match &query.annotation.result_type {
                ResultType::Unit => {
                    writeln!(out, "    let result = match statement.next()? {{")?;
                    writeln!(
                        out,
                        "        Row => panic!(\"Query '{}' unexpectedly returned a row.\"),",
                        query.annotation.name.resolve(input)
                    )?;
                    writeln!(out, "        Done => (),")?;
                    writeln!(out, "    }};")?;
                }
                ResultType::Option(..) => {
                    writeln!(out, "    let result = match statement.next()? {{")?;
                    writeln!(out, "        Row => Some(decode_row(statement)?),")?;
                    writeln!(out, "        Done => None,")?;
                    writeln!(out, "    }};")?;
                    // Call next() until Done, even though we know we should be
                    // done at this point. Without it, we cannot commit, SQLite
                    // complains: "SQL statements in progress".
                    // Should we join the two conditions with &&? It saves two
                    // lines of code and rightward drift, but having a
                    // side-effect not be executed due to short circuiting && is
                    // quite subtle, I would not call that readable code.
                    writeln!(out, "    if result.is_some() {{")?;
                    writeln!(out, "        if statement.next()? != Done {{")?;
                    writeln!(out, "            panic!(\"Query '{}' should return at most one row.\");",
                        query.annotation.name.resolve(input)
                    )?;
                    writeln!(out, "        }}")?;
                    writeln!(out, "    }}")?;
                }
                ResultType::Single(..) => {
                    writeln!(out, "    let result = match statement.next()? {{")?;
                    writeln!(out, "        Row => decode_row(statement)?,")?;
                    writeln!(
                        out,
                        "        Done => panic!(\"Query '{}' should return exactly one row.\"),",
                        query.annotation.name.resolve(input)
                    )?;
                    writeln!(out, "    }};")?;
                    // Call next() until Done, see also the note further above.
                    writeln!(out, "    if statement.next()? != Done {{")?;
                    writeln!(out, "        panic!(\"Query '{}' should return exactly one row.\");",
                        query.annotation.name.resolve(input)
                    )?;
                    writeln!(out, "    }}")?;
                }
                ResultType::Iterator(..) => {
                    writeln!(out, "    let result = Iter {{ statement, decode_row }};")?;
                }
            }

            writeln!(out, "    Ok(result)")?;
            writeln!(out, "}}")?;
        }
    }

    // TODO: Make this configurable.
    out.write_all(MAIN.as_bytes())?;

    Ok(())
}
