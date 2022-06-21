use crate::error::{PResult, ParseError};
use crate::lexer::annotation as ann;
use crate::lexer::sql;
use crate::parser::annotation as parse_ann;
use crate::Span;

type Annotation = crate::ast::Annotation<Span>;
type Fragment = crate::ast::Fragment<Span>;
type Query = crate::ast::Query<Span>;
type Section = crate::ast::Section<Span>;
type TypedIdent = crate::ast::TypedIdent<Span>;

/// Document parser.
///
/// Parses a tokenized SQL document into a list of queries with their metadata.
pub struct Parser<'a> {
    input: &'a [u8],
    tokens: &'a [(sql::Token, Span)],
    cursor: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [u8], tokens: &'a [(sql::Token, Span)]) -> Parser<'a> {
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
            .unwrap_or(Span { start: 0, end: 0 });
        let err = ParseError { span, message };
        Err(err)
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
                    let span_bytes = &self.input[span.start..span.end];
                    let num_newlines = span_bytes.iter().filter(|ch| **ch == b'\n').count();
                    if num_newlines >= 2 {
                        // If there was a blank line, that marks the end of the
                        // section, and given that we did not yet switch to
                        // query parsing mode, this means it was a verbatim
                        // section.
                        return Ok(Section::Verbatim(section_span));
                    }
                }
                sql::Token::Comment => {
                    // Cut off the leading "--" by bumping the start offset.
                    let comment_span = Span {
                        start: span.start + 2,
                        end: span.end,
                    };

                    // Potentially this comment could contain an annotation.
                    // Before we lex the entire thing, check if it contains the
                    // '@' marker.
                    let span_bytes = &self.input[comment_span.start..comment_span.end];
                    if span_bytes.contains(&b'@') {
                        let mut comment_lexer = ann::Lexer::new(self.input);
                        comment_lexer.run(comment_span);
                        // TODO
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
                    comments.push(comment_span);
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
                Some(sql::Token::Space) => {
                    self.consume();
                    continue;
                }
                Some(sql::Token::Comment) => {
                    // Cut off the leading "--" by bumping the start offset.
                    let span = self.tokens[self.cursor].1;
                    let comment_span = Span {
                        start: span.start + 2,
                        end: span.end,
                    };
                    comment_lexer.run(comment_span);
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
    fn consume_until_matching_close(&mut self) -> PResult<()> {
        let start_token = self
            .peek()
            .expect("Must be called with opening token under cursor.");
        let end_token = match start_token {
            sql::Token::LParen => sql::Token::RParen,
            sql::Token::LBrace => sql::Token::RBrace,
            sql::Token::LBracket => sql::Token::RBracket,
            _ => panic!("Invalid start token for this method."),
        };
        self.consume();

        while let Some(token) = self.peek() {
            self.consume();

            if token == end_token {
                return Ok(());
            }

            match token {
                sql::Token::LParen | sql::Token::LBrace | sql::Token::LBracket => {
                    return self.consume_until_matching_close();
                }
                sql::Token::RParen => return self.error("Found unmatched ')'."),
                sql::Token::RBrace => return self.error("Found unmatched '}'."),
                sql::Token::RBracket => return self.error("Found unmatched ']'."),
                _ => {}
            }
        }

        // TODO: With more detailed error types, we could even point out the
        // bracket that is not closed.
        match end_token {
            sql::Token::RParen => self.error("Found unclosed '('."),
            sql::Token::RBrace => self.error("Found unclosed '{'."),
            sql::Token::RBracket => self.error("Found unclosed '['."),
            _ => unreachable!("End token is one of the above three."),
        }
    }

    /// Skip whitespace, then parse a double quoted string as typed identifier.
    ///
    /// Returns the parsed identifier and type, but also the span of the quoted
    /// string, excluding any preceding whitespace but including the quotes.
    fn parse_typed_ident(&mut self) -> PResult<(Span, TypedIdent)> {
        while let Some(token) = self.peek() {
            let span = self.tokens[self.cursor].1;
            self.consume();

            match token {
                sql::Token::Space => continue,
                sql::Token::DoubleQuoted => {
                    // Lex everything in between the quotes.
                    let mut lexer = ann::Lexer::new(self.input);
                    let unquoted_span = Span {
                        start: span.start + 1,
                        end: span.end - 1,
                    };
                    lexer.run(unquoted_span);

                    // Then parse that.
                    let mut parser = parse_ann::Parser::new(self.input, lexer.tokens());
                    let result = parser.parse_typed_ident()?;
                    return Ok((span, result));
                }
                _ => {
                    return self.error(
                        "Unexpected token, expected a quoted typed column, e.g. \"age: u64\".",
                    )
                }
            }
        }

        self.error("Unexpected end of input, expected a quoted typed column, e.g. \"age: u64\".")
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
                // inside those brackets. E.g. if you have a subquery, we don't
                // care about a "select ... as ..." inside there.
                sql::Token::LParen | sql::Token::LBrace | sql::Token::LBracket => {
                    // TODO: There can still be parameters inside parens that we
                    // should break out as individual fragments!
                    self.consume_until_matching_close()?;
                }
                sql::Token::Ident => match span.resolve(self.input) {
                    // TODO: Recognize keyword in the lexer.
                    "as" | "AS" | "As" | "aS" => {
                        // TODO: Due to bizarre SQL syntax, not every top-level
                        // "AS" is necessarily part of a SELECT or RETURNING,
                        // deal with those cases.
                        self.consume();
                        let (hole_span, ident) = self.parse_typed_ident()?;
                        fragment.end = hole_span.start;
                        fragments.push(Fragment::Verbatim(fragment));
                        fragments.push(Fragment::TypedIdent(hole_span, ident));
                        fragment.start = hole_span.end;
                        fragment.end = hole_span.end;
                    }
                    _other_ident => {
                        self.consume();
                    }
                },
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
