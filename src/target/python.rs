// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2023 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

/// Shared code generation for all Python targets.

use crate::{Span, NamedDocument};
use crate::ast::{Annotation, ArgType, ResultType};
use crate::codegen::pretty::Block;

pub fn header_comment(documents: &[NamedDocument]) -> Block {
    use crate::version::{REV, VERSION};

    let mut block = Block::new();

    let mut header = "# This file was generated by Squiller ".to_string();
    header.push_str(VERSION);
    match REV {
        Some(rev) => {
            header.push_str(" (commit ");
            header.push_str(&rev[..10]);
            header.push_str(").");
        }
        None => header.push_str(" (unspecified checkout)."),
    }
    block.push_line(header);
    block.push_line_str("# Input files:");
    for doc in documents {
        block.push_line(format!("# - {}", doc.fname.to_string_lossy()));
    }

    block
}

pub fn function_signature(
    ann: &Annotation<Span>,
    input: &str,
) -> Block {
    let mut block = Block::new();
    block.push_line_str("");
    block.push_line_str("");

    let mut line = "def ".to_string();
    line.push_str(ann.name.resolve(input));
    line.push_str("(tx: Transaction");

    match &ann.arguments {
        ArgType::Args(args) => {
            for arg in args {
                // TODO: Include types.
                line.push_str(", ");
                line.push_str(arg.ident.resolve(input));
            }
        }
        ArgType::Struct {
            var_name,
            type_name,
            ..
        } => {
            line.push_str(", ");
            line.push_str(var_name.resolve(input));
            line.push_str(": ");
            line.push_str(type_name.resolve(input));
        }
    }

    line.push_str(") -> ");

    match &ann.result_type {
        ResultType::Unit => line.push_str("None:"),
        ResultType::Option(_t) => {
            // TODO: Write the actual type.
            // TODO: Ensure import.
            line.push_str("Optional[Any]:");
        }
        ResultType::Single(_t) => {
            // TODO: Write the actual type.
            line.push_str("Any:");
        }
        ResultType::Iterator(_t) => {
            // TODO: Write the actual type.
            // TODO: Ensure import.
            line.push_str("Iterator[Any]:");
        }
    }

    block.push_line(line);

    block
}

/// Format the docstring, if there are doc comments.
pub fn docstring(
    docs: &[Span],
    input: &str,
) -> Block {
    let mut block = Block::new();

    if !docs.is_empty() {
        block.push_line_str("\"\"\"");
        for doc_line in docs {
            // The comment lines usually start with a space that went after
            // the "--" that starts the comment. In Python docstrings, we
            // don't want to start the line with a space, so remove them.
            let doc_line_str = doc_line.resolve(input);
            let line_content = match doc_line_str.as_bytes().first() {
                Some(b' ') => &doc_line_str[1..],
                _ => doc_line_str,
            };
            block.push_line_str(line_content);
        }
        block.push_line_str("\"\"\"");
    }

    block
}
