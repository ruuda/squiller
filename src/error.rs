use crate::Span;

#[derive(Debug)]
pub struct ParseError {
    pub span: Span,
    pub message: &'static str,
}

/// A parse result, either the parsed value, or a parse error.
pub type PResult<T> = std::result::Result<T, ParseError>;
