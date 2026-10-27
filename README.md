# ambler

A predictable architecture for writing programs as graphs, in Rust.

A program is a set of small **nodes** joined into a **walk**. Each node does one thing to shared state and reports what happened. The walk decides where to go next. Because the wiring is an exhaustive `match`, a missing edge is a compile error rather than a runtime surprise.

> [!NOTE]
> This crate began as a port of the TypeScript Ambler runner and has since been redesigned around Rust idioms (typed outcomes, native async traits, generic dependency injection). It no longer mirrors the TypeScript API.

## Quick start

Requires Rust 1.85 or later.

```bash
cargo run --example counter
```

The example asks for a starting number, counts up once a second, and stops at random. Invalid input sends the walk back to the start. See [`specs/counter.md`](specs/counter.md) for the behaviour it implements.

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

`Count::new()` uses `SystemUtils`; `Count::with_utils(mock)` is for tests. The full node is in [`src/nodes/count.rs`](src/nodes/count.rs).

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

Then `walk.run(Id::Start, &mut state).await` steps until a node halts. The complete walk, including a test of its wiring, is in [`examples/counter.rs`](examples/counter.rs).

> [!TIP]
> Don't add a `_ =>` arm when matching outcomes. Leaving it out is what makes a newly added outcome fail the build until it is wired.

## Project layout

```text
src/lib.rs          Node and Walk traits, Transition
src/nodes/          Reusable nodes (count, start, stop) and shared state traits
examples/counter.rs A complete walk with its State, Id enum and wiring test
specs/              Plain-language spec for each walk
notes/              Design notes (Zettelkasten, Markdown source of truth)
.agents/skills/     Agent skills for creating nodes, utils, tests, walks and specs
```

## Development

```bash
cargo test                                   # unit tests and the example's wiring test
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

## Limitations

The traits use native `async fn`, which is not `dyn`-compatible. Every walk is a concrete type, so graphs assembled at runtime from arbitrary nodes are not supported.
