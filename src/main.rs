use chrono::{DateTime, Local};
use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    process::exit,
    time::SystemTime,
};
use tracing::{debug, info, trace};
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
        display_usage();
        exit(0);
    }
    match Target::parse(args) {
        Ok(target) => {
            if !target.should_run_command() {
                debug!("Nothing to do.");
                exit(target.needs_rebuild.into());
            } else {
                match target.run_command() {
                    Ok(code) => exit(code),
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

struct Target {
    command: Vec<String>,
    needs_rebuild: bool,
}

impl Target {
    // Parse the arguments into a Target struct.
    fn parse(args: Vec<String>) -> Result<Target, Error> {
        enum State {
            Output,
            Input,
            Command,
        }

        use State::*;

        // Option<File>
        let mut newest_output = File::default();
        let mut command = Vec::<String>::new();
        let mut state = Output;
        let mut have_inputs = false;
        let mut newer = false;

        for arg in args {
            match (&state, arg.as_str()) {
                (Output, ":") => state = Input,
                (Output, _) => {
                    let newest = match find_newest(&arg) {
                        Ok(n) => n,
                        Err(e) if e.kind() == ErrorKind::NotFound => {
                            info!("output {} does not exist", arg);
                            newer = true;
                            continue;
                        }
                        Err(e) => return Err(e),
                    };
                    if newest > newest_output {
                        debug!("{} is the newest output", newest);
                        newest_output = newest
                    }
                }
                (Input, "--") => state = Command,
                (Input, _) => {
                    if newer {
                        continue;
                    }
                    have_inputs = true;
                    let newest = find_newest(&arg)?;
                    if newest > newest_output {
                        debug!(
                            "input {} is newer than output {}, rebuilding",
                            newest, newest_output
                        );
                        newer = true;
                    } else {
                        trace!(
                            "input {} is not newer than newest output {}",
                            newest,
                            newest_output
                        )
                    }
                }
                (Command, _) => command.push(arg),
            }
        }
        // Always rebuild if no inputs are provided.
        if !have_inputs {
            trace!("no inputs provided, forcing rebuild");
            newer = true;
        }
        info!("newest output is {}", newest_output);
        Ok(Target {
            command,
            needs_rebuild: newer,
        })
    }

    fn should_run_command(&self) -> bool {
        self.needs_rebuild && !self.command.is_empty()
    }

    fn run_command(&self) -> Result<i32, Error> {
        let mut shell_command = if self.command.len() > 1 {
            shell_words::join(&self.command)
        } else {
            self.command[0].clone()
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
}

struct File {
    modified: SystemTime,
    path: String,
}

fn round_to_s(ts: SystemTime) -> u64 {
    ts.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        round_to_s(self.modified) == round_to_s(other.modified) && self.path == other.path
    }
}

impl PartialOrd for File {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(round_to_s(self.modified).cmp(&round_to_s(other.modified)))
    }
}

impl Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.path, format_timestamp(self.modified),)
    }
}

impl Default for File {
    fn default() -> Self {
        Self {
            modified: SystemTime::UNIX_EPOCH,
            path: String::new(),
        }
    }
}

impl File {
    /// Return a copy of the File with the most recent modified time if newer than the current.
    fn most_recent(&self, modified: SystemTime) -> Self {
        Self {
            path: self.path.to_string(),
            modified: if modified > self.modified {
                modified
            } else {
                self.modified
            },
        }
    }
}

/// Recurse into directories to find the newest file.
///
/// Returns the newest file's modified time and its path.
fn find_newest(path: &str) -> Result<File, Error> {
    let mut newest = File {
        path: path.to_string(),
        modified: SystemTime::UNIX_EPOCH,
    };
    let metadata =
        std::fs::metadata(path).map_err(|e| Error::new(e.kind(), format!("{path}: {e}")))?;

    if !metadata.is_dir() {
        let modified = metadata
            .modified()
            .map_err(|e| Error::new(e.kind(), format!("{path}: {e}")))?;
        return Ok(newest.most_recent(modified));
    }

    for entry in
        std::fs::read_dir(path).map_err(|e| Error::new(e.kind(), format!("{path}: {e}")))?
    {
        if let Some(path) = entry?.path().to_str() {
            let next_file = find_newest(path)?;
            if next_file > newest {
                newest = next_file;
            }
        }
    }
    Ok(newest)
}

fn display_usage() {
    eprintln!(
        r#"One-liner `make` rules on the command-line.

Usage: `mk <output> [<output> ...] [: <input> [<input> ...]] [-- <command>...]`

Compare timestamps of inputs and outputs, exiting with a non-zero status
or executing command if any input is newer than all outputs. If an input or
output is a directory, it is recursed into.

If a command is provided it is run through `bash -c`. If a single command
argument is provided it will be run as-is, otherwise all arguments will be
joined with shell quoting.

eg.

    mk main.o : main.c -- cc -c main.c && \
        mk main : main.o -- cc -o main main.o

Like make, if a command is prefixed with `@` it will not be echoed.

Use `MK_LOG=trace` to see debug output.
"#
    );
}

/// Format a Timestamp as the elapsed seconds, and milliseconds since the time.
fn format_timestamp(ts: SystemTime) -> String {
    let ts: DateTime<Local> = ts.into();
    let elapsed = Local::now().signed_duration_since(ts);
    format!("{}s", elapsed)
}
