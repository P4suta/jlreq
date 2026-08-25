// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Versioned NDJSON black-box conformance runner.

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
use validation::{validate_request, validate_response};

const PROTOCOL: &str = "jlreq.conformance/1";
const SPEC: &str = jlreq::SPECIFICATION;
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

fn main() -> ExitCode {
    match parse_invocation(env::args().skip(1)) {
        Ok(Invocation::Help) => {
            print_help();
            ExitCode::SUCCESS
        },
        Ok(Invocation::Version) => {
            println!("jlreq-conformance {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        },
        Ok(Invocation::Command(options, command)) => run_command(&options, command),
        Err(error) => {
            eprintln!("jlreq-conformance: {error}");
            eprintln!("try 'jlreq-conformance --help' for usage");
            ExitCode::from(2)
        },
    }
}

#[derive(Debug, Clone)]
struct Options {
    verbose: bool,
    timeout: Duration,
    max_message_bytes: usize,
    max_suite_bytes: usize,
    max_cases: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            verbose: false,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECONDS),
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            max_suite_bytes: DEFAULT_MAX_SUITE_BYTES,
            max_cases: DEFAULT_MAX_CASES,
        }
    }
}

#[derive(Debug, Clone)]
enum CliCommand {
    Validate {
        suite: Option<String>,
    },
    List {
        suite: Option<String>,
    },
    Run {
        engine: String,
        suite: Option<String>,
    },
}

#[derive(Debug, Clone)]
enum Invocation {
    Help,
    Version,
    Command(Options, CliCommand),
}

fn parse_invocation(arguments: impl IntoIterator<Item = String>) -> Result<Invocation, String> {
    let mut options = Options::default();
    let mut positional = Vec::new();
    let mut arguments = arguments.into_iter().peekable();
    let mut parse_options = true;
    while let Some(argument) = arguments.next() {
        if parse_options && argument == "--" {
            parse_options = false;
        } else if parse_options && matches!(argument.as_str(), "-h" | "--help") {
            return Ok(Invocation::Help);
        } else if parse_options && matches!(argument.as_str(), "-V" | "--version") {
            return Ok(Invocation::Version);
        } else if parse_options && argument == "--verbose" {
            options.verbose = true;
        } else if parse_options && argument.starts_with("--") {
            let (name, inline) = argument
                .split_once('=')
                .map_or((argument.as_str(), None), |(name, value)| {
                    (name, Some(value))
                });
            let value = match name {
                "--timeout-seconds"
                | "--max-message-bytes"
                | "--max-suite-bytes"
                | "--max-cases" => inline.map(str::to_owned).or_else(|| arguments.next()),
                _ => return Err(format!("unknown option {argument:?}")),
            }
            .ok_or_else(|| format!("{name} needs a positive integer"))?;
            match name {
                "--timeout-seconds" => {
                    let seconds = positive_u64(&value, name)?;
                    options.timeout = Duration::from_secs(seconds);
                },
                "--max-message-bytes" => {
                    options.max_message_bytes = positive_usize(&value, name)?;
                },
                "--max-suite-bytes" => {
                    options.max_suite_bytes = positive_usize(&value, name)?;
                },
                "--max-cases" => options.max_cases = positive_usize(&value, name)?,
                _ => return Err(format!("unknown option {argument:?}")),
            }
        } else {
            positional.push(argument);
        }
    }

    let Some(command) = positional.first().map(String::as_str) else {
        return Err("a command is required".to_owned());
    };
    let command = match command {
        "validate" if positional.len() <= 2 => CliCommand::Validate {
            suite: positional.get(1).cloned(),
        },
        "list" if positional.len() <= 2 => CliCommand::List {
            suite: positional.get(1).cloned(),
        },
        "run" if (2..=3).contains(&positional.len()) => CliCommand::Run {
            engine: positional[1].clone(),
            suite: positional.get(2).cloned(),
        },
        "validate" => return Err("usage: jlreq-conformance validate [SUITE.ndjson|-]".to_owned()),
        "list" => return Err("usage: jlreq-conformance list [SUITE.ndjson]".to_owned()),
        "run" => {
            return Err("usage: jlreq-conformance run ENGINE [SUITE.ndjson]".to_owned());
        },
        other => return Err(format!("unknown command {other:?}")),
    };
    Ok(Invocation::Command(options, command))
}

fn positive_u64(value: &str, option: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{option} needs a positive integer"))
}

fn positive_usize(value: &str, option: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{option} needs a positive integer"))
}

fn print_help() {
    println!(
        "jlreq-conformance {}\n\
         \n\
         Usage:\n\
           jlreq-conformance [OPTIONS] validate [SUITE.ndjson|-]\n\
           jlreq-conformance [OPTIONS] list [SUITE.ndjson]\n\
           jlreq-conformance [OPTIONS] run ENGINE [SUITE.ndjson]\n\
         \n\
         Options:\n\
           -h, --help                    Show this help\n\
           -V, --version                 Show the package version\n\
               --verbose                 Show bounded JSON differences\n\
               --timeout-seconds N       No-I/O timeout (default: 30)\n\
               --max-message-bytes N     Per-line limit (default: 1048576)\n\
               --max-suite-bytes N       Stream limit (default: 268435456)\n\
               --max-cases N             Message/case limit (default: 200000)",
        env!("CARGO_PKG_VERSION")
    );
}

fn run_command(options: &Options, command: CliCommand) -> ExitCode {
    match command {
        CliCommand::Validate { suite } => {
            match read_messages(suite.as_deref(), false, false, options) {
                Ok(messages) => {
                    eprintln!("validated {} message(s)", messages.len());
                    ExitCode::SUCCESS
                },
                Err(error) => protocol_exit(&error),
            }
        },
        CliCommand::List { suite } => match read_cases(suite.as_deref(), true, options) {
            Ok(cases) => {
                for case in cases {
                    println!("{}", case.id);
                }
                ExitCode::SUCCESS
            },
            Err(error) => protocol_exit(&error),
        },
        CliCommand::Run { engine, suite } => {
            let cases = match read_cases(suite.as_deref(), true, options) {
                Ok(cases) => cases,
                Err(error) => return protocol_exit(&error),
            };
            match run_engine(&engine, cases, options) {
                Ok(0) => ExitCode::SUCCESS,
                Ok(differences) => {
                    eprintln!("{differences} conformance case(s) differed");
                    ExitCode::from(1)
                },
                Err(error) => protocol_exit(&error),
            }
        },
    }
}

fn protocol_exit(error: &str) -> ExitCode {
    eprintln!("jlreq-conformance: {error}");
    ExitCode::from(2)
}

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
    let mut total = 0_usize;
    let mut line_number = 0_usize;
    loop {
        let previous_total = total;
        let Some(line) = read_limited_line(
            reader,
            options.max_message_bytes,
            options.max_suite_bytes,
            &mut total,
            stream_name,
        )?
        else {
            break;
        };
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

fn read_limited_line(
    reader: &mut dyn BufRead,
    max_message_bytes: usize,
    max_total_bytes: usize,
    total: &mut usize,
    stream_name: &str,
) -> Result<Option<Vec<u8>>, String> {
    let mut line = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| format!("could not read {stream_name}: {error}"))?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let content = newline.unwrap_or(available.len());
        let consumed = content.saturating_add(usize::from(newline.is_some()));
        *total = total.saturating_add(consumed);
        if *total > max_total_bytes {
            return Err(format!(
                "{stream_name} exceeds the {max_total_bytes} byte total limit"
            ));
        }
        if line.len().saturating_add(content) > max_message_bytes {
            return Err(format!(
                "{stream_name} message exceeds the {max_message_bytes} byte line limit"
            ));
        }
        line.extend_from_slice(&available[..content]);
        reader.consume(consumed);
        if newline.is_some() {
            return Ok(Some(line));
        }
    }
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

#[derive(Debug)]
enum EngineEvent {
    Response(Value),
    WriterDone(Result<(), String>),
    ReaderDone(Result<(), String>),
}

#[derive(Debug)]
struct StderrCapture {
    retained: Vec<u8>,
    discarded: usize,
}

fn run_engine(engine: &str, cases: Vec<Case>, options: &Options) -> Result<usize, String> {
    let mut child = Command::new(engine)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not start engine {engine:?}: {error}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| stop_unstarted_child(&mut child, "engine stdin was not piped"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| stop_unstarted_child(&mut child, "engine stdout was not piped"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| stop_unstarted_child(&mut child, "engine stderr was not piped"))?;

    let cases = Arc::new(cases);
    let activity = Arc::new(Mutex::new(Instant::now()));
    let (sender, receiver) = mpsc::sync_channel(EVENT_CHANNEL_CAPACITY);

    let writer = spawn_writer(
        stdin,
        Arc::clone(&cases),
        sender.clone(),
        Arc::clone(&activity),
    )
    .map_err(|error| {
        stop_child(&mut child);
        format!("could not start engine stdin writer: {error}")
    })?;
    let reader = match spawn_reader(stdout, options.clone(), sender, Arc::clone(&activity)) {
        Ok(reader) => reader,
        Err(error) => {
            stop_child(&mut child);
            let _ = writer.join();
            return Err(format!("could not start engine stdout reader: {error}"));
        },
    };
    let stderr_reader = match spawn_stderr(stderr) {
        Ok(stderr_reader) => stderr_reader,
        Err(error) => {
            stop_child(&mut child);
            drop(receiver);
            let _ = writer.join();
            let _ = reader.join();
            return Err(format!("could not start engine stderr drainer: {error}"));
        },
    };

    let expected: BTreeMap<_, _> = cases
        .iter()
        .enumerate()
        .map(|(index, case)| (case.id.clone(), index))
        .collect();
    let mut seen = BTreeSet::new();
    let mut differences = 0_usize;
    let mut writer_done = false;
    let mut reader_done = false;
    let mut status = None;
    let mut failure = None;

    while engine_work_pending(
        failure.is_some(),
        (writer_done, reader_done, status.is_some()),
    ) {
        if status.is_none() {
            match child.try_wait() {
                Ok(found) => status = found,
                Err(error) => failure = Some(format!("could not inspect engine status: {error}")),
            }
        }
        let idle = activity_elapsed(&activity).unwrap_or(options.timeout);
        if idle >= options.timeout {
            failure = Some(format!(
                "engine made no stdin/stdout progress for {} second(s)",
                options.timeout.as_secs()
            ));
            break;
        }
        let remaining = options.timeout.saturating_sub(idle);
        let wait = WATCHDOG_TICK.min(remaining);
        match receiver.recv_timeout(wait) {
            Ok(EngineEvent::Response(response)) => {
                if let Err(error) = compare_response(
                    &response,
                    &expected,
                    &cases,
                    &mut seen,
                    &mut differences,
                    options.verbose,
                ) {
                    failure = Some(error);
                }
            },
            Ok(EngineEvent::WriterDone(result)) => {
                writer_done = true;
                if let Err(error) = result {
                    failure = Some(error);
                }
            },
            Ok(EngineEvent::ReaderDone(result)) => {
                reader_done = true;
                if let Err(error) = result {
                    failure = Some(error);
                }
            },
            Err(RecvTimeoutError::Timeout) => {},
            Err(RecvTimeoutError::Disconnected) => {
                if workers_disconnected_early(writer_done, reader_done) {
                    failure = Some("engine I/O workers stopped unexpectedly".to_owned());
                }
            },
        }
    }

    if failure.is_some() {
        let _ = child.kill();
    }
    drop(receiver);
    let waited = child.wait();
    let writer_join = join_worker(writer, "stdin writer");
    let reader_join = join_worker(reader, "stdout reader");
    let captured = join_stderr(stderr_reader);

    if let Some(error) = failure {
        return Err(with_stderr(error, captured.as_ref().ok()));
    }
    writer_join?;
    reader_join?;
    let status = status
        .or_else(|| waited.ok())
        .ok_or_else(|| "could not wait for engine".to_owned())?;
    if !status.success() {
        return Err(with_stderr(
            format!("engine exited with {status}"),
            captured.as_ref().ok(),
        ));
    }
    let captured = captured?;
    if seen.len() != cases.len() {
        let missing = cases
            .iter()
            .filter(|case| !seen.contains(case.id.as_str()))
            .take(3)
            .map(|case| case.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(with_stderr(
            format!(
                "engine omitted {} response(s){}",
                cases.len().saturating_sub(seen.len()),
                if missing.is_empty() {
                    String::new()
                } else {
                    format!(": {missing}")
                }
            ),
            Some(&captured),
        ));
    }
    Ok(differences)
}

fn engine_work_pending(failed: bool, progress: (bool, bool, bool)) -> bool {
    let (writer_done, reader_done, status_seen) = progress;
    !(failed || (writer_done && reader_done && status_seen))
}

fn workers_disconnected_early(writer_done: bool, reader_done: bool) -> bool {
    !(writer_done && reader_done)
}

fn stop_unstarted_child(child: &mut Child, message: &str) -> String {
    stop_child(child);
    message.to_owned()
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn spawn_writer(
    stdin: ChildStdin,
    cases: Arc<Vec<Case>>,
    sender: SyncSender<EngineEvent>,
    activity: Arc<Mutex<Instant>>,
) -> io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("jlreq-engine-stdin".to_owned())
        .spawn(move || {
            let result = write_requests(stdin, &cases, &activity);
            let _ = sender.send(EngineEvent::WriterDone(result));
        })
}

fn write_requests(
    mut stdin: ChildStdin,
    cases: &[Case],
    activity: &Mutex<Instant>,
) -> Result<(), String> {
    for case in cases {
        let message = serde_json::json!({
            "protocol": PROTOCOL,
            "spec": SPEC,
            "id": case.id,
            "request": case.request,
        });
        serde_json::to_writer(&mut stdin, &message).map_err(|error| {
            format!(
                "engine stdin could not encode request {:?}: {error}",
                case.id
            )
        })?;
        stdin
            .write_all(b"\n")
            .and_then(|()| stdin.flush())
            .map_err(|error| {
                format!(
                    "engine stdin could not write request {:?}: {error}",
                    case.id
                )
            })?;
        mark_activity(activity);
    }
    Ok(())
}

fn spawn_reader(
    stdout: ChildStdout,
    options: Options,
    sender: SyncSender<EngineEvent>,
    activity: Arc<Mutex<Instant>>,
) -> io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("jlreq-engine-stdout".to_owned())
        .spawn(move || {
            let result = read_responses(stdout, &options, &sender, &activity);
            let _ = sender.send(EngineEvent::ReaderDone(result));
        })
}

fn read_responses(
    stdout: ChildStdout,
    options: &Options,
    sender: &SyncSender<EngineEvent>,
    activity: &Mutex<Instant>,
) -> Result<(), String> {
    let mut reader = BufReader::new(stdout);
    let mut total = 0_usize;
    let mut line_number = 0_usize;
    let mut response_count = 0_usize;
    loop {
        let previous_total = total;
        let Some(line) = read_limited_line(
            &mut reader,
            options.max_message_bytes,
            options.max_suite_bytes,
            &mut total,
            "engine stdout",
        )?
        else {
            break;
        };
        if total == previous_total {
            return Err("engine stdout reader made no progress".to_owned());
        }
        line_number = line_number.saturating_add(1);
        let line = std::str::from_utf8(&line)
            .map_err(|error| format!("engine line {line_number} is not UTF-8: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        response_count = response_count.saturating_add(1);
        if response_count > options.max_cases {
            return Err(format!(
                "engine stdout exceeds the {} response limit",
                options.max_cases
            ));
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("engine line {line_number}: invalid JSON: {error}"))?;
        validate_envelope(&value, false)
            .map_err(|error| format!("engine line {line_number}: {error}"))?;
        mark_activity(activity);
        sender
            .send(EngineEvent::Response(value))
            .map_err(|_| "engine response consumer stopped".to_owned())?;
    }
    Ok(())
}

fn spawn_stderr(stderr: ChildStderr) -> io::Result<JoinHandle<StderrCapture>> {
    thread::Builder::new()
        .name("jlreq-engine-stderr".to_owned())
        .spawn(move || drain_stderr(stderr))
}

fn drain_stderr(mut stderr: ChildStderr) -> StderrCapture {
    let mut retained = Vec::new();
    let mut discarded = 0_usize;
    let mut buffer = [0_u8; 8192];
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                let room = STDERR_RETAIN_BYTES.saturating_sub(retained.len());
                let keep = read.min(room);
                retained.extend_from_slice(&buffer[..keep]);
                discarded = discarded.saturating_add(read.saturating_sub(keep));
            },
        }
    }
    StderrCapture {
        retained,
        discarded,
    }
}

fn mark_activity(activity: &Mutex<Instant>) {
    if let Ok(mut last) = activity.lock() {
        *last = Instant::now();
    }
}

fn activity_elapsed(activity: &Mutex<Instant>) -> Option<Duration> {
    activity.lock().ok().map(|last| last.elapsed())
}

fn join_worker(worker: JoinHandle<()>, name: &str) -> Result<(), String> {
    worker.join().map_err(|_| format!("engine {name} panicked"))
}

fn join_stderr(worker: JoinHandle<StderrCapture>) -> Result<StderrCapture, String> {
    worker
        .join()
        .map_err(|_| "engine stderr drainer panicked".to_owned())
}

fn with_stderr(message: String, captured: Option<&StderrCapture>) -> String {
    let Some(captured) = captured else {
        return message;
    };
    if captured.retained.is_empty() && captured.discarded == 0 {
        return message;
    }
    let stderr = String::from_utf8_lossy(&captured.retained);
    if captured.discarded == 0 {
        format!("{message}; engine stderr: {}", stderr.trim_end())
    } else {
        format!(
            "{message}; engine stderr: {} [discarded {} byte(s)]",
            stderr.trim_end(),
            captured.discarded
        )
    }
}

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
        let mut reader = Cursor::new(b"abc\n".as_slice());
        assert_eq!(
            read_limited_line(&mut reader, 3, 4, &mut total, "test"),
            Ok(Some(b"abc".to_vec()))
        );
        assert_eq!(total, 4);

        let mut total = 0;
        let mut reader = Cursor::new(b"abc\n".as_slice());
        assert!(read_limited_line(&mut reader, 2, 4, &mut total, "test").is_err());

        let mut total = 0;
        let mut reader = Cursor::new(b"abc\n".as_slice());
        assert!(read_limited_line(&mut reader, 3, 3, &mut total, "test").is_err());
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
