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
    Done,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Token {
    Space,
    Ident,
    Punct,
    Param,
    SingleQuoted,
    DoubleQuoted,
    Comment,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Semicolon,
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
        let input = &self.input[self.start..];

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
        panic!(
            "I don't know what to do with this byte here: {} (0x{:02x})",
            char::from_u32(input[0] as u32).unwrap(),
            input[0],
        );
    }

    fn lex_in_quote(&mut self, quote: u8, token: Token) -> (usize, State) {
        use std::str;
        let input = &self.input[self.start..];

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

        let token = match self.input[self.start] {
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

#[test]
fn it_lexes_example_users() {
    let input = std::fs::read("examples/users.sql").unwrap();
    Lexer::new(&input).run();
}
