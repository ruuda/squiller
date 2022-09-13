// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

fuzz_target!(|input_bytes: &[u8]| {
    // Processing may result in an error, but it should not hang or panic.
    let fname: PathBuf = "fuzz".into();
    let _ = squiller::NamedDocument::process_input(&fname, input_bytes);
});
