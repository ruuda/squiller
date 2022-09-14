// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use squiller::cli;
use squiller::cli::{Cmd, USAGE};
use squiller::target::{Target, TARGETS};
use squiller::NamedDocument;

fn print_available_targets() -> io::Result<()> {
    let mut stdout = std::io::stdout();

    writeln!(&mut stdout, "Supported targets:\n")?;

    let max_width = TARGETS
        .iter()
        .map(|target| target.name.len())
        .max()
        .expect("There is at least one target value.");

    for target in TARGETS {
        let pad = " ".repeat(max_width - target.name.len());
        writeln!(&mut stdout, "  {}{}  {}", pad, target.name, target.help)?;
    }

    Ok(())
}

fn process_inputs(out: &mut dyn Write, target: &Target, inputs: &[(&Path, Vec<u8>)]) {
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
    let cmd = match cli::parse(std::env::args().collect()) {
        Ok(cmd) => cmd,
        Err(err) => {
            println!("{} See 'squiller --help'.", err);
            std::process::exit(1);
        }
    };

    let (target, input_files) = match cmd {
        Cmd::Help => {
            println!("{}", USAGE.trim());
            std::process::exit(0);
        }
        Cmd::TargetHelp => {
            print_available_targets().expect("Oh no, failed to print.");
            std::process::exit(0);
        }
        Cmd::Version => {
            todo!("print version");
        }
        Cmd::Generate { target, fnames } => {
            let target = match Target::from_name(&target) {
                Some(t) => t,
                None => {
                    println!(
                        "Unknown target '{}'. See 'squiller --target=help' \
                        for supported targets.",
                        target,
                    );
                    std::process::exit(1);
                }
            };
            (target, fnames)
        }
    };

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let fname_stdin: PathBuf = "stdin".into();

    let inputs: Vec<_> = input_files
        .iter()
        .map(|fname| match fname.as_str() {
            "-" => {
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

    process_inputs(&mut stdout, target, &inputs);
}
