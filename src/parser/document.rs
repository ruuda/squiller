use crate::lexer::sql;
use crate::Span;

type Annotation = crate::ast::Annotation<Span>;
type Query = crate::ast::Query<Span>;
type TypedIdent = crate::ast::TypedIdent<Span>;

/// Document parser.
///
/// Parses a tokenized SQL document into a list of queries with their metadata.
pub struct Parser<'a> {
    input: &'a [u8],
    cursor: usize,
    tokens: Vec<(sql::Token, Span)>,

    /// All comments since the last blank line.
    comments: Vec<Span>,

    /// Tokens of the SQL query since the start of the most recent annotation.
    fragments: Vec<Span>,

    /// All parameters since the start of the most recent annotation.
    parameters: Vec<Span>,

    /// All named selected items since the start of the most recent annotation.
    columns: Vec<TypedIdent>,

    /// The most recent unresolved annotation.
    annotation: Option<Annotation>,
}
