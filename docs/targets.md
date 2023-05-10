# Targets

Squiller can generate code for the following targets.

## python-psycopg2

_Vaporware warning: Development of this target is in progress._

Target Python and Postgres through the [Psycopg2](https://www.psycopg.org/)
package. Generated code includes type annotations. This target is tested against
the following versions, although other versions may work:

 * Python 3.10.5
 * Psycopg2 2.9.3

## python-sqlite

_Vaporware warning: Development of this target is in progress._

Target Python and SQLite through the [sqlite3 module][py-sqlite3] in the
standard library. Generated code includes type annotations. This target is
tested against the following version, although other versions may work:

[py-sqlite3]: https://docs.python.org/3/library/sqlite3.html

 * Python 3.10.5

## rust-sqlite

Target Rust and SQLite through the [sqlite](https://lib.rs/crates/sqlite) crate.
This target is tested against the following versions, although other versions
may work:

 * Rust 1.57.0, 2018 edition
 * Sqlite crate 0.26.0
