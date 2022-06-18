use crate::is_ascii_identifier;
use crate::Span;

#[derive(Debug)]
enum State {
    Base,
    InAnnotation,
    InIdent,
    Done,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Annotation,
    Ident,
    LParen,
    RParen,
    Lt,
    Gt,
    Colon,
    Semicolon,
    Comma,
    Arrow,
}

pub struct Lexer<'a> {
    input: &'a [u8],
    start: usize,
    state: State,
    tokens: Vec<(Token, Span)>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a [u8]) -> Lexer<'a> {
        Lexer {
            input: input,
            start: 0,
            state: State::Base,
            tokens: Vec::new(),
        }
    }

    pub fn tokens(&self) -> &[(Token, Span)] {
        &self.tokens
    }

    pub fn into_tokens(self) -> Vec<(Token, Span)> {
        self.tokens
    }

    fn push(&mut self, token: Token, len: usize) {
        let span = Span {
            start: self.start,
            end: self.start + len,
        };
        self.tokens.push((token, span));
    }

    /// Lex the span until completion.
    pub fn run(&mut self, span: Span) {
        self.start = span.start;
        self.state = State::Base;

        while self.start < span.end {
            let (start, state) = match self.state {
                State::Base => self.lex_base(),
                State::InAnnotation => self.lex_in_annotation(),
                State::InIdent => self.lex_in_ident(),
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
    }

    fn lex_base(&mut self) -> (usize, State) {
        let input = &self.input[self.start..];

        if input.len() == 0 {
            return (self.start, State::Done);
        }
        if input[0] == b'@' {
            return (self.start, State::InAnnotation);
        }
        if input[0] == b'(' {
            self.push(Token::LParen, 1);
            return (self.start + 1, State::Base);
        }
        if input[0] == b')' {
            self.push(Token::RParen, 1);
            return (self.start + 1, State::Base);
        }
        if input[0] == b'<' {
            self.push(Token::Lt, 1);
            return (self.start + 1, State::Base);
        }
        if input[0] == b'>' {
            self.push(Token::Gt, 1);
            return (self.start + 1, State::Base);
        }
        if input[0] == b':' {
            self.push(Token::Colon, 1);
            return (self.start + 1, State::Base);
        }
        if input[0] == b';' {
            self.push(Token::Semicolon, 1);
            return (self.start + 1, State::Base);
        }
        if input[0] == b',' {
            self.push(Token::Comma, 1);
            return (self.start + 1, State::Base);
        }
        if input.starts_with(b"->") {
            self.push(Token::Arrow, 2);
            return (self.start + 2, State::Base);
        }
        if input[0].is_ascii_whitespace() {
            return (self.start + 1, State::Base);
        }

        // Everything that is not an explicitly recognized punctuation token,
        // and not a space, is an identifier.
        (self.start, State::InIdent)
    }

    fn lex_skip_then_while<F: FnMut(u8) -> bool>(
        &mut self,
        n_skip: usize,
        mut include: F,
        token: Token,
    ) -> (usize, State) {
        let input = &self.input[self.start..];

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

    fn lex_in_annotation(&mut self) -> (usize, State) {
        self.lex_skip_then_while(1, is_ascii_identifier, Token::Annotation)
    }

    fn lex_in_ident(&mut self) -> (usize, State) {
        // The following characters are or may start punctuation of their own.
        // Anything else aside from whitespace can be part of an "identifier",
        // in particular &. This means that "&str" can be an identifier, which
        // makes dealing with types a bit easier.
        let end_chars = b",;:-<>()";
        self.lex_skip_then_while(
            0,
            |ch| !ch.is_ascii_whitespace() && !end_chars.contains(&ch),
            Token::Ident,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_tokens(input: &[u8], expected_tokens: &[(Token, &str)]) {
        use std::str;
        let span = Span {
            start: 0,
            end: input.len(),
        };
        let mut lexer = Lexer::new(input);
        lexer.run(span);

        for (i, expected) in expected_tokens.iter().enumerate() {
            assert!(
                lexer.tokens().len() > i,
                "Lexer has too few tokens, expected {} but got {}.",
                expected_tokens.len(),
                lexer.tokens().len(),
            );
            let actual = &lexer.tokens()[i];
            let actual_slice = &input[actual.1.start..actual.1.end];
            let actual_str = str::from_utf8(actual_slice).unwrap();
            assert_eq!(
                (actual.0, actual_str),
                *expected,
                "Mismatch at token {}.",
                i,
            );
        }
    }

    #[test]
    fn test_lex_no_arguments() {
        test_tokens(
            b" @query get_foo() -> i64;",
            &[
                (Token::Annotation, "@query"),
                (Token::Ident, "get_foo"),
                (Token::LParen, "("),
                (Token::RParen, ")"),
                (Token::Arrow, "->"),
                (Token::Ident, "i64"),
                (Token::Semicolon, ";"),
            ],
        );
    }

    #[test]
    fn test_lex_generic_return_type() {
        test_tokens(
            b"@query get_foo ( ) -> Option<User>;",
            &[
                (Token::Annotation, "@query"),
                (Token::Ident, "get_foo"),
                (Token::LParen, "("),
                (Token::RParen, ")"),
                (Token::Arrow, "->"),
                (Token::Ident, "Option"),
                (Token::Lt, "<"),
                (Token::Ident, "User"),
                (Token::Gt, ">"),
                (Token::Semicolon, ";"),
            ],
        );
    }

    #[test]
    fn test_lex_tuple_return_type() {
        test_tokens(
            b"@query get_name_and_age() -> (String, i64);",
            &[
                (Token::Annotation, "@query"),
                (Token::Ident, "get_name_and_age"),
                (Token::LParen, "("),
                (Token::RParen, ")"),
                (Token::Arrow, "->"),
                (Token::LParen, "("),
                (Token::Ident, "String"),
                (Token::Comma, ","),
                (Token::Ident, "i64"),
                (Token::RParen, ")"),
                (Token::Semicolon, ";"),
            ],
        );
    }

    #[test]
    fn test_lex_single_simple_argument() {
        test_tokens(
            b"@query get_user_by_name(name: &str) -> User;",
            &[
                (Token::Annotation, "@query"),
                (Token::Ident, "get_user_by_name"),
                (Token::LParen, "("),
                (Token::Ident, "name"),
                (Token::Colon, ":"),
                (Token::Ident, "&str"),
                (Token::RParen, ")"),
                (Token::Arrow, "->"),
                (Token::Ident, "User"),
                (Token::Semicolon, ";"),
            ],
        );
    }

    #[test]
    fn test_lex_double_simple_argument() {
        test_tokens(
            // Spice it up a bit, also omit the spaces.
            b"@query get_nearest_beacon(lng:f64,lat:f64)->i64;",
            &[
                (Token::Annotation, "@query"),
                (Token::Ident, "get_nearest_beacon"),
                (Token::LParen, "("),
                (Token::Ident, "lng"),
                (Token::Colon, ":"),
                (Token::Ident, "f64"),
                (Token::Comma, ","),
                (Token::Ident, "lat"),
                (Token::Colon, ":"),
                (Token::Ident, "f64"),
                (Token::RParen, ")"),
                (Token::Arrow, "->"),
                (Token::Ident, "i64"),
                (Token::Semicolon, ";"),
            ],
        );
    }
}
