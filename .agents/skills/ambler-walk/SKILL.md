---
name: ambler-walk
description: Creates a complete Ambler walk — the Rust example that wires the nodes (examples/<name>.rs) and the Markdown spec (specs/<name>.md) — and ensures all required nodes exist. Use this whenever a user wants to add a new program or flow to an Ambler Rust project, even if they say "new walk", "add a program", "wire up these nodes", or just describe what they want the app to do.
metadata:
  author: leandro
  version: "3.0-rust"
---

# Ambler Walk (Rust)

This skill guides you in creating a complete Ambler walk. A walk is a state-machine graph consisting of:

1. `src/walks/<name>.rs` — Rust module defining the shared `State`, the `Id` enum, a struct owning the nodes, and the `Walk` impl wiring each node's outcomes to successors.
2. `specs/<name>.md` — Markdown specification describing the shared state and the logic/transitions of each step.
3. `tests/<name>.rs` — Integration test verifying the walk wiring end-to-end with mock utils.
4. Optional runner: `examples/<name>.rs` (or `src/main.rs` / `src/bin/<name>.rs`) — Executable entrypoint that invokes `ambler::walks::<name>::run()`.

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
use ambler::nodes::count::{self, Count};
use ambler::nodes::start::{self, Start};
use ambler::nodes::stop::{self, Stop};
use ambler::nodes::HasCount;
use ambler::{Node, Transition, Walk};

#[derive(Clone, Debug, Default)]
pub struct State {
    pub count: i32,
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
pub enum Id {
    Start,
    Count,
    Stop,
}

#[derive(Default)]
pub struct Counter<A = start::SystemUtils, B = count::SystemUtils, C = stop::SystemUtils> {
    pub start: Start<A>,
    pub count: Count<B>,
    pub stop: Stop<C>,
}

impl Counter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<A, B, C> Counter<A, B, C> {
    #[must_use]
    pub fn with_utils(start: Start<A>, count: Count<B>, stop: Stop<C>) -> Self {
        Self { start, count, stop }
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

/// Runs the walk starting from its initial state.
pub async fn run() -> Result<(), start::Error> {
    Counter::new().run(Id::Start, &mut State { count: 0 }).await
}
```

**Key rules:**
- Import `Node`, `Transition` and `Walk` from `ambler`, and each node module (plus the state traits it needs) from `ambler::nodes`. `Node` must be in scope to call `.run`.
- Define the `State` struct at the top of the file and implement every state trait required by the walk's nodes.
- Define an `Id` enum with one variant per node. It needs `Clone, Copy`; add `Debug, PartialEq, Eq` for tests and diagnostics.
- Define a struct that owns one value of each node, and make it generic over each node's utils with the `SystemUtils` defaults. `Counter::new()` (on the defaulted type) gives production wiring; `with_utils(...)` allows building it with mock utils.
- `Walk::step` matches on `Id` and, for each node, matches **exhaustively** on its `Outcome`, producing `Transition::Next(Id::…)` or `Transition::Halt`. Do not add a `_ =>` arm: a new outcome variant must break the build until it is wired. Likewise every `Id` variant needs an arm, so there is no "node not found" at runtime.
- Choose `type Error` to fit the nodes: use a node's own error if only one node can fail, or define a walk-level `thiserror` enum with `#[from]` conversions for several. For a node with `Error = Infallible`, unwrap it with `let Ok(outcome) = node.run(state).await;` rather than `?`.
- Bounds on the `impl`: every node's `Utils` trait plus `Sync`, so the step future is `Send`.
- Terminal nodes answer with `Transition::Halt`.
- `Walk::run(start_id, &mut state)` runs the loop; do not hand-write one.

---

## Step 6 — Add Tests and Runners

1. **Integration Test (`tests/<name>.rs`)**:
   Add an integration test in `tests/<name>.rs`. Build the walk with a single scripted mock that implements every node's `Utils` for `&Script`, and assert on the final state and recorded output. Cover each branch of the graph (e.g. invalid input looping back to `Start`). See `tests/counter.rs`.

2. **Runner (`examples/<name>.rs` or `src/main.rs`)**:
   If the walk is demonstrated as an example, create a thin runner in `examples/<name>.rs`:
   ```rust
   #[tokio::main]
   async fn main() -> Result<(), ambler::nodes::start::Error> {
       ambler::walks::<name>::run().await
   }
   ```
   Or if it is the main program, invoke it from `src/main.rs`.

---

## Step 7 — Verify

Run the walk to confirm it behaves as specified:

```bash
cargo run --example <name>
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
- [ ] `State` implements every state trait the walk's nodes require.
- [ ] Every node used in the walk has a corresponding `src/nodes/<node_name>.rs`, registered in `src/nodes/mod.rs`, with passing tests.
- [ ] Every `Outcome` variant of every node is matched explicitly, with no wildcard arm, and every `Id` has an arm.
- [ ] `tests/<name>.rs` contains integration test coverage testing walk wiring with mock utils.
- [ ] All tests pass (`cargo test`) with no compiler or clippy warnings.
- [ ] The walk runs end-to-end without errors (`cargo run` or `cargo run --example <name>`).
