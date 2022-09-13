#![no_main]

use libfuzzer_sys::fuzz_target;
use squiller::lexer::document::Lexer;
use squiller::parser::document::Parser;
use squiller::error::PResult;

fn handle_input(input: &str) -> PResult<()> {
    let lexer = Lexer::new(&input);
    let tokens = lexer.run()?;
    let mut parser = Parser::new(&input, &tokens);
    let _ = parser.parse_document()?;
    Ok(())
}

fuzz_target!(|input: &str| {
    // Parsing may fail, but it should not hang or panic.
    let _ = handle_input(input);
});
