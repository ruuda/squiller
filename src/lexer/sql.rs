use crate::is_ascii_identifier;
use crate::Span;

#[derive(Debug)]
enum State {
    Base,
    InSingleQuote,
    InDoubleQuote,
    InComment,
    InParam,
    InSpace,
    InIdent,
    InPunct,
    InControl,
    InMultibyte,
    Done,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Token {
    /// A sequence of ascii whitespace.
    Space,
    /// A sequence of ascii alphanumeric or _, not starting with a digit.
    Ident,
    /// A sequence of ascii control characters.
    Control,
    /// A sequence of non-ascii code points.
    Unicode,
    /// A query parameter, starting with `:`.
    Param,
    /// Content between single quotes.
    SingleQuoted,
    /// Content between double quotes.
    DoubleQuoted,
    /// A comment that starts with `--` and ends at a newline (not included).
    Comment,
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

    /// Lex the input until completion.
    pub fn run(mut self) -> Vec<(Token, Span)> {
        loop {
            let (start, state) = match self.state {
                State::Base => self.lex_base(),
                State::InSingleQuote => self.lex_in_single_quote(),
                State::InDoubleQuote => self.lex_in_double_quote(),
                State::InComment => self.lex_in_comment(),
                State::InParam => self.lex_in_param(),
                State::InSpace => self.lex_in_space(),
                State::InIdent => self.lex_in_ident(),
                State::InPunct => self.lex_in_punct(),
                State::InControl => self.lex_in_control(),
                State::InMultibyte => self.lex_in_multibyte(),
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

        self.tokens
    }

    fn lex_base(&mut self) -> (usize, State) {
        let input = &self.input.as_bytes()[self.start..];

        if input.len() == 0 {
            return (self.start, State::Done);
        }
        if input.starts_with(b"--") {
            return (self.start, State::InComment);
        }
        if input.starts_with(b"'") {
            return (self.start, State::InSingleQuote);
        }
        if input.starts_with(b"\"") {
            return (self.start, State::InDoubleQuote);
        }
        if input[0].is_ascii_whitespace() {
            return (self.start, State::InSpace);
        }
        if input.len() > 1 && input[0] == b':' && input[1].is_ascii_alphabetic() {
            return (self.start, State::InParam);
        }
        if input[0].is_ascii_punctuation() {
            return (self.start, State::InPunct);
        }
        if input[0].is_ascii_alphabetic() || input[0].is_ascii_digit() {
            return (self.start, State::InIdent);
        }
        if input[0].is_ascii_control() {
            return (self.start, State::InControl);
        }
        if input[0] > 127 {
            return (self.start, State::InMultibyte);
        }
        panic!(
            "I don't know what to do with this byte here: {} (0x{:02x})",
            char::from_u32(input[0] as u32).unwrap(),
            input[0],
        );
    }

    fn lex_in_quote(&mut self, quote: u8, token: Token) -> (usize, State) {
        use std::str;
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
                return (self.start + i + 1, State::Base);
            }
        }

        panic!(
            "Unclosed quote: {} for input {}",
            char::from_u32(quote as u32).unwrap(),
            str::from_utf8(input).unwrap(),
        );
    }

    fn lex_in_single_quote(&mut self) -> (usize, State) {
        self.lex_in_quote(b'\'', Token::SingleQuoted)
    }

    fn lex_in_double_quote(&mut self) -> (usize, State) {
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

    fn lex_in_comment(&mut self) -> (usize, State) {
        self.lex_while(|ch| ch != b'\n', Token::Comment)
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

    fn lex_in_control(&mut self) -> (usize, State) {
        self.lex_while(|ch| ch.is_ascii_control(), Token::Control)
    }

    fn lex_in_multibyte(&mut self) -> (usize, State) {
        self.lex_while(|ch| ch > 127, Token::Unicode)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_tokens(input: &str, expected_tokens: &[(Token, &str)]) {
        let tokens = Lexer::new(input).run();

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
    fn it_lexes_example_users() {
        let input = std::fs::read_to_string("examples/users.sql").unwrap();
        Lexer::new(&input).run();
    }

    #[test]
    fn it_handles_ascii_control_bytes() {
        test_tokens(
            "\x01",
            &[(Token::Control, "\x01")]
        );
    }

    #[test]
    fn it_handles_utf8_non_ascii_sequences() {
        test_tokens(
            "Älmhult",
            &[
                (Token::Unicode, "Ä"),
                (Token::Ident, "lmhult"),
            ]
        );
    }
}
