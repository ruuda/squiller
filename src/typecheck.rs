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

use crate::ast::{Annotation, Document, PrimitiveType, Query, Section, Type, TypedIdent};
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

/// Check the query for consistency and resolve its types.
///
/// Resolving means converting `Type::Simple` into either `Type::Primitive` or
/// `Type::Struct`. Furthermore, we ensure that every query parameter that
/// occurs in the query is known (either because the query argument is a struct,
/// or because the parameter was listed explicitly).
pub fn resolve_types(input: &str, query: Query<Span>) -> TResult<Query<Span>> {
    let annotation = resolve_annotation(input, query.annotation)?;

    let query = Query {
        annotation: annotation,
        ..query
    };

    Ok(query)
}

/// Apply `resolve_types` to every query in the document.
pub fn check_document(input: &str, doc: Document<Span>) -> TResult<Document<Span>> {
    let mut sections = Vec::with_capacity(doc.sections.len());

    for section in doc.sections {
        match section {
            Section::Verbatim(s) => sections.push(Section::Verbatim(s)),
            Section::Query(q) => sections.push(Section::Query(resolve_types(input, q)?)),
        }
    }

    let result = Document { sections };

    Ok(result)
}
