#![no_main]

use libfuzzer_sys::fuzz_target;
use querybinder::error::Result;
use querybinder::lexer::document::Lexer;
use querybinder::parser::document::Parser;
use querybinder::typecheck;

fn handle_input(input: &str) -> Result<()> {
    let lexer = Lexer::new(&input);
    let tokens = lexer.run()?;
    let mut parser = Parser::new(&input, &tokens);
    let doc = parser.parse_document()?;
    let _ = typecheck::check_document(&input, doc)?;
    Ok(())
}

fuzz_target!(|input: &str| {
    // Processing may result in an error, but it should not hang or panic.
    let _ = handle_input(input);
});
