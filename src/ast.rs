use crate::Span;

/// Supported types.
#[derive(Debug, Eq, PartialEq)]
pub enum Type<T> {
    Unit,
    Simple(T),
    Iterator(Box<Type<T>>),
    Option(Box<Type<T>>),
    Tuple(Vec<Type<T>>),
    Struct(T, Vec<TypedIdent<T>>),
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
pub struct TypedIdent<T> {
    pub ident: T,
    pub type_: Type<T>,
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
pub struct Annotation<T> {
    pub name: T,
    pub parameters: Vec<TypedIdent<T>>,
    pub result: Type<T>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Query<T> {
    pub name: T,
    pub docs: Vec<T>,
    pub parameters: Vec<TypedIdent<T>>,
    pub result: Type<T>,
    pub fragments: Vec<T>,
}
