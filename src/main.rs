use std::{
    io::{Error, ErrorKind},
    process::exit,
    time::SystemTime,
};
use tracing::{debug, trace};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Compare timestamps of inputs and outputs, exiting with a non-zero status if
/// any input is newer than all outputs.
///
/// Outputs are all arguments up until a single ":" argument, and the input is
/// all subsequent arguments up to "--". If a "--" argument is present, all
/// arguments after it are executed as a command if any input is newer than all
/// outputs.
fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_env("MK_LOG"))
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!(
            r#"Usage: mk <output>... : <input>... [-- <command>...]

One-liner "make" targets on the command-line.

Compare timestamps of inputs and outputs, exiting with a non-zero status
or executing command if any input is newer than all outputs. If an input or
output is a directory, it is recursed into.

If a command is provided it is run through "bash -c". If a single command
argument is provided it will be run as-is, otherwise all arguments will be
joined with shell quoting.

eg.

    mk main.o : main.c -- cc -c main.c && \
        mk main : main.o -- cc -o main main.o

Like make, if a command is prefixed with @ it will not be echoed.

Use MK_LOG=trace to see debug output.
"#
        );
        exit(0);
    }
    match Newer::new(args) {
        Ok(newer) => {
            if !newer.should_run_command() {
                debug!("Nothing to do.");
                exit(if newer.newer { 1 } else { 0 });
            } else {
                match run_command(newer.command) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("{}", e);
                        exit(1);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    }
}

struct Newer {
    command: Vec<String>,
    newer: bool,
}

impl Newer {
    fn new(args: Vec<String>) -> Result<Newer, Error> {
        let mut newest_output: SystemTime = SystemTime::UNIX_EPOCH;
        let mut command = Vec::<String>::new();
        let mut state = 'O';
        let mut have_inputs = false;
        let mut newer = false;

        for arg in args {
            match arg.as_str() {
                ":" => {
                    state = 'I';
                    continue;
                }
                "--" => {
                    state = 'C';
                    continue;
                }
                _ => (),
            }
            match state {
                'O' => {
                    let newest = match find_newest(arg.clone()) {
                        Ok(newest) => newest,
                        Err(e) => {
                            if e.kind() == ErrorKind::NotFound {
                                continue;
                            } else {
                                return Err(e);
                            }
                        }
                    };
                    if newest > newest_output {
                        debug!("{} is the newest output", arg);
                        newest_output = newest
                    }
                }
                'I' => {
                    have_inputs = true;
                    let newest = find_newest(arg.clone())?;
                    if newest > newest_output {
                        trace!("input {} is newer than newest output", arg);
                        newer = true;
                    } else {
                        trace!("input {} is not newer than newest output", arg)
                    }
                }
                'C' => command.push(arg),
                _ => unreachable!(),
            }
        }
        // Always rebuild if no inputs are provided.
        if !have_inputs {
            newer = true;
        }
        Ok(Newer { command, newer })
    }

    fn should_run_command(&self) -> bool {
        self.newer && !self.command.is_empty()
    }
}

fn run_command(command: Vec<String>) -> Result<i32, Error> {
    let mut shell_command = if command.len() > 1 {
        shell_words::join(command)
    } else {
        command[0].clone()
    };
    // If the command starts with `@`, don't echo it.
    if shell_command.starts_with('@') {
        shell_command = shell_command[1..].to_string();
    } else {
        println!("{}", &shell_command);
    }
    Ok(std::process::Command::new("bash")
        .args(vec!["-c", shell_command.as_str()])
        .status()?
        .code()
        .unwrap_or(-1))
}

fn find_newest(path: String) -> Result<SystemTime, Error> {
    let mut newest = SystemTime::UNIX_EPOCH;
    let metadata = std::fs::metadata(path.clone())
        .map_err(|e| Error::new(e.kind(), format!("{path}: {e}")))?;

    if !metadata.is_dir() {
        let modified = metadata.modified()?;
        return if modified > newest {
            Ok(modified)
        } else {
            Ok(newest)
        };
    }

    for entry in
        std::fs::read_dir(path.clone()).map_err(|e| Error::new(e.kind(), format!("{path}: {e}")))?
    {
        let entry = entry?;
        let path = entry.path();
        let modified = find_newest(path.to_str().unwrap().to_string())?;
        if modified > newest {
            newest = modified;
        }
    }
    Ok(newest)
}
