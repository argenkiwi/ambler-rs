//! Example walk: asks for a number, counts up from it once a second, and stops at random.
//!
//! Run with `cargo run --example counter`; see `spec.md`.

// Nodes expose constructors the example itself does not call.
#![allow(dead_code)]

mod nodes;
mod utils;
mod walks;

#[tokio::main]
async fn main() -> Result<(), walks::counter::Error> {
    walks::counter::run().await
}
