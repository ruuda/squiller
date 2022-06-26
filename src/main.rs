// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::io;
use std::path::PathBuf;

use querybinder::error::Error;
use querybinder::lexer::sql::Lexer;
use querybinder::parser::document::Parser;
use querybinder::target::Target;

use clap::ValueEnum;

#[derive(clap::Parser, Debug)]
#[clap(version)]
pub struct Args {
    /// Target to generate code for, use --target=help to list supported targets.
    #[clap(arg_enum, long = "target", short = 't', hide_possible_values = true)]
    pub target: Target,

    /// SQL files to process.
    #[clap(value_parser, value_name = "FILE", required = true)]
    pub input_files: Vec<PathBuf>,
}

impl Args {
    /// Alternative name for `parse` to avoid `Parser` name collision.
    pub fn get() -> Self {
        use clap::Parser;
        Self::parse()
    }
}

fn print_available_targets() -> io::Result<()> {
    use std::io::Write;
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    writeln!(&mut stdout, "Supported targets:\n")?;

    for variant in Target::value_variants() {
        let v = variant
            .to_possible_value()
            .expect("All variants should be documented.");
        let name = v.get_name();
        let help = v.get_help().expect("All variants should have a help text.");
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(&mut stdout, "  {:>5}", name)?;
        stdout.set_color(ColorSpec::new().set_fg(None))?;
        writeln!(&mut stdout, "    {}", help)?;
    }

    Ok(())
}

fn process_input(
    input_bytes: &[u8],
    target: Target,
    out: &mut dyn io::Write,
) -> Result<(), Box<dyn Error>> {
    let input_str = querybinder::str_from_utf8(input_bytes)?;
    let tokens = Lexer::new(&input_str).run()?;
    let mut parser = Parser::new(&input_str, &tokens);
    let doc = parser.parse_document()?;
    target
        .process_file(&input_str, doc, out)
        .expect("Failed to print output.");
    Ok(())
}

fn main() {
    let args = Args::get();

    if args.target == Target::Help {
        print_available_targets().expect("Oh no, failed to print.");
        std::process::exit(0);
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for fname in &args.input_files {
        let input = std::fs::read(fname).expect("Failed to read input file.");
        let result = process_input(&input, args.target, &mut stdout);
        if let Err(err) = result {
            err.print(&fname, &input);
            std::process::exit(1);
        }
    }
}
