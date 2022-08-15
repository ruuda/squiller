// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! The "typecheck" phase.
//!
//! There is not much to really typecheck, but we do have to resolve some
//! references and perform a few consistency checks. For example, every
//! parameter should be listed in the function signature, or it needs to have a
//! type annotation. For lack of a better word, we call this the "typecheck"
//! phase.

use std::collections::hash_map::{Entry, HashMap};
use std::collections::hash_set::HashSet;

use crate::ast::{Annotation, Document, Fragment, PrimitiveType, Query, Section, Type, TypedIdent};
use crate::error::{TResult, TypeError};
use crate::Span;

fn resolve_type(input: &str, type_: Type<Span>) -> TResult<Type<Span>> {
    match type_ {
        Type::Unit => Ok(type_),
        Type::Primitive(..) => unreachable!("We don't have primitive types yet at this stage."),
        Type::Iterator(s, inner) => Ok(Type::Iterator(s, Box::new(resolve_type(input, *inner)?))),
        Type::Option(s, inner) => Ok(Type::Option(s, Box::new(resolve_type(input, *inner)?))),
        Type::Tuple(s, ts) => {
            let resolved: Vec<_> = ts
                .into_iter()
                .map(|t| resolve_type(input, t))
                .collect::<TResult<Vec<_>>>()?;
            Ok(Type::Tuple(s, resolved))
        }
        Type::Struct(..) => unreachable!("We don't have struct types yet at this stage."),
        Type::Simple(span) => {
            match span.resolve(input) {
                "str" => Ok(Type::Primitive(span, PrimitiveType::Str)),
                "i32" => Ok(Type::Primitive(span, PrimitiveType::I32)),
                "i64" => Ok(Type::Primitive(span, PrimitiveType::I64)),
                "bytes" => Ok(Type::Primitive(span, PrimitiveType::Bytes)),
                other
                    if other
                        .bytes()
                        .next()
                        .map(|ch| ch.is_ascii_uppercase())
                        .unwrap_or(false) =>
                {
                    // If it starts with an uppercase letter, then we assume
                    // it's a struct.
                    Ok(Type::Struct(span, Vec::new()))
                }
                _other => {
                    // If it doesn't start with an uppercase letter though and
                    // we also didn't resolve it to a primitive type already,
                    // then we don't know what to do and report an error.
                    let err = TypeError::with_hint(
                        span,
                        "Unknown type.",
                        "User-defined types should start with an uppercase letter.",
                    );
                    Err(err)
                }
            }
        }
    }
}

fn resolve_annotation(input: &str, ann: Annotation<Span>) -> TResult<Annotation<Span>> {
    let mut parameters = Vec::with_capacity(ann.parameters.len());

    for param in ann.parameters {
        parameters.push(TypedIdent {
            type_: resolve_type(input, param.type_)?,
            ..param
        });
    }

    let result_type = resolve_type(input, ann.result_type)?;

    let result = Annotation {
        parameters: parameters,
        result_type: result_type,
        ..ann
    };

    Ok(result)
}

/// Holds the state across various stages of checking a query.
struct QueryChecker<'a> {
    /// Input file that the spans reference.
    input: &'a str,

    /// All the arguments specified in the annotation.
    query_args: HashMap<&'a str, &'a TypedIdent<Span>>,

    /// Arguments that are referenced in the query body.
    query_args_used: HashSet<&'a str>,

    /// Typed parameters in the query body.
    ///
    /// The key does not include the leading `:`, but the typed ident value does.
    input_fields: HashMap<&'a str, &'a TypedIdent<Span>>,

    /// Typed parameters in the query body in the order in which they occur.
    ///
    /// Does not contain duplicates, only the first reference.
    input_fields_vec: Vec<&'a TypedIdent<Span>>,

    /// Typed identifiers in the query body.
    output_fields: HashMap<&'a str, &'a TypedIdent<Span>>,

    /// Typed identifiers in the query body in the order in which they occur.
    output_fields_vec: Vec<&'a TypedIdent<Span>>,
}

impl<'a> QueryChecker<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            query_args: HashMap::new(),
            query_args_used: HashSet::new(),
            input_fields: HashMap::new(),
            input_fields_vec: Vec::new(),
            output_fields: HashMap::new(),
            output_fields_vec: Vec::new(),
        }
    }

    /// Check the query for consistency and resolve its types.
    ///
    /// Resolving means converting `Type::Simple` into either `Type::Primitive` or
    /// `Type::Struct`. Furthermore, we ensure that every query parameter that
    /// occurs in the query is known (either because the query argument is a struct,
    /// or because the parameter was listed explicitly).
    pub fn resolve_types<'b: 'a>(input: &'b str, query: Query<Span>) -> TResult<Query<Span>> {
        let annotation = resolve_annotation(input, query.annotation)?;
        let mut checker = Self::new(input);

        checker.populate_query_args(&annotation)?;

        // TODO: Need to resolve types in fragments as well.
        for fragment in &query.fragments {
            checker.populate_inputs_outputs(fragment)?;
        }

        let query = Query {
            annotation: annotation,
            ..query
        };

        Ok(query)
    }

    fn populate_query_args(&mut self, annotation: &'a Annotation<Span>) -> TResult<()> {
        // Populate the query args map with the args those provided in the
        // annotation, and at the same time ensure there are no duplicates.
        for arg in &annotation.parameters {
            let name = arg.ident.resolve(self.input);
            match self.query_args.entry(name) {
                Entry::Vacant(vacancy) => vacancy.insert(arg),
                Entry::Occupied(previous) => {
                    let error = TypeError::with_note(
                        arg.ident,
                        "Redefinition of query parameter.",
                        previous.get().ident,
                        "First defined here.",
                    );
                    return Err(error);
                }
            };
        }
        Ok(())
    }

    /// Handle a single fragment of the query body, populate inputs and outputs.
    fn populate_inputs_outputs(&mut self, fragment: &'a Fragment<Span>) -> TResult<()> {
        match fragment {
            Fragment::Verbatim(..) => return Ok(()),
            Fragment::TypedIdent(_span, ti) => {
                // A typed identifier is an output that the query selects.
                let name = ti.ident.resolve(self.input);
                match self.output_fields.entry(name) {
                    Entry::Vacant(vacancy) => {
                        vacancy.insert(ti);
                        self.output_fields_vec.push(ti);
                    }
                    Entry::Occupied(_) => {
                        panic!("TODO: Report duplicate select error.");
                    }
                }
            }
            Fragment::Param(span) => {
                // If there is a bare parameter without type annotation, then it
                // must be defined already.

                // Trim off the `:` that query parameters start with.
                let name = span.trim_start(1).resolve(self.input);
                match self.query_args.get(name) {
                    Some(..) => {
                        // Record that the argument was used, so that we can
                        // warn about unused arguments later.
                        self.query_args_used.insert(name);
                    }
                    None => {
                        panic!("TODO: Report unknown query param error.");
                    }
                }
            }
            Fragment::TypedParam(_span, ti) => {
                // A typed parameter is an input to the query that should not
                // occur in the arguments already.
                let name = ti.ident.trim_start(1).resolve(self.input);
                match self.input_fields.entry(name) {
                    Entry::Vacant(vacancy) => {
                        vacancy.insert(ti);
                        self.input_fields_vec.push(ti);
                    }
                    Entry::Occupied(_) => {
                        panic!("TODO: Verify that the two are compatible.");
                    }
                }
                match self.query_args.get(name) {
                    None => { /* Fine, no conflict. */ }
                    Some(_) => {
                        panic!("TODO: Verify that the two are compatible.");
                    }
                }
            }
        }
        Ok(())
    }
}

/// Apply `resolve_types` to every query in the document.
pub fn check_document(input: &str, doc: Document<Span>) -> TResult<Document<Span>> {
    let mut sections = Vec::with_capacity(doc.sections.len());

    for section in doc.sections {
        match section {
            Section::Verbatim(s) => sections.push(Section::Verbatim(s)),
            Section::Query(q) => {
                sections.push(Section::Query(QueryChecker::resolve_types(input, q)?))
            }
        }
    }

    let result = Document { sections };

    Ok(result)
}
