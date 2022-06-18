use crate::Span;

/// Supported types.
#[derive(Debug, Eq, PartialEq)]
pub enum Type<TSpan> {
    Unit,
    Simple(TSpan),
    Iterator(Box<Type<TSpan>>),
    Option(Box<Type<TSpan>>),
    Tuple(Vec<Type<TSpan>>),
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

#[derive(Debug, Eq, PartialEq)]
pub struct Annotation<TSpan> {
    pub name: TSpan,
    pub parameters: Vec<TypedIdent<TSpan>>,
    pub result: Type<TSpan>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Query<TSpan> {
    pub name: TSpan,
    pub docs: Vec<TSpan>,
    pub parameters: Vec<TypedIdent<TSpan>>,
    pub result: Type<TSpan>,
    pub fragments: Vec<TSpan>,
}
