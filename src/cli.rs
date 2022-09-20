// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

// This CLI parser is adapted from the one in Tako [1] which is copyrighted by
// Arian van Putten, Ruud van Asseldonk, and Tako Marks, and licensed Apache2.
// [1] https://github.com/ruuda/tako

use std::fmt;
use std::vec;

const USAGE: &'static str = r#"
Squiller -- Generate boilerplate from annotated SQL queries.

Usage:
  squiller --target <target> <file>...
  squiller --target help
  squiller -h | --help
  squiller --version

Arguments:
  <file>...             One or more input files to process, or '-' for stdin.

Options:
  -h --help             Show this screen.
  -t --target <target>  Target to generate code for, use '--target=help' to
                        list supported targets.
  --version             Show version.
"#;

#[derive(Debug, Eq, PartialEq)]
pub enum Cmd {
    Generate { target: String, fnames: Vec<String> },
    TargetHelp,
    Help,
    Version,
}

enum Arg<T> {
    Plain(T),
    Short(T),
    Long(T),
}

impl Arg<String> {
    fn as_ref(&self) -> Arg<&str> {
        match *self {
            Arg::Plain(ref x) => Arg::Plain(&x[..]),
            Arg::Short(ref x) => Arg::Short(&x[..]),
            Arg::Long(ref x) => Arg::Long(&x[..]),
        }
    }

    fn into_string(self) -> String {
        match self {
            Arg::Plain(x) => x,
            Arg::Short(x) => x,
            Arg::Long(x) => x,
        }
    }
}

impl fmt::Display for Arg<String> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Arg::Plain(ref x) => write!(f, "{}", x),
            Arg::Short(ref x) => write!(f, "-{}", x),
            Arg::Long(ref x) => write!(f, "--{}", x),
        }
    }
}

struct ArgIter {
    /// Underlying args iterator.
    args: vec::IntoIter<String>,

    /// Whether we have observed a `--` argument.
    is_raw: bool,

    /// Leftover to return after an `--foo=bar` or `-fbar`-style argument.
    ///
    /// `--foo=bar` is returned as `Long(foo)` followed by `Plain(bar)`.
    /// `-fbar` is returned as `Short(f)` followed by `Plain(bar)`.
    leftover: Option<String>,
}

impl ArgIter {
    pub fn new(args: Vec<String>) -> ArgIter {
        ArgIter {
            args: args.into_iter(),
            is_raw: false,
            leftover: None,
        }
    }
}

impl Iterator for ArgIter {
    type Item = Arg<String>;

    fn next(&mut self) -> Option<Arg<String>> {
        if self.leftover.is_some() {
            return self.leftover.take().map(Arg::Plain);
        }

        let arg = self.args.next()?;

        if self.is_raw {
            return Some(Arg::Plain(arg));
        }

        if &arg == "--" {
            self.is_raw = true;
            return self.next();
        }

        if arg.starts_with("--") {
            let mut flag = String::from(&arg[2..]);
            if let Some(i) = flag.find('=') {
                self.leftover = Some(flag.split_off(i + 1));
                flag.truncate(i);
            }
            return Some(Arg::Long(flag));
        }

        if arg == "-" {
            return Some(Arg::Plain(arg));
        }

        if arg.starts_with("-") {
            let mut flag = String::from(&arg[1..]);
            if flag.len() > 1 {
                self.leftover = Some(flag.split_off(1));
                flag.truncate(1);
            }
            return Some(Arg::Short(flag));
        }

        Some(Arg::Plain(arg))
    }
}

pub fn parse(argv: Vec<String>) -> Result<Cmd, String> {
    let mut args = ArgIter::new(argv);

    // Skip executable name.
    args.next();

    let mut fnames = Vec::new();
    let mut target = None;
    let mut is_help = false;
    let mut is_version = false;

    while let Some(arg) = args.next() {
        match arg.as_ref() {
            Arg::Plain(..) => fnames.push(arg.into_string()),
            Arg::Short("t") | Arg::Long("target") => match args.next() {
                Some(Arg::Plain(t)) => target = Some(t),
                _ => return Err(format!("Expected target name after '{}'.", arg)),
            },
            Arg::Long("version") => {
                is_help = false;
                is_version = true;
            }
            Arg::Short("h") | Arg::Long("help") => {
                is_version = false;
                is_help = true;
            }
            _ => return Err(format!("Unknown option '{}'.", arg)),
        }
    }

    if is_help {
        return Ok(Cmd::Help);
    }

    if is_version {
        return Ok(Cmd::Version);
    }

    let target = match target {
        None => return Err("No target specified.".into()),
        Some(t) => t,
    };

    if target == "help" {
        return Ok(Cmd::TargetHelp);
    }

    if fnames.is_empty() {
        return Err("No input files specified.".into());
    }

    Ok(Cmd::Generate { target, fnames })
}

/// Print usage/help info, for `--help`.
pub fn print_usage() {
    println!("{}", USAGE.trim());
}

/// Print version info, for `--version`.
pub fn print_version() {
    use crate::version::{VERSION, REV};
    print!("Squiller {}, ", VERSION);
    match REV {
        Some(rev) => println!("built from commit {}", rev),
        None => println!("built from an unspecified checkout"),
    }
}

#[cfg(test)]
mod test {
    use super::{parse, Cmd};

    fn parse_slice(args: &[&'static str]) -> Result<Cmd, String> {
        let argv = args.iter().map(|&s| s.into()).collect();
        parse(argv)
    }

    #[test]
    fn parse_parses_help() {
        let expected = Ok(Cmd::Help);
        assert_eq!(parse_slice(&["squiller", "-h"]), expected);
        assert_eq!(parse_slice(&["squiller", "--help"]), expected);
        assert_eq!(parse_slice(&["squiller", "--version", "--help"]), expected);
    }

    #[test]
    fn parse_parses_version() {
        let expected = Ok(Cmd::Version);
        assert_eq!(parse_slice(&["squiller", "--version"]), expected);
        assert_eq!(parse_slice(&["squiller", "--help", "--version"]), expected);
    }

    #[test]
    fn parse_parses_target_help() {
        let expected = Ok(Cmd::TargetHelp);
        assert_eq!(parse_slice(&["squiller", "--target=help"]), expected);
        assert_eq!(parse_slice(&["squiller", "--target", "help"]), expected);
        assert_eq!(parse_slice(&["squiller", "-t", "help"]), expected);
        assert_eq!(parse_slice(&["squiller", "-thelp"]), expected);
    }

    #[test]
    fn parse_parses_generate() {
        let expected = Ok(Cmd::Generate {
            target: "foo".into(),
            fnames: vec!["bar".into(), "baz".into()],
        });
        assert_eq!(parse_slice(&["squiller", "-tfoo", "bar", "baz"]), expected);
        assert_eq!(
            parse_slice(&["squiller", "--target=foo", "bar", "baz"]),
            expected,
        );
        assert_eq!(
            parse_slice(&["squiller", "bar", "baz", "-t", "foo"]),
            expected,
        );
        assert_eq!(
            parse_slice(&["squiller", "--target=foo", "bar", "--", "baz"]),
            expected,
        );
    }

    #[test]
    fn parse_handles_raw_args() {
        let expected = Ok(Cmd::Generate {
            target: "foo".into(),
            fnames: vec!["--bar".into(), "--".into(), "-t".into()],
        });
        assert_eq!(
            parse_slice(&["squiller", "-tfoo", "--", "--bar", "--", "-t"]),
            expected,
        );
    }

    #[test]
    fn parse_handles_dash_as_arg() {
        let expected = Ok(Cmd::Generate {
            target: "foo".into(),
            fnames: vec!["-".into()],
        });
        assert_eq!(
            parse_slice(&["squiller", "-tfoo", "-"]),
            expected,
        );
    }

    #[test]
    fn parse_returns_error_on_misuse() {
        assert_eq!(
            parse_slice(&["squiller", "--frobnicate"]),
            Err("Unknown option '--frobnicate'.".into()),
        );
        assert_eq!(
            parse_slice(&["squiller", "--target"]),
            Err("Expected target name after '--target'.".into()),
        );
        assert_eq!(
            parse_slice(&["squiller", "--target", "-h"]),
            Err("Expected target name after '--target'.".into()),
        );
        assert_eq!(
            parse_slice(&["squiller", "-t"]),
            Err("Expected target name after '-t'.".into()),
        );
        assert_eq!(
            parse_slice(&["squiller"]),
            Err("No target specified.".into()),
        );
        assert_eq!(
            parse_slice(&["squiller", "--target=foo"]),
            Err("No input files specified.".into()),
        );
    }
}
