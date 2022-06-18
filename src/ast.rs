use crate::Span;

/// Supported types.
pub enum Type {
    Unit,
    Simple(Span),
    Iterator(Box<Type>),
    Option(Box<Type>),
    Tuple(Vec<Type>),
    Struct(Span, Vec<TypedIdent>),
}

/// An identifier and a type, e.g. `name: &str`.
pub struct TypedIdent {
    pub ident: Span,
    pub type_: Type,
}

pub struct Annotation {
    pub name: Span,
    pub parameters: Vec<TypedIdent>,
    pub result: Type,
}

pub struct Query {
    pub name: Span,
    pub docs: Vec<Span>,
    pub parameters: Vec<TypedIdent>,
    pub result: Type,
    pub fragments: Vec<Span>,
}
