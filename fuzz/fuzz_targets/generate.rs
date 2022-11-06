// Squiller -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;
use squiller::target::TARGETS;

fuzz_target!(|input_bytes: &[u8]| {
    // The last byte of the input indicates the target we want to fuzz. This
    // ensures that we can mostly share the corpus between this fuzzer and the
    // `typecheck` and `parse` corpora.
    let (input_bytes, target_index) = match input_bytes.last() {
        Some(k) => (&input_bytes[..input_bytes.len() - 1], *k as usize),
        None => return,
    };

    // Pick a target that we want to fuzz this iteration. The target at index 0
    // is the help target, that one we can't generate code for.
    let target = match TARGETS.get(1 + target_index) {
        Some(target) => target,
        None => return,
    };

    let fname: PathBuf = "fuzz".into();
    let doc = match squiller::NamedDocument::process_input(&fname, input_bytes) {
        Ok(doc) => doc,
        Err(_) => return,
    };

    let mut out = Vec::new();
    let _ = (target.handler)(&mut out, &[doc]);
});
