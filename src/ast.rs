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
pub struct TypedIdent2<TSpan> {
    pub ident: TSpan,
    pub type_: SimpleType<TSpan>,
}

impl TypedIdent2<Span> {
    pub fn resolve<'a>(&self, input: &'a str) -> TypedIdent2<&'a str> {
        TypedIdent2 {
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
    Struct(TSpan, Vec<TypedIdent2<TSpan>>),
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

impl<TSpan> Type<TSpan> {
    /// Call the function on every type, including nested types.
    ///
    /// Performs a depth-first traversal. Calls the predicate on the outer type
    /// before calling it on the inner type.
    pub fn traverse<F, E>(&self, f: &mut F) -> Result<(), E>
    where
        F: FnMut(&Type<TSpan>) -> Result<(), E>,
    {
        f(self)?;

        match self {
            Type::Iterator(_, inner) => inner.traverse(f)?,
            Type::Option(_, inner) => inner.traverse(f)?,
            Type::Tuple(_, fields) => {
                for field_type in fields {
                    field_type.traverse(f)?;
                }
            }
            Type::Struct(_, fields) => {
                for field in fields {
                    field.type_.traverse(f)?;
                }
            }
            _ => {}
        }

        Ok(())
    }
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

/// An annotation comment that describes the query that follows it.
#[derive(Debug, Eq, PartialEq)]
pub struct Annotation<TSpan> {
    pub name: TSpan,
    pub parameters: Vec<TypedIdent<TSpan>>,
    pub result_type: ResultType<TSpan>,
}

impl<TSpan> Annotation<TSpan> {
    /// Call the function on every type, first parameters, then the result type.
    ///
    /// See also [`Type::traverse`].
    pub fn traverse<F, E>(&self, f: &mut F) -> Result<(), E>
    where
        F: FnMut(&Type<TSpan>) -> Result<(), E>,
    {
        for param in &self.parameters {
            f(&param.type_)?;
        }
        Ok(())
    }
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
