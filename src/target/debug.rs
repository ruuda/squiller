// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::io;

use crate::ast::{Document, Fragment, Section, Type};
use crate::Span;

fn print_type(out: &mut dyn io::Write, input: &str, type_: &Type<Span>) -> io::Result<()> {
    let yellow = "\x1b[33m";
    let reset = "\x1b[0m";

    write!(out, "{}", yellow)?;

    match type_ {
        Type::Unit => {
            panic!("Unit should never be printed.");
        }
        Type::Simple(..) => {
            panic!("Simple types should have been resolved by now.");
        }
        Type::Primitive(span, _) => {
            write!(out, "{}{}{}", yellow, span.resolve(input), reset)?;
        }
        Type::Iterator(_span, inner) => {
            write!(out, "{}Iterator{}<", yellow, reset)?;
            print_type(out, input, inner)?;
            write!(out, ">")?;
        }
        Type::Option(_span, inner) => {
            write!(out, "{}Option{}<", yellow, reset)?;
            print_type(out, input, inner)?;
            write!(out, ">")?;
        }
        Type::Tuple(_span, fields) => {
            write!(out, "(")?;
            let mut is_first = true;
            for field_type in fields {
                if !is_first {
                    write!(out, ", ")?;
                }
                print_type(out, input, field_type)?;
                is_first = false;
            }
            write!(out, ")")?;
        }
        Type::Struct(name_span, fields) => {
            writeln!(out, "{}{}{} {{", yellow, name_span.resolve(input), reset)?;
            for field in fields {
                write!(out, "--   {}: ", field.ident.resolve(input))?;
                print_type(out, input, &field.type_)?;
                writeln!(out, ",")?;
            }
            write!(out, "-- }}")?;
        }
    }

    Ok(())
}

/// Pretty-print the parsed file, for debugging purposes.
pub fn process_file(
    input: &str,
    parsed: Document<Span>,
    out: &mut dyn io::Write,
) -> io::Result<()> {
    let red = "\x1b[31m";
    let green = "\x1b[32m";
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
                    write!(out, "-- {}: ", param.ident.resolve(input))?;
                    print_type(out, input, &param.type_)?;
                    writeln!(out)?;
                }

                match annotation.result_type {
                    Type::Unit => {}
                    _ => {
                        write!(out, "-- -> ")?;
                        print_type(out, input, &annotation.result_type)?;
                        writeln!(out)?;
                    }
                }

                for fragment in &query.fragments {
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
                            print_type(out, input, &parsed.type_)?;
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
                            print_type(out, input, &parsed.type_)?;
                            write!(out, "{}", end.resolve(input))?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
