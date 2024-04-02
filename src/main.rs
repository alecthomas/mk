mod error;
mod file;
mod target;

use error::{from_io_error, from_walkdir_error, Error};
use file::File;
use std::process::exit;
use target::Target;
use tracing::debug;
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
        println!("{}", USAGE);
        exit(0);
    }

    let target = Target::parse(args).unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(1);
    });
    if !target.should_run_command() {
        debug!("Nothing to do.");
        exit(target.needs_rebuild.into());
    }

    let code = target.run_command().unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(1);
    });
    exit(code);
}

const USAGE: &str = r#"# One-liner `make` rules on the command-line.

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
"#;
