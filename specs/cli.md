# Program Specifications

`cargo ambler <verb> [options]` is the native Ambler Tooling Protocol CLI for Rust: it scaffolds nodes, walks and utils in the project in the current directory, and checks the project against the Ambler conventions. Cargo runs `cargo-ambler ambler <verb> …` for `cargo ambler <verb> …`; the binary also runs directly as `cargo-ambler <verb> …`.

Exit codes: 0 on success, 1 if a check failed or the thing to create already exists, 2 for invalid arguments, an unknown verb or an unexpected error.

## Nodes

### Route
- Initial node.
- Drops a leading `ambler` argument, then takes the next argument as the verb and leaves the rest as the verb's arguments.
- `node` transitions to `PLAN_NODE`, `walk` to `PLAN_WALK` and `util` to `PLAN_UTIL`.
- `check` transitions to `INSPECT`, remembering whether `--lint-only` was given. Any other argument to `check` records the `check` usage as the reason and transitions to `REPORT`.
- `help`, `--help` or `-h` transitions to `HELP`.
- No verb records the usage as the reason and transitions to `REPORT`. Any other verb records `Unknown command: <verb>` followed by the usage, and transitions to `REPORT`. Both exit 2.

### Plan Node
- Usage: `node <name> (--outcomes <a,b> | --terminal) [--fallible] [--state <field>[:<Type>]]...`.
- Reads the node's name, outcomes, whether it can fail and the state it needs from the arguments, and the registered nodes and state traits from the project.
- Works out the node file (its contract, an unimplemented body and one failing test per branch) and the new contents of the nodes module: the node registered in alphabetical order, plus a new state trait for each `--state <field>:<Type>`.
- If everything fits, it transitions to `WRITE`.
- If the arguments are invalid, a reused trait is missing or this is not an Ambler project, it records why and transitions to `REPORT` (exit 2). If the node or a new trait already exists, it does the same with exit 1.

### Plan Walk
- Usage: `walk <name> --nodes <start,next,...> [--edge <node>.<Outcome>=<node|halt>]... [--no-runner]`.
- Reads the walk's name, its nodes (the first is the start) and the edges from the arguments, and each node's outcomes, error, state traits and utils from the project.
- Works out the walk file, with the given edges wired and the others left to be written, and the walks module with it registered. It adds the walk's spec if there is none, and a runner unless `--no-runner` was given: the main binary if the project has none, otherwise a binary named after the walk.
- If everything fits, it transitions to `WRITE`.
- If the arguments are invalid, a node does not exist yet, an edge names an unknown node or outcome or this is not an Ambler project, it records why and transitions to `REPORT` (exit 2). If the walk or its runner already exists, it does the same with exit 1.

### Plan Util
- Usage: `util <name>`.
- Reads the module name from the arguments and the registered utilities from the project, and works out the new module file and the utilities module with it registered in alphabetical order.
- If the name is valid and free, it transitions to `WRITE`.
- If the name is missing or invalid or this is not an Ambler project, it records why and transitions to `REPORT` (exit 2). If the module already exists, it does the same with exit 1.

### Write
- Writes the planned files, formats any generated Rust source, and prints each path and then what to do next.
- Terminates. If a file cannot be written the program fails with that error (exit 2).

### Inspect
- Usage: `check [--lint-only]`.
- Reads the project's manifest, sources and specs and checks them against the structural conventions, printing one line per violation with its file, line and rule.
- If there are no violations, it transitions to `VERIFY`, or terminates if `--lint-only` was given.
- If there are violations, it records how many and transitions to `REPORT` (exit 1).
- If the project cannot be read the program fails with that error (exit 2).

### Verify
- Runs the checks in order, printing each command before running it: the formatting check (`cargo fmt --check`), clippy with warnings denied and the tests.
- If all pass, it terminates.
- If one fails, it skips the rest, records which and transitions to `REPORT` (exit 1).
- If a check command cannot be run the program fails with that error (exit 2).

### Help
- Prints the usage of every verb and the exit codes on standard output, and terminates.

### Report
- Prints the recorded reason on standard error and terminates. The program exits with the recorded exit code.

## Shared State

The shared state consists of:
- `args`: the arguments after the verb.
- `lint_only`: whether `check` stops after the conventions.
- `plan`: the files to write, and the next steps to print afterwards.
- `message`: why the program failed, if it did.
- `exit_code`: the exit code to end with when it failed: 1 or 2.
