// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use crate::Span;

/// The primitive types that we support.
///
/// These types map to SQL types on the one hand, and types in the target
/// language on the other.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PrimitiveType {
    Str,
    I32,
    I64,
    Bytes,
}

/// Types of parameters and results.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type<TSpan> {
    /// The unit type, for queries that do not return anything.
    ///
    /// The unit type cannot be listed explicitly in annotations.
    Unit,

    /// A simple type that for our purposes cannot be broken down further.
    ///
    /// Used early in the compilation process, before resolving types. The
    /// parser outputs simple types and does not resolve them to structs or
    /// primitive types.
    Simple(TSpan),

    /// A primitive type.
    ///
    /// Primitive types are not produced by the parser, they are generated from
    /// simple types by the type resolution phase.
    Primitive(TSpan, PrimitiveType),

    /// An iterator, for queries that may return multiple rows.
    ///
    /// Field 0 contains the span of the full type.
    Iterator(TSpan, Box<Type<TSpan>>),

    /// An option, for queries that return zero or one rows, or nullable parameters.
    ///
    /// Field 0 contains the span of the full type.
    Option(TSpan, Box<Type<TSpan>>),

    /// A tuple of zero or more types.
    ///
    /// Field 0 contains the span of the full type.
    Tuple(TSpan, Vec<Type<TSpan>>),

    /// A named struct with typed fields.
    ///
    /// Structs fields cannot be listed explicitly in annotations; the
    /// annotation lists the name, and the fields are determined from the query.
    Struct(TSpan, Vec<TypedIdent<TSpan>>),
}

impl Type<Span> {
    pub fn resolve<'a>(&self, input: &'a str) -> Type<&'a str> {
        match self {
            Type::Unit => Type::Unit,
            Type::Simple(span) => Type::Simple(span.resolve(input)),
            Type::Primitive(span, t) => Type::Primitive(span.resolve(input), *t),
            Type::Iterator(s, t) => Type::Iterator(s.resolve(input), Box::new(t.resolve(input))),
            Type::Option(s, t) => Type::Option(s.resolve(input), Box::new(t.resolve(input))),
            Type::Tuple(s, ts) => Type::Tuple(
                s.resolve(input),
                ts.iter().map(|t| t.resolve(input)).collect(),
            ),
            Type::Struct(name, fields) => Type::Struct(
                name.resolve(input),
                fields.iter().map(|f| f.resolve(input)).collect(),
            ),
        }
    }

    /// For types that can occur literally in the source, return their span.
    pub fn span(&self) -> Span {
        match self {
            Type::Unit => panic!("Unit does not have a span."),
            Type::Simple(s) => *s,
            Type::Primitive(s, _) => *s,
            Type::Iterator(s, _) => *s,
            Type::Option(s, _) => *s,
            Type::Tuple(s, _) => *s,
            Type::Struct(s, _) => *s,
        }
    }

    /// For nested types, return the innermost type.
    ///
    /// For non-nested type, return itself. For structs and tuples, which can
    /// contain multiple inner types, also returns the type itself without
    /// descending further.
    pub fn inner(&self) -> &Type<Span> {
        match self {
            Type::Unit => self,
            Type::Simple(..) => self,
            Type::Primitive(..) => self,
            Type::Iterator(_, inner) => inner.inner(),
            Type::Option(_, inner) => inner.inner(),
            Type::Tuple(..) => self,
            Type::Struct(..) => self,
        }
    }

    /// Same as [`inner`], but mutable.
    pub fn inner_mut(&mut self) -> &mut Type<Span> {
        match self {
            Type::Unit => self,
            Type::Simple(..) => self,
            Type::Primitive(..) => self,
            Type::Iterator(_, inner) => inner.inner_mut(),
            Type::Option(_, inner) => inner.inner_mut(),
            Type::Tuple(..) => self,
            Type::Struct(..) => self,
        }
    }
}

impl<'a> Type<&'a str> {
    /// Test equivalence of the types, regardless of the spans or formatting.
    pub fn is_equal_to(&self, other: &Type<&'a str>) -> bool {
        match (self, other) {
            (Type::Unit, Type::Unit) => true,
            (Type::Simple(s1), Type::Simple(s2)) => s1 == s2,
            (Type::Primitive(_, t1), Type::Primitive(_, t2)) => t1 == t2,
            (Type::Iterator(_, t1), Type::Iterator(_, t2)) => t1.is_equal_to(t2),
            (Type::Option(_, t1), Type::Option(_, t2)) => t1.is_equal_to(t2),
            (Type::Tuple(_, fields1), Type::Tuple(_, fields2)) => {
                fields1.len() == fields2.len()
                    && fields1
                        .iter()
                        .zip(fields2)
                        .all(|(t1, t2)| t1.is_equal_to(t2))
            }
            (Type::Struct(name1, fields1), Type::Struct(name2, fields2)) => {
                name1 == name2
                    && fields1.len() == fields2.len()
                    && fields1
                        .iter()
                        .zip(fields2)
                        .all(|(f1, f2)| f1.ident == f2.ident && f1.type_.is_equal_to(&f2.type_))
            }
            _ => false,
        }
    }
}

/// An identifier and a type, e.g. `name: &str`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedIdent<TSpan> {
    pub ident: TSpan,
    pub type_: Type<TSpan>,
}

impl TypedIdent<Span> {
    pub fn resolve<'a>(&self, input: &'a str) -> TypedIdent<&'a str> {
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
    pub fn resolve<'a>(&self, input: &'a str) -> Annotation<&'a str> {
        Annotation {
            name: self.name.resolve(input),
            parameters: self.parameters.iter().map(|p| p.resolve(input)).collect(),
            result_type: self.result_type.resolve(input),
        }
    }
}

/// A part of a query.
///
/// We break down queries in consecutive spans of four kinds:
///
/// * Verbatim content where we don't really care about its inner structure.
/// * Typed identifiers, the quoted part in a `select ... as "ident: type"`
///   select. These are kept separately, such that we can replace this with
///   just `ident` in the final query.
/// * Untyped parameters. These include the leading `:`.
/// * Parameters followed by a type comment. These include the leading `:`.
#[derive(Debug, Eq, PartialEq)]
pub enum Fragment<TSpan> {
    Verbatim(TSpan),
    TypedIdent(TSpan, TypedIdent<TSpan>),
    Param(TSpan),
    TypedParam(TSpan, TypedIdent<TSpan>),
}

impl Fragment<Span> {
    pub fn resolve<'a>(&self, input: &'a str) -> Fragment<&'a str> {
        match self {
            Fragment::Verbatim(s) => Fragment::Verbatim(s.resolve(input)),
            Fragment::TypedIdent(s, ti) => {
                Fragment::TypedIdent(s.resolve(input), ti.resolve(input))
            }
            Fragment::Param(s) => Fragment::Param(s.resolve(input)),
            Fragment::TypedParam(s, ti) => {
                Fragment::TypedParam(s.resolve(input), ti.resolve(input))
            }
        }
    }

    /// The span that this fragment spans.
    pub fn span(&self) -> Span {
        match self {
            Fragment::Verbatim(s) => *s,
            Fragment::TypedIdent(s, _) => *s,
            Fragment::Param(s) => *s,
            Fragment::TypedParam(s, _) => *s,
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
    pub fn resolve<'a>(&self, input: &'a str) -> Query<&'a str> {
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
    pub fn resolve<'a>(&self, input: &'a str) -> Section<&'a str> {
        match self {
            Section::Verbatim(s) => Section::Verbatim(s.resolve(input)),
            Section::Query(q) => Section::Query(q.resolve(input)),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Document<TSpan> {
    pub sections: Vec<Section<TSpan>>,
}

impl Document<Span> {
    pub fn resolve<'a>(&self, input: &'a str) -> Document<&'a str> {
        Document {
            sections: self.sections.iter().map(|s| s.resolve(input)).collect(),
        }
    }
}
