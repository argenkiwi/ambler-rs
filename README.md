# ambler

A predictable architecture for writing programs as graphs, in Rust.

A program is a set of small **nodes** joined into a **walk**. Each node does one thing to shared state and reports what happened. The walk decides where to go next. Because the wiring is an exhaustive `match`, a missing edge is a compile error rather than a runtime surprise.

> [!NOTE]
> This crate began as a port of the TypeScript Ambler runner and has since been redesigned around Rust idioms (typed outcomes, native async traits, generic dependency injection). It no longer mirrors the TypeScript API.

## Quick start

Requires Rust 1.85 or later.

`ambler` is the native CLI of the Ambler Tooling Protocol for Rust: `cargo ambler node|walk|util|check`. Install it once, or add it to a project (see [Command line](#command-line-cargo-ambler)):

```bash
cargo install --git https://github.com/argenkiwi/ambler-rs
cargo ambler --help
```

### Example

A small counter walk ships in [`examples/counter/`](examples/counter/):

```bash
cargo run --example counter
```

It asks for a starting number, counts up once a second, and stops at random. Invalid input sends the walk back to the start. See [`spec.md`](examples/counter/spec.md) for the behaviour it implements.

## Concepts

| Concept | Rust | Role |
|---|---|---|
| State | Your own struct, plus small traits such as `HasCount` | Shared data every node reads and mutates through `&mut S`. |
| Node | `Node<S>` | One step. Returns an `Outcome` enum (one variant per outgoing edge) or an `Error`. |
| Utils | A per-node `Utils` trait | Every side effect (printing, stdin, sleeping, randomness) so tests can swap in mocks. |
| Walk | `Walk` | Owns the nodes, matches each outcome to a `Transition::Next(id)` or `Transition::Halt`, and runs the loop. |

Nodes never name their successors, so the same node can be reused in any walk.

## Writing a node

A node is a struct generic over its utils, defaulting to the real ones:

```rust
impl<S, U> Node<S> for Count<U>
where
    S: HasCount + Send,
    U: Utils + Sync,
{
    type Outcome = Outcome; // Again | Finished
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<Outcome, Infallible> {
        self.utils.print(&format!("Current count: {}", state.count()));
        self.utils.sleep(Duration::from_secs(1)).await;
        *state.count_mut() += 1;

        Ok(if self.utils.random_bool(0.5) { Outcome::Again } else { Outcome::Finished })
    }
}
```

`Count::new()` uses `SystemUtils`; `Count::with_utils(mock)` is for tests. The full node is in [`examples/counter/nodes/count.rs`](examples/counter/nodes/count.rs).

## Writing a walk

A walk owns its nodes and wires every outcome:

```rust
async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, start::Error> {
    Ok(match id {
        Id::Start => match self.start.run(state).await? {
            start::Outcome::Success => Transition::Next(Id::Count),
            start::Outcome::InvalidInput => Transition::Next(Id::Start),
        },
        // ...
    })
}
```

Then `walk.run(Id::Start, &mut state).await` steps until a node halts. The complete walk, including a test of its wiring, is in [`examples/counter/walks/counter.rs`](examples/counter/walks/counter.rs), and [`examples/counter/main.rs`](examples/counter/main.rs) runs it.

> [!TIP]
> Don't add a `_ =>` arm when matching outcomes. Leaving it out is what makes a newly added outcome fail the build until it is wired.

## Project layout

```text
src/engine.rs       Node and Walk traits, Transition (re-exported from the crate root)
src/cli.rs          `ambler::cli::run`, re-exporting the `cli` walk for xtask crates
src/walks/cli.rs    The CLI: one walk that routes each verb to its nodes (specs/cli.md);
                    src/main.rs, the `cargo-ambler` binary, runs it
src/nodes/          The CLI's nodes
src/scaffold/       Generators and linters for Ambler projects
examples/counter/   Example walk with its nodes, utils and spec
templates/          Templates for scaffolded projects
notes/              Design notes (Zettelkasten, Markdown source of truth)
.agents/skills/     Agent skills for creating nodes, utils, tests, walks and specs
```

## Command line: `cargo ambler`

This crate is the Rust implementation of the Ambler Tooling Protocol, the command line that [ambler](https://github.com/argenkiwi/ambler) hands `node`, `walk`, `util` and `check` to.

```sh
cargo ambler node ask --outcomes answered --state answer:String
cargo ambler walk chat --nodes start,ask
cargo ambler util journal
cargo ambler check            # conventions, rustfmt, clippy, tests
```

Exit codes: 0 success, 1 a check failed or the thing to create already exists, 2 invalid arguments or not an Ambler project.

### Without installing anything globally

Cargo cannot depend on a binary, so projects embed the CLI through a small `xtask` crate (the one `ambler new --target rust` creates):

```toml
# xtask/Cargo.toml
[dependencies]
ambler = { git = "https://github.com/argenkiwi/ambler-rs" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
// xtask/src/main.rs
#[tokio::main]
async fn main() -> std::process::ExitCode {
    ambler::cli::run(std::env::args().skip(1).collect()).await
}
```

```toml
# .cargo/config.toml
[alias]
ambler = "run --quiet -p xtask --"
```

`cargo ambler <verb>` then runs the project-local copy.

## Development

```bash
cargo test                                   # node and walk tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Nodes are built test-first: every outcome and error gets one `#[tokio::test]`, with all utils mocked.

The [`.agents/skills`](.agents/skills) directory holds step-by-step guides for coding agents:

| Skill | Use it to |
|---|---|
| `ambler-node` | Add a node using TDD |
| `ambler-test` | Add tests to an existing node |
| `ambler-util` | Extract a reusable helper into `src/utils/` |
| `ambler-walk` | Wire nodes into a new walk and spec |
| `ambler-spec` | Write a walk's Markdown spec |

## Projects built with ambler

- [azk](https://github.com/argenkiwi/azk): a CLI for agents to follow the Zettelkasten note-taking methodology.
- [PiDE](https://github.com/argenkiwi/pide): a containerised Pi coding agent backed by a local LLM.

## Limitations

The traits use native `async fn`, which is not `dyn`-compatible. Every walk is a concrete type, so graphs assembled at runtime from arbitrary nodes are not supported.
