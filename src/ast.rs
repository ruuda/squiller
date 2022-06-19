// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use crate::Span;

/// Types of parameters and results.
#[derive(Debug, Eq, PartialEq)]
pub enum Type<TSpan> {
    /// The unit type, for queries that do not return anything.
    Unit,

    /// A simple type that for our purposes cannot be broken down further.
    Simple(TSpan),

    /// An iterator, for queries that may return multiple rows.
    Iterator(Box<Type<TSpan>>),

    /// An option, for queries that return zero or one rows, or nullable parameters.
    Option(Box<Type<TSpan>>),

    /// A tuple of zero or more types.
    Tuple(Vec<Type<TSpan>>),

    /// A named struct with typed fields.
    ///
    /// Structs fields cannot be listed explicitly in annotations; the
    /// annotation lists the name, and the fields are determined from the query.
    Struct(TSpan, Vec<TypedIdent<TSpan>>),
}

impl Type<Span> {
    pub fn resolve<'a>(&self, input: &'a [u8]) -> Type<&'a str> {
        match self {
            Type::Unit => Type::Unit,
            Type::Simple(span) => Type::Simple(span.resolve(input)),
            Type::Iterator(t) => Type::Iterator(Box::new(t.resolve(input))),
            Type::Option(t) => Type::Option(Box::new(t.resolve(input))),
            Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| t.resolve(input)).collect()),
            Type::Struct(name, fields) => Type::Struct(
                name.resolve(input),
                fields.iter().map(|f| f.resolve(input)).collect(),
            ),
        }
    }
}

/// An identifier and a type, e.g. `name: &str`.
#[derive(Debug, Eq, PartialEq)]
pub struct TypedIdent<TSpan> {
    pub ident: TSpan,
    pub type_: Type<TSpan>,
}

impl TypedIdent<Span> {
    pub fn resolve<'a>(&self, input: &'a [u8]) -> TypedIdent<&'a str> {
        TypedIdent {
            ident: self.ident.resolve(input),
            type_: self.type_.resolve(input),
        }
    }
}

/// An annotation comment that describes the query that follows it.
#[derive(Debug, Eq, PartialEq)]
pub struct Annotation<TSpan> {
    pub name: TSpan,
    pub parameters: Vec<TypedIdent<TSpan>>,
    pub result_type: Type<TSpan>,
}

impl Annotation<Span> {
    pub fn resolve<'a>(&self, input: &'a [u8]) -> Annotation<&'a str> {
        Annotation {
            name: self.name.resolve(input),
            parameters: self.parameters.iter().map(|p| p.resolve(input)).collect(),
            result_type: self.result_type.resolve(input),
        }
    }
}

/// A part of a query.
///
/// We break down queries in consecutive spans of three kinds:
///
/// * Verbatim content where we don't really care about its inner structure.
/// * Typed identifiers, the quoted part in a `select ... as "ident: type"`
///   select. These are kept separately, such that we can replace this with
///   just `ident` in the final query.
/// * Parameters. These include the leading `:`.
#[derive(Debug, Eq, PartialEq)]
pub enum Fragment<TSpan> {
    Verbatim(TSpan),
    TypedIdent(TSpan, TypedIdent<TSpan>),
    Param(TSpan),
}

impl Fragment<Span> {
    pub fn resolve<'a>(&self, input: &'a [u8]) -> Fragment<&'a str> {
        match self {
            Fragment::Verbatim(s) => Fragment::Verbatim(s.resolve(input)),
            Fragment::TypedIdent(s, ti) => {
                Fragment::TypedIdent(s.resolve(input), ti.resolve(input))
            }
            Fragment::Param(s) => Fragment::Param(s.resolve(input)),
        }
    }
}

/// An annotated query.
#[derive(Debug, Eq, PartialEq)]
pub struct Query<TSpan> {
    /// The lines of the comment that precedes the query, without `--`-prefix.
    pub docs: Vec<TSpan>,

    /// The annotation, which includes the name, parameters, and result type.
    pub annotation: Annotation<TSpan>,

    /// The spans that together reconstruct the query, including whitespace.
    ///
    /// These spans are mostly verbatim, although the preceding comments are
    /// omitted, and any `select ... as "name: type"` selections will have the
    /// `"name: type"` token replaced with just `name`.
    pub fragments: Vec<Fragment<TSpan>>,
}

impl Query<Span> {
    pub fn resolve<'a>(&self, input: &'a [u8]) -> Query<&'a str> {
        Query {
            docs: self.docs.iter().map(|d| d.resolve(input)).collect(),
            annotation: self.annotation.resolve(input),
            fragments: self.fragments.iter().map(|f| f.resolve(input)).collect(),
        }
    }
}

/// A section of a document.
///
/// Section consists either of a single annotated query, which we parse further
/// to extract the details, or some section that is *not* an annotated query,
/// which we preserve verbatim, to ensure that the parser is lossless.
#[derive(Debug, Eq, PartialEq)]
pub enum Section<TSpan> {
    Verbatim(TSpan),
    Query(Query<TSpan>),
}

impl Section<Span> {
    pub fn resolve<'a>(&self, input: &'a [u8]) -> Section<&'a str> {
        match self {
            Section::Verbatim(s) => Section::Verbatim(s.resolve(input)),
            Section::Query(q) => Section::Query(q.resolve(input)),
        }
    }
}
