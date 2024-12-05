use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::error::{Error, ResultExt};
use crate::File;
use tracing::{debug, info, trace};
use walkdir::WalkDir;

pub struct Target {
    outputs: Vec<String>,
    command: Vec<String>,
    pub needs_rebuild: bool,
}

impl Target {
    // Parse the arguments into a Target struct.
    pub fn parse(args: Vec<String>) -> Result<Target, Error> {
        // Option<File>
        let mut newest_output = File::default();
        let mut outputs = Vec::new();
        let mut inputs = Vec::new();
        let mut command = Vec::new();
        let mut needs_rebuild = false;

        let mut current_vec = &mut outputs;
        for arg in args {
            match arg.as_str() {
                ":" => current_vec = &mut inputs,
                "--" => current_vec = &mut command,
                _ => current_vec.push(arg),
            }
        }

        if outputs.is_empty() {
            return Err(Error::MissingOutputs);
        }

        // Find latest output
        for output in outputs.iter() {
            let newest = match find_newest(output) {
                Err(e) if e.is_not_found() => {
                    info!(r#"output "{}" does not exist, rebuilding"#, output);
                    needs_rebuild = true;
                    break;
                }
                Err(e) => return Err(e),
                Ok(n) => n,
            };
            if newest > newest_output {
                debug!("{} is the newest output", newest);
                newest_output = newest
            }
        }

        for input in inputs.iter() {
            let newest = match find_newest(input) {
                Ok(n) => n,
                Err(e) if e.is_not_found() => {
                    return Err(Error::MissingInput(input.clone()));
                }
                Err(e) => return Err(e),
            };
            if newest > newest_output {
                debug!(
                    "input {} is newer than output {}, rebuilding",
                    newest, newest_output
                );
                return Ok(Target {
                    outputs,
                    command,
                    needs_rebuild: true,
                });
            } else {
                trace!(
                    "input {} is older than newest output {}",
                    newest,
                    newest_output
                )
            }
        }

        if newest_output == File::default() {
            info!("no outputs found");
            if command.is_empty() {
                return Err(Error::MissingOutput(outputs[0].clone()));
            }
        } else {
            info!("newest output is {}", newest_output);
        }
        Ok(Target {
            outputs,
            command,
            needs_rebuild,
        })
    }

    /// Returns true if the command should be run.
    pub fn should_run_command(&self) -> bool {
        self.needs_rebuild && !self.command.is_empty()
    }

    // Run the command and verify that all outputs exist.
    // Will not display the command if it is prefixed with `@`.
    pub fn run_command(&self) -> Result<(), Error> {
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
        let status = std::process::Command::new("bash")
            .args(vec!["-c", shell_command.as_str()])
            .status()?
            .code()
            .unwrap_or(-1);
        if status != 0 {
            return Err(Error::CommandFailed(status));
        }
        for output in self.outputs.iter() {
            match std::fs::metadata(output) {
                Ok(m) => m,
                Err(e) if e.kind() == ErrorKind::NotFound => {
                    return Err(Error::MissingOutput(output.clone()));
                }
                Err(e) => return Err(e.into()),
            };
        }
        Ok(())
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
    for entry in WalkDir::new(path).follow_links(true) {
        let entry = entry.map_err_path_context(path)?;
        let path = entry.path().to_path_buf();
        let metadata = entry.metadata().map_err_path_context(path.clone())?;
        if !metadata.is_file() {
            continue;
        }
        let modified = metadata.modified().map_err_path_context(path.clone())?;

        if modified > newest.modified {
            newest = File { path, modified };
        }
    }
    Ok(newest)
}
