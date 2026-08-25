// SPDX-FileCopyrightText: 2026 jlreq contributors
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
        "jlreq-conformance-{label}-{}.ndjson",
        std::process::id()
    ))
}

#[test]
fn sample_engine_matches_the_builtin_black_box_suite() {
    let status = Command::new(env!("CARGO_BIN_EXE_jlreq-conformance"))
        .args(["run", env!("CARGO_BIN_EXE_jlreq-sample-engine")])
        .status()
        .expect("the conformance runner starts the sample engine");
    assert!(status.success(), "built-in suite differed: {status}");
}

#[test]
fn builtin_suite_publishes_the_single_character_flush_ruby_reading() {
    let case = include_str!("../suite.ndjson")
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("suite case is JSON"))
        .find(|case| case["id"] == "3.3.6/group-ruby-single-character-flush-start-aligned")
        .expect("the published single-character flush reading has a black-box case");

    assert_eq!(case["expected"]["lines"][0]["attachments"][0]["inline"], 0);
}

#[test]
fn sample_engine_reports_builder_input_errors_with_exit_two() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_jlreq-sample-engine"))
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the sample engine starts");
    let request = r#"{"protocol":"jlreq.conformance/1","spec":"jlreq-2020-08-11+unicode-17.0.0","id":"missing-tab-stop","request":{"source":"A\tB","size":{"inline":1000,"block":1000},"frame":"proportional","clusters":[{"range":[0,1],"advance":500},{"range":[1,2],"advance":500},{"range":[2,3],"advance":500}],"line_extent":5000}}"#;
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
fn sample_engine_rejects_an_oversize_input_line() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_jlreq-sample-engine"))
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the sample engine starts");
    let mut input = child.stdin.take().expect("engine stdin is piped");
    let oversized = vec![b' '; 1024 * 1024 + 1];
    let _ = input.write_all(&oversized);
    drop(input);
    let output = child.wait_with_output().expect("engine exits");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("input message exceeds the 1048576 byte line limit")
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
    let output = Command::new(env!("CARGO_BIN_EXE_jlreq-conformance"))
        .args([
            "run",
            env!("CARGO_BIN_EXE_jlreq-sample-engine"),
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
    let output = Command::new(env!("CARGO_BIN_EXE_jlreq-conformance"))
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

#[test]
fn cli_help_version_and_stdin_contract_are_executable() {
    let help = Command::new(env!("CARGO_BIN_EXE_jlreq-conformance"))
        .arg("--help")
        .output()
        .expect("help command starts");
    assert_eq!(help.status.code(), Some(0));
    let help = String::from_utf8_lossy(&help.stdout);
    for option in [
        "--verbose",
        "--timeout-seconds",
        "--max-message-bytes",
        "--max-suite-bytes",
        "--max-cases",
    ] {
        assert!(help.contains(option), "help documents {option}");
    }

    let version = Command::new(env!("CARGO_BIN_EXE_jlreq-conformance"))
        .arg("--version")
        .output()
        .expect("version command starts");
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        "jlreq-conformance 0.1.0"
    );

    let mut request: serde_json::Value = serde_json::from_str(
        include_str!("../suite.ndjson")
            .lines()
            .next()
            .expect("built-in suite has a case"),
    )
    .expect("built-in case JSON");
    let object = request.as_object_mut().expect("case object");
    object.remove("rules");
    object.remove("expected");

    let mut child = Command::new(env!("CARGO_BIN_EXE_jlreq-conformance"))
        .arg("validate")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("stdin validator starts");
    child
        .stdin
        .take()
        .expect("validator stdin")
        .write_all(format!("{request}\n").as_bytes())
        .expect("wire request is written");
    let output = child.wait_with_output().expect("stdin validator exits");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("validated 1 message(s)"));
}
