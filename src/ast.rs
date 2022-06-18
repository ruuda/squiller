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

/// An annotated query.
#[derive(Debug, Eq, PartialEq)]
pub struct Query<TSpan> {
    /// The name of the query, from the annotation.
    pub name: TSpan,

    /// The lines of the comment that precedes the query, without `--`-prefix.
    pub docs: Vec<TSpan>,

    /// The parameters that occur in the query, and their types.
    pub parameters: Vec<TypedIdent<TSpan>>,

    /// The type of data that the query produces.
    pub result_type: Type<TSpan>,

    /// The spans that together reconstruct the query, including whitespace.
    ///
    /// These spans are mostly verbatim, although the preceding comments are
    /// omitted, and any `select ... as "name: type"` selections will have the
    /// `"name: type"` token replaced with just `name`.
    pub fragments: Vec<TSpan>,
}
