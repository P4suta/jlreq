// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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

