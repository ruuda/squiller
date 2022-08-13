use crate::error::{PResult, ParseError};
use crate::lexer::annotation as ann;
use crate::lexer::sql;
use crate::parser::annotation as parse_ann;
use crate::Span;

type Annotation = crate::ast::Annotation<Span>;
type Document = crate::ast::Document<Span>;
type Fragment = crate::ast::Fragment<Span>;
type Query = crate::ast::Query<Span>;
type Section = crate::ast::Section<Span>;
type TypedIdent = crate::ast::TypedIdent<Span>;

/// Document parser.
///
/// Parses a tokenized SQL document into a list of queries with their metadata.
pub struct Parser<'a> {
    input: &'a str,
    tokens: &'a [(sql::Token, Span)],
    cursor: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str, tokens: &'a [(sql::Token, Span)]) -> Parser<'a> {
        Parser {
            input: input,
            tokens: tokens,
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
                    .unwrap_or(Span { start: 0, end: 0 })
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
    fn peek(&self) -> Option<sql::Token> {
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

    /// Parse a single section from the document.
    pub fn parse_document(&mut self) -> PResult<Document> {
        let mut sections = Vec::new();
        while self.peek().is_some() {
            sections.push(self.parse_section()?);
        }
        let result = Document { sections };
        Ok(result)
    }

    /// Parse a single section from the document.
    pub fn parse_section(&mut self) -> PResult<Section> {
        debug_assert!(
            self.peek().is_some(),
            "Cannot call `parse_section` with no tokens left.",
        );

        let section_start_span = self.tokens[self.cursor].1;
        let mut comments = Vec::new();
        let mut section_span = Span {
            start: section_start_span.start,
            end: section_start_span.end,
        };

        while self.peek().is_some() {
            let (token, span) = self.tokens[self.cursor];
            section_span.end = span.end;
            self.consume();

            match token {
                sql::Token::Space => {
                    let span_bytes = &self.input.as_bytes()[span.start..span.end];
                    let num_newlines = span_bytes.iter().filter(|ch| **ch == b'\n').count();
                    if num_newlines >= 2 {
                        // If there was a blank line, that marks the end of the
                        // section, and given that we did not yet switch to
                        // query parsing mode, this means it was a verbatim
                        // section.
                        return Ok(Section::Verbatim(section_span));
                    }
                }
                sql::Token::CommentInner => {
                    // Potentially this comment could contain an annotation.
                    // Before we lex the entire thing, check if it contains the
                    // '@' marker.
                    let span_bytes = &self.input.as_bytes()[span.start..span.end];
                    if span_bytes.contains(&b'@') {
                        let mut comment_lexer = ann::Lexer::new(self.input);
                        comment_lexer.run(span);
                        match comment_lexer.tokens().first() {
                            // If the comment starts with an annotation, then
                            // this means we are inside a query section, and we
                            // continue parsing in query mode.
                            Some((ann::Token::Annotation, _)) => {
                                let query = self.parse_query(comments, comment_lexer)?;
                                return Ok(Section::Query(query));
                            }
                            _ => {}
                        }
                    }

                    // If it was not an annotation, we still record the comment,
                    // because if we later encounter an annotation, the
                    // preceding comments serve as the doc comment for the query.
                    comments.push(span);
                }
                _ => {}
            }
        }

        // If we reached the end of the document without producing a query,
        // then we must be in an unstructured section.
        return Ok(Section::Verbatim(section_span));
    }

    /// Parse annotations inside a comment.
    ///
    /// When we enter this state, we already have one comment line that contains
    /// an annotation, but the annotation may be spread over multiple lines, so
    /// consume those up to the start of the query itself.
    fn parse_annotation(&mut self, mut comment_lexer: ann::Lexer<'a>) -> PResult<Annotation> {
        loop {
            match self.peek() {
                Some(sql::Token::Space)
                | Some(sql::Token::CommentStart)
                | Some(sql::Token::CommentEnd) => {
                    self.consume();
                    continue;
                }
                Some(sql::Token::CommentInner) => {
                    let span = self.tokens[self.cursor].1;
                    comment_lexer.run(span);
                    self.consume();
                }
                None => {
                    return self.error("Unexpected end of input, expected query after annotation.")
                }
                _ => {
                    // If it's not a comment or whitespace, then this must be
                    // the start of the query itself, so the annotation ends
                    // here.
                    let mut parser = parse_ann::Parser::new(self.input, comment_lexer.tokens());
                    return parser.parse_annotation();
                }
            }
        }
    }

    /// Consume tokens enclosed by (), [], or {}.
    ///
    /// The cursor must be on the opening bracket, and this method will consume
    /// everything up to and including the matching closing bracket.
    ///
    /// If any parameter is encountered inside the brackets, the span is added
    /// to the list of fragments, and the current fragment is updated.
    fn consume_until_matching_close(
        &mut self,
        fragments: &mut Vec<Fragment>,
        fragment: &mut Span,
    ) -> PResult<()> {
        let start_token = self
            .peek()
            .expect("Must be called with opening token under cursor.");
        let end_token = match start_token {
            sql::Token::LParen => sql::Token::RParen,
            sql::Token::LBrace => sql::Token::RBrace,
            sql::Token::LBracket => sql::Token::RBracket,
            _ => panic!("Invalid start token for this method: {:?}.", start_token),
        };
        let start_span = self.tokens[self.cursor].1;
        self.consume();

        while let Some(token) = self.peek() {
            let span = self.tokens[self.cursor].1;

            if token == end_token {
                self.consume();
                return Ok(());
            }

            match token {
                sql::Token::LParen | sql::Token::LBrace | sql::Token::LBracket => {
                    // TODO: This might cause a stack overflow for deeply nested
                    // parens. Add some kind of depth counter to limit this.
                    self.consume_until_matching_close(fragments, fragment)?;
                }
                sql::Token::RParen => return self.error("Found unmatched ')'."),
                sql::Token::RBrace => return self.error("Found unmatched '}'."),
                sql::Token::RBracket => return self.error("Found unmatched ']'."),
                sql::Token::Param => {
                    fragment.end = span.start;
                    fragments.push(Fragment::Verbatim(*fragment));
                    fragments.push(Fragment::Param(span));
                    fragment.start = span.end;
                    fragment.end = span.end;
                    self.consume();
                }
                sql::Token::Semicolon => {
                    // The statement ends here, but we havent' found a closing
                    // bracket yet, fall through to the end error here.
                    break;
                }
                _ => {
                    self.consume();
                }
            }
        }

        match end_token {
            sql::Token::RParen => {
                self.error_with_note("Expected ')'.", start_span, "Unmatched '(' opened here.")
            }
            sql::Token::RBrace => {
                self.error_with_note("Expected '}'.", start_span, "Unmatched '{' opened here.")
            }
            sql::Token::RBracket => {
                self.error_with_note("Expected ']'.", start_span, "Unmatched '[' opened here.")
            }
            _ => unreachable!("End token is one of the above three."),
        }
    }

    /// Skip whitespace, then parse a double quoted string as typed identifier.
    ///
    /// Returns the parsed identifier and type, but also the span of the quoted
    /// string, excluding any preceding whitespace but including the quotes.
    fn parse_type_annotation(&mut self, type_span: Span) -> PResult<Fragment> {
        let mut lexer = ann::Lexer::new(self.input);
        lexer.run(type_span);

        if lexer.tokens().len() == 0 {
            let err = ParseError {
                span: type_span,
                message: "Empty type annotation, expected a type after the ':'.",
                note: None,
            };
            return Err(err);
        }

        let mut parser = parse_ann::Parser::new(self.input, lexer.tokens());
        let mut type_ = parser.parse_type()?;

        // Consume the CommentInner token that we are parsing the annotation from.
        let annotation_token_index = self.cursor;
        self.consume();

        // The comment we were inside of could have been a /* */ style comment
        // with an end token. Consume that as well, if it's there.
        if let Some(sql::Token::CommentEnd) = self.peek() {
            self.consume();
        }

        let end_span = self.tokens[self.cursor - 1].1;
        let mut result: Option<Fragment> = None;

        // Now that we have the annotation itself, we need to walk back to find
        // the token which is being annotated. We must be inside a comment, so
        // preceding this token must be a CommentStart token that we skip over.
        for i in (0..annotation_token_index - 1).rev() {
            let (prev_token, prev_span) = self.tokens[i];
            let ident = TypedIdent {
                ident: prev_span,
                type_: type_,
            };
            let full_span = Span {
                start: prev_span.start,
                end: end_span.end,
            };
            match prev_token {
                sql::Token::Space => {
                    // We put the type in the typed ident and then if we are not
                    // going to use it we pull it back out here, to make the
                    // borrow checker happy, to avoid cloning the type.
                    type_ = ident.type_;
                    continue;
                }
                sql::Token::Ident => {
                    result = Some(Fragment::TypedIdent(full_span, ident));
                    break;
                }
                sql::Token::Param => {
                    result = Some(Fragment::TypedParam(full_span, ident));
                    break;
                }
                _ => break,
            }
        }

        match result {
            None => {
                self.cursor = annotation_token_index;
                self.error(
                    "Invalid type annotation, expected \
                    an identifier or parameter before the annotation.",
                )
            }
            Some(fragment) => Ok(fragment),
        }
    }

    /// Parse a single section from the document.
    fn parse_query(
        &mut self,
        comments: Vec<Span>,
        comment_lexer: ann::Lexer<'a>,
    ) -> PResult<Query> {
        let annotation = self.parse_annotation(comment_lexer)?;

        let fragment_start = match self.tokens.get(self.cursor) {
            None => return self.error("Expected query after annotation."),
            Some((_, span)) => span.start,
        };

        let mut fragments = Vec::new();
        let mut fragment = Span {
            start: fragment_start,
            end: fragment_start,
        };

        while let Some((token, span)) = self.tokens.get(self.cursor) {
            match token {
                // If there are things enclosed in brackets, we do not look
                // inside those brackets for things aside from parameters. E.g.
                // if you have a subquery, we don't care about type comments in
                // inside there. TODO: That is wrong, we do need to parse type
                // comments, inside parens.
                sql::Token::LParen | sql::Token::LBrace | sql::Token::LBracket => {
                    self.consume_until_matching_close(&mut fragments, &mut fragment)?;
                }
                sql::Token::CommentInner => {
                    // If there is a comment, and it starts with a `:`,
                    // optionally preceded by whitespace, then we interpret that
                    // as a type comment. So first, check if we are in that case
                    // at all.
                    let content = span.resolve(self.input);
                    let colon_pos = match content.find(":") {
                        None => {
                            self.consume();
                            continue;
                        }
                        Some(i) => i,
                    };
                    if !content[..colon_pos]
                        .bytes()
                        .all(|ch| ch.is_ascii_whitespace())
                    {
                        self.consume();
                        continue;
                    }

                    // If we get here, then we define this to be a type comment.
                    // (Perhaps it's not preceded by a valid identifier or
                    // parameter, but then we define that as an error, instead
                    // of considering this a normal comment.)
                    let type_span = Span {
                        start: span.start + colon_pos + 1,
                        end: span.end,
                    };
                    let hole_fragment = self.parse_type_annotation(type_span)?;
                    let hole_span = hole_fragment.span();

                    match hole_fragment {
                        frag @ Fragment::TypedIdent(..) => {
                            fragment.end = hole_span.start;
                            debug_assert!(fragment.start < fragment.end);
                            fragments.push(Fragment::Verbatim(fragment));
                            fragments.push(frag);
                        }
                        frag @ Fragment::TypedParam(..) => {
                            // If this type annotation turned out to annotate a
                            // parameter, then we replace the parameter fragment
                            // that we pushed previously with the new typed
                            // parameter fragment.
                            fragments.pop();
                            fragment = fragments
                                .pop()
                                .expect("Must have a fragment before parameter fragment.")
                                .span();
                            fragment.end = hole_span.start;
                            debug_assert!(fragment.start < fragment.end);
                            fragments.push(Fragment::Verbatim(fragment));
                            fragments.push(frag);
                        }
                        _ => panic!("Invalid fragment: {:?}", hole_fragment),
                    }
                    fragment.start = hole_span.end;
                    fragment.end = hole_span.end;
                }
                sql::Token::Param => {
                    fragment.end = span.start;
                    fragments.push(Fragment::Verbatim(fragment));
                    fragments.push(Fragment::Param(*span));
                    fragment.start = span.end;
                    fragment.end = span.end;
                    self.consume();
                }
                sql::Token::Semicolon => {
                    // The semicolon marks the end of the query.
                    fragment.end = span.end;
                    fragments.push(Fragment::Verbatim(fragment));
                    self.consume();

                    let result = Query {
                        docs: comments,
                        annotation: annotation,
                        fragments: fragments,
                    };
                    return Ok(result);
                }
                _other_token => {
                    self.consume();
                }
            }
        }

        self.error("Unexpected end of input, annotated query does not end with ';'.")
    }
}

#[cfg(test)]
mod test {
    use super::Parser;
    use crate::ast::{Annotation, Fragment, Query, Section, Type, TypedIdent};
    use crate::error::Error;
    use crate::lexer::sql::Lexer;
    use crate::Span;

    fn with_parser<F: FnOnce(&mut Parser)>(input: &str, f: F) {
        let tokens = Lexer::new(input).run().expect("Failed to lex the input.");
        let mut parser = Parser::new(input, &tokens);
        f(&mut parser)
    }

    #[test]
    fn parse_section_handles_newline_in_annotation() {
        let input = "
        -- @query multiline_signature(
        --   key: &str,
        --   value: &str,
        -- ) -> i64
        SELECT * FROM kv;
        ";
        with_parser(input, |p| {
            let result = p.parse_section().unwrap().resolve(input);
            let expected = Section::Query(Query {
                docs: vec![],
                annotation: Annotation {
                    name: "multiline_signature",
                    parameters: vec![
                        TypedIdent {
                            ident: "key",
                            type_: Type::Simple("&str"),
                        },
                        TypedIdent {
                            ident: "value",
                            type_: Type::Simple("&str"),
                        },
                    ],
                    result_type: Type::Simple("i64"),
                },
                fragments: vec![Fragment::Verbatim("SELECT * FROM kv;")],
            });
            assert_eq!(result, expected);
        });
    }

    #[test]
    fn unmatched_paren_at_statement_end_causes_error() {
        let input = "
        -- @query q()
        SELECT ( FROM t;
        ";
        with_parser(input, |p| {
            let result = p.parse_section();
            assert!(result.is_err());
            let err: Box<dyn Error> = result.err().unwrap().into();
            assert!(err.message().contains("Expected ')'"));
        });
    }

    #[test]
    fn empty_type_annotation_is_error() {
        let input = r#"
        -- @query q()
        SELECT id /* : */ FROM t;
        "#;
        with_parser(input, |p| {
            let err = p.parse_section().err().unwrap();
            let wider_span = Span {
                start: err.span.start - 2,
                end: err.span.end + 2,
            };
            assert_eq!(wider_span.resolve(input), " : */");
        });
    }

    #[test]
    fn it_parses_a_type_annotation() {
        let input = "
        -- @query q()
        SELECT name /* : str */ FROM t;
        ";
        with_parser(input, |p| {
            let result = p.parse_section().unwrap();
            let query = match result {
                Section::Query(q) => q,
                _ => panic!("Expected query."),
            };
            let fragments = query.resolve(input).fragments;
            let expected = [
                Fragment::Verbatim("SELECT "),
                Fragment::TypedIdent(
                    "name /* : str */",
                    TypedIdent {
                        ident: "name",
                        type_: Type::Simple("str"),
                    },
                ),
                Fragment::Verbatim(" FROM t;"),
            ];
            assert_eq!(fragments, expected);
        });
    }

    #[test]
    fn it_parses_a_sinple_comment_without_newline() {
        // The fuzzer found this input to cause OOM. The problem was not in the
        // parser, but still, let's add this as a regression test.
        let input = "---@";
        with_parser(input, |p| {
            let result = p.parse_section().unwrap();
            assert_eq!(result.resolve(input), Section::Verbatim("---@"));
        });
    }

    #[test]
    fn handles_typed_params_with_annotation_in_inline_comment() {
        let input = "/* @query q() */ SELECT a from b where c = :c /* :str */;";
        with_parser(input, |p| {
            let result = p.parse_section().unwrap().resolve(input);
            let expected = Section::Query(Query {
                docs: vec![],
                annotation: Annotation {
                    name: "q",
                    parameters: vec![],
                    result_type: Type::Unit,
                },
                fragments: vec![
                    Fragment::Verbatim("SELECT a from b where c = "),
                    Fragment::TypedParam(
                        ":c /* :str */",
                        TypedIdent {
                            ident: ":c",
                            type_: Type::Simple("str"),
                        },
                    ),
                    Fragment::Verbatim(";"),
                ],
            });
            assert_eq!(result, expected);
        });
    }

    #[test]
    fn it_parses_a_query_with_invalid_type_annotation() {
        // The fuzzer found this input to trigger an assertion failure.
        let input = "--@query q()\nx--:T";
        with_parser(input, |p| {
            let result = p.parse_section();
            assert!(result.is_err());
        });
    }
}
