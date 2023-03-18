// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

use std::path::Path;

use crate::Span;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub trait Error: std::fmt::Debug {
    /// The source location of the error.
    fn span(&self) -> Span;

    /// The error message.
    ///
    ///  * Shorter is better.
    ///  * Simpler is better (no jargon).
    ///  * The expected thing goes first, the actual thing goes second.
    fn message(&self) -> &str;

    /// Optionally, a note about error.
    ///
    /// For example, an unmatched parenthesis can point to the opening paren.
    fn note(&self) -> Option<(&str, Span)>;

    /// Optionally, a hint on how to fix the problem.
    fn hint(&self) -> Option<&str>;
}

impl dyn Error {
    pub fn print(&self, fname: &Path, input: &[u8]) {
        let bold_red = "\x1b[31;1m";
        let bold_yellow = "\x1b[33;1m";
        let reset = "\x1b[0m";

        let highlight = highlight_span_in_line(fname, input, self.span(), bold_red);
        eprint!("{}", highlight);
        eprintln!("{}Error:{} {}", bold_red, reset, self.message());

        if let Some((note, note_span)) = self.note() {
            let highlight = highlight_span_in_line(fname, input, note_span, bold_yellow);
            eprint!("\n{}", highlight);
            eprintln!("{}Note:{} {}", bold_yellow, reset, note);
        }

        if let Some(hint) = self.hint() {
            eprintln!("\n{}Hint:{} {}", bold_yellow, reset, hint);
        }
    }
}

fn highlight_span_in_line(fname: &Path, input: &[u8], span: Span, highlight_ansi: &str) -> String {
    use std::cmp;
    use std::fmt::Write;
    use unicode_width::UnicodeWidthStr;

    // Locate the line that contains the error.
    let mut line = 1;
    let mut line_start = 0;
    let mut line_end = 0;
    for (&c, i) in input.iter().zip(0..) {
        if i == span.start {
            break;
        }
        if c == b'\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    for (&c, i) in input[line_start..].iter().zip(line_start..) {
        if c == b'\n' {
            line_end = i;
            break;
        }
    }
    if line_end <= line_start {
        line_end = input.len();
    }

    // Try as best as we can to report the error. However, if the parse failed
    // because the input was invalid UTF-8, there is little we can do.
    let line_content = String::from_utf8_lossy(&input[line_start..line_end]);

    // The length of the mark can be longer than the line, for example when
    // token to mark was a multiline string literal. In that case, highlight
    // only up to the newline, don't extend the tildes too far.
    let indent_content = &line_content[..span.start - line_start];
    let as_of_error = &line_content[span.start - line_start..];
    let error_content = &as_of_error[..cmp::min(span.len(), as_of_error.len())];

    // The width of the error is not necessarily the number of bytes,
    // measure the Unicode width of the span to underline.
    let indent_width = indent_content.width();
    let mark_width = cmp::max(1, error_content.width());

    let line_num_str = line.to_string();
    let line_num_pad: String = line_num_str.chars().map(|_| ' ').collect();
    let mark_indent: String = " ".repeat(indent_width);
    let mark_under: String = "~".repeat(mark_width);
    let fname_str = fname.to_string_lossy();

    let reset = "\x1b[0m";

    let mut result = String::new();
    // Note, the unwraps here are safe because writing to a string does not fail.
    writeln!(
        &mut result,
        "{}--> {}:{}:{}",
        line_num_pad,
        fname_str,
        line,
        span.start - line_start
    )
    .unwrap();
    writeln!(&mut result, "{} |", line_num_pad).unwrap();
    writeln!(&mut result, "{} | {}", line_num_str, line_content).unwrap();
    writeln!(
        &mut result,
        "{} | {}{}^{}{}",
        line_num_pad,
        mark_indent,
        highlight_ansi,
        &mark_under[1..],
        reset
    )
    .unwrap();

    result
}

#[derive(Debug)]
pub struct ParseError {
    pub span: Span,
    pub message: &'static str,
    pub note: Option<(&'static str, Span)>,
}

impl From<ParseError> for Box<dyn Error> {
    fn from(err: ParseError) -> Self {
        Box::new(err)
    }
}

impl Error for ParseError {
    fn span(&self) -> Span {
        self.span
    }
    fn message(&self) -> &str {
        self.message
    }
    fn note(&self) -> Option<(&str, Span)> {
        self.note
    }
    fn hint(&self) -> Option<&str> {
        None
    }
}

/// A parse result, either the parsed value, or a parse error.
pub type PResult<T> = std::result::Result<T, ParseError>;

#[derive(Debug)]
pub struct TypeError {
    pub span: Span,
    pub message: &'static str,
    pub note: Option<(String, Span)>,
    pub hint: Option<String>,
}

impl TypeError {
    pub fn new(span: Span, message: &'static str) -> Self {
        Self {
            span,
            message,
            note: None,
            hint: None,
        }
    }

    pub fn with_note(
        span: Span,
        message: &'static str,
        note_span: Span,
        note: &'static str,
    ) -> Self {
        Self {
            span,
            message,
            note: Some((note.to_string(), note_span)),
            hint: None,
        }
    }

    pub fn with_hint(span: Span, message: &'static str, hint: &'static str) -> Self {
        Self {
            span,
            message,
            note: None,
            hint: Some(hint.to_string()),
        }
    }
}

impl From<TypeError> for Box<dyn Error> {
    fn from(err: TypeError) -> Self {
        Box::new(err)
    }
}

impl Error for TypeError {
    fn span(&self) -> Span {
        self.span
    }
    fn message(&self) -> &str {
        self.message
    }
    fn note(&self) -> Option<(&str, Span)> {
        self.note.as_ref().map(|(note, span)| (&note[..], *span))
    }
    fn hint(&self) -> Option<&str> {
        match &self.hint {
            None => None,
            Some(hint) => Some(&hint[..]),
        }
    }
}

/// A typechecking result, either the typed value, or an error.
pub type TResult<T> = std::result::Result<T, TypeError>;

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn highlight_span_handles_eof_span() {
        let fname: PathBuf = "x.sql".into();
        let input = b"foo";
        let color = "";
        let span = Span { start: 3, end: 3 };
        let result = highlight_span_in_line(&fname, input, span, color);
        let lines: Vec<_> = result.lines().collect();
        // The arrow points outside of the input, but that should be fine.
        assert_eq!(lines[2], "1 | foo");
        assert_eq!(lines[3], "  |    ^\x1b[0m");
    }
}
