// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end verification of the bundled engine-neutral suite.

use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

fn temporary_suite(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "kumihan-conformance-{label}-{}.ndjson",
        std::process::id()
    ))
}

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

#[test]
fn runner_returns_one_when_an_engine_differs() {
    let mut case: serde_json::Value = serde_json::from_str(
        include_str!("../suite.ndjson")
            .lines()
            .next()
            .expect("the built-in suite has a first case"),
    )
    .expect("the first built-in case is JSON");
    case["expected"]["lines"][0]["inline_extent"] = serde_json::Value::from(3999);

    let path = temporary_suite("mismatch");
    fs::write(&path, format!("{case}\n")).expect("the mismatching suite is written");
    let output = Command::new(env!("CARGO_BIN_EXE_kumihan-conformance"))
        .args([
            "run",
            env!("CARGO_BIN_EXE_kumihan-sample-engine"),
            path.to_str().expect("the temporary path is UTF-8"),
        ])
        .output()
        .expect("the conformance runner starts");
    let _ = fs::remove_file(path);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("1 conformance case(s) differed"),
        "stderr reports the mismatch count"
    );
}

#[test]
fn validator_returns_two_for_a_protocol_error() {
    let path = temporary_suite("invalid");
    fs::write(&path, "{}\n").expect("the invalid suite is written");
    let output = Command::new(env!("CARGO_BIN_EXE_kumihan-conformance"))
        .args([
            "validate",
            path.to_str().expect("the temporary path is UTF-8"),
        ])
        .output()
        .expect("the conformance validator starts");
    let _ = fs::remove_file(path);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("protocol is required and must be a string"),
        "stderr reports the protocol error"
    );
}
