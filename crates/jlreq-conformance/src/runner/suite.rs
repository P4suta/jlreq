// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[derive(Debug, Clone)]
struct Case {
    id: String,
    request: Value,
    expected: Value,
}

fn suite_reader(path: Option<&str>, builtin_when_absent: bool) -> Result<Box<dyn BufRead>, String> {
    match path {
        None if builtin_when_absent => Ok(Box::new(Cursor::new(BUILTIN_SUITE.as_bytes()))),
        Some("-") | None => Ok(Box::new(BufReader::new(io::stdin()))),
        Some(path) => fs::File::open(Path::new(path))
            .map(|file| Box::new(BufReader::new(file)) as Box<dyn BufRead>)
            .map_err(|error| format!("could not open suite {path:?}: {error}")),
    }
}

fn read_messages(
    path: Option<&str>,
    builtin_when_absent: bool,
    cases: bool,
    options: &Options,
) -> Result<Vec<Value>, String> {
    let mut reader = suite_reader(path, builtin_when_absent)?;
    parse_reader(&mut *reader, cases, options, "suite")
}

fn parse_reader(
    reader: &mut dyn BufRead,
    cases: bool,
    options: &Options,
    stream_name: &str,
) -> Result<Vec<Value>, String> {
    let mut messages = Vec::new();
    let mut line = Vec::new();
    let mut total = 0_usize;
    let mut line_number = 0_usize;
    loop {
        let previous_total = total;
        if !read_limited_line(
            reader,
            &mut line,
            options.max_message_bytes,
            options.max_suite_bytes,
            &mut total,
            stream_name,
        )? {
            break;
        }
        if total == previous_total {
            return Err(format!("{stream_name} reader made no progress"));
        }
        line_number = line_number.saturating_add(1);
        let line = std::str::from_utf8(&line)
            .map_err(|error| format!("line {line_number}: input is not UTF-8: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        if messages.len() >= options.max_cases {
            return Err(format!(
                "{stream_name} exceeds the {} message limit",
                options.max_cases
            ));
        }
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
    let mut seen = BTreeSet::new();
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

fn read_cases(
    path: Option<&str>,
    builtin_when_absent: bool,
    options: &Options,
) -> Result<Vec<Case>, String> {
    let values = read_messages(path, builtin_when_absent, true, options)?;
    cases_from_values(values)
}

fn cases_from_values(values: Vec<Value>) -> Result<Vec<Case>, String> {
    let mut seen = BTreeSet::new();
    values
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
            if !seen.insert(id.clone()) {
                return Err(format!("suite case id {id:?} is repeated"));
            }
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

#[cfg(test)]
fn parse_messages(input: &str, cases: bool) -> Result<Vec<Value>, String> {
    let mut reader = Cursor::new(input.as_bytes());
    parse_reader(&mut reader, cases, &Options::default(), "NDJSON stream")
}

#[cfg(test)]
fn parse_cases(input: &str) -> Result<Vec<Case>, String> {
    cases_from_values(parse_messages(input, true)?)
}

