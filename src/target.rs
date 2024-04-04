use std::path::PathBuf;
use std::time::SystemTime;

use crate::error::{Error, ResultExt};
use crate::File;
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
                        Err(e) if e.is_not_found() => {
                            info!("output {} does not exist", arg);
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tempfile::{tempdir, TempDir};

    use super::*;

    #[derive(Debug, Clone, Copy)]
    enum InputCase {
        None,
        Old,
        Equal,
        New,
    }

    impl InputCase {
        fn permute_time(&self, time: SystemTime) -> SystemTime {
            match self {
                InputCase::Old => time - Duration::from_secs(1),
                InputCase::Equal | InputCase::None => time,
                InputCase::New => time + Duration::from_secs(1),
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    enum OutputCase {
        None,
        Missing,
        Some,
    }

    fn all_test_cases() -> Vec<(InputCase, OutputCase)> {
        let input_cases = [
            InputCase::None,
            InputCase::Old,
            InputCase::Equal,
            InputCase::New,
        ];
        let output_cases = [OutputCase::None, OutputCase::Missing, OutputCase::Some];

        input_cases
            .into_iter()
            .flat_map(|ic| output_cases.map(|oc| (ic, oc)))
            .collect()
    }

    fn setup_test_case(tempdir: &TempDir, ic: InputCase, oc: OutputCase) -> Vec<String> {
        let now = SystemTime::now() - Duration::from_secs(60); // Back in time so avoid flaky tests
        let mut args = vec![];

        // Setup the output...
        let output_path = tempdir
            .path()
            .join("output-file")
            .to_str()
            .unwrap()
            .to_owned();

        match oc {
            OutputCase::None => (),
            OutputCase::Missing => args.push(output_path.clone()),
            OutputCase::Some => {
                std::fs::File::create_new(&output_path)
                    .unwrap()
                    .set_modified(now)
                    .unwrap();
                args.push(output_path.clone());
            }
        }

        // Setup the input...
        args.push(":".into());
        let input_path = tempdir
            .path()
            .join("input-file")
            .to_str()
            .unwrap()
            .to_owned();

        match ic {
            InputCase::None => (),
            ic @ _ => {
                std::fs::File::create_new(&input_path)
                    .unwrap()
                    .set_modified(ic.permute_time(now))
                    .unwrap();
                args.push(input_path);
            }
        }

        args.push("--".into());
        args.push("touch".into());
        args.push(output_path.into());

        args
    }

    fn check_build(args: &Vec<String>, expected_needs_build: bool) {
        let target = Target::parse(args.clone()).unwrap();
        assert_eq!(target.needs_rebuild, expected_needs_build);
        assert_eq!(target.should_run_command(), expected_needs_build);

        let expected_code = 0;
        let actual_code = target.run_command().unwrap();
        assert_eq!(actual_code, expected_code);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_needs_rebuild() {
        for case in all_test_cases() {
            let tempdir = tempdir().unwrap();
            let args = setup_test_case(&tempdir, case.0, case.1);

            let expected_needs_build = match case {
                (_, OutputCase::None | OutputCase::Missing) => true,
                (InputCase::None, _) => true,
                (InputCase::New, OutputCase::Some) => true,
                (InputCase::Old | InputCase::Equal, OutputCase::Some) => false,
            };
            check_build(&args, expected_needs_build);

            let expected_needs_rebuild = match case {
                (_, OutputCase::None) => true,
                (InputCase::None, _) => true,
                (_, OutputCase::Missing) => false, // We built, so no rebuild
                (InputCase::New, OutputCase::Some) => false, // We built, so no rebuild
                (InputCase::Old | InputCase::Equal, OutputCase::Some) => false, // No build, so no rebuild
            };
            check_build(&args, expected_needs_rebuild);
        }
    }
}
