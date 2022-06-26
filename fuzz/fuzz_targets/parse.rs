#![no_main]

use libfuzzer_sys::fuzz_target;
use querybinder::lexer::sql::Lexer;
use querybinder::parser::document::Parser;

fuzz_target!(|input: &[u8]| {
    // Parsing may fail, but it should not hang or panic.
    let tokens = Lexer::new(&input).run();
    let mut parser = Parser::new(&input, &tokens);
    let _ = parser.parse_document();
});
