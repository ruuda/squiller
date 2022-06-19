// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use crate::Span;

use querybinder::lexer::sql::Lexer;
use querybinder::parser::document::Parser;

fn main() {
    for arg in std::env::args().skip(1) {
        let input = std::fs::read(arg).expect("Failed to read input file.");
        let tokens = Lexer::new(&input).run();
        let mut parser = Parser::new(&input, &tokens);
        while let Ok(section) = parser.parse_section() {
            let section_str = section.resolve(&input);
            println!("{:#?}", section_str);
        }
    }
}
