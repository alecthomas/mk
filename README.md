# One-liner `make` rules on the command-line.

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


Usage: mk [OPTIONS] <OUTPUT ... [: INPUT ...] [-- COMMAND ...]>...

Arguments:
  <OUTPUT ... [: INPUT ...] [-- COMMAND ...]>...


Options:
  -C, --chdir <DIR>
          Change to directory DIR before doing anything

          [default: .]

      --log-level <LOG_LEVEL>
          Set log level

          [default: error]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
