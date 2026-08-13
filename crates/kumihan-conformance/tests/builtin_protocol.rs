// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end verification of the bundled engine-neutral suite.

use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn sample_engine_matches_the_builtin_black_box_suite() {
    let status = Command::new(env!("CARGO_BIN_EXE_kumihan-conformance"))
        .args(["run", env!("CARGO_BIN_EXE_kumihan-sample-engine")])
        .status()
        .expect("the conformance runner starts the sample engine");
    assert!(status.success(), "built-in suite differed: {status}");
}

#[test]
fn sample_engine_reports_builder_input_errors_with_exit_two() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kumihan-sample-engine"))
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the sample engine starts");
    let request = r#"{"protocol":"kumihan.conformance/1","spec":"jlreq-2020-08-11+unicode-17.0.0","id":"missing-tab-stop","request":{"source":"A\tB","size":{"inline":1000,"block":1000},"frame":"proportional","clusters":[{"range":[0,1],"advance":500},{"range":[1,2],"advance":500},{"range":[2,3],"advance":500}],"line_extent":5000}}"#;
    child
        .stdin
        .take()
        .expect("engine stdin is piped")
        .write_all(format!("{request}\n").as_bytes())
        .expect("request is written");
    let output = child.wait_with_output().expect("engine exits");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("input.insufficient-tab-stops"),
        "stderr carries the stable InputError code"
    );
}
