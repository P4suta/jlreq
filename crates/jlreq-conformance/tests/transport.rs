// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cross-platform subprocess transport regressions.

#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used
)]

use std::{
    env, fs,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    process::{Command, Output},
    time::{Duration, Instant},
};

use serde_json::{Value, json};

const MODE_ENV: &str = "JLREQ_SYNTHETIC_ENGINE_MODE";
const COUNT_ENV: &str = "JLREQ_SYNTHETIC_ENGINE_COUNT";
const PROTOCOL: &str = "jlreq.conformance/1";
const SPEC: &str = "jlreq-2020-08-11+unicode-17.0.0";

fn main() {
    if let Ok(mode) = env::var(MODE_ENV) {
        synthetic_engine(&mode);
        return;
    }

    transport_regressions();
}

fn transport_regressions() {
    let pair = suite(2, None);
    assert_success("unordered", &pair, &[]);
    assert_success("normal-limit", &suite(1, None), &[]);

    let large_id = "i".repeat(70 * 1024);
    assert_success("normal", &suite(1, Some(&large_id)), &[]);

    let many = suite(2_000, None);
    assert_success("write-before-read", &many, &[(COUNT_ENV, "2000")]);
    assert_success("stderr-flood", &pair, &[]);

    assert_protocol_error("duplicate", &pair, "duplicate response id", &[]);
    assert_protocol_error("unknown", &pair, "unknown response id", &[]);
    assert_protocol_error("missing", &pair, "omitted 1 response(s): case-000001", &[]);
    assert_protocol_error(
        "extra-response",
        &suite(1, None),
        "engine stdout exceeds the 1 response limit",
        &[],
    );
    assert_protocol_error(
        "huge-line",
        &pair,
        "engine stdout message exceeds the 1048576 byte line limit",
        &[],
    );
    assert_protocol_error("midway-stop", &many, "engine", &[]);

    let started = Instant::now();
    assert_protocol_error(
        "stall",
        &pair,
        "made no stdin/stdout progress for 1 second",
        &[],
    );
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "the watchdog must kill and join the stalled child promptly"
    );

    let flood = run("stderr-flood-fail", &pair, &[]);
    assert_eq!(flood.status.code(), Some(2), "{}", stderr(&flood));
    assert!(stderr(&flood).contains("[discarded "));
    assert!(
        flood.stderr.len() <= 1024 * 1024 + 4_096,
        "stderr retention must remain bounded"
    );
}

fn assert_success(mode: &str, contents: &str, extra_env: &[(&str, &str)]) {
    let output = run(mode, contents, extra_env);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

fn assert_protocol_error(mode: &str, contents: &str, expected: &str, extra_env: &[(&str, &str)]) {
    let output = run(mode, contents, extra_env);
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(
        stderr(&output).contains(expected),
        "expected {expected:?} in {:?}",
        stderr(&output)
    );
}

fn run(mode: &str, contents: &str, extra_env: &[(&str, &str)]) -> Output {
    let path = temporary_suite(mode);
    fs::write(&path, contents).expect("synthetic suite is written");
    let executable = env::current_exe().expect("transport test executable has a path");
    let mut command = Command::new(env!("CARGO_BIN_EXE_jlreq-conformance"));
    command.args(["--timeout-seconds", "1"]).env(MODE_ENV, mode);
    if matches!(mode, "extra-response" | "normal-limit") {
        command.args(["--max-cases", "1"]);
    }
    command.args([
        "run",
        executable.to_str().expect("test executable path is UTF-8"),
        path.to_str().expect("suite path is UTF-8"),
    ]);
    for (name, value) in extra_env {
        command.env(name, value);
    }
    let output = command.output().expect("conformance runner starts");
    fs::remove_file(path).expect("temporary suite is removed");
    output
}

fn temporary_suite(label: &str) -> PathBuf {
    env::temp_dir().join(format!(
        "jlreq-transport-{label}-{}.ndjson",
        std::process::id()
    ))
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn suite(count: usize, first_id: Option<&str>) -> String {
    let mut suite = String::new();
    for index in 0..count {
        let id = first_id
            .filter(|_| index == 0)
            .map_or_else(|| format!("case-{index:06}"), str::to_owned);
        let case = json!({
            "protocol": PROTOCOL,
            "spec": SPEC,
            "id": id,
            "rules": ["synthetic-transport"],
            "request": {
                "source": "A",
                "size": {"inline": 1000, "block": 1000},
                "frame": "proportional",
                "clusters": [{"range": [0, 1], "advance": 500}],
                "line_extent": 1000
            },
            "expected": empty_layout()
        });
        suite.push_str(&case.to_string());
        suite.push('\n');
    }
    suite
}

fn synthetic_engine(mode: &str) {
    match mode {
        "write-before-read" => {
            let count = env::var(COUNT_ENV)
                .expect("write-first count is provided")
                .parse::<usize>()
                .expect("write-first count is numeric");
            let stdout = io::stdout();
            let mut output = BufWriter::new(stdout.lock());
            for index in 0..count {
                write_response(&mut output, &format!("case-{index:06}"));
            }
            output.flush().expect("responses are flushed");
            io::copy(&mut io::stdin().lock(), &mut io::sink()).expect("requests are drained");
        },
        "stall" => std::thread::sleep(Duration::from_secs(30)),
        "huge-line" => {
            let mut input = BufReader::new(io::stdin().lock());
            let mut ignored = String::new();
            input.read_line(&mut ignored).expect("one request is read");
            io::stdout()
                .lock()
                .write_all(&vec![b'x'; 1024 * 1024 + 1])
                .expect("oversize output is written");
        },
        "stderr-flood" | "stderr-flood-fail" => {
            io::stderr()
                .lock()
                .write_all(&vec![b'x'; 2 * 1024 * 1024])
                .expect("stderr flood is written");
            if mode == "stderr-flood-fail" {
                std::process::exit(9);
            }
            respond_to_input(false, false, false);
        },
        "midway-stop" => {
            let mut line = String::new();
            io::stdin()
                .lock()
                .read_line(&mut line)
                .expect("one request is read");
            std::process::exit(9);
        },
        "unordered" => respond_to_input(true, false, false),
        "duplicate" => respond_to_input(false, true, false),
        "missing" => respond_to_input(false, false, true),
        "unknown" => {
            let mut line = String::new();
            io::stdin()
                .lock()
                .read_line(&mut line)
                .expect("one request is read");
            write_response(&mut io::stdout().lock(), "unknown-case");
        },
        "extra-response" => {
            let mut line = String::new();
            io::stdin()
                .lock()
                .read_line(&mut line)
                .expect("one request is read");
            let mut output = io::stdout().lock();
            write_response(&mut output, "case-000000");
            write_response(&mut output, "unknown-case");
        },
        "normal" | "normal-limit" => respond_to_input(false, false, false),
        other => panic!("unknown synthetic engine mode {other:?}"),
    }
}

fn respond_to_input(reverse: bool, duplicate_first: bool, omit_last: bool) {
    let input = BufReader::new(io::stdin().lock());
    let mut ids = input
        .lines()
        .map(|line| {
            let value: Value = serde_json::from_str(&line.expect("request is read"))
                .expect("request is valid JSON");
            value["id"].as_str().expect("request has id").to_owned()
        })
        .collect::<Vec<_>>();
    if reverse {
        ids.reverse();
    }
    if omit_last {
        ids.pop();
    }
    let stdout = io::stdout();
    let mut output = stdout.lock();
    for id in &ids {
        write_response(&mut output, id);
    }
    if duplicate_first {
        write_response(&mut output, ids.first().expect("suite is non-empty"));
    }
}

fn write_response(output: &mut dyn Write, id: &str) {
    serde_json::to_writer(
        &mut *output,
        &json!({
            "protocol": PROTOCOL,
            "spec": SPEC,
            "id": id,
            "response": empty_layout()
        }),
    )
    .expect("response is encoded");
    output.write_all(b"\n").expect("response is written");
    output.flush().expect("response is flushed");
}

fn empty_layout() -> Value {
    json!({"lines": [], "diagnostics": []})
}
