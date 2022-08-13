use crate::error::{PResult, ParseError};
use crate::is_ascii_identifier;
use crate::Span;

#[derive(Debug)]
enum State {
    Base,
    InSingleQuote,
    InDoubleQuote,
    InLineComment,
    InInlineComment,
    InParam,
    InSpace,
    InIdent,
    InPunct,
    Done,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Token {
    /// A sequence of ascii whitespace.
    Space,
    /// A sequence of ascii alphanumeric or _, not starting with a digit.
    Ident,
    /// A query parameter, starting with `:`.
    Param,
    /// Content between single quotes.
    SingleQuoted,
    /// Content between double quotes.
    DoubleQuoted,
    /// The `--` or `/*` that open comments.
    CommentStart,
    /// The `*/` that closes comments. (But not a newline after `--`.)
    CommentEnd,
    /// A comment, excluding its opening and closing tokens, and excluding terminating newline.
    CommentInner,
    /// `(`.
    LParen,
    /// `)`.
    RParen,
    /// `[`.
    LBracket,
    /// `]`.
    RBracket,
    /// `{`.
    LBrace,
    /// `}`.
    RBrace,
    /// `;`.
    Semicolon,
    /// Punctuation that is not any of the previous punctuation tokens.
    Punct,
}

pub struct Lexer<'a> {
    input: &'a str,
    start: usize,
    state: State,
    tokens: Vec<(Token, Span)>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Lexer<'a> {
        Lexer {
            input: input,
            start: 0,
            state: State::Base,
            tokens: Vec::new(),
        }
    }

    fn push(&mut self, token: Token, len: usize) {
        let span = Span {
            start: self.start,
            end: self.start + len,
        };
        self.tokens.push((token, span));
    }

    /// Build a parse error at the current cursor location.
    fn error_while<F: FnMut(u8) -> bool, T>(
        &self,
        mut include: F,
        message: &'static str,
    ) -> PResult<T> {
        let input = &self.input.as_bytes()[self.start..];
        let mut err_end = self.start;
        for ch in input {
            if include(*ch) {
                err_end += 1;
                continue;
            }
        }
        let error = ParseError {
            span: Span {
                start: self.start,
                end: err_end,
            },
            message: message,
            note: None,
        };
        Err(error)
    }

    /// Lex the input until completion.
    pub fn run(mut self) -> PResult<Vec<(Token, Span)>> {
        loop {
            let (start, state) = match self.state {
                State::Base => self.lex_base()?,
                State::InSingleQuote => self.lex_in_single_quote()?,
                State::InDoubleQuote => self.lex_in_double_quote()?,
                State::InLineComment => self.lex_in_line_comment(),
                State::InInlineComment => self.lex_in_inline_comment()?,
                State::InParam => self.lex_in_param(),
                State::InSpace => self.lex_in_space(),
                State::InIdent => self.lex_in_ident(),
                State::InPunct => self.lex_in_punct(),
                State::Done => break,
            };

            // Uncomment the following to debug print while lexing.
            // if let Some((last_tok, last_span)) = self.tokens.last().as_ref() {
            //     use std::str;
            //     println!("start:{:?} state:{:?} tip_tok:{:?} tip_span:{:?}",
            //         start,
            //         state,
            //         last_tok,
            //         str::from_utf8(
            //             &self.input[last_span.start..last_span.end]
            //         ).unwrap(),
            //     );
            // }

            self.start = start;
            self.state = state;
        }

        Ok(self.tokens)
    }

    fn lex_base(&mut self) -> PResult<(usize, State)> {
        let input = &self.input.as_bytes()[self.start..];

        if input.len() == 0 {
            return Ok((self.start, State::Done));
        }
        if input.starts_with(b"--") {
            return Ok((self.start, State::InLineComment));
        }
        if input.starts_with(b"/*") {
            return Ok((self.start, State::InInlineComment));
        }
        if input.starts_with(b"'") {
            return Ok((self.start, State::InSingleQuote));
        }
        if input.starts_with(b"\"") {
            return Ok((self.start, State::InDoubleQuote));
        }
        if input[0].is_ascii_whitespace() {
            return Ok((self.start, State::InSpace));
        }
        if input.len() > 1 && input[0] == b':' && input[1].is_ascii_alphabetic() {
            return Ok((self.start, State::InParam));
        }
        if input[0].is_ascii_punctuation() {
            return Ok((self.start, State::InPunct));
        }
        if input[0].is_ascii_alphabetic() || input[0].is_ascii_digit() {
            return Ok((self.start, State::InIdent));
        }
        if input[0].is_ascii_control() {
            return self.error_while(
                |ch| ch.is_ascii_control(),
                "Control characters are not supported here.",
            );
        }
        if input[0] > 127 {
            // Multi-byte sequences of non-ascii code points are fine in strings
            // and comments, but not in raw SQL where we expect identifiers.
            return self.error_while(
                |ch| ch > 127,
                "Non-ascii characters are not supported here.",
            );
        }

        unreachable!(
            "We should have handled all bytes, but we forgot {} (0x{:02x})",
            char::from_u32(input[0] as u32).unwrap(),
            input[0],
        );
    }

    fn lex_in_quote(&mut self, quote: u8, token: Token) -> PResult<(usize, State)> {
        let input = &self.input.as_bytes()[self.start..];

        // Skip over the initial opening quote.
        for (i, &ch) in input.iter().enumerate().skip(1) {
            // Indexing does not go out of bounds here because we start at 1.
            if ch == quote && input[i - 1] == b'\\' {
                // An escaped quote should not end the token.
                continue;
            }
            if ch == quote {
                self.push(token, i + 1);
                return Ok((self.start + i + 1, State::Base));
            }
        }

        let error = ParseError {
            span: Span {
                start: self.start,
                end: self.input.len(),
            },
            message: "Unexpected end of input, string literal is not closed.",
            note: None,
        };
        Err(error)
    }

    fn lex_in_single_quote(&mut self) -> PResult<(usize, State)> {
        self.lex_in_quote(b'\'', Token::SingleQuoted)
    }

    fn lex_in_double_quote(&mut self) -> PResult<(usize, State)> {
        self.lex_in_quote(b'"', Token::DoubleQuoted)
    }

    fn lex_skip_then_while<F: FnMut(u8) -> bool>(
        &mut self,
        n_skip: usize,
        mut include: F,
        token: Token,
    ) -> (usize, State) {
        let input = &self.input.as_bytes()[self.start..];

        for (len, ch) in input.iter().enumerate().skip(n_skip) {
            if include(*ch) {
                continue;
            }
            self.push(token, len);
            return (self.start + len, State::Base);
        }

        self.push(token, input.len());
        (self.start + input.len(), State::Done)
    }

    fn lex_while<F: FnMut(u8) -> bool>(&mut self, include: F, token: Token) -> (usize, State) {
        self.lex_skip_then_while(0, include, token)
    }

    fn lex_in_line_comment(&mut self) -> (usize, State) {
        // The `--` is its own token.
        self.push(Token::CommentStart, 2);
        self.start += 2;
        self.lex_while(|ch| ch != b'\n', Token::CommentInner)
    }

    fn lex_in_inline_comment(&mut self) -> PResult<(usize, State)> {
        // The `/*` is its own token.
        self.push(Token::CommentStart, 2);
        self.start += 2;

        let input = &self.input.as_bytes()[self.start..];

        for len in 0..input.len() {
            if input[len..].starts_with(b"*/") {
                self.push(Token::CommentInner, len);
                self.start += len;
                self.push(Token::CommentEnd, 2);
                return Ok((self.start + 2, State::Base));
            }
        }

        // If we did not return by now, then the comment is unclosed. Reset the
        // start position to the start of the comment to get a nicer error.
        self.start -= 2;
        self.error_while(|_ch| true, "Unclosed /* */ comment.")
    }

    fn lex_in_param(&mut self) -> (usize, State) {
        self.lex_skip_then_while(1, is_ascii_identifier, Token::Param)
    }

    fn lex_in_space(&mut self) -> (usize, State) {
        // Space tokens are preserved, because we want to be able to replicate
        // the query literally later on, including formatting.
        self.lex_while(|ch| ch.is_ascii_whitespace(), Token::Space)
    }

    fn lex_in_ident(&mut self) -> (usize, State) {
        self.lex_while(is_ascii_identifier, Token::Ident)
    }

    fn lex_in_punct(&mut self) -> (usize, State) {
        debug_assert!(self.start < self.input.len());

        let token = match self.input.as_bytes()[self.start] {
            // For those characters, we have single-character tokens.
            b'(' => Token::LParen,
            b')' => Token::RParen,
            b'{' => Token::LBrace,
            b'}' => Token::RBrace,
            b'[' => Token::LBracket,
            b']' => Token::RBracket,
            b';' => Token::Semicolon,
            // If it's not one of those, then we make one token until either the
            // punctuation ends, or we do hit one of those.
            _ => {
                let end_punct_chars = b"'\"(){}[];";
                return self.lex_while(
                    |ch| ch.is_ascii_punctuation() && !end_punct_chars.contains(&ch),
                    Token::Punct,
                );
            }
        };
        self.push(token, 1);
        (self.start + 1, State::Base)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_tokens(input: &str, expected_tokens: &[(Token, &str)]) {
        let tokens = Lexer::new(input).run().expect("Failed to lex at all.");

        for (i, expected) in expected_tokens.iter().enumerate() {
            assert!(
                tokens.len() > i,
                "Lexer has too few tokens, expected {} but got {}.",
                expected_tokens.len(),
                tokens.len(),
            );
            let actual = &tokens[i];
            let actual_str = &input[actual.1.start..actual.1.end];
            assert_eq!(
                (actual.0, actual_str),
                *expected,
                "Mismatch at token {}.",
                i,
            );
        }
    }

    #[test]
    fn example_file_users_can_be_lexed() {
        let input = std::fs::read_to_string("examples/users.sql").unwrap();
        Lexer::new(&input).run().expect("Failed to lex input.");
    }

    #[test]
    fn it_lexes_simple_tokens() {
        let input = r#"
        -- Comment
        SELECT 'a' FROM "b" WHERE :c = 1;
        "#;
        test_tokens(
            input,
            &[
                (Token::Space, "\n        "),
                (Token::CommentStart, "--"),
                (Token::CommentInner, " Comment"),
                (Token::Space, "\n        "),
                (Token::Ident, "SELECT"),
                (Token::Space, " "),
                (Token::SingleQuoted, "'a'"),
                (Token::Space, " "),
                (Token::Ident, "FROM"),
                (Token::Space, " "),
                (Token::DoubleQuoted, "\"b\""),
                (Token::Space, " "),
                (Token::Ident, "WHERE"),
                (Token::Space, " "),
                (Token::Param, ":c"),
                (Token::Space, " "),
                (Token::Punct, "="),
                (Token::Space, " "),
                (Token::Ident, "1"),
                (Token::Semicolon, ";"),
            ],
        );
    }

    #[test]
    fn it_lexes_inline_comments() {
        let input = r#"
        SELECT /* hello */ FROM
        "#;
        test_tokens(
            input,
            &[
                (Token::Space, "\n        "),
                (Token::Ident, "SELECT"),
                (Token::Space, " "),
                (Token::CommentStart, "/*"),
                (Token::CommentInner, " hello "),
                (Token::CommentEnd, "*/"),
                (Token::Space, " "),
                (Token::Ident, "FROM"),
                (Token::Space, "\n        "),
            ],
        );
    }

    #[test]
    fn it_lexes_semicolons_after_inline_comments() {
        let input = "SELECT /* */;";
        test_tokens(
            input,
            &[
                (Token::Ident, "SELECT"),
                (Token::Space, " "),
                (Token::CommentStart, "/*"),
                (Token::CommentInner, " "),
                (Token::CommentEnd, "*/"),
                (Token::Semicolon, ";"),
            ],
        );
    }

    #[test]
    fn ascii_control_bytes_result_in_error() {
        let input = "\x01";
        let error = Lexer::new(input).run().err().unwrap();
        assert_eq!(error.span, Span { start: 0, end: 1 });
        assert!(error.message.contains("Control characters"));
    }

    #[test]
    fn non_ascii_sequences_result_in_error() {
        let input = "Älmhult";
        let error = Lexer::new(input).run().err().unwrap();
        assert_eq!(error.span.resolve(input), "Ä");
        assert_eq!(error.span, Span { start: 0, end: 2 });
        assert!(error.message.contains("Non-ascii"));
    }

    #[test]
    fn unmatched_quotes_result_in_error() {
        let input = "an 'unclosed";
        let error = Lexer::new(input).run().err().unwrap();
        assert_eq!(
            error.span,
            Span {
                start: 3,
                end: input.len()
            }
        );
        assert_eq!(error.span.resolve(input), "'unclosed");
    }

    #[test]
    fn unmatched_inline_comment_reports_error_at_comment_start() {
        let input = "select /* unclosed";
        let error = Lexer::new(input).run().err().unwrap();
        assert_eq!(
            error.span,
            Span {
                start: "select ".len(),
                end: input.len()
            }
        );
        assert_eq!(error.span.resolve(input), "/* unclosed");
    }
}
