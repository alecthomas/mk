# One-liner "make" targets

Usage: `mktg <output>... : <input>... [-- <command>...]`

Compare timestamps of inputs and outputs, exiting with a non-zero status
or executing command if any input is newer than all outputs. If an input or
output is a directory, it is recursed into.

If a command is provided it is run through "bash -c". If a single command
argument is provided it will be run as-is, otherwise all arguments will be
joined with shell quoting.

eg.

    mktg main.o : main.c -- cc -c main.c && \
        mktg main : main.o -- cc -o main main.o
