// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use squiller::target::Target;
use squiller::NamedDocument;

use clap::ValueEnum;

#[derive(clap::Parser, Debug)]
#[clap(version)]
pub struct Args {
    /// Target to generate code for, use --target=help to list supported targets.
    #[clap(arg_enum, long = "target", short = 't', hide_possible_values = true)]
    pub target: Target,

    /// SQL files to process, or "-" for stdin.
    #[clap(value_parser, value_name = "FILE")]
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
    use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    writeln!(&mut stdout, "Supported targets:\n")?;

    let possible_values: Vec<_> = Target::value_variants()
        .iter()
        .map(|variant| {
            variant
                .to_possible_value()
                .expect("All variants should be documented.")
        })
        .collect();

    let max_width = possible_values
        .iter()
        .map(|v| v.get_name().len())
        .max()
        .expect("There is at least one possible value.");

    for v in possible_values {
        let name = v.get_name();
        let help = v.get_help().expect("All variants should have a help text.");
        let pad = " ".repeat(max_width - name.len());
        write!(&mut stdout, "  {}", pad)?;
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(&mut stdout, "{}", name)?;
        stdout.set_color(ColorSpec::new().set_fg(None))?;
        writeln!(&mut stdout, "    {}", help)?;
    }

    Ok(())
}

fn process_inputs(out: &mut dyn Write, target: Target, inputs: &[(&Path, Vec<u8>)]) {
    let mut documents = Vec::with_capacity(inputs.len());

    for (fname, input_bytes) in inputs {
        let named_document = match NamedDocument::process_input(fname, input_bytes) {
            Ok(doc) => doc,
            Err(err) => {
                err.print(fname, input_bytes);
                std::process::exit(1);
            }
        };
        documents.push(named_document);
    }

    target
        .process_files(out, &documents[..])
        .expect("Failed to write output.");
}

fn main() {
    let args = Args::get();

    if args.target == Target::Help {
        print_available_targets().expect("Oh no, failed to print.");
        std::process::exit(0);
    } else {
        if args.input_files.len() == 0 {
            println!("No input files provided.");
            std::process::exit(0);
        }
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let fname_stdin: PathBuf = "stdin".into();

    let inputs: Vec<_> = args
        .input_files
        .iter()
        .map(|fname| match fname.to_str() {
            Some("-") => {
                let mut bytes = Vec::new();
                std::io::stdin()
                    .read_to_end(&mut bytes)
                    .expect("Failed to read input from stdin.");
                (fname_stdin.as_ref(), bytes)
            }
            _ => {
                let bytes = std::fs::read(fname).expect("Failed to read input file.");
                (fname.as_ref(), bytes)
            }
        })
        .collect();

    process_inputs(&mut stdout, args.target, &inputs);
}
