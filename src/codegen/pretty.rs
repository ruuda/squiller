// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! Utilities for pretty-printing ASTs.

use std::io;
use std::mem;

pub type Result = io::Result<()>;

pub enum Block {
    Line(String),
    Stack(Vec<Block>),
    Indent(Box<Block>),
}

impl Block {
    pub fn new() -> Block {
        Block::Stack(Vec::new())
    }

    pub fn push_block(&mut self, block: Block) {
        if let Block::Stack(blocks) = self {
            // If we are appending two stacks, then we can fold them into a
            // single stack and avoid some nesting.
            match block {
                Block::Stack(mut more_blocks) => blocks.append(&mut more_blocks),
                other => blocks.push(other),
            }
        } else {
            let mut dummy = Block::Stack(Vec::new());
            mem::swap(&mut dummy, self);
            *self = Block::Stack(vec![dummy, block]);
        }
    }

    pub fn push_line(&mut self, fragment: String) {
        self.push_block(Block::Line(fragment));
    }

    pub fn push_line_str(&mut self, fragment: &str) {
        self.push_block(Block::Line(fragment.to_string()));
    }

    pub fn indent(self) -> Block {
        Block::Indent(Box::new(self))
    }

    /// Pretty-print the block tree at a given indentation level.
    fn format_internal(&self, out: &mut dyn io::Write, indent: u32) -> io::Result<()> {
        let thirty_two_spaces = "                                ";
        let indent_str = &thirty_two_spaces[..indent as usize];

        match self {
            Block::Line(line) => writeln!(out, "{}{}", indent_str, line)?,
            Block::Indent(block) => block.format_internal(out, indent + 4)?,
            Block::Stack(blocks) => {
                for block in blocks {
                    block.format_internal(out, indent)?;
                }
            }
        }

        Ok(())
    }

    /// Pretty-print the block tree.
    pub fn format(&self, out: &mut dyn io::Write) -> io::Result<()> {
        let indent = 0;
        self.format_internal(out, indent)
    }
}
