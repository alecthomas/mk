# One-liner "make" targets

Usage:

```
mktg <outputs> : <inputs> [ -- <command>]
```

eg.

```
mktg main.o : main.c -- cc -c main.c && \
 mktg main : main.o -- cc -o main main.o
```
