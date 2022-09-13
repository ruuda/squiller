# Squiller

Generate boilerplate code from annotated SQL queries.

## Synopsis

    squiller --target <target> <file>...
    squiller --target help
    squiller --help

## Description

Squiller parses all of the given inputs files, generates code from them for the
specified target, and writes that to stdout. `<file>...` can be one or more
<abbr>UTF-8</abbr> text files that contain <abbr>SQL</abbr>, or `-` to read
from stdin.


## Options

### `--target`

Specifies the target language and database driver to generate code for. Targets
follow the `<language>-<driver>` naming scheme, all lowercase. The special value
`help` lists all supported targets. In that case, no input files need to be
specified.

### `--help`

Print usage information.

### `--version`

Print version information.
