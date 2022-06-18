use crate::Span;

enum State {
    Base,
    InSingleQuote,
    InDoubleQuote,
    InComment,
    InQueryParam,
    InSpace,
    InIdent,
    InPunct,
    Done,
}

pub enum Token {
    Space,
    Ident,
    Punct,
    QueryParam,
    SingleQuoted,
    DoubleQuoted,
    Comment,
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
                State::InQueryParam => self.lex_in_query_param(),
                State::InSpace => self.lex_in_space(),
                State::InIdent => self.lex_in_ident(),
                State::InPunct => self.lex_in_punct(),
                State::Done => break,
            };
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
            return (self.start, State::InQueryParam);
        }
        if input[0].is_ascii_punctuation() {
            return (self.start, State::InPunct);
        }
        if input[0].is_ascii_alphabetic() {
            return (self.start, State::InIdent);
        }
        panic!(
            "I don't know what to do with this byte here: {} (0x{:02x})",
            char::from_u32(input[0] as u32).unwrap(),
            input[0],
        );
    }

    fn lex_in_single_quote(&mut self) -> (usize, State) {
        unimplemented!();
    }

    fn lex_in_double_quote(&mut self) -> (usize, State) {
        unimplemented!();
    }

    fn lex_in_comment(&mut self) -> (usize, State) {
        unimplemented!();
    }

    fn lex_in_query_param(&mut self) -> (usize, State) {
        unimplemented!();
    }

    fn lex_while<F: FnMut(u8) -> bool>(
        &mut self, 
        mut include: F,
        token: Token,
    ) -> (usize, State) {
        let input = &self.input[self.start..];

        for (len, ch) in input.iter().enumerate() {
            if include(*ch) {
                continue
            }
            self.push(token, len);
            return (self.start + len, State::Base);
        }

        self.push(token, input.len());
        (self.start + input.len(), State::Done)
    }

    fn lex_in_space(&mut self) -> (usize, State) {
        self.lex_while(|ch| ch.is_ascii_whitespace(), Token::Space)
    }

    fn lex_in_ident(&mut self) -> (usize, State) {
        self.lex_while(|ch| ch.is_ascii_alphanumeric(), Token::Ident)
    }

    fn lex_in_punct(&mut self) -> (usize, State) {
        self.lex_while(|ch| ch.is_ascii_punctuation(), Token::Ident)
    }
}

#[test]
fn it_lexes_example_users() {
    let input = std::fs::read("examples/users.sql").unwrap();
    let result = Lexer::new(&input).run();
}
