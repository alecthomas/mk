# One-liner "make" targets on the command-line

Usage: `mk <output>... : <input>... [-- <command>...]`

Compare timestamps of inputs and outputs, exiting with a non-zero status
or executing command if any input is newer than all outputs. If an input or
output is a directory, it is recursed into.

If a command is provided it is run through "bash -c". If a single command
argument is provided it will be run as-is, otherwise all arguments will be
joined with shell quoting.

eg.

    mk main.o : main.c -- cc -c main.c && \
        mk main : main.o -- cc -o main main.o

Use MK_LOG=trace to see debug output.
