// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use std::hint::black_box;

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

#[path = "../../crates/jlreq-conformance/src/validation.rs"]
mod validation;

const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
const MAX_CASES: usize = 200_000;

fuzz_target!(|data: &[u8]| {
    for line in data.split(|byte| *byte == b'\n').take(MAX_CASES + 1) {
        if line.is_empty() || line.len() > MAX_MESSAGE_BYTES {
            continue;
        }
        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        if value.get("request").is_some() {
            let _ = black_box(validation::validate_request(&value));
        }
        if value.get("response").is_some() {
            let _ = black_box(validation::validate_response(&value));
        }
    }
});
