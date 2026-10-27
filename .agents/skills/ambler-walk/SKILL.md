---
name: ambler-walk
description: Creates a complete Ambler walk — the Rust example that wires the nodes (examples/<name>.rs) and the Markdown spec (specs/<name>.md) — and ensures all required nodes exist. Use this whenever a user wants to add a new program or flow to an Ambler Rust project, even if they say "new walk", "add a program", "wire up these nodes", or just describe what they want the app to do.
metadata:
  author: leandro
  version: "3.0-rust"
---

# Ambler Walk (Rust)

This skill guides you in creating a complete Ambler walk. A walk is a state-machine program consisting of two files:

1. `examples/<name>.rs` — Rust program defining the shared `State`, the `Id` enum, a struct that owns the nodes, and the `Walk` impl that wires each node's outcomes to its successors.
2. `specs/<name>.md` — Markdown specification describing the shared state and the logic/transitions of each step.

---

## Step 1 — Identify the Walk

- Determine the walk name (lowercase `snake_case`, e.g. `my_walk`). The file will be `examples/<name>.rs`, and Cargo picks it up as the example `<name>` automatically.
- Clarify the walk's purpose: what program does it implement?
- Identify the nodes (steps) required and their transitions.

---

## Step 2 — Research and Reuse

Before creating new nodes, research existing ones to promote reuse and consistency:

- **Review Existing Specs:** Browse `specs/*.md` to identify nodes that already implement the required logic (e.g., "Ollama Check", "Model Select"). This is more efficient than reading source code and helps maintain architectural alignment.
- **Check Node Implementation:** For each node identified, verify its existence in `src/nodes/<node_name>.rs` and that it is registered in `src/nodes/mod.rs`. Note which state traits (e.g., `HasCount`) each node requires — the walk's `State` must implement all of them — and each node's `Outcome` variants and `Error` type, since the walk must handle every one.

---

## Step 3 — Create the Specification File (`specs/<name>.md`)

Create the Markdown spec using the `/ambler-spec` skill. This acts as the **blueprint** for the entire walk, defining the shared state and the logic for every node.

---

## Step 4 — Ensure Nodes Exist

For each node defined in the specification:

- If it does **not** exist in `src/nodes/<node_name>.rs`, create it using the `/ambler-node` skill. This runs a full TDD pass — skeleton, tests (red), implementation, tests (green) — before completing. No separate test step is needed.
- If the node already exists but has no `#[cfg(test)] mod tests` block, use the `/ambler-test` skill to retrofit coverage.

---

## Step 5 — Create the Example File (`examples/<name>.rs`)

Example: `examples/counter.rs`.

```rust
use ambler::nodes::count::{self, Count};
use ambler::nodes::start::{self, Start};
use ambler::nodes::stop::{self, Stop};
use ambler::nodes::HasCount;
use ambler::{Node, Transition, Walk};

#[derive(Clone, Debug)]
struct State {
    count: i32,
}

impl HasCount for State {
    fn count(&self) -> i32 {
        self.count
    }
    fn count_mut(&mut self) -> &mut i32 {
        &mut self.count
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id {
    Start,
    Count,
    Stop,
}

#[derive(Default)]
struct Counter<A = start::SystemUtils, B = count::SystemUtils, C = stop::SystemUtils> {
    start: Start<A>,
    count: Count<B>,
    stop: Stop<C>,
}

impl Counter {
    fn new() -> Self {
        Self::default()
    }
}

impl<A, B, C> Walk for Counter<A, B, C>
where
    A: start::Utils + Sync,
    B: count::Utils + Sync,
    C: stop::Utils + Sync,
{
    type Id = Id;
    type State = State;
    type Error = start::Error;

    async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, start::Error> {
        Ok(match id {
            Id::Start => match self.start.run(state).await? {
                start::Outcome::Success => Transition::Next(Id::Count),
                start::Outcome::InvalidInput => Transition::Next(Id::Start),
            },
            Id::Count => {
                let Ok(outcome) = self.count.run(state).await;
                match outcome {
                    count::Outcome::Again => Transition::Next(Id::Count),
                    count::Outcome::Finished => Transition::Next(Id::Stop),
                }
            }
            Id::Stop => {
                let Ok(()) = self.stop.run(state).await;
                Transition::Halt
            }
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), start::Error> {
    Counter::new().run(Id::Start, &mut State { count: 0 }).await
}
```

**Key rules:**
- Import `Node`, `Transition` and `Walk` from `ambler`, and each node module (plus the state traits it needs) from `ambler::nodes`. `Node` must be in scope to call `.run`.
- Define the `State` struct at the top of the file and implement every state trait required by the walk's nodes.
- Define an `Id` enum with one variant per node. It needs `Clone, Copy`; add `Debug, PartialEq, Eq` for tests and diagnostics.
- Define a struct that owns one value of each node, and make it generic over each node's utils with the `SystemUtils` defaults. `Counter::new()` (on the defaulted type) gives production wiring; tests build it with mock utils.
- `Walk::step` matches on `Id` and, for each node, matches **exhaustively** on its `Outcome`, producing `Transition::Next(Id::…)` or `Transition::Halt`. Do not add a `_ =>` arm: a new outcome variant must break the build until it is wired. Likewise every `Id` variant needs an arm, so there is no "node not found" at runtime.
- Choose `type Error` to fit the nodes: use a node's own error if only one node can fail, or define a walk-level `thiserror` enum with `#[from]` conversions for several. For a node with `Error = Infallible`, unwrap it with `let Ok(outcome) = node.run(state).await;` rather than `?`.
- Bounds on the `impl`: every node's `Utils` trait plus `Sync`, so the step future is `Send`.
- Terminal nodes answer with `Transition::Halt`.
- `Walk::run(start_id, &mut state)` runs the loop; do not hand-write one.
- Run the walk with `#[tokio::main]` and return the walk's error from `main`.

**Test the wiring.** Add a `#[cfg(test)] mod tests` block to the example (the `[[example]]` entry in `Cargo.toml` sets `test = true`). Build the walk with a single scripted mock that implements every node's `Utils` for `&Script`, and assert on the final state and what was printed. Cover each branch of the graph (e.g., an invalid input that loops back to `Start`). See `examples/counter.rs`.

---

## Step 6 — Check Dependencies and Arguments

Cargo discovers `examples/<name>.rs` automatically, but a walk with tests needs a `[[example]]` entry so `cargo test` runs them:

```toml
[[example]]
name = "<name>"
test = true
```

Only touch `[dependencies]` if the walk itself (not its nodes or utils) needs a new crate — add it with `cargo add <crate>`. Node and util dependencies are handled by `/ambler-node` and `/ambler-util`.

If the walk reads command-line arguments (e.g., via `std::env::args()` when building the initial `State`), users pass them after `--`:

```bash
cargo run --example my_walk -- <arg1> <arg2>
```

---

## Step 7 — Verify

Run the walk to confirm it behaves as specified:

```bash
cargo run --example <name>
```

Then run the tests and lints:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

---

## Checklist

Before finishing, confirm:

- [ ] `specs/<name>.md` exists and matches the `Id` variants in the `.rs` file.
- [ ] `examples/<name>.rs` exists with the correct `State`, `Id`, node-owning struct, `Walk` impl and initial state.
- [ ] `State` implements every state trait the walk's nodes require.
- [ ] Every node used in the walk has a corresponding `src/nodes/<node_name>.rs`, registered in `src/nodes/mod.rs`, with passing tests (enforced by `/ambler-node`).
- [ ] Every `Outcome` variant of every node is matched explicitly, with no wildcard arm, and every `Id` has an arm.
- [ ] The example has a wiring test and a `[[example]]` entry with `test = true`.
- [ ] All tests pass (`cargo test`) with no compiler or clippy warnings.
- [ ] The walk runs end-to-end without errors (`cargo run --example <name>`).
