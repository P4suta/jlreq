// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

fn compare_response(
    response: &Value,
    expected_ids: &BTreeMap<String, usize>,
    cases: &[Case],
    seen: &mut BTreeSet<String>,
    differences: &mut usize,
    verbose: bool,
) -> Result<(), String> {
    let object = response
        .as_object()
        .ok_or_else(|| "validated response stopped being an object".to_owned())?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "validated response lost its id".to_owned())?;
    let Some(index) = expected_ids.get(id).copied() else {
        return Err(format!("engine returned unknown response id {id:?}"));
    };
    if !seen.insert(id.to_owned()) {
        return Err(format!("engine returned duplicate response id {id:?}"));
    }
    let actual = object
        .get("response")
        .ok_or_else(|| "validated response lost its body".to_owned())?;
    let expected = &cases[index].expected;
    if actual != expected {
        *differences = differences.saturating_add(1);
        if verbose {
            let difference = first_difference(expected, actual);
            eprintln!(
                "DIFF {id} path={} expected={} actual={}",
                difference.path, difference.expected, difference.actual
            );
        } else {
            eprintln!("DIFF {id}");
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct JsonDifference {
    path: String,
    expected: String,
    actual: String,
}

fn first_difference(expected: &Value, actual: &Value) -> JsonDifference {
    fn walk(expected: Option<&Value>, actual: Option<&Value>, path: &str) -> JsonDifference {
        match (expected, actual) {
            (Some(Value::Array(expected)), Some(Value::Array(actual))) => {
                let length = expected.len().max(actual.len());
                for index in 0..length {
                    if expected.get(index) != actual.get(index) {
                        let next_path = bounded(&format!("{path}[{index}]"), 256);
                        return walk(expected.get(index), actual.get(index), &next_path);
                    }
                }
            },
            (Some(Value::Object(expected)), Some(Value::Object(actual))) => {
                let keys: BTreeSet<_> = expected.keys().chain(actual.keys()).collect();
                for key in keys {
                    if expected.get(key) != actual.get(key) {
                        let rendered_key =
                            serde_json::to_string(key).unwrap_or_else(|_| "?".to_owned());
                        let next_path = bounded(&format!("{path}[{rendered_key}]"), 256);
                        return walk(expected.get(key), actual.get(key), &next_path);
                    }
                }
            },
            _ => {},
        }
        JsonDifference {
            path: bounded(path, 256),
            expected: render_json(expected),
            actual: render_json(actual),
        }
    }
    walk(Some(expected), Some(actual), "$")
}

fn render_json(value: Option<&Value>) -> String {
    value.map_or_else(
        || "<missing>".to_owned(),
        |value| {
            bounded(
                &serde_json::to_string(value).unwrap_or_else(|_| "<?>".to_owned()),
                512,
            )
        },
    )
}

fn bounded(value: &str, maximum: usize) -> String {
    let mut output: String = value.chars().take(maximum).collect();
    if value.chars().count() > maximum {
        output.push('…');
    }
    output
}

