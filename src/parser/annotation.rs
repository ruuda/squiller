use crate::error::{PResult, ParseError};
use crate::lexer::annotation::Token;
use crate::Span;

type Annotation = crate::ast::Annotation<Span>;
type Type = crate::ast::Type<Span>;
type TypedIdent = crate::ast::TypedIdent<Span>;

/// Annotation parser.
///
/// The annotation parser can parse annotation comments and its constituents,
/// including typed identifiers that are also used in `select ... as "id: type"`
/// clauses.
pub struct Parser<'a> {
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
        let span = self
            .tokens
            .get(self.cursor)
            .map(|t| t.1)
            .unwrap_or(Span { start: 0, end: 0 });
        let err = ParseError { span, message };
        Err(err)
    }

    /// Return the token under the cursor, if there is one.
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.cursor).map(|t| t.0)
    }

    /// Return the token and its span under the cursor, if there is one.
    fn peek_with_span(&self) -> Option<(Token, &'a str)> {
        self.tokens
            .get(self.cursor)
            .map(|t| (t.0, t.1.resolve(self.input)))
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
        match self.peek_with_span() {
            Some((Token::LParen, _)) => Ok(Type::Tuple(self.parse_tuple()?)),
            Some((Token::Ident, span)) => match span {
                "Iterator" => {
                    self.consume();
                    let inner = self.parse_inner_generic_type()?;
                    Ok(Type::Iterator(Box::new(inner)))
                }
                "Option" => {
                    self.consume();
                    let inner = self.parse_inner_generic_type()?;
                    Ok(Type::Option(Box::new(inner)))
                }
                _ => {
                    let span = self.consume();
                    Ok(Type::Simple(span))
                }
            },
            Some(_) => self.error("Unexpected token, expected a type here."),
            None => self.error("Unexpected end of input, expected a type here."),
        }
    }

    /// Parse a type surrounded by angle brackets.
    fn parse_inner_generic_type(&mut self) -> PResult<Type> {
        self.expect_consume(Token::Lt, "Expected a '<' here, after a generic type.")?;
        let result = self.parse_type()?;
        self.expect_consume(Token::Gt, "Expected a '>' here to close a generic type.")?;
        Ok(result)
    }

    /// Parse a tuple, the cursor should be on the opening paren.
    fn parse_tuple(&mut self) -> PResult<Vec<Type>> {
        self.expect_consume(Token::LParen, "Expected a '(' here to start a tuple.")?;
        let mut elements = Vec::new();
        loop {
            if let Some(Token::RParen) = self.peek() {
                self.consume();
                return Ok(elements);
            }

            elements.push(self.parse_type()?);

            match self.peek() {
                // Don't consume, the next iterator of the loop will do that.
                Some(Token::RParen) => continue,

                // After a comma, we can either start again with a new element,
                // or the rparen can still follow, so the trailing comma is
                // optional.
                Some(Token::Comma) => {
                    self.consume();
                }

                Some(_unexpected) => {
                    return self.error("Unexpected token inside a tuple, expected ',' or ')' here.")
                }

                None => return self.error("Unexpected end of input, a tuple is not closed."),
            }
        }
    }

    /// Parse an argument list, the cursor should be on the opening paren.
    fn parse_parameter_list(&mut self) -> PResult<Vec<TypedIdent>> {
        self.expect_consume(
            Token::LParen,
            "Expected a '(' here to start the parameter list.",
        )?;
        let mut elements = Vec::new();
        loop {
            if let Some(Token::RParen) = self.peek() {
                self.consume();
                return Ok(elements);
            }

            elements.push(self.parse_typed_ident()?);

            match self.peek() {
                // Don't consume, the next iterator of the loop will do that.
                Some(Token::RParen) => continue,

                // After a comma, we can either start again with a new element,
                // or the rparen can still follow, so the trailing comma is
                // optional.
                Some(Token::Comma) => {
                    self.consume();
                }

                Some(_unexpected) => {
                    return self.error(
                        "Unexpected token inside a parameter list, expected ',' or ')' here.",
                    )
                }

                None => {
                    return self.error("Unexpected end of input, a parameter list is not closed.")
                }
            }
        }
    }

    pub fn parse_annotation(&mut self) -> PResult<Annotation> {
        // 1. The @query that marks the start of the annotation.
        match self.peek_with_span() {
            Some((Token::Annotation, "@query")) => self.consume(),
            Some((Token::Annotation, _)) => {
                return self.error("Invalid annotation, only '@query' is understood.")
            }
            Some(_) => return self.error("Invalid annotation, expected '@query' here."),
            None => return self.error("Unexpected end of input, expected '@query' here."),
        };

        // 2. The name of the query..
        let name = self.expect_consume(Token::Ident, "Expected an identifier here.")?;

        // 3. The list of query parameters, including parens.
        let parameters = self.parse_parameter_list()?;

        // 4. Optionally an arrow followed by the return type.
        let result_type = match self.peek() {
            None => Type::Unit,
            Some(Token::Arrow) => {
                self.consume();
                self.parse_type()?
            }
            Some(_unexpected) => {
                return self.error(
                    "Unexpected token, \
                    expected either the end of the annotation and start of the query, \
                    or '->' followed by a result type.",
                )
            }
        };

        let result = Annotation {
            name,
            parameters,
            result_type,
        };
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::Parser;
    use crate::ast::{Annotation, Type, TypedIdent};
    use crate::lexer::annotation::Lexer;
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
    fn test_parse_type_simple() {
        let input = b"i64";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Simple("i64");
            assert_eq!(result, expected);
        });

        let input = b"&str";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Simple("&str");
            assert_eq!(result, expected);
        });

        let input = b"User";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Simple("User");
            assert_eq!(result, expected);
        });
    }

    #[test]
    fn test_parse_type_generic() {
        let input = b"Option<i64>";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Option(Box::new(Type::Simple("i64")));
            assert_eq!(result, expected);
        });

        let input = b"Iterator<i64>";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Iterator(Box::new(Type::Simple("i64")));
            assert_eq!(result, expected);
        });

        // The generics we support, only support a single type argument.
        // A comma is a syntax error.
        let input = b"Iterator<i64, i64>";
        with_parser(input, |p| assert!(p.parse_type().is_err()));
    }

    #[test]
    fn test_parse_type_tuple() {
        let input = b"()";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Tuple(Vec::new());
            assert_eq!(result, expected);
        });

        let input = b"(f64)";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Tuple(vec![Type::Simple("f64")]);
            assert_eq!(result, expected);
        });

        // Test for trailing comma too.
        let input = b"(f64,)";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Tuple(vec![Type::Simple("f64")]);
            assert_eq!(result, expected);
        });

        let input = b"(f64, String)";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Tuple(vec![Type::Simple("f64"), Type::Simple("String")]);
            assert_eq!(result, expected);
        });

        // Also confirm that the following are parse errors.
        let invalid_inputs: &[&'static [u8]] = &[b"(,)", b"(f32, <)", b"(", b"(f32", b"(f32,"];
        for input in invalid_inputs {
            with_parser(input, |p| assert!(p.parse_type().is_err()));
        }
    }

    #[test]
    fn test_parse_typed_ident() {
        let input = b"id: i64";
        with_parser(input, |p| {
            let result = p.parse_typed_ident().unwrap().resolve(input);
            let expected = TypedIdent {
                ident: "id",
                type_: Type::Simple("i64"),
            };
            assert_eq!(result, expected);
        });
    }

    #[test]
    fn test_parse_annotation() {
        let input = b"@query drop_table_users()";
        with_parser(input, |p| {
            let result = p.parse_annotation().unwrap().resolve(input);
            let expected = Annotation {
                name: "drop_table_users",
                parameters: vec![],
                result_type: Type::Unit,
            };
            assert_eq!(result, expected);
        });

        // Test both with and without trailing comma.
        let inputs: &[&'static [u8]] = &[
            b"@query delete_user_by_id(id: i64)",
            b"@query delete_user_by_id(id: i64,)",
        ];
        for input in inputs {
            with_parser(input, |p| {
                let result = p.parse_annotation().unwrap().resolve(input);
                let expected = Annotation {
                    name: "delete_user_by_id",
                    parameters: vec![TypedIdent {
                        ident: "id",
                        type_: Type::Simple("i64"),
                    }],
                    result_type: Type::Unit,
                };
                assert_eq!(result, expected);
            });
        }

        // Test both with and without trailing comma. Also we play with the
        // whitespace a bit here.
        let inputs: &[&'static [u8]] = &[
            b"@query get_widgets_in_range (low : i64 , high : i64)",
            b"@query get_widgets_in_range(low:i64,high:i64,)",
        ];
        for input in inputs {
            with_parser(input, |p| {
                let result = p.parse_annotation().unwrap().resolve(input);
                let expected = Annotation {
                    name: "get_widgets_in_range",
                    parameters: vec![
                        TypedIdent {
                            ident: "low",
                            type_: Type::Simple("i64"),
                        },
                        TypedIdent {
                            ident: "high",
                            type_: Type::Simple("i64"),
                        },
                    ],
                    result_type: Type::Unit,
                };
                assert_eq!(result, expected);
            });
        }

        let input = b"@query get_next_id() -> i64";
        with_parser(input, |p| {
            let result = p.parse_annotation().unwrap().resolve(input);
            let expected = Annotation {
                name: "get_next_id",
                parameters: vec![],
                result_type: Type::Simple("i64"),
            };
            assert_eq!(result, expected);
        });
    }
}
