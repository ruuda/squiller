// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use crate::ast::{PrimitiveType, StatementType};
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
        let alt_float = ["float", "float4", "float8", "double"];
        match self.peek_with_span() {
            Some((Token::Ident, span)) => {
                let result = match span.resolve(self.input) {
                    "str" => PrimitiveType::Str,
                    "i32" => PrimitiveType::I32,
                    "i64" => PrimitiveType::I64,
                    "f32" => PrimitiveType::F32,
                    "f64" => PrimitiveType::F64,
                    "bytes" => PrimitiveType::Bytes,
                    unknown if alt_str.contains(&&unknown.to_ascii_lowercase()[..]) => {
                        return self.error("Unknown type, did you mean 'str'?");
                    }
                    unknown if alt_int.contains(&&unknown.to_ascii_lowercase()[..]) => {
                        return self.error("Unknown type, did you mean 'i32' or 'i64'?");
                    }
                    unknown if alt_float.contains(&&unknown.to_ascii_lowercase()[..]) => {
                        return self.error("Unknown type, did you mean 'f32' or 'f64'?");
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
        let (inner, primitive) = self.parse_primitive_type()?;

        // If a primitive type is followed by a question mark, that
        // makes it an option type, if it's followed by anything else,
        // it remains primitive.
        let result = match self.peek() {
            Some(Token::Question) => SimpleType::Option {
                outer: Span {
                    start: inner.start,
                    end: self.consume().end,
                },
                inner: inner,
                type_: primitive,
            },
            _ => SimpleType::Primitive {
                inner: inner,
                type_: primitive,
            },
        };
        Ok(result)
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

        // We first do a pass to collect all arguments as complex types, and
        // then later we validate.
        let mut arguments: Vec<(Span, ComplexType)> = Vec::new();
        loop {
            if let Some(Token::RParen) = self.peek() {
                self.consume();
                break;
            }

            let ident = self.expect_consume(Token::Ident, "Expected an identifier here.")?;
            self.expect_consume(
                Token::Colon,
                "Expected a ':' here before the start of the type.",
            )?;
            let type_ = self.parse_complex_type()?;

            arguments.push((ident, type_));

            match self.peek() {
                Some(Token::RParen) => {
                    self.consume();
                    break;
                }
                Some(Token::Comma) => {
                    self.consume();
                    continue;
                }
                Some(_unexpected) => {
                    return self
                        .error("Unexpected token in query arguments, expected ')' or ',' here.")
                }
                None => {
                    return self.error_with_note(
                        "Unexpected end of input, expected ')' to close the query arguments.",
                        start_span,
                        "Unmatched '(' opened here.",
                    )
                }
            }
        }

        let err_tuple = |span: Span| {
            Err(ParseError {
                span,
                message: "Tuples can only be used in result types, not in arguments.",
                note: None,
            })
        };

        match arguments.len() {
            0 => return Ok(ArgType::Args(Vec::new())),
            1 => match arguments.pop().unwrap() {
                (var_name, ComplexType::Struct(type_name, fields)) => {
                    let result = ArgType::Struct {
                        var_name,
                        type_name,
                        fields,
                    };
                    return Ok(result);
                }
                (_, ComplexType::Tuple(span, _fields)) => return err_tuple(span),
                (var_name, ComplexType::Simple(t)) => {
                    let ti = TypedIdent {
                        ident: var_name,
                        type_: t,
                    };
                    return Ok(ArgType::Args(vec![ti]));
                }
            },
            _ => {}
        }

        let mut simple_args = Vec::with_capacity(arguments.len());
        for (var_name, arg) in arguments.drain(..) {
            match arg {
                ComplexType::Struct(type_name, _fields) => {
                    return Err(ParseError {
                        span: type_name,
                        message: "Struct arguments can only be used in queries that take a single argument.",
                        note: None,
                    });
                }
                ComplexType::Tuple(span, _fields) => return err_tuple(span),
                ComplexType::Simple(t) => {
                    let ti = TypedIdent {
                        ident: var_name,
                        type_: t,
                    };
                    simple_args.push(ti);
                }
            }
        }

        Ok(ArgType::Args(simple_args))
    }

    pub fn parse_annotation(&mut self) -> PResult<(Annotation, StatementType)> {
        // 1. The @query or @begin that marks the start of the annotation.
        let stmt_type = match self.peek_with_span() {
            Some((Token::Marker, mark)) => match mark.resolve(self.input) {
                "@query" => StatementType::Single,
                "@begin" => StatementType::Multi,
                _ => return self.error("Invalid annotation, expected '@query' or '@begin' here."),
            },
            Some(_) => {
                return self.error("Invalid annotation, expected '@query' or '@begin' here.")
            }
            None => {
                return self.error("Unexpected end of input, expected '@query' or '@begin' here.")
            }
        };
        self.consume();

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
        Ok((result, stmt_type))
    }
}

#[cfg(test)]
mod test {
    use super::Parser;
    use crate::ast::{
        Annotation, ArgType, ComplexType, PrimitiveType, ResultType, SimpleType, StatementType,
        TypedIdent,
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
    fn test_parse_simple_type_primitive() {
        let input = "i64";
        with_parser(input, |p| {
            let result = p.parse_simple_type().unwrap().resolve(input);
            let expected = SimpleType::Primitive {
                inner: "i64",
                type_: PrimitiveType::I64,
            };
            assert_eq!(result, expected);
        });

        let input = "str";
        with_parser(input, |p| {
            let result = p.parse_simple_type().unwrap().resolve(input);
            let expected = SimpleType::Primitive {
                inner: "str",
                type_: PrimitiveType::Str,
            };
            assert_eq!(result, expected);
        });

        let input = "bytes";
        with_parser(input, |p| {
            let result = p.parse_simple_type().unwrap().resolve(input);
            let expected = SimpleType::Primitive {
                inner: "bytes",
                type_: PrimitiveType::Bytes,
            };
            assert_eq!(result, expected);
        });
    }

    #[test]
    fn test_parse_simple_type_option() {
        let input = "i64?";
        with_parser(input, |p| {
            let result = p.parse_simple_type().unwrap().resolve(input);
            let expected = SimpleType::Option {
                inner: "i64",
                outer: "i64?",
                type_: PrimitiveType::I64,
            };
            assert_eq!(result, expected);
        });

        let input = "i64 ?";
        with_parser(input, |p| {
            let result = p.parse_simple_type().unwrap().resolve(input);
            let expected = SimpleType::Option {
                inner: "i64",
                outer: "i64 ?",
                type_: PrimitiveType::I64,
            };
            assert_eq!(result, expected);
        });

        // Try a few syntax errors as well.
        with_parser("(i64)?", |p| assert!(p.parse_simple_type().is_err()));
        with_parser("(i64?)", |p| assert!(p.parse_simple_type().is_err()));
    }

    #[test]
    fn test_parse_complex_type_tuple() {
        let input = "()";
        with_parser(input, |p| {
            let result = p.parse_complex_type().unwrap().resolve(input);
            let expected = ComplexType::Tuple("()", Vec::new());
            assert_eq!(result, expected);
        });

        let input = "(i64)";
        with_parser(input, |p| {
            let result = p.parse_complex_type().unwrap().resolve(input);
            let expected = ComplexType::Tuple(
                "(i64)",
                vec![SimpleType::Primitive {
                    inner: "i64",
                    type_: PrimitiveType::I64,
                }],
            );
            assert_eq!(result, expected);
        });

        // Test for trailing comma too.
        let input = "(i64,)";
        with_parser(input, |p| {
            let result = p.parse_complex_type().unwrap().resolve(input);
            let expected = ComplexType::Tuple(
                "(i64,)",
                vec![SimpleType::Primitive {
                    inner: "i64",
                    type_: PrimitiveType::I64,
                }],
            );
            assert_eq!(result, expected);
        });

        let input = "(i64, str)";
        with_parser(input, |p| {
            let result = p.parse_complex_type().unwrap().resolve(input);
            let expected = ComplexType::Tuple(
                "(i64, str)",
                vec![
                    SimpleType::Primitive {
                        inner: "i64",
                        type_: PrimitiveType::I64,
                    },
                    SimpleType::Primitive {
                        inner: "str",
                        type_: PrimitiveType::Str,
                    },
                ],
            );
            assert_eq!(result, expected);
        });

        // Also confirm that the following are parse errors.
        let invalid_inputs: &[&'static str] = &["(,)", "(f32, <)", "(", "(f32", "(f32,"];
        for input in invalid_inputs {
            with_parser(input, |p| assert!(p.parse_complex_type().is_err()));
        }
    }

    #[test]
    fn test_parse_typed_ident() {
        let input = "id: i64";
        with_parser(input, |p| {
            let result = p.parse_typed_ident().unwrap().resolve(input);
            let expected = TypedIdent {
                ident: "id",
                type_: SimpleType::Primitive {
                    inner: "i64",
                    type_: PrimitiveType::I64,
                },
            };
            assert_eq!(result, expected);
        });
    }

    #[test]
    fn test_parse_annotation_basic() {
        let input = "@query drop_table_users()";
        with_parser(input, |p| {
            let result = p.parse_annotation().unwrap();
            let expected = Annotation {
                name: "drop_table_users",
                arguments: ArgType::Args(vec![]),
                result_type: ResultType::Unit,
            };
            assert_eq!(result.0.resolve(input), expected);
            assert_eq!(result.1, StatementType::Single);
        });
    }

    #[test]
    fn test_parse_annotation_begin_multi_statement() {
        let input = "@begin init_schema()";
        with_parser(input, |p| {
            let result = p.parse_annotation().unwrap();
            let expected = Annotation {
                name: "init_schema",
                arguments: ArgType::Args(vec![]),
                result_type: ResultType::Unit,
            };
            assert_eq!(result.0.resolve(input), expected);
            assert_eq!(result.1, StatementType::Multi);
        });
    }

    #[test]
    fn test_parse_annotation_query_argument() {
        // Test with wonky whitespace, and a trailing comma.
        let inputs: &[&'static str] = &[
            "@query delete_user_by_id(id: i64)",
            "@query delete_user_by_id(id: i64,)",
            "@query delete_user_by_id( id : i64 )",
        ];
        for input in inputs {
            with_parser(input, |p| {
                let result = p.parse_annotation().unwrap();
                let expected = Annotation {
                    name: "delete_user_by_id",
                    arguments: ArgType::Args(vec![TypedIdent {
                        ident: "id",
                        type_: SimpleType::Primitive {
                            inner: "i64",
                            type_: PrimitiveType::I64,
                        },
                    }]),
                    result_type: ResultType::Unit,
                };
                assert_eq!(result.0.resolve(input), expected);
                assert_eq!(result.1, StatementType::Single);
            });
        }
    }

    #[test]
    fn test_parse_annotation_arguments_trailing_comma() {
        // Test both with and without trailing comma. Also we play with the
        // whitespace a bit here.
        let inputs: &[&'static str] = &[
            "@query get_widgets_in_range (low : i64 , high : i64 , )",
            "@query get_widgets_in_range(low:i64,high:i64,)",
        ];
        for input in inputs {
            with_parser(input, |p| {
                let result = p.parse_annotation().unwrap();
                let expected = Annotation {
                    name: "get_widgets_in_range",
                    arguments: ArgType::Args(vec![
                        TypedIdent {
                            ident: "low",
                            type_: SimpleType::Primitive {
                                inner: "i64",
                                type_: PrimitiveType::I64,
                            },
                        },
                        TypedIdent {
                            ident: "high",
                            type_: SimpleType::Primitive {
                                inner: "i64",
                                type_: PrimitiveType::I64,
                            },
                        },
                    ]),
                    result_type: ResultType::Unit,
                };
                assert_eq!(result.0.resolve(input), expected);
                assert_eq!(result.1, StatementType::Single);
            });
        }
    }

    #[test]
    fn test_parse_annotation_result_type() {
        let input = "@query get_next_id() ->1 i64";
        with_parser(input, |p| {
            let result = p.parse_annotation().unwrap();
            let expected = Annotation {
                name: "get_next_id",
                arguments: ArgType::Args(vec![]),
                result_type: ResultType::Single(ComplexType::Simple(SimpleType::Primitive {
                    inner: "i64",
                    type_: PrimitiveType::I64,
                })),
            };
            assert_eq!(result.0.resolve(input), expected);
            assert_eq!(result.1, StatementType::Single);
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
