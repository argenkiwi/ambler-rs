//! Default application binary entrypoint.
//!
//! Run with `cargo run`.

#[tokio::main]
async fn main() -> Result<(), ambler::nodes::start::Error> {
    ambler::walks::counter::run().await
}
