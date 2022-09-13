// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

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
