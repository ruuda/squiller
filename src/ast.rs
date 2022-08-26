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

/// A simple type is a type that is not composite. It's primitive or a nullable primitive.
///
/// Simple types can be used everywhere, as opposed to complex types, which can
/// only be used in limited places.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimpleType<TSpan> {
    Primitive {
        inner: TSpan,
        type_: PrimitiveType,
    },
    Option {
        outer: TSpan,
        inner: TSpan,
        type_: PrimitiveType,
    },
}

impl<TSpan> SimpleType<TSpan> {
    pub fn span(&self) -> TSpan
    where
        TSpan: Copy,
    {
        match &self {
            SimpleType::Primitive { inner, .. } => *inner,
            SimpleType::Option { outer, .. } => *outer,
        }
    }

    pub fn inner_type(&self) -> PrimitiveType {
        match self {
            SimpleType::Primitive { type_, .. } => *type_,
            SimpleType::Option { type_, .. } => *type_,
        }
    }

    /// Test equivalence of the types, regardless of the spans or formatting.
    pub fn is_equal_to(&self, other: &SimpleType<TSpan>) -> bool {
        match (self, other) {
            (
                SimpleType::Primitive { type_: lhs, .. },
                SimpleType::Primitive { type_: rhs, .. },
            ) => lhs == rhs,
            (SimpleType::Option { type_: lhs, .. }, SimpleType::Option { type_: rhs, .. }) => {
                lhs == rhs
            }
            _ => false,
        }
    }
}

impl SimpleType<Span> {
    pub fn resolve<'a>(&self, input: &'a str) -> SimpleType<&'a str> {
        match self {
            SimpleType::Primitive { inner, type_ } => SimpleType::Primitive {
                inner: inner.resolve(input),
                type_: *type_,
            },
            SimpleType::Option {
                inner,
                outer,
                type_,
            } => SimpleType::Option {
                outer: outer.resolve(input),
                inner: inner.resolve(input),
                type_: *type_,
            },
        }
    }
}

/// An identifier and a type, e.g. `name: &str`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedIdent<TSpan> {
    pub ident: TSpan,
    pub type_: SimpleType<TSpan>,
}

impl TypedIdent<Span> {
    pub fn resolve<'a>(&self, input: &'a str) -> TypedIdent<&'a str> {
        TypedIdent {
            ident: self.ident.resolve(input),
            type_: self.type_.resolve(input),
        }
    }
}

/// A complex type is either a simple type, or an aggregate of multiple simple types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComplexType<TSpan> {
    Simple(SimpleType<TSpan>),

    /// A tuple of zero or more types.
    ///
    /// Field 0 contains the span of the full tuple.
    Tuple(TSpan, Vec<SimpleType<TSpan>>),

    /// A struct with zero or more fields.
    ///
    /// Field 0 contains the span of the name of the struct.
    Struct(TSpan, Vec<TypedIdent<TSpan>>),
}

impl ComplexType<Span> {
    pub fn resolve<'a>(&self, input: &'a str) -> ComplexType<&'a str> {
        match self {
            ComplexType::Simple(inner) => ComplexType::Simple(inner.resolve(input)),
            ComplexType::Tuple(outer, fields) => {
                let fields = fields.iter().map(|t| t.resolve(input)).collect();
                ComplexType::Tuple(outer.resolve(input), fields)
            }
            ComplexType::Struct(name, fields) => {
                let fields = fields.iter().map(|t| t.resolve(input)).collect();
                ComplexType::Struct(name.resolve(input), fields)
            }
        }
    }
}

/// The cardinality of the query, and the result type.
#[derive(Debug, Eq, PartialEq)]
pub enum ResultType<TSpan> {
    /// The query returns zero rows, the function returns unit.
    Unit,
    /// The query returns zero or one row, the function returns `Option<T>`.
    Option(ComplexType<TSpan>),
    /// The query returns exactly one row, the function returns `T`.
    Single(ComplexType<TSpan>),
    /// The query returns zero or more rows, the function returns `Iterator<Item=T>`.
    Iterator(ComplexType<TSpan>),
}

impl<TSpan> ResultType<TSpan> {
    pub fn get(&self) -> Option<&ComplexType<TSpan>> {
        match self {
            ResultType::Unit => None,
            ResultType::Option(t) => Some(t),
            ResultType::Single(t) => Some(t),
            ResultType::Iterator(t) => Some(t),
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut ComplexType<TSpan>> {
        match self {
            ResultType::Unit => None,
            ResultType::Option(t) => Some(t),
            ResultType::Single(t) => Some(t),
            ResultType::Iterator(t) => Some(t),
        }
    }
}

impl ResultType<Span> {
    pub fn resolve<'a>(&self, input: &'a str) -> ResultType<&'a str> {
        match self {
            ResultType::Unit => ResultType::Unit,
            ResultType::Option(t) => ResultType::Option(t.resolve(input)),
            ResultType::Single(t) => ResultType::Single(t.resolve(input)),
            ResultType::Iterator(t) => ResultType::Iterator(t.resolve(input)),
        }
    }
}

/// Inputs to the query, either named as separate arguments, or a single struct.
#[derive(Debug, Eq, PartialEq)]
pub enum ArgType<TSpan> {
    /// One or more named arguments, e.g. `(name: str, age: i32)`.
    ///
    /// Each one corresponds to a query parameter, `:name` and `:age` in the
    /// example.
    Args(Vec<TypedIdent<TSpan>>),

    /// A named struct and its fields, e.g. `(user: User)`.
    ///
    /// The fields are not populated by the parser, as there is no syntax to
    /// specify them in-line, the fields are inferred from the query body.
    ///
    /// Each field on the struct corresponds to a query parameter.
    Struct {
        var_name: TSpan,
        type_name: TSpan,
        fields: Vec<TypedIdent<TSpan>>,
    },
}

impl ArgType<Span> {
    pub fn resolve<'a>(&self, input: &'a str) -> ArgType<&'a str> {
        match self {
            ArgType::Args(args) => ArgType::Args(args.iter().map(|ti| ti.resolve(input)).collect()),
            ArgType::Struct {
                var_name,
                type_name,
                fields,
            } => ArgType::Struct {
                var_name: var_name.resolve(input),
                type_name: type_name.resolve(input),
                fields: fields.iter().map(|ti| ti.resolve(input)).collect(),
            },
        }
    }
}

/// An annotation comment that describes the query that follows it.
#[derive(Debug, Eq, PartialEq)]
pub struct Annotation<TSpan> {
    pub name: TSpan,
    pub arguments: ArgType<TSpan>,
    pub result_type: ResultType<TSpan>,
}

impl Annotation<Span> {
    pub fn resolve<'a>(&self, input: &'a str) -> Annotation<&'a str> {
        Annotation {
            name: self.name.resolve(input),
            arguments: self.arguments.resolve(input),
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

impl<TSpan> Query<TSpan> {
    /// Extract all parameters from the query body (both typed and untyped).
    pub fn iter_parameters<'a>(&self) -> impl Iterator<Item = TSpan> + '_
    where
        TSpan: Copy,
    {
        self.fragments.iter().filter_map(|fragment| match fragment {
            Fragment::Verbatim(..) => None,
            Fragment::TypedIdent(..) => None,
            Fragment::Param(span) => Some(*span),
            Fragment::TypedParam(_full_span, ti) => Some(ti.ident),
        })
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

impl<TSpan> Document<TSpan> {
    /// Extract all queries from the document.
    pub fn iter_queries<'a>(&self) -> impl Iterator<Item = &Query<TSpan>> {
        self.sections.iter().filter_map(|section| match section {
            Section::Verbatim(..) => None,
            Section::Query(q) => Some(q),
        })
    }
}
