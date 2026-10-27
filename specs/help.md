# Program Specifications

This program is the `help` verb of the `azk` command line: it makes azk self-documenting, so an agent can learn how every verb behaves — its syntax, exact output shape and gotchas — from the CLI itself rather than from an external skill. With no argument it prints a general guide; with a verb, that verb's guide.

The guides are text compiled into the program, not Markdown files read at runtime, so an installed binary needs no access to its own source directory.

Unlike every other verb, `help` prints plain Markdown text to stdout, not JSON: it is documentation meant to be read. Only a bad invocation — a verb azk doesn't have — reaches `USAGE` and exits 1.

## Nodes

### Input
- Initial node of the program.
- Reads the optional verb from the first argument, case-insensitively. No verb resolves to the general guide.
- If the verb has a guide (or none was given), transitions to `PRINT`. Otherwise builds a usage line listing the verbs that do, and transitions to `USAGE`.

### Print
- Terminal node of the program.
- Prints the resolved guide as plain text.
- Terminates.

### Usage
- Terminal node, reached only from `INPUT`.
- Shared with every other walk that can be invoked badly.
- Prints the pending usage message to stderr.
- Terminates. The process then exits 1.

## Shared State

`args` is the only seeded field — the walk takes it from the command line.

- `args`: The verb's arguments.
- `usage`: The message `USAGE` should print. Its presence is what makes the run exit 1.
- `topic`: `general`, or the verb whose guide to print.
