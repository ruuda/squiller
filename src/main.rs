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
