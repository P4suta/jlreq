// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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
    let mut line = Vec::new();
    let mut total = 0_usize;
    let mut line_number = 0_usize;
    let mut response_count = 0_usize;
    loop {
        let previous_total = total;
        if !read_limited_line(
            &mut reader,
            &mut line,
            options.max_message_bytes,
            options.max_suite_bytes,
            &mut total,
            "engine stdout",
        )? {
            break;
        }
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

