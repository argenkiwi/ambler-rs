---
name: ambler-test
description: Adds or updates tests for an existing Ambler node that already has an implementation. Use this when a node exists in src/nodes/<name>.rs but has no #[cfg(test)] module, when tests need extending after new branches were added, or when retrofitting TDD onto existing code. For creating a brand-new node with tests, use `/ambler-node` instead.
metadata:
  author: leandro
  version: "3.0-rust"
---

# Ambler Test (Rust)

Follow these steps to add or update tests for an existing node in `src/nodes/`.

> For brand-new nodes, use `/ambler-node` — it creates both the implementation and tests via a TDD cycle. This skill is for retrofitting tests onto nodes that already exist.

---

## 1. Read the existing node

Read `src/nodes/<name>.rs` in full. Identify:

- **State traits** (the bounds on `S`, e.g. `S: HasCount + Send` — defined in `src/nodes/mod.rs`), the **`Outcome`** variants, the **`Error`** type, and the **`Utils`** trait methods.
- **Async utils**: Do any util methods return `impl Future<Output = T> + Send`? Those are mocked with `async fn`.
- **All branches**: Every place the node returns an `Outcome` variant or an error is one branch. List them all — each becomes a test.

---

## 2. Check for existing tests

Check whether the node file already has a `#[cfg(test)] mod tests` block at the bottom.

- If it **does not exist**: write a complete new test module covering every branch.
- If it **does exist**: read it, identify which branches already have coverage, and list the gaps. Only add tests for uncovered branches — do not overwrite passing tests unless the node's interface changed.

---

## 3. Write or update the `#[cfg(test)] mod tests` block

Append the module to the bottom of `src/nodes/<name>.rs`, with one `#[tokio::test]` per uncovered branch. All utils must be mocked.

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

    struct MockUtils {
        input: io::Result<Option<&'static str>>,
    }

    impl Utils for MockUtils {
        // Override each util to be deterministic and side-effect-free.
        fn read_line(&self, _message: &str) -> io::Result<Option<String>> {
            match &self.input {
                Ok(input) => Ok(input.map(str::to_string)),
                Err(e) => Err(io::Error::new(e.kind(), "mock")),
            }
        }
        fn print(&self, _message: &str) {}
    }

    async fn run(input: &'static str, count: i32) -> (Outcome, i32) {
        let utils = MockUtils { input: Ok(Some(input)) };
        let mut state = MockState { count };
        let outcome = Start::with_utils(utils).run(&mut state).await.unwrap();
        (outcome, state.count)
    }

    #[tokio::test]
    async fn succeeds_with_input_number() {
        assert_eq!(run("42", 0).await, (Outcome::Success, 42));
    }

    #[tokio::test]
    async fn reports_invalid_input_and_keeps_count() {
        assert_eq!(run("abc", 123).await, (Outcome::InvalidInput, 123));
    }
}
```

For **async utils**, mock them with an `async fn` that returns immediately:

```rust
    impl Utils for MockUtils {
        fn print(&self, _message: &str) {}
        async fn sleep(&self, _duration: Duration) {}
        fn random_bool(&self, _p: f64) -> bool {
            self.heads
        }
    }
```

For a node that cannot fail (`Error = Infallible`), unwrap with `let Ok(outcome) = node.run(&mut state).await;`. For a failing branch, assert `matches!(result, Err(Error::Io(_)))` and that the state was left untouched.

To assert on what a util received, record it behind a `std::sync::Mutex` in a `#[derive(Default)]` mock, implement `Utils` for `&MockUtils`, and pass `Node::with_utils(&utils)` (see `src/nodes/stop.rs`).

### Test rules

- **One test per branch** — cover every `Outcome` variant and every error the node can return.
- **Mock all `Utils`** — a `MockUtils` struct with no real I/O, network, or timing.
- **Mock the state** — a `MockState` struct implementing only the traits the node requires.
- **Assert the `Outcome` variant and the state** — nodes do not know node ids.
- **Test names**: `snake_case`, `<expected_behaviour>_<condition>` (e.g., `reports_invalid_input_and_keeps_count`).
- **Always `#[tokio::test]` and `.await`** — `run` returns a future, even for synchronous nodes.

---

## 4. Run tests — must pass

```bash
cargo test nodes::<name>::
```

All tests must pass. If any fail, fix the implementation (not the tests).

---

## 5. Checklist before finishing

- [ ] `src/nodes/<name>.rs` ends with a `#[cfg(test)] mod tests` block.
- [ ] It has one `#[tokio::test]` per branch.
- [ ] `MockState` and `MockUtils` are defined — no real I/O, network, or timing.
- [ ] Every outcome and error path has a dedicated test.
- [ ] Test names follow `<expected_behaviour>_<condition>` in `snake_case`.
- [ ] All tests pass (`cargo test`).
