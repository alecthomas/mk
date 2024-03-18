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
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: mktg <output>... : <input>... [-- <command>...]");
        std::process::exit(1);
    }
    let newer = Newer::new(args);
    match newer {
        Ok(newer) => {
            if !newer.should_run_command() {
                std::process::exit(0);
            }
            match run_command(newer.command) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
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
                    let newest = match find_newest(arg) {
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
                        newest_output = newest
                    }
                }
                'I' => {
                    let metadata = std::fs::metadata(&arg)?;
                    if metadata.modified()? > newest_output {
                        newer = true;
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
    Ok(std::process::Command::new(command[0].clone())
        .args(command.iter().skip(1))
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
