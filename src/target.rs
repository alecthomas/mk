use std::path::PathBuf;
use std::time::SystemTime;

use crate::Error::NotFound;
use crate::File;
use crate::{from_io_error, from_walkdir_error, Error};
use tracing::{debug, info, trace};
use walkdir::WalkDir;

pub struct Target {
    command: Vec<String>,
    pub needs_rebuild: bool,
}

impl Target {
    // Parse the arguments into a Target struct.
    pub fn parse(args: Vec<String>) -> Result<Target, Error> {
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
        let mut needs_rebuild = false;

        for arg in args {
            match (&state, arg.as_str()) {
                (Output, ":") => state = Input,
                (Output, _) => {
                    let newest = match find_newest(&arg) {
                        Err(NotFound(path)) => {
                            info!("output {} does not exist", path.display());
                            needs_rebuild = true;
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
                    if needs_rebuild {
                        continue;
                    }
                    have_inputs = true;
                    let newest = find_newest(&arg)?;
                    if newest > newest_output {
                        debug!(
                            "input {} is newer than output {}, rebuilding",
                            newest, newest_output
                        );
                        needs_rebuild = true;
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
        if !have_inputs && !needs_rebuild {
            trace!("no inputs provided, forcing rebuild");
            needs_rebuild = true;
        }
        if newest_output == File::default() {
            info!("no outputs found");
        } else {
            info!("newest output is {}", newest_output);
        }
        Ok(Target {
            command,
            needs_rebuild,
        })
    }

    pub fn should_run_command(&self) -> bool {
        self.needs_rebuild && !self.command.is_empty()
    }

    pub fn run_command(&self) -> Result<i32, Error> {
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
    for entry in WalkDir::new(path).follow_links(true) {
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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_target() {
        let tmp_dir = tempdir().unwrap();

        let src = tmp_dir.path().join("a.c");
        let dest = tmp_dir.path().join("a.out");

        std::fs::write(&src, "int main() { return 0; }").unwrap();

        let target = Target::parse(
            [
                "target/debug/mk",
                dest.to_str().unwrap(),
                ":",
                src.to_str().unwrap(),
                "--",
                "cc",
                "-o",
                dest.to_str().unwrap(),
                src.to_str().unwrap(),
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        );
        let target = match target {
            Err(e) => panic!("{}", e),
            Ok(t) => t,
        };
        assert!(target.needs_rebuild);
        assert!(target.should_run_command());
        assert_eq!(target.run_command().unwrap(), 0);
        assert!(std::fs::metadata(&dest).is_ok());
    }
}
