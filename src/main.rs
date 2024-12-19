mod error;
mod file;
mod target;

use crate::error::Error;
use clap::Parser;
use file::File;
use std::process::exit;
use target::Target;
use tracing::{debug, level_filters::LevelFilter, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(version, long_about = USAGE)]
struct Args {
    #[arg(
        long,
        short = 'C',
        value_name = "DIR",
        default_value = ".",
        help = "Change to directory DIR before doing anything"
    )]
    chdir: String,
    #[arg(long, default_value = "error", help = "Set log level")]
    log_level: Level,
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        required = true,
        value_name = "OUTPUT ... [: INPUT ...] [-- COMMAND ...]"
    )]
    args: Vec<String>,
}

/// Compare timestamps of inputs and outputs, exiting with a non-zero status if
/// any input is newer than all outputs.
///
/// Outputs are all arguments up until a single ":" argument, and the input is
/// all subsequent arguments up to "--". If a "--" argument is present, all
/// arguments after it are executed as a command if any input is newer than all
/// outputs.
fn main() {
    let args = Args::parse();
    // Chdir before doing anything.
    if let Err(e) = std::env::set_current_dir(&args.chdir) {
        eprintln!("mk: error: chdir failed: {}: {}", args.chdir, e);
        exit(1);
    }
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::from_level(args.log_level).into())
                .with_env_var("MK_LOG")
                .from_env_lossy(),
        )
        .init();

    if args.args.is_empty() {
        println!("{}", USAGE);
        exit(0);
    }

    let target = Target::parse(args.args).unwrap_or_else(|e| {
        eprintln!("mk: error: {}", e);
        exit(1);
    });
    if !target.should_run_command() {
        debug!("Nothing to do.");
        exit(0);
    }

    match target.run_command(args.chdir.as_str()) {
        Ok(()) => exit(0),
        Err(Error::CommandFailed(code)) => exit(code),
        Err(e) => {
            eprintln!("mk: error: {}", e);
            exit(1);
        }
    }
}

const USAGE: &str = r#"
One-liner `make` rules on the command-line.

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

Use `MK_LOG=trace` or `--log-level=trace` to see debug output.
"#;

#[cfg(test)]
mod main_test;
