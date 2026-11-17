//! Runs the counter walk. Run with `cargo run --bin counter`.

#[tokio::main]
async fn main() -> Result<(), ambler::walks::counter::Error> {
    ambler::walks::counter::run().await
}
