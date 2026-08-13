// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end verification of the bundled engine-neutral suite.

use std::process::Command;

#[test]
fn sample_engine_matches_the_builtin_black_box_suite() {
    let status = Command::new(env!("CARGO_BIN_EXE_kumihan-conformance"))
        .args(["run", env!("CARGO_BIN_EXE_kumihan-sample-engine")])
        .status()
        .expect("the conformance runner starts the sample engine");
    assert!(status.success(), "built-in suite differed: {status}");
}
