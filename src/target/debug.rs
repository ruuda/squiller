// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::io;

use crate::ast::{ArgType, ComplexType, Fragment, ResultType, Section, SimpleType, Statement};
use crate::{NamedDocument, Span};

fn print_simple_type(
    out: &mut dyn io::Write,
    input: &str,
    type_: &SimpleType<Span>,
) -> io::Result<()> {
    let yellow = "\x1b[33m";
    let reset = "\x1b[0m";
    match type_ {
        SimpleType::Primitive { inner, .. } => {
            write!(out, "{}{}{}", yellow, inner.resolve(input), reset)
        }
        SimpleType::Option { inner, .. } => {
            write!(
                out,
                "{}option{}<{}{}{}>",
                yellow,
                reset,
                yellow,
                inner.resolve(input),
                reset
            )
        }
    }
}

fn print_complex_type(
    out: &mut dyn io::Write,
    input: &str,
    type_: &ComplexType<Span>,
) -> io::Result<()> {
    let yellow = "\x1b[33m";
    let reset = "\x1b[0m";
    match type_ {
        ComplexType::Simple(t) => print_simple_type(out, input, t)?,
        ComplexType::Tuple(_span, fields) => {
            write!(out, "(")?;
            let mut is_first = true;
            for field_type in fields {
                if !is_first {
                    write!(out, ", ")?;
                }
                print_simple_type(out, input, field_type)?;
                is_first = false;
            }
            write!(out, ")")?;
        }
        ComplexType::Struct(name_span, fields) => {
            writeln!(out, "{}{}{} {{", yellow, name_span.resolve(input), reset)?;
            for field in fields {
                write!(out, "--   {}: ", field.ident.resolve(input))?;
                print_simple_type(out, input, &field.type_)?;
                writeln!(out, ",")?;
            }
            write!(out, "-- }}")?;
        }
    }
    Ok(())
}

/// Pretty-print the parsed file, for debugging purposes.
pub fn print_statement(
    out: &mut dyn io::Write,
    input: &str,
    statement: &Statement<Span>,
) -> io::Result<()> {
    let blue = "\x1b[34;1m";
    let white = "\x1b[37;1m";
    let reset = "\x1b[0m";

    for fragment in &statement.fragments {
        match fragment {
            Fragment::Verbatim(s) => {
                write!(out, "{}", s.resolve(input))?;
            }
            Fragment::TypedIdent(raw, parsed) => {
                write!(out, "{}{}{}", blue, parsed.ident.resolve(input), reset)?;
                let mid = Span {
                    start: parsed.ident.end,
                    end: parsed.type_.span().start,
                };
                let end = Span {
                    start: parsed.type_.span().end,
                    end: raw.end,
                };
                write!(out, "{}", mid.resolve(input))?;
                print_simple_type(out, input, &parsed.type_)?;
                write!(out, "{}", end.resolve(input))?;
            }
            Fragment::Param(s) => {
                write!(out, "{}{}{}", white, s.resolve(input), reset)?;
            }
            Fragment::TypedParam(raw, parsed) => {
                write!(out, "{}{}{}", white, parsed.ident.resolve(input), reset)?;
                let mid = Span {
                    start: parsed.ident.end,
                    end: parsed.type_.span().start,
                };
                let end = Span {
                    start: parsed.type_.span().end,
                    end: raw.end,
                };
                write!(out, "{}", mid.resolve(input))?;
                print_simple_type(out, input, &parsed.type_)?;
                write!(out, "{}", end.resolve(input))?;
            }
        }
    }

    Ok(())
}

/// Pretty-print the parsed file, for debugging purposes.
pub fn process_documents(out: &mut dyn io::Write, documents: &[NamedDocument]) -> io::Result<()> {
    let red = "\x1b[31m";
    let green = "\x1b[32m";
    let reset = "\x1b[0m";

    for named_document in documents {
        let input = named_document.input;
        let document = &named_document.document;
        for section in &document.sections {
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

                    match &annotation.arguments {
                        ArgType::Args(args) => {
                            for param in args {
                                write!(out, "-- {}: ", param.ident.resolve(input))?;
                                print_simple_type(out, input, &param.type_)?;
                                writeln!(out)?;
                            }
                        }
                        ArgType::Struct {
                            var_name,
                            type_name,
                            fields,
                        } => {
                            writeln!(
                                out,
                                "-- {}: {} {{",
                                var_name.resolve(input),
                                type_name.resolve(input),
                            )?;
                            for field in fields {
                                write!(out, "--   {}: ", field.ident.resolve(input))?;
                                print_simple_type(out, input, &field.type_)?;
                                writeln!(out)?;
                            }
                            writeln!(out, "-- }}")?;
                        }
                    }

                    match &annotation.result_type {
                        ResultType::Unit => {}
                        ResultType::Option(t) => {
                            write!(out, "-- ->? ")?;
                            print_complex_type(out, input, &t)?;
                            writeln!(out)?;
                        }
                        ResultType::Single(t) => {
                            write!(out, "-- ->1 ")?;
                            print_complex_type(out, input, &t)?;
                            writeln!(out)?;
                        }
                        ResultType::Iterator(t) => {
                            write!(out, "-- ->* ")?;
                            print_complex_type(out, input, &t)?;
                            writeln!(out)?;
                        }
                    }

                    for statement in &query.statements {
                        print_statement(out, input, statement)?;
                    }
                }
            }
        }
    }

    Ok(())
}
