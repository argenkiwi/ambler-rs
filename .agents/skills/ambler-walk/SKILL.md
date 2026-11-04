---
name: ambler-walk
description: Creates a complete Ambler walk — the Rust module that wires the nodes (src/walks/<name>.rs, with its wiring test inline), the Markdown spec (specs/<name>.md) and the binary that runs it — and ensures all required nodes exist. Use this whenever a user wants to add a new program or flow to an Ambler Rust project, even if they say "new walk", "add a program", "wire up these nodes", or just describe what they want the app to do.
metadata:
  author: leandro
  version: "3.0-rust"
---

# Ambler Walk (Rust)

This skill guides you in creating a complete Ambler walk. A walk is a state-machine graph consisting of:

1. `src/walks/<name>.rs` — Rust module defining the shared `State`, the `Id` enum, a struct owning the nodes, the `Walk` impl wiring each node's outcomes to successors, and an inline `#[cfg(test)]` wiring test. It exports only `Error` and `run()`.
2. `specs/<name>.md` — Markdown specification describing the shared state and the logic/transitions of each step.
3. A runner — `src/main.rs` if this is the branch's primary program, otherwise `src/bin/<name>.rs` — that calls `ambler::walks::<name>::run()`.

---

## Step 1 — Identify the Walk

- Determine the walk name (lowercase `snake_case`, e.g. `my_walk`). The file will be `src/walks/<name>.rs`, registered in `src/walks/mod.rs`.
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

## Step 5 — Create the Walk File (`src/walks/<name>.rs`)

Example: `src/walks/counter.rs` (registered with `pub mod counter;` in `src/walks/mod.rs`).

```rust
use crate::nodes::HasCount;
use crate::nodes::count::{self, Count};
use crate::nodes::start::{self, Start};
use crate::nodes::stop::{self, Stop};
use crate::{Node, Transition, Walk};

/// What can stop the walk early. Only the start node can fail.
pub type Error = start::Error;

#[derive(Clone, Debug, Default)]
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
    /// Production wiring: every node uses its `SystemUtils`.
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
    type Error = Error;

    async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, Error> {
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

/// Runs the counter walk starting at 0.
///
/// # Errors
///
/// Returns [`Error`] if reading user input fails.
pub async fn run() -> Result<(), Error> {
    Counter::new().run(Id::Start, &mut State::default()).await
}
```

**Key rules:**
- Import `Node`, `Transition` and `Walk` from `crate`, and each node module (plus the state traits it needs) from `crate::nodes`. `Node` must be in scope to call `.run`.
- The module's public surface is exactly `pub type Error` (or `pub enum Error`) and `pub async fn run()`. `State`, `Id` and the node-owning struct stay private; the inline tests can still reach them.
- Define the `State` struct at the top of the file and implement every state trait required by the walk's nodes. Derive `Default` when the walk starts from an empty state.
- Define an `Id` enum with one variant per node. It needs `Clone, Copy`; add `Debug, PartialEq, Eq` for tests and diagnostics.
- Define a struct that owns one value of each node, and make it generic over each node's utils with the `SystemUtils` defaults. `Counter::new()` (on the defaulted type — `Counter::default()` alone can't infer the type parameters) gives production wiring; tests build the struct literal with mock utils.
- `Walk::step` matches on `Id` and, for each node, matches **exhaustively** on its `Outcome`, producing `Transition::Next(Id::…)` or `Transition::Halt`. Do not add a `_ =>` arm: a new outcome variant must break the build until it is wired. Likewise every `Id` variant needs an arm, so there is no "node not found" at runtime.
- Choose `Error` to fit the nodes: alias a node's own error if only one node can fail (`pub type Error = start::Error;`), or define a walk-level `thiserror` enum with `#[from]` conversions for several. Runners name only `ambler::walks::<name>::Error`, never a node's error. For a node with `Error = Infallible`, unwrap it with `let Ok(outcome) = node.run(state).await;` rather than `?`.
- Bounds on the `impl`: every node's `Utils` trait plus `Sync`, so the step future is `Send`.
- Terminal nodes answer with `Transition::Halt`.
- `Walk::run(start_id, &mut state)` runs the loop; do not hand-write one.
- `run()` builds the initial state. If the walk needs command-line arguments, read them there (`std::env::args()`); users pass them with `cargo run -- <arg1> <arg2>`.

**Test the wiring.** Append a `#[cfg(test)] mod tests` block to the walk file, as nodes do. Build the walk with a single scripted mock that implements every node's `Utils` for `&Script`, and assert on the final state and what was printed. Cover each branch of the graph (e.g. an invalid input that loops back to `Start`). See `src/walks/counter.rs`.

---

## Step 6 — Add the Runner and Dependencies

Each branch of this template is one program, so `src/main.rs` runs its primary walk. Point it at the new walk:

```rust
#[tokio::main]
async fn main() -> Result<(), ambler::walks::<name>::Error> {
    ambler::walks::<name>::run().await
}
```

If the branch already has a primary walk and this one is secondary, put the same code in `src/bin/<name>.rs` instead; Cargo discovers it as the binary `<name>`.

Only touch `[dependencies]` if the walk itself (not its nodes or utils) needs a new crate — add it with `cargo add <crate>`. Node and util dependencies are handled by `/ambler-node` and `/ambler-util`.

---

## Step 7 — Verify

Run the walk to confirm it behaves as specified:

```bash
cargo run                 # primary walk (src/main.rs)
cargo run --bin <name>    # secondary walk (src/bin/<name>.rs)
```

Then run tests and lints:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

---

## Checklist

Before finishing, confirm:

- [ ] `specs/<name>.md` exists and matches the `Id` variants in the walk.
- [ ] `src/walks/<name>.rs` exists with `State`, `Id`, node-owning struct, `Walk` impl, and is registered in `src/walks/mod.rs`.
- [ ] The walk module exports only `Error` and `run()`.
- [ ] `State` implements every state trait the walk's nodes require.
- [ ] Every node used in the walk has a corresponding `src/nodes/<node_name>.rs`, registered in `src/nodes/mod.rs`, with passing tests.
- [ ] Every `Outcome` variant of every node is matched explicitly, with no wildcard arm, and every `Id` has an arm.
- [ ] The walk file ends with a `#[cfg(test)] mod tests` wiring test covering each branch of the graph.
- [ ] `src/main.rs` (or `src/bin/<name>.rs`) calls `ambler::walks::<name>::run()` and returns `ambler::walks::<name>::Error`.
- [ ] All tests pass (`cargo test`) with no compiler or clippy warnings.
- [ ] The walk runs end-to-end without errors (`cargo run` or `cargo run --bin <name>`).
