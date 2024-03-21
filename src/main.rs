mod error;
mod file;

use crate::Error::NotFound;
use error::{from_io_error, from_walkdir_error, Error};
use file::File;
use std::{path::PathBuf, process::exit, time::SystemTime};
use tracing::{debug, info, trace};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use walkdir::WalkDir;

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
                        Err(NotFound(path)) => {
                            info!("output {} does not exist", path.display());
                            newer = true;
                            continue;
                        }
                        Err(e) => return Err(e),
                        Ok(n) => n,
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
        if !have_inputs && !newer {
            trace!("no inputs provided, forcing rebuild");
            newer = true;
        }
        if newest_output == File::default() {
            info!("no outputs found");
        } else {
            info!("newest output is {}", newest_output);
        }
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

/// Recurse into directories to find the newest file.
///
/// Returns the newest file's modified time and its path.
fn find_newest(path: &str) -> Result<File, Error> {
    let mut newest = File {
        path: PathBuf::from(path),
        modified: SystemTime::UNIX_EPOCH,
    };
    for entry in WalkDir::new(path) {
        let entry = entry.map_err(from_walkdir_error(PathBuf::from(path)))?;
        let path = entry.path().to_path_buf();
        let metadata = entry.metadata().map_err(from_walkdir_error(path.clone()))?;
        if !metadata.is_file() {
            continue;
        }
        let modified = metadata.modified().map_err(from_io_error(path.clone()))?;

        if modified > newest.modified {
            newest = File { path, modified };
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
