use crate::lex_annotation::Token;
use crate::Span;

type Annotation = crate::ast::Annotation<Span>;
type Type = crate::ast::Type<Span>;
type TypedIdent = crate::ast::TypedIdent<Span>;

#[derive(Debug)]
struct ParseError {
    span: Span,
    message: &'static str,
}

/// A parse result, either the parsed value, or a parse error.
type PResult<T> = std::result::Result<T, ParseError>;

struct Parser<'a> {
    input: &'a [u8],
    tokens: &'a [(Token, Span)],
    cursor: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [u8], tokens: &'a [(Token, Span)]) -> Parser<'a> {
        Parser {
            input,
            tokens,
            cursor: 0,
        }
    }

    /// Build a parse error at the current cursor location.
    fn error<T>(&self, message: &'static str) -> PResult<T> {
        let span = if self.cursor < self.tokens.len() {
            self.tokens[self.cursor].1
        } else {
            Span { start: 0, end: 0 }
        };
        let err = ParseError { span, message };
        Err(err)
    }

    /// Return the token under the cursor, if there is one.
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.cursor).map(|t| t.0)
    }

    /// Advance the cursor by one token, consuming the token under the cursor.
    ///
    /// Returns the span of the consumed token.
    fn consume(&mut self) -> Span {
        let result = self.tokens[self.cursor].1;

        self.cursor += 1;
        debug_assert!(
            self.cursor <= self.tokens.len(),
            "Cursor should not go more than beyond the last token.",
        );

        result
    }

    /// Consume one token. If it does not match, return the error message.
    fn expect_consume(&mut self, expected: Token, message: &'static str) -> PResult<Span> {
        match self.peek() {
            Some(token) if token == expected => Ok(self.consume()),
            _ => self.error(message),
        }
    }

    /// Parse a typed identifier, such as `id: i64`.
    pub fn parse_typed_ident(&mut self) -> PResult<TypedIdent> {
        let ident = self.expect_consume(Token::Ident, "Expected an identifier here.")?;
        self.expect_consume(
            Token::Colon,
            "Expected a ':' here before the start of the type signature.",
        )?;
        let type_ = self.parse_type()?;

        let result = TypedIdent { ident, type_ };
        Ok(result)
    }

    /// Parse a simple type, tuple, or generic type (iterator or option).
    ///
    /// The unit type cannot be parsed, it is marked by absense, and struct
    /// types have no explicit syntax in annotations either, they get
    /// contsructed at a higher level.
    pub fn parse_type(&mut self) -> PResult<Type> {
        // TODO: Actually parse.
        Ok(Type::Unit)
    }
}

#[cfg(test)]
mod test {
    use super::Parser;
    use crate::ast::{Annotation, Type, TypedIdent};
    use crate::lex_annotation::Lexer;
    use crate::Span;

    fn with_parser<F: FnOnce(&mut Parser)>(input: &[u8], f: F) {
        let all_span = Span {
            start: 0,
            end: input.len(),
        };
        let mut lexer = Lexer::new(input);
        lexer.run(all_span);
        let tokens = lexer.into_tokens();
        let mut parser = Parser::new(input, &tokens);
        f(&mut parser)
    }

    #[test]
    fn test_parse_typed_ident() {
        let input = b"id: i64";
        with_parser(input, |p| {
            let result = p.parse_typed_ident().unwrap();
            let expected = TypedIdent {
                ident: "id",
                type_: Type::Simple("i64"),
            };
            assert_eq!(result.resolve(input), expected);
        });
    }
}
