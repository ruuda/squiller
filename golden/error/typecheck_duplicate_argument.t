-- @query q(dup: i32, dup: i32)
SELECT 1;


thread 'main' panicked at 'TODO: Report duplicate arg error.', src/typecheck.rs:165:21
stack backtrace:
   0: std::panicking::begin_panic
             at /rustc/f1edd0429582dd29cccacaf50fd134b05593bd9c/library/std/src/panicking.rs:543:12
   1: querybinder::typecheck::QueryChecker::populate_query_args
             at ./src/typecheck.rs:165:21
   2: querybinder::typecheck::QueryChecker::resolve_types
             at ./src/typecheck.rs:142:9
   3: querybinder::typecheck::check_document
             at ./src/typecheck.rs:239:46
   4: querybinder::process_input
             at ./src/main.rs:71:15
   5: querybinder::main
             at ./src/main.rs:104:22
   6: core::ops::function::FnOnce::call_once
             at /rustc/f1edd0429582dd29cccacaf50fd134b05593bd9c/library/core/src/ops/function.rs:227:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
