//! The program this branch builds: runs its primary walk.
//!
//! Run with `cargo run`. Additional walks get their own `src/bin/<name>.rs`.

#[tokio::main]
async fn main() -> Result<(), ambler::walks::counter::Error> {
    ambler::walks::counter::run().await
}
