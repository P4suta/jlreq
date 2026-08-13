// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Versioned NDJSON black-box conformance runner.

mod validation;

use std::{
    env, fs,
    io::{self, Read, Write},
    path::Path,
    process::{Command, ExitCode, Stdio},
};

use serde_json::{Map, Value};
use validation::{validate_request, validate_response};

const PROTOCOL: &str = "kumihan.conformance/1";
const SPEC: &str = kumihan::SPECIFICATION;
const BUILTIN_SUITE: &str = include_str!("../suite.ndjson");
#[cfg(test)]
const PROTOCOL_SCHEMA: &str = include_str!("../protocol.schema.json");

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let Some(command) = arguments.next() else {
        usage();
        return ExitCode::from(2);
    };
    let rest: Vec<_> = arguments.collect();
    match command.as_str() {
        "validate" => validate_command(&rest),
        "list" => list_command(&rest),
        "run" => run_command(&rest),
        _ => {
            usage();
            ExitCode::from(2)
        },
    }
}

fn usage() {
    eprintln!("usage: kumihan-conformance <run|validate|list> [arguments]");
}

fn validate_command(arguments: &[String]) -> ExitCode {
    if arguments.len() > 1 {
        usage();
        return ExitCode::from(2);
    }
    let input = match read_suite(arguments.first().map(String::as_str)) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("kumihan-conformance: {error}");
            return ExitCode::from(2);
        },
    };
    match parse_messages(&input, false) {
        Ok(messages) => {
            eprintln!("validated {} message(s)", messages.len());
            ExitCode::SUCCESS
        },
        Err(error) => {
            eprintln!("kumihan-conformance: {error}");
            ExitCode::from(2)
        },
    }
}

fn list_command(arguments: &[String]) -> ExitCode {
    if arguments.len() > 1 {
        usage();
        return ExitCode::from(2);
    }
    let input = match read_suite_or_builtin(arguments.first().map(String::as_str)) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("kumihan-conformance: {error}");
            return ExitCode::from(2);
        },
    };
    match parse_cases(&input) {
        Ok(cases) => {
            for case in cases {
                println!("{}", case.id);
            }
            ExitCode::SUCCESS
        },
        Err(error) => {
            eprintln!("kumihan-conformance: {error}");
            ExitCode::from(2)
        },
    }
}

fn run_command(arguments: &[String]) -> ExitCode {
    if arguments.is_empty() || arguments.len() > 2 {
        eprintln!("usage: kumihan-conformance run ENGINE [SUITE.ndjson]");
        return ExitCode::from(2);
    }
    let input = match read_suite_or_builtin(arguments.get(1).map(String::as_str)) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("kumihan-conformance: {error}");
            return ExitCode::from(2);
        },
    };
    let cases = match parse_cases(&input) {
        Ok(cases) => cases,
        Err(error) => {
            eprintln!("kumihan-conformance: {error}");
            return ExitCode::from(2);
        },
    };
    match run_engine(&arguments[0], &cases) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(differences) => {
            eprintln!("{differences} conformance case(s) differed");
            ExitCode::from(1)
        },
        Err(error) => {
            eprintln!("kumihan-conformance: {error}");
            ExitCode::from(2)
        },
    }
}

#[derive(Debug)]
struct Case {
    id: String,
    request: Value,
    expected: Value,
}

fn read_suite(path: Option<&str>) -> Result<String, String> {
    match path {
        Some(path) if path != "-" => {
            fs::read_to_string(Path::new(path)).map_err(|error| error.to_string())
        },
        _ => {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .map_err(|error| error.to_string())?;
            Ok(input)
        },
    }
}

fn read_suite_or_builtin(path: Option<&str>) -> Result<String, String> {
    match path {
        None => Ok(BUILTIN_SUITE.to_owned()),
        Some(path) => read_suite(Some(path)),
    }
}

fn parse_messages(input: &str, cases: bool) -> Result<Vec<Value>, String> {
    let mut messages = Vec::new();
    for (line_index, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let line_number = line_index.saturating_add(1);
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("line {line_number}: invalid JSON: {error}"))?;
        validate_envelope(&value, cases).map_err(|error| format!("line {line_number}: {error}"))?;
        messages.push(value);
    }
    if messages.is_empty() {
        return Err("the NDJSON stream contains no messages".to_owned());
    }
    Ok(messages)
}

fn validate_envelope(value: &Value, case: bool) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "each message must be a JSON object".to_owned())?;
    if let Some(field) = object.keys().find(|field| {
        ![
            "protocol", "spec", "id", "rules", "request", "response", "expected",
        ]
        .contains(&field.as_str())
    }) {
        return Err(format!("unknown envelope field {field:?}"));
    }
    required_string(object, "protocol", PROTOCOL)?;
    required_string(object, "spec", SPEC)?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "id must be a string".to_owned())?;
    if id.is_empty() {
        return Err("id must not be empty".to_owned());
    }
    let has_request = object.get("request").is_some_and(Value::is_object);
    let has_response = object.get("response").is_some_and(Value::is_object);
    let has_expected = object.get("expected").is_some();
    if has_response && has_expected {
        return Err("an engine response cannot contain expected".to_owned());
    }
    if has_expected && !has_request {
        return Err("expected is valid only beside a suite request".to_owned());
    }
    match (has_expected, object.get("rules")) {
        (true, Some(rules)) => validate_rules(rules)?,
        (true, None) => return Err("a suite case needs non-empty rules metadata".to_owned()),
        (false, Some(_)) => return Err("rules metadata is valid only in a suite case".to_owned()),
        (false, None) => {},
    }
    if case {
        if !has_request || !object.get("expected").is_some_and(Value::is_object) {
            return Err("a suite case needs object-valued request and expected fields".to_owned());
        }
        validate_request(
            object
                .get("request")
                .ok_or_else(|| "validated suite case lost its request".to_owned())?,
        )?;
        validate_response(
            object
                .get("expected")
                .ok_or_else(|| "validated suite case lost its expected response".to_owned())?,
        )?;
    } else if has_request == has_response {
        return Err("a protocol message needs exactly one of request or response".to_owned());
    } else if let Some(request) = object.get("request") {
        validate_request(request)?;
        if let Some(expected) = object.get("expected") {
            validate_response(expected)?;
        }
    } else if let Some(response) = object.get("response") {
        validate_response(response)?;
    }
    Ok(())
}

fn validate_rules(value: &Value) -> Result<(), String> {
    let rules = value
        .as_array()
        .ok_or_else(|| "suite rules must be an array".to_owned())?;
    if rules.is_empty() {
        return Err("suite rules must not be empty".to_owned());
    }
    let mut seen = std::collections::BTreeSet::new();
    for rule in rules {
        let rule = rule
            .as_str()
            .filter(|rule| !rule.is_empty())
            .ok_or_else(|| "each suite rule must be a non-empty string".to_owned())?;
        if !seen.insert(rule) {
            return Err(format!("suite rule {rule:?} is repeated"));
        }
    }
    Ok(())
}

fn required_string(object: &Map<String, Value>, name: &str, expected: &str) -> Result<(), String> {
    match object.get(name).and_then(Value::as_str) {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!("{name} is {actual:?}, expected {expected:?}")),
        None => Err(format!("{name} is required and must be a string")),
    }
}

fn parse_cases(input: &str) -> Result<Vec<Case>, String> {
    parse_messages(input, true)?
        .into_iter()
        .map(|value| {
            let object = value
                .as_object()
                .ok_or_else(|| "validated case stopped being an object".to_owned())?;
            let id = object
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| "validated case lost its id".to_owned())?
                .to_owned();
            let request = object
                .get("request")
                .cloned()
                .ok_or_else(|| "validated case lost its request".to_owned())?;
            let expected = object
                .get("expected")
                .cloned()
                .ok_or_else(|| "validated case lost its expected response".to_owned())?;
            Ok(Case {
                id,
                request,
                expected,
            })
        })
        .collect()
}

fn run_engine(engine: &str, cases: &[Case]) -> Result<usize, String> {
    let mut child = Command::new(engine)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not start engine {engine:?}: {error}"))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "engine stdin was not piped".to_owned())?;
        for case in cases {
            let message = serde_json::json!({
                "protocol": PROTOCOL,
                "spec": SPEC,
                "id": case.id,
                "request": case.request,
            });
            serde_json::to_writer(&mut *stdin, &message).map_err(|error| error.to_string())?;
            stdin.write_all(b"\n").map_err(|error| error.to_string())?;
        }
    }
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .map_err(|error| format!("could not wait for engine: {error}"))?;
    if !output.status.success() {
        return Err(format!("engine exited with {}", output.status));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("engine output was not UTF-8: {error}"))?;
    let responses = parse_messages(&stdout, false)?;
    if responses.len() != cases.len() {
        return Err(format!(
            "engine returned {} response(s) for {} request(s)",
            responses.len(),
            cases.len()
        ));
    }

    let mut differences = 0_usize;
    for (case, response) in cases.iter().zip(responses) {
        let object = response
            .as_object()
            .ok_or_else(|| "validated response stopped being an object".to_owned())?;
        if object.get("id").and_then(Value::as_str) != Some(case.id.as_str()) {
            return Err(format!("response id does not match case {:?}", case.id));
        }
        if object.get("response") != Some(&case.expected) {
            differences = differences.saturating_add(1);
            eprintln!("DIFF {}", case.id);
        }
    }
    Ok(differences)
}

#[cfg(test)]
mod tests {
    use super::{BUILTIN_SUITE, PROTOCOL_SCHEMA, parse_cases, parse_messages};
    use serde_json::json;

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
            "protocol": "kumihan.conformance/1",
            "spec": "jlreq-2020-08-11+unicode-17.0.0",
            "id": "missing-input",
            "request": {}
        });
        assert!(parse_messages(&message.to_string(), false).is_err());
    }

    #[test]
    fn response_body_is_validated_instead_of_treated_as_an_opaque_object() {
        let message = json!({
            "protocol": "kumihan.conformance/1",
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
            "protocol": "kumihan.conformance/1",
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
            "protocol": "kumihan.conformance/1",
            "spec": "jlreq-2020-08-11+unicode-17.0.0",
            "id": "mixed",
            "response": {"lines": [], "diagnostics": []},
            "expected": {"lines": [], "diagnostics": []}
        });
        assert!(parse_messages(&mixed_output.to_string(), false).is_err());
    }
}
