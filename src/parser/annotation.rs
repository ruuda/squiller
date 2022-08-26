use crate::ast::PrimitiveType;
use crate::error::{PResult, ParseError};
use crate::lexer::annotation::Token;
use crate::Span;

type Annotation = crate::ast::Annotation<Span>;
type ArgType = crate::ast::ArgType<Span>;
type ResultType = crate::ast::ResultType<Span>;
type TypedIdent = crate::ast::TypedIdent<Span>;
type SimpleType = crate::ast::SimpleType<Span>;
type ComplexType = crate::ast::ComplexType<Span>;

/// Annotation parser.
///
/// The annotation parser can parse annotation comments and its constituents,
/// including typed identifiers that are also used in `select ... as "id: type"`
/// clauses.
pub struct Parser<'a> {
    input: &'a str,
    tokens: &'a [(Token, Span)],
    cursor: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str, tokens: &'a [(Token, Span)]) -> Parser<'a> {
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
            .unwrap_or_else(|| {
                self.tokens
                    .last()
                    .map(|t| Span {
                        start: t.1.end,
                        end: t.1.end,
                    })
                    .expect("Should not try to parse annotation without tokens.")
            });

        let err = ParseError {
            span,
            message,
            note: None,
        };

        Err(err)
    }

    /// Build a parse error at the current cursor location, and a note elsewhere.
    fn error_with_note<T>(
        &self,
        message: &'static str,
        note_span: Span,
        note: &'static str,
    ) -> PResult<T> {
        self.error(message).map_err(|err| ParseError {
            note: Some((note, note_span)),
            ..err
        })
    }

    /// Return the token under the cursor, if there is one.
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.cursor).map(|t| t.0)
    }

    /// Return the token and its span under the cursor, if there is one.
    fn peek_with_span(&self) -> Option<(Token, Span)> {
        self.tokens.get(self.cursor).map(|t| (t.0, t.1))
    }

    /// Return the token before the cursor, assuming it exists.
    fn previous_span(&self) -> Span {
        self.tokens[self.cursor - 1].1
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
            "Expected a ':' here before the start of the type.",
        )?;

        let type_ = self.parse_simple_type()?;

        let result = TypedIdent { ident, type_ };
        Ok(result)
    }

    /// Parse a primitive type.
    pub fn parse_primitive_type(&mut self) -> PResult<(Span, PrimitiveType)> {
        // We list some alternative spellings of types that people might
        // reasonably expect to work, so we can point them in the right
        // direction in the error message.
        let alt_str = ["str", "string"];
        let alt_int = [
            "int",
            "i8",
            "i16",
            "i32",
            "i64",
            "uint",
            "u8",
            "u16",
            "u32",
            "u64",
            "int4",
            "int8",
            "integer",
            "bigint",
            "biginteger",
        ];
        match self.peek_with_span() {
            Some((Token::Ident, span)) => {
                let result = match span.resolve(self.input) {
                    "str" => PrimitiveType::Str,
                    "i32" => PrimitiveType::I32,
                    "i64" => PrimitiveType::I64,
                    "bytes" => PrimitiveType::Bytes,
                    unknown if alt_str.contains(&&unknown.to_ascii_lowercase()[..]) => {
                        return self.error("Unknown type, did you mean 'str'?");
                    }
                    unknown if alt_int.contains(&&unknown.to_ascii_lowercase()[..]) => {
                        return self.error("Unknown type, did you mean 'i32' or 'i64'?");
                    }
                    _ => {
                        return self.error("Unknown type, expected a primitive type here.");
                    }
                };
                self.consume();
                Ok((span, result))
            }
            Some(_not_ident) => return self.error("Expected a primitive type here."),
            None => return self.error("Unexpected end of input, expected a primitive type here."),
        }
    }

    /// Parse a simple type (primitive or option).
    pub fn parse_simple_type(&mut self) -> PResult<SimpleType> {
        // We list some alternative spellings of types that people might
        // reasonably expect to work, so we can point them in the right
        // direction in the error message.
        let alt_opt = ["option", "optional", "maybe", "null", "nullable"];
        let span = match self.peek_with_span() {
            Some((Token::Ident, span)) => span,
            Some(_not_ident) => return self.error("Expected a simple type here."),
            None => return self.error("Unexpected end of input, expected a simple type here."),
        };
        match span.resolve(self.input) {
            "option" => {
                self.consume();
                self.expect_consume(Token::Lt, "Expected a '<' after 'option'.")?;
                let (inner, primitive) = self.parse_primitive_type()?;
                self.expect_consume(Token::Gt, "Expected a '>' here to close the 'option<'.")?;
                let result = SimpleType::Option {
                    outer: Span {
                        start: span.start,
                        end: self.previous_span().end,
                    },
                    inner: inner,
                    type_: primitive,
                };
                Ok(result)
            }
            name => {
                let (inner, primitive) = match self.parse_primitive_type() {
                    Ok(t) => t,
                    // If we failed to parse a primitive type, but it looks like
                    // the user might have meant to write an option, use a tailored
                    // error message to point the user in the right direction.
                    Err(..) if alt_opt.contains(&&name.to_ascii_lowercase()[..]) => {
                        return self.error("Unknown type, did you mean 'Option'?");
                    }
                    Err(err) => return Err(err),
                };
                let result = SimpleType::Primitive {
                    inner: inner,
                    type_: primitive,
                };
                Ok(result)
            }
        }
    }

    /// Parse a complex type.
    ///
    /// Complex types can be either a regular simple type, or a struct or tuple.
    pub fn parse_complex_type(&mut self) -> PResult<ComplexType> {
        match self.peek_with_span() {
            Some((Token::LParen, span)) => {
                let inner = self.parse_tuple()?;
                let final_span = self.previous_span();
                let full_span = Span {
                    start: span.start,
                    end: final_span.end,
                };
                Ok(ComplexType::Tuple(full_span, inner))
            }
            Some((Token::Ident, span)) => {
                // If it's an identifier, then it can be a user-defined type (a
                // struct), or a builtin type. Struct names start with an uppercase
                // letter, and no builtin types do, so that's how we distinguish.
                let is_struct = span
                    .resolve(self.input)
                    .chars()
                    .next()
                    .expect("Parser does not produce empty spans.")
                    .is_ascii_uppercase();
                if is_struct {
                    self.consume();
                    Ok(ComplexType::Struct(span, Vec::new()))
                } else {
                    let simple = self.parse_simple_type()?;
                    Ok(ComplexType::Simple(simple))
                }
            }
            Some(_) => self.error("Expected a type here."),
            None => self.error("Unexpected end of input, expected a type here."),
        }
    }

    /// Parse a tuple, the cursor should be on the opening paren.
    fn parse_tuple(&mut self) -> PResult<Vec<SimpleType>> {
        self.expect_consume(Token::LParen, "Expected a '(' here to start a tuple.")?;
        let mut elements = Vec::new();
        loop {
            if let Some(Token::RParen) = self.peek() {
                self.consume();
                return Ok(elements);
            }

            elements.push(self.parse_simple_type()?);

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
    fn parse_arguments(&mut self) -> PResult<ArgType> {
        self.expect_consume(
            Token::LParen,
            "Expected a '(' here to start the query arguments.",
        )?;

        let start_span = self.tokens[self.cursor - 1].1;
        let mut arguments = Vec::new();

        match self.peek() {
            Some(Token::RParen) => {
                self.consume();
                return Ok(ArgType::Args(arguments));
            }
            Some(Token::Ident) => {
                let ident = self.consume();
                self.expect_consume(
                    Token::Colon,
                    "Expected a ':' here before the start of the type.",
                )?;
                let type_name = self.expect_consume(Token::Ident, "Expected a type here.")?;

                // TODO: Deduplicate this between parse_complex_type.
                let is_struct = type_name
                    .resolve(self.input)
                    .chars()
                    .next()
                    .expect("Parser does not produce empty spans.")
                    .is_ascii_uppercase();

                if is_struct {
                    // TODO: This makes the trailing comma disallowed, which is
                    // a bit inconsistent if we do allow it after a single non-
                    // struct arg.
                    self.expect_consume(
                        Token::RParen,
                        "Expected a ')' here, queries that take a struct \
                        can only take a single argument.",
                    )?;
                    let result = ArgType::Struct {
                        var_name: ident,
                        type_name: type_name,
                        fields: Vec::new(),
                    };
                    return Ok(result);
                } else {
                    // Unconsume the identifier that we already consumed, we
                    // will parse a simple type instead.
                    self.cursor -= 1;
                    let arg = TypedIdent {
                        ident: ident,
                        type_: self.parse_simple_type()?,
                    };
                    arguments.push(arg);
                }
            }
            Some(_unexpected) => {
                return self.error(
                    "Unexpected token in query arguments, expected ')' or an identifier here.",
                )
            }
            None => {
                return self.error_with_note(
                    "Unexpected end of input, expected ')' to close the query arguments.",
                    start_span,
                    "Unmatched '(' opened here.",
                )
            }
        }

        loop {
            if let Some(Token::RParen) = self.peek() {
                self.consume();
                return Ok(ArgType::Args(arguments));
            }

            // For now, the parser is simpler if we don't allow a trailing comma.
            // Only allow it here, before the start of the next argument.
            self.expect_consume(Token::Comma, "Expected a ',' here.")?;

            arguments.push(self.parse_typed_ident()?);
        }
    }

    pub fn parse_annotation(&mut self) -> PResult<Annotation> {
        // 1. The @query that marks the start of the annotation.
        match self.peek_with_span() {
            Some((Token::Annotation, ann)) if ann.resolve(self.input) == "@query" => self.consume(),
            Some((Token::Annotation, _)) => {
                return self.error("Invalid annotation, only '@query' is understood.")
            }
            Some(_) => return self.error("Invalid annotation, expected '@query' here."),
            None => return self.error("Unexpected end of input, expected '@query' here."),
        };

        // 2. The name of the query..
        let name = self.expect_consume(Token::Ident, "Expected an identifier here.")?;

        // 3. The query arguments, including parens.
        let arguments = self.parse_arguments()?;

        // 4. Optionally an arrow followed by the result type.
        let result_type = match self.peek() {
            None => ResultType::Unit,
            Some(Token::ArrowOpt) => {
                self.consume();
                let type_ = self.parse_complex_type()?;
                ResultType::Option(type_)
            }
            Some(Token::ArrowOne) => {
                self.consume();
                let type_ = self.parse_complex_type()?;
                ResultType::Single(type_)
            }
            Some(Token::ArrowStar) => {
                self.consume();
                let type_ = self.parse_complex_type()?;
                ResultType::Iterator(type_)
            }
            Some(Token::Arrow) => {
                return self.error(
                    "A return type arrow must include the number of rows \
                    that the query will return: '->?' for zero or one, \
                    '->1' for exactly one, and '->*' for zero or more.",
                )
            }
            Some(_unexpected) => {
                return self.error(
                    "Expected either the end of the annotation and start of the query, \
                    or '->' followed by a cardinality and result type.",
                )
            }
        };

        let result = Annotation {
            name,
            arguments,
            result_type,
        };
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::Parser;
    use crate::ast::{
        Annotation, ComplexType, PrimitiveType, ResultType, SimpleType, Type, TypedIdent,
    };
    use crate::lexer::annotation::Lexer;
    use crate::Span;

    fn with_parser<F: FnOnce(&mut Parser)>(input: &str, f: F) {
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
        let input = "i64";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Simple("i64");
            assert_eq!(result, expected);
        });

        let input = "&str";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Simple("&str");
            assert_eq!(result, expected);
        });

        let input = "User";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Simple("User");
            assert_eq!(result, expected);
        });
    }

    #[test]
    fn test_parse_type_generic() {
        let input = "Option<i64>";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Option("Option<i64>", Box::new(Type::Simple("i64")));
            assert_eq!(result, expected);
        });

        let input = "Iterator<i64>";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Iterator("Iterator<i64>", Box::new(Type::Simple("i64")));
            assert_eq!(result, expected);
        });

        // The generics we support, only support a single type argument.
        // A comma is a syntax error.
        let input = "Iterator<i64, i64>";
        with_parser(input, |p| assert!(p.parse_type().is_err()));
    }

    #[test]
    fn test_parse_type_tuple() {
        let input = "()";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Tuple("()", Vec::new());
            assert_eq!(result, expected);
        });

        let input = "(f64)";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Tuple("(f64)", vec![Type::Simple("f64")]);
            assert_eq!(result, expected);
        });

        // Test for trailing comma too.
        let input = "(f64,)";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Tuple("(f64,)", vec![Type::Simple("f64")]);
            assert_eq!(result, expected);
        });

        let input = "(f64, String)";
        with_parser(input, |p| {
            let result = p.parse_type().unwrap().resolve(input);
            let expected = Type::Tuple(
                "(f64, String)",
                vec![Type::Simple("f64"), Type::Simple("String")],
            );
            assert_eq!(result, expected);
        });

        // Also confirm that the following are parse errors.
        let invalid_inputs: &[&'static str] = &["(,)", "(f32, <)", "(", "(f32", "(f32,"];
        for input in invalid_inputs {
            with_parser(input, |p| assert!(p.parse_type().is_err()));
        }
    }

    #[test]
    fn test_parse_typed_ident() {
        let input = "id: i64";
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
        let input = "@query drop_table_users()";
        with_parser(input, |p| {
            let result = p.parse_annotation().unwrap().resolve(input);
            let expected = Annotation {
                name: "drop_table_users",
                parameters: vec![],
                result_type: ResultType::Unit,
            };
            assert_eq!(result, expected);
        });

        // Test both with and without trailing comma.
        let inputs: &[&'static str] = &[
            "@query delete_user_by_id(id: i64)",
            "@query delete_user_by_id(id: i64,)",
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
                    result_type: ResultType::Unit,
                };
                assert_eq!(result, expected);
            });
        }

        // Test both with and without trailing comma. Also we play with the
        // whitespace a bit here.
        let inputs: &[&'static str] = &[
            "@query get_widgets_in_range (low : i64 , high : i64)",
            "@query get_widgets_in_range(low:i64,high:i64,)",
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
                    result_type: ResultType::Unit,
                };
                assert_eq!(result, expected);
            });
        }

        let input = "@query get_next_id() ->1 i64";
        with_parser(input, |p| {
            let result = p.parse_annotation().unwrap().resolve(input);
            let expected = Annotation {
                name: "get_next_id",
                parameters: vec![],
                result_type: ResultType::Single(ComplexType::Simple(SimpleType::Primitive {
                    inner: "i64",
                    type_: PrimitiveType::I64,
                })),
            };
            assert_eq!(result, expected);
        });
    }

    #[test]
    fn test_error_on_unexpected_end_is_past_end() {
        let input = "id";
        with_parser(input, |p| {
            let err = p.parse_typed_ident().err().unwrap();
            assert_eq!(err.span, Span { start: 2, end: 2 });
        });

        let input = "id:";
        with_parser(input, |p| {
            let err = p.parse_typed_ident().err().unwrap();
            assert_eq!(err.span, Span { start: 3, end: 3 });
        });
    }
}
