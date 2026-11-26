# cargo ambler

Scaffolds and checks a Rust Ambler project: a program built as walks (state machines) of small, tested nodes. It is the project's own CLI (its `xtask`), so it needs nothing installed beyond Cargo.

    cargo ambler node <name> …                  add a node: its contract, a todo body, failing tests
    cargo ambler util <name>                    add a module of side-effectful functions
    cargo ambler walk <name> --nodes … --edge … wire nodes into a walk, with its spec and runner
    cargo ambler check [--lint-only]            verify the conventions, then fmt, clippy and tests
    cargo ambler help [topic]                   this guide, or one topic

Topics: a verb (node, util, walk, check), or conventions, node-design, testing, spec.

Exit codes: 0 success, 1 a check failed or the thing to create already exists, 2 invalid arguments or an unexpected error.

Where the `ambler` command is installed, `ambler <verb>` runs the same thing. It also creates projects (`ambler new`) and implements a spec with an agent loop (`ambler pave`); neither is needed to work inside a project.

## The four ideas

- State: one struct per walk. Nodes see it only through `Has*` traits, so they stay reusable.
- Node: one step. It reads and changes the state, performs side effects through its `Utils`, and returns an `Outcome`. It never names the node that comes next.
- Utils: a node's side effects, one trait method each. `SystemUtils` is the real thing; tests pass a mock.
- Walk: owns the nodes and matches each outcome to the next node. The match is exhaustive, so an unwired outcome does not compile.

## Layout

    src/engine.rs   Node and Walk traits, Transition (identical in every project)
    src/main.rs     runs the primary walk (other walks: src/bin/<walk>.rs)
    src/walks/      one file per walk, with an inline wiring test
    src/nodes/      one file per node; mod.rs also holds the Has* state traits
    src/utils/      side-effectful functions nodes delegate to
    specs/          one plain-language spec per walk
    xtask/          runs this CLI behind the `cargo ambler` alias

## Usual order of work

1. Describe the walk in `specs/<walk>.md` (`cargo ambler help spec`).
2. For each node: `cargo ambler node`, then tests, then the body (`cargo ambler help node-design`, `cargo ambler help testing`).
3. `cargo ambler walk` to wire them; finish the wiring test (`cargo ambler help walk`).
4. `cargo ambler check`.
