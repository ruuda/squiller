# Changelog

## 0.4.0

Released 2023-09-10.

 * The `rust-sqlite` target now includes annotations to silence some compiler
   and Clippy warnings.
 * Experimental and incomplete support for the new `python-sqlite` target.
   This target is a work in progress, and not usable at this time.

## 0.3.0

Released 2023-03-18.

 * Print errors to stderr instead of stdout. This should make Squiller easier to
   integrate into build pipelines.
 * Experimental and incomplete support for the new `python-psycopg2` target.
   This target is a work in progress.
 * Fix a crash found through fuzzing.

## 0.2.0

Released 2022-10-09.

 * Add support for multi-statement queries, demarcated by `@begin` and `@end`.

## 0.1.0

Released 2022-09-20.

 * Initial release, the application should still be considered beta quality.
 * Rudimentary support for the `rust-sqlite` target.
 * Some types listed in the documentation are not implemented.
