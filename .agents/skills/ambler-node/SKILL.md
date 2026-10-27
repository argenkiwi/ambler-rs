---
name: ambler-node
description: Creates a new Ambler node in src/nodes/ — with inline tests, using test-driven development. Use this whenever the user wants to add a node, step, or state to an Ambler Rust project — even if they phrase it as "add a step", "create a handler", or describe the behaviour without using the word "node". Also use when creating a node WITH tests or following TDD for a new node. For adding tests to an existing node that already has an implementation, use `/ambler-test` instead.
metadata:
  author: leandro
  version: "3.0-rust"
---

# Ambler Node (Rust)

Follow these steps to create a new node using test-driven development. Tests are written before the implementation.

A node is a struct that implements `Node<S>`. It mutates the shared state through `&mut S` and returns an `Outcome` enum saying what happened. It never names the node that comes next — the walk wires each outcome to a successor (see `/ambler-walk`). Every node module exports the same items: `Outcome`, `Utils`, `SystemUtils` and the node struct itself (named after the module, e.g. `Count`), plus an `Error` enum if the node can fail.

```rust
pub trait Node<S> {
    type Outcome;
    type Error;
    fn run(&self, state: &mut S) -> impl Future<Output = Result<Self::Outcome, Self::Error>> + Send;
}
```

## 1. Gather requirements

Before writing any code, determine:

- **Research Existing Nodes:** Check `specs/*.md` and `src/nodes/` to see if a node with similar behaviour already exists. It is always better to reuse or adapt an existing node than to create a duplicate.
- **Node name**: The purpose of the node (e.g., `retry`, `prompt`, `validate`). The file will be named `src/nodes/<name>.rs` (`snake_case`, no `_node` suffix) and the struct `<Name>` (`PascalCase`).
- **State shape**: What does this node read or mutate? State is expressed as traits with borrowed accessors (e.g., `HasCount { fn count(&self) -> i32; fn count_mut(&mut self) -> &mut i32; }`). Shared state traits live in `src/nodes/mod.rs` — reuse an existing one where possible, otherwise add a new one there. Require only the traits the node actually touches.
- **Outcomes**: What distinct results can this node report? Each becomes one variant of the `Outcome` enum (e.g., `Success`, `InvalidInput`). A terminal node uses `type Outcome = ();`.
- **Errors**: What can go wrong that is not a normal outcome (I/O failure, for example)? Each becomes a variant of a `thiserror` `Error` enum. A node that cannot fail uses `type Error = std::convert::Infallible;`.
- **Utils**: What side-effectful operations does the node perform? List them (e.g., `print`, `read_line`, `sleep`, `random_bool`). Each becomes a method on the `Utils` trait with a production implementation on `SystemUtils`. Prefer domain-level signatures (`sleep(Duration)`, `random_bool(p)`) over raw ones (`sleep(u64)`, `random() -> f64`), and return `io::Result` from anything that can fail.
- **Behaviour**: What does the node do, and how does it choose which outcome to report?
- **Branches**: List every outcome and error the node can produce. Each becomes one test case. The types and branches decided here are the contract — if you find you need to change them during implementation, restart from this step.

---

## 2. Create a skeleton `src/nodes/<name>.rs`

Register the module in `src/nodes/mod.rs` (keep the list alphabetical), and add any new state trait there alongside the existing ones:

```rust
pub mod count;
pub mod <name>;
pub mod start;
pub mod stop;

/// A state that has a count field.
pub trait HasCount {
    fn count(&self) -> i32;
    fn count_mut(&mut self) -> &mut i32;
}
```

Then write only the type exports and a stub `run`. The body must be `todo!()` so that tests fail in step 4. (The skeleton will warn about an unused `utils` field; that goes away in step 5.)

```rust
use crate::Node;
use crate::nodes::HasCount;
use std::convert::Infallible;

/// Where [`<Name>`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Success, // from requirements
    Failure,
}

pub trait Utils {
    fn print(&self, message: &str);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemUtils;

impl Utils for SystemUtils {
    fn print(&self, message: &str) {
        println!("{message}");
    }
}

/// One-sentence description of what the node does.
#[derive(Clone, Debug, Default)]
pub struct <Name><U = SystemUtils> {
    utils: U,
}

impl <Name> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<U> <Name><U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for <Name><U>
where
    S: HasCount + Send,
    U: Utils + Sync,
{
    type Outcome = Outcome;
    type Error = Infallible;

    async fn run(&self, _state: &mut S) -> Result<Outcome, Infallible> {
        todo!()
    }
}
```

For a node whose utils are asynchronous, declare the method with a `Send` future (no boxing needed):

```rust
pub trait Utils {
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send;
}

impl Utils for SystemUtils {
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        tokio::time::sleep(duration)
    }
}
```

For a fallible node, add the error enum and use it as `type Error`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not read the starting number")]
    Io(#[from] std::io::Error),
}
```

Verify the skeleton compiles before writing tests:

```bash
cargo check
```

---

## 3. Write the tests in `src/nodes/<name>.rs`

Append an inline `#[cfg(test)] mod tests` block to the bottom of the node file. Write one `#[tokio::test]` per branch identified in step 1. All utils must be mocked — no real I/O, network, or timing.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockState {
        count: i32,
    }

    impl HasCount for MockState {
        fn count(&self) -> i32 {
            self.count
        }
        fn count_mut(&mut self) -> &mut i32 {
            &mut self.count
        }
    }

    struct MockUtils;

    impl Utils for MockUtils {
        fn print(&self, _message: &str) {}
    }

    #[tokio::test]
    async fn increments_count_and_succeeds() {
        let mut state = MockState { count: 1 };

        let Ok(outcome) = <Name>::with_utils(MockUtils).run(&mut state).await;

        assert_eq!(outcome, Outcome::Success);
        assert_eq!(state.count, 2);
    }
}
```

Use `let Ok(outcome) = ...;` when `Error = Infallible`; use `.unwrap()` or `matches!(result, Err(Error::Io(_)))` for fallible nodes.

**Async utils** are mocked with an `async fn` that returns immediately:

```rust
    impl Utils for MockUtils {
        async fn sleep(&self, _duration: Duration) {}
    }
```

To make a mock configurable per test, give it fields (e.g., `struct MockUtils { heads: bool }`) and pass them to `with_utils`. To assert on what a util was called with, record it behind a `Mutex` and implement `Utils` for `&MockUtils`, so the test keeps ownership of the mock (from `src/nodes/stop.rs`):

```rust
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockUtils {
        message: Mutex<Option<String>>,
    }

    impl Utils for &MockUtils {
        fn print(&self, message: &str) {
            *self.message.lock().unwrap() = Some(message.to_string());
        }
    }

    // In the test:
    let utils = MockUtils::default();
    Stop::with_utils(&utils).run(&mut state).await.unwrap();
    assert_eq!(utils.message.lock().unwrap().as_deref(), Some("Final count: 15"));
```

### Test rules

- **One test per branch** — one for each outcome and each error the node can produce.
- **Mock all `Utils`** — a `MockUtils` struct implementing `Utils` deterministically, with no side effects.
- **Mock the state** — a `MockState` struct implementing only the traits the node requires.
- **Assert the `Outcome` variant and the state** — never node ids; nodes do not know them.
- **Test names**: `snake_case`, `<expected_behaviour>_<condition>` (e.g., `reports_invalid_input_and_keeps_count`).
- **Always `#[tokio::test]` and `.await`** — `run` returns a future, even for synchronous nodes.

---

## 4. Run tests — expect failure

```bash
cargo test nodes::<name>::
```

Tests must fail with a `not yet implemented` panic (or assertion errors). This confirms the tests are actually exercising the code path.

- If tests fail with `not yet implemented` → correct, proceed to step 5.
- If tests fail due to a **compile error** → the skeleton types don't match the test — fix the skeleton types, not the tests.
- If tests **pass** → the skeleton was too complete; ensure the `run` body is `todo!()`.

---

## 5. Implement `src/nodes/<name>.rs`

Replace the stub body with the real logic and drop the leading underscore from `state`. The type exports (`Outcome`, `Error`, `Utils`, `SystemUtils`, the node struct) and the `Node` impl signature stay exactly as written in step 2.

```rust
async fn run(&self, state: &mut S) -> Result<Outcome, Infallible> {
    self.utils.print(&format!("Current count: {}", state.count()));
    *state.count_mut() += 1;
    Ok(Outcome::Success)
}
```

### Key rules

- **State**: Mutate it only through the trait accessors on the `&mut S` you were given (e.g., `*state.count_mut() += 1`) — never through shared or global mutable state.
- **Outcomes**: Return an `Outcome` variant — never construct or mention a node id inside the node.
- **Errors**: Propagate failures with `?` into the node's `Error` enum. Do not swallow them with `.ok()` or `unwrap_or_default()` unless "no value" is a documented outcome.
- **Utils**: All side effects go through `self.utils.*`. Complex or reusable logic (e.g., LLM calls, file I/O, extra crates) should be moved to `src/utils/` via `/ambler-util`.
- **`Send` futures**: `run` must return a `Send` future, so the node needs `U: Utils + Sync` and `S: Send`. Do not hold a `std::sync::MutexGuard` across an `.await`.
- **Termination**: A terminal node has `type Outcome = ();`; the walk answers it with `Transition::Halt`.
- **Do not modify the tests** between steps 4 and 6 — the tests define the contract; the implementation must satisfy them.

---

## 5a. Adapting Existing Nodes (Optional)

Nodes depend on state traits, not concrete fields, so reusing an existing node with a walk whose state uses different field names needs no adapter node. Implement the node's trait on the walk's `State`, mapping it to whichever field holds the value:

```rust
use ambler::nodes::HasCount;

struct State {
    total: i32,
}

impl HasCount for State {
    fn count(&self) -> i32 {
        self.total
    }
    fn count_mut(&mut self) -> &mut i32 {
        &mut self.total
    }
}
```

Only write a wrapping node if the existing node's behaviour itself must change, and then treat it as a new node (start again from step 1).

---

## 6. Run tests — must pass

```bash
cargo test nodes::<name>::
```

All tests must be green, with no compiler warnings (`cargo clippy --all-targets -- -D warnings`). Fix the implementation (never the tests) until they pass.

---

## 7. Checklist before finishing

- [ ] `src/nodes/<name>.rs` exists and `pub mod <name>;` is in `src/nodes/mod.rs`.
- [ ] Any new shared state trait is defined in `src/nodes/mod.rs`.
- [ ] `Outcome` has one variant per outgoing edge (or the node is terminal with `type Outcome = ();`).
- [ ] `<Name><U = SystemUtils>` implements `Node<S>` with `S: <Traits> + Send` and `U: Utils + Sync`, and has `new()` and `with_utils()`.
- [ ] The state trait bounds are minimal.
- [ ] `SystemUtils` provides real implementations.
- [ ] State is only changed through trait accessors on `&mut S`.
- [ ] Failures are returned through `Error`, not swallowed or panicked.
- [ ] Shared logic is in `src/utils/`.
- [ ] The `#[cfg(test)] mod tests` block has one `#[tokio::test]` per branch.
- [ ] All `Utils` in tests are mocked.
- [ ] Tests were written (step 3) before the full implementation (step 5).
- [ ] Tests failed on step 4 before passing on step 6.
- [ ] No tests were modified between steps 4 and 6.
