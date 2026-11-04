//! Asks for a number, counts up from it once a second, and stops at random.
//!
//! Run with `cargo run --example counter`. See `specs/counter.md`.

#[tokio::main]
async fn main() -> Result<(), ambler::nodes::start::Error> {
    ambler::walks::counter::run().await
}
