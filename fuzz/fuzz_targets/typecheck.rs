#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

fuzz_target!(|input_bytes: &[u8]| {
    // Processing may result in an error, but it should not hang or panic.
    let fname: PathBuf = "fuzz".into();
    let _ = squiller::NamedDocument::process_input(&fname, input_bytes);
});
