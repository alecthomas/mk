use std::{
    io::{Error, ErrorKind},
    time::SystemTime,
};

/// Compare timestamps of inputs and outputs, exiting with a non-zero status if
/// any input is newer than all outputs.
///
/// Outputs are all arguments up until a single ":" argument, and the input is
/// all subsequent arguments up to "--". If a "--" argument is present, all
/// arguments after it are executed as a command if any input is newer than all
/// outputs.
fn main() {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!(
            r#"Usage: mktg <output>... : <input>... [-- <command>...]

Compare timestamps of inputs and outputs, exiting with a non-zero status
or executing command if any input is newer than all outputs. If an input or
output is a directory, it is recursed into.

If a command is provided it is run through "bash -c". If a single command
argument is provided it will be run as-is, otherwise all arguments will be
joined with shell quoting.

eg.

    mktg main.o : main.c -- cc -c main.c && \
        mktg main : main.o -- cc -o main main.o

Use RUST_LOG=trace to see debug output.
"#
        );
        std::process::exit(0);
    }
    match Newer::new(args) {
        Ok(newer) => {
            if !newer.should_run_command() {
                log::debug!("Nothing to do.");
            } else {
                match run_command(newer.command) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
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
                        log::debug!("{} is the newest output", arg);
                        newest_output = newest
                    }
                }
                'I' => {
                    let newest = find_newest(arg.clone())?;
                    if newest > newest_output {
                        log::debug!("input {} is newer than newest output", arg);
                        newer = true;
                    } else {
                        log::trace!("input {} is not newer than newest output", arg)
                    }
                }
                'C' => command.push(arg),
                _ => unreachable!(),
            }
        }
        Ok(Newer { command, newer })
    }

    fn should_run_command(&self) -> bool {
        self.newer && !self.command.is_empty()
    }
}

fn run_command(command: Vec<String>) -> Result<i32, Error> {
    let shell_command = if command.len() > 1 {
        shell_words::join(command)
    } else {
        command[0].clone()
    };
    Ok(std::process::Command::new("bash")
        .args(vec!["-c", shell_command.as_str()])
        .status()?
        .code()
        .unwrap_or(-1))
}

fn find_newest(path: String) -> Result<SystemTime, Error> {
    let mut newest = SystemTime::UNIX_EPOCH;
    let metadata = std::fs::metadata(path.clone())?;

    if !metadata.is_dir() {
        let modified = metadata.modified()?;
        return if modified > newest {
            Ok(modified)
        } else {
            Ok(newest)
        };
    }

    for entry in std::fs::read_dir(path.clone())? {
        let entry = entry?;
        let path = entry.path();
        let modified = find_newest(path.to_str().unwrap().to_string())?;
        if modified > newest {
            newest = modified;
        }
    }
    return Ok(newest);
}
