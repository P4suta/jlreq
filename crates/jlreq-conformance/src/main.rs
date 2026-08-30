// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Versioned NDJSON black-box conformance runner.

mod transport;
mod validation;

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{self, BufRead, BufReader, Cursor, Read, Write},
    path::Path,
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitCode, Stdio},
    sync::{
        Arc, Mutex,
        mpsc::{self, RecvTimeoutError, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde_json::{Map, Value};
use transport::read_limited_line;
use validation::{validate_request, validate_response};

const PROTOCOL: &str = "jlreq.conformance/1";
const SPEC: &str = jlreq_core::SPECIFICATION;
const BUILTIN_SUITE: &str = include_str!("../suite.ndjson");
#[cfg(test)]
const PROTOCOL_SCHEMA: &str = include_str!("../protocol.schema.json");

const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_MAX_MESSAGE_BYTES: usize = 1024 * 1024;
const DEFAULT_MAX_SUITE_BYTES: usize = 256 * 1024 * 1024;
const DEFAULT_MAX_CASES: usize = 200_000;
const STDERR_RETAIN_BYTES: usize = 1024 * 1024;
const EVENT_CHANNEL_CAPACITY: usize = 64;
const WATCHDOG_TICK: Duration = Duration::from_millis(100);
// The runner stages share one private module contract while keeping parsing, process control,
// and comparison independently maintainable.
include!("runner/cli.rs");
include!("runner/suite.rs");
include!("runner/process.rs");
include!("runner/compare.rs");
#[cfg(test)]
mod tests {
    use super::{
        BUILTIN_SUITE, CliCommand, Invocation, Options, PROTOCOL_SCHEMA, STDERR_RETAIN_BYTES,
        StderrCapture, activity_elapsed, bounded, engine_work_pending, first_difference,
        join_worker, mark_activity, parse_cases, parse_invocation, parse_messages, positive_u64,
        positive_usize, read_limited_line, required_string, stop_unstarted_child,
        validate_envelope, validate_rules, with_stderr, workers_disconnected_early,
    };
    use serde_json::json;
    use std::{
        io::Cursor,
        process::{Command, Stdio},
        sync::Mutex,
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn builtin_suite_is_a_valid_versioned_case_stream() {
        let cases = parse_cases(BUILTIN_SUITE).expect("built-in suite");
        assert!(!cases.is_empty());
    }

    #[test]
    fn suite_rule_metadata_is_required_and_never_sent_on_the_wire() {
        let case: serde_json::Value = BUILTIN_SUITE
            .lines()
            .map(|line| serde_json::from_str(line).expect("case JSON"))
            .find(|case: &serde_json::Value| case["id"] == "quick-start/two-lines")
            .expect("quick-start case");
        assert_eq!(case["rules"], json!(["3.1.10"]));

        let mut missing = case.clone();
        missing
            .as_object_mut()
            .expect("case object")
            .remove("rules");
        assert!(parse_messages(&missing.to_string(), true).is_err());

        let mut wire_request = case;
        wire_request
            .as_object_mut()
            .expect("case object")
            .remove("expected");
        assert!(parse_messages(&wire_request.to_string(), false).is_err());
    }

    #[test]
    fn wrong_protocol_is_an_input_error() {
        let message =
            r#"{"protocol":"old","spec":"jlreq-2020-08-11+unicode-17.0.0","id":"x","request":{}}"#;
        assert!(parse_messages(message, false).is_err());
    }

    #[test]
    fn request_body_is_validated_instead_of_treated_as_an_opaque_object() {
        let message = json!({
            "protocol": "jlreq.conformance/1",
            "spec": "jlreq-2020-08-11+unicode-17.0.0",
            "id": "missing-input",
            "request": {}
        });
        assert!(parse_messages(&message.to_string(), false).is_err());
    }

    #[test]
    fn response_body_is_validated_instead_of_treated_as_an_opaque_object() {
        let message = json!({
            "protocol": "jlreq.conformance/1",
            "spec": "jlreq-2020-08-11+unicode-17.0.0",
            "id": "bad-output",
            "response": {"lines": "not-an-array", "diagnostics": []}
        });
        assert!(parse_messages(&message.to_string(), false).is_err());
    }

    #[test]
    fn all_style_settings_belong_to_the_closed_typed_vocabulary() {
        let mut case: serde_json::Value =
            serde_json::from_str(BUILTIN_SUITE.lines().next().expect("built-in case"))
                .expect("built-in case JSON");
        case["request"]["style"] = json!({"made.up.setting": "anything"});
        assert!(parse_messages(&case.to_string(), true).is_err());
    }

    #[test]
    fn committed_schema_describes_bodies_and_all_twenty_two_style_settings() {
        let schema: serde_json::Value =
            serde_json::from_str(PROTOCOL_SCHEMA).expect("protocol schema JSON");
        assert!(schema["$defs"]["request"].is_object());
        assert!(schema["$defs"]["response"].is_object());
        assert!(schema["properties"]["rules"].is_object());
        assert_eq!(
            schema["$defs"]["styleSettings"]["properties"]
                .as_object()
                .expect("style properties")
                .len(),
            23,
            "profile plus 22 typed settings"
        );
    }

    #[test]
    fn repeated_symbol_attachment_has_no_shaped_range() {
        let response = json!({
            "protocol": "jlreq.conformance/1",
            "spec": "jlreq-2020-08-11+unicode-17.0.0",
            "id": "emphasis",
            "response": {
                "lines": [{
                    "range": [0, 3],
                    "inline_origin": 0,
                    "block_origin": 0,
                    "inline_extent": 1000,
                    "block_extent": 1000,
                    "clusters": [],
                    "attachments": [{
                        "construct": 0,
                        "range": [0, 0],
                        "inline": 0,
                        "block": -1000,
                        "advance": 0,
                        "size": {"inline": 1000, "block": 1000},
                        "writing_mode": "horizontal-tb",
                        "transform": "identity",
                        "symbol": "・"
                    }]
                }],
                "diagnostics": []
            }
        });
        assert!(parse_messages(&response.to_string(), false).is_ok());
    }

    #[test]
    fn envelope_vocabulary_and_message_roles_are_closed() {
        let mut unknown: serde_json::Value =
            serde_json::from_str(BUILTIN_SUITE.lines().next().expect("built-in case"))
                .expect("built-in case JSON");
        unknown["extension"] = json!(true);
        assert!(parse_messages(&unknown.to_string(), false).is_err());

        let mixed_output = json!({
            "protocol": "jlreq.conformance/1",
            "spec": "jlreq-2020-08-11+unicode-17.0.0",
            "id": "mixed",
            "response": {"lines": [], "diagnostics": []},
            "expected": {"lines": [], "diagnostics": []}
        });
        assert!(parse_messages(&mixed_output.to_string(), false).is_err());
    }

    #[test]
    fn duplicate_suite_ids_are_rejected() {
        let case = BUILTIN_SUITE.lines().next().expect("built-in case");
        assert!(parse_cases(&format!("{case}\n{case}\n")).is_err());
    }

    #[test]
    fn cli_options_have_documented_defaults_and_accept_equals_syntax() {
        let invocation = parse_invocation([
            "--verbose".to_owned(),
            "--timeout-seconds=7".to_owned(),
            "run".to_owned(),
            "engine".to_owned(),
        ])
        .expect("valid invocation");
        let Invocation::Command(options, _) = invocation else {
            panic!("expected command");
        };
        assert!(options.verbose);
        assert_eq!(options.timeout.as_secs(), 7);
        let defaults = Options::default();
        assert!(!defaults.verbose);
        assert_eq!(defaults.timeout, Duration::from_secs(30));
        assert_eq!(defaults.max_message_bytes, 1_048_576);
        assert_eq!(defaults.max_suite_bytes, 268_435_456);
        assert_eq!(defaults.max_cases, 200_000);
        assert_eq!(STDERR_RETAIN_BYTES, 1_048_576);
    }

    #[test]
    fn cli_parses_every_limit_and_each_command_shape() {
        let invocation = parse_invocation([
            "--max-message-bytes".to_owned(),
            "11".to_owned(),
            "--max-suite-bytes=22".to_owned(),
            "--max-cases".to_owned(),
            "33".to_owned(),
            "list".to_owned(),
            "suite.ndjson".to_owned(),
        ])
        .expect("all limits are accepted");
        let Invocation::Command(options, CliCommand::List { suite }) = invocation else {
            panic!("expected list command");
        };
        assert_eq!(options.max_message_bytes, 11);
        assert_eq!(options.max_suite_bytes, 22);
        assert_eq!(options.max_cases, 33);
        assert_eq!(suite.as_deref(), Some("suite.ndjson"));

        let Invocation::Command(_, CliCommand::Validate { suite }) =
            parse_invocation(["validate".to_owned(), "-".to_owned()]).expect("validate command")
        else {
            panic!("expected validate command");
        };
        assert_eq!(suite.as_deref(), Some("-"));

        let Invocation::Command(_, CliCommand::Run { engine, suite }) = parse_invocation([
            "run".to_owned(),
            "engine".to_owned(),
            "suite.ndjson".to_owned(),
        ])
        .expect("run command") else {
            panic!("expected run command");
        };
        assert_eq!(engine, "engine");
        assert_eq!(suite.as_deref(), Some("suite.ndjson"));
    }

    #[test]
    fn cli_rejects_zero_limits_and_wrong_positional_counts() {
        for option in [
            "--timeout-seconds",
            "--max-message-bytes",
            "--max-suite-bytes",
            "--max-cases",
        ] {
            assert!(
                parse_invocation([option.to_owned(), "0".to_owned(), "list".to_owned()]).is_err(),
                "{option}"
            );
        }
        assert_eq!(positive_u64("9", "option"), Ok(9));
        assert!(positive_u64("0", "option").is_err());
        assert_eq!(positive_usize("9", "option"), Ok(9));
        assert!(positive_usize("0", "option").is_err());

        for arguments in [
            vec!["validate", "one", "two"],
            vec!["list", "one", "two"],
            vec!["run"],
            vec!["run", "engine", "suite", "extra"],
        ] {
            assert!(
                parse_invocation(arguments.into_iter().map(str::to_owned)).is_err(),
                "invalid positional shape"
            );
        }
    }

    #[test]
    fn line_and_total_limits_are_inclusive() {
        let mut total = 0;
        let mut line = Vec::new();
        let mut reader = Cursor::new(b"abc\nx\n".as_slice());
        assert_eq!(
            read_limited_line(&mut reader, &mut line, 3, 4, &mut total, "test"),
            Ok(true)
        );
        assert_eq!(line, b"abc");
        assert_eq!(total, 4);
        let retained_capacity = line.capacity();
        assert_eq!(
            read_limited_line(&mut reader, &mut line, 3, 6, &mut total, "test"),
            Ok(true)
        );
        assert_eq!(line, b"x");
        assert_eq!(line.capacity(), retained_capacity);
        assert_eq!(total, 6);

        let mut total = 0;
        let mut line = Vec::new();
        let mut reader = Cursor::new(b"abc\n".as_slice());
        assert!(read_limited_line(&mut reader, &mut line, 2, 4, &mut total, "test").is_err());

        let mut total = 0;
        let mut line = Vec::new();
        let mut reader = Cursor::new(b"abc\n".as_slice());
        assert!(read_limited_line(&mut reader, &mut line, 3, 3, &mut total, "test").is_err());
    }

    #[test]
    fn suite_case_shape_and_rules_fail_at_the_envelope_boundary() {
        let mut case: serde_json::Value = serde_json::from_str(
            BUILTIN_SUITE
                .lines()
                .next()
                .expect("built-in suite has a case"),
        )
        .expect("built-in case JSON");
        case.as_object_mut().expect("case object").remove("request");
        assert_eq!(
            validate_envelope(&case, true),
            Err("expected is valid only beside a suite request".to_owned())
        );

        let mut case: serde_json::Value = serde_json::from_str(
            BUILTIN_SUITE
                .lines()
                .next()
                .expect("built-in suite has a case"),
        )
        .expect("built-in case JSON");
        case["expected"] = json!(false);
        assert_eq!(
            validate_envelope(&case, true),
            Err("a suite case needs object-valued request and expected fields".to_owned())
        );

        assert!(validate_rules(&json!([])).is_err());
        assert!(validate_rules(&json!(["3.1", "3.1"])).is_err());
        let object = json!({"protocol": "wrong"});
        assert!(
            required_string(
                object.as_object().expect("envelope object"),
                "protocol",
                "right"
            )
            .is_err()
        );
    }

    #[test]
    fn watchdog_helpers_observe_activity_and_worker_failure() {
        let old = Instant::now()
            .checked_sub(Duration::from_secs(5))
            .expect("five seconds before now is representable");
        let activity = Mutex::new(old);
        assert!(
            activity_elapsed(&activity).expect("unpoisoned activity") >= Duration::from_secs(4)
        );
        mark_activity(&activity);
        assert!(activity_elapsed(&activity).expect("unpoisoned activity") < Duration::from_secs(1));

        let worker = thread::spawn(|| panic!("intentional worker failure"));
        assert_eq!(
            join_worker(worker, "reader"),
            Err("engine reader panicked".to_owned())
        );
    }

    #[test]
    fn engine_completion_predicates_cover_every_partial_state() {
        assert!(engine_work_pending(false, (false, false, false)));
        assert!(engine_work_pending(false, (true, false, true)));
        assert!(engine_work_pending(false, (false, true, true)));
        assert!(engine_work_pending(false, (true, true, false)));
        assert!(!engine_work_pending(false, (true, true, true)));
        assert!(!engine_work_pending(true, (false, false, false)));

        assert!(workers_disconnected_early(false, false));
        assert!(workers_disconnected_early(true, false));
        assert!(workers_disconnected_early(false, true));
        assert!(!workers_disconnected_early(true, true));
    }

    #[test]
    #[ignore = "helper subprocess for child-termination test"]
    fn ignored_child_waits_until_killed() {
        thread::sleep(Duration::from_secs(30));
    }

    #[test]
    fn stopping_an_unstarted_pipeline_kills_and_waits_for_the_child() {
        let mut child = Command::new(std::env::current_exe().expect("test executable path"))
            .args([
                "--ignored",
                "--exact",
                "tests::ignored_child_waits_until_killed",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("helper child starts");
        assert!(
            child.try_wait().expect("helper status").is_none(),
            "helper child must still be running"
        );

        let message = stop_unstarted_child(&mut child, "pipeline did not start");
        let stopped = child.try_wait().expect("stopped child status").is_some();
        if !stopped {
            let _ = child.kill();
            let _ = child.wait();
        }
        assert_eq!(message, "pipeline did not start");
        assert!(stopped, "stop_child must reap the helper process");
    }

    #[test]
    fn stderr_and_json_rendering_are_bounded_at_exact_edges() {
        let empty = StderrCapture {
            retained: Vec::new(),
            discarded: 0,
        };
        assert_eq!(with_stderr("failure".to_owned(), Some(&empty)), "failure");

        let retained = StderrCapture {
            retained: b"details\n".to_vec(),
            discarded: 0,
        };
        assert_eq!(
            with_stderr("failure".to_owned(), Some(&retained)),
            "failure; engine stderr: details"
        );
        let discarded = StderrCapture {
            retained: Vec::new(),
            discarded: 7,
        };
        assert_eq!(
            with_stderr("failure".to_owned(), Some(&discarded)),
            "failure; engine stderr:  [discarded 7 byte(s)]"
        );

        assert_eq!(bounded("ab", 2), "ab");
        assert_eq!(bounded("abc", 2), "ab…");
        assert_eq!(bounded("abcd", 2), "ab…");
    }

    #[test]
    fn verbose_difference_finds_a_bounded_leaf_path() {
        let difference = first_difference(
            &json!({"lines": [{"clusters": [1, 2, 3]}]}),
            &json!({"lines": [{"clusters": [1, 9, 3]}]}),
        );
        assert_eq!(difference.path, "$[\"lines\"][0][\"clusters\"][1]");
        assert_eq!(difference.expected, "2");
        assert_eq!(difference.actual, "9");
    }
}
